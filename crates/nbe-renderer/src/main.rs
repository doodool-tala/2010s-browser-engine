//! # nbe-renderer (binary)
//!
//! The renderer process. v1 (PP-04): a protocol-conforming
//! placeholder; the render pipeline and event loop arrive at PP-06.
//! Speaks length-prefixed JSON envelopes on stdin/stdout per ADR-0005:
//! Initialize → Ready{pid}, Ping{nonce} → Pong{nonce}, Shutdown →
//! exit 0. NBE_RENDERER_FAULT=exit-early exits 1 at startup (fault
//! injection for crash-detection tests).

use std::io::{Read, Write};
use std::process::ExitCode;

use nbe_core::error::ModuleError;
use nbe_ipc::frame::{encode_frame, FrameDecoder};
use nbe_ipc::ids::ProcessId;
use nbe_ipc::message::{IpcEnvelope, IpcMessage};
use tracing_subscriber::EnvFilter;

/// Renderer logging goes to stderr (ADR-0006): stdout is the IPC wire
/// and must carry only length-prefixed frames. Same deterministic
/// contract as nbe_core::logging: no wall-clock, no ANSI, NBE_LOG level.
fn init_logging() {
    let level = std::env::var("NBE_LOG").unwrap_or_else(|_| "info".to_string());
    let filter = EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false)
        .without_time()
        .with_writer(std::io::stderr)
        .init();
}

fn main() -> ExitCode {
    init_logging();

    let pid = match parse_pid(std::env::args().skip(1)) {
        Some(pid) => pid,
        None => {
            tracing::error!("usage: nbe-renderer --pid <n>");
            return ExitCode::FAILURE;
        }
    };

    if std::env::var("NBE_RENDERER_FAULT")
        .map(|v| v == "exit-early")
        .unwrap_or(false)
    {
        tracing::error!(pid = %pid, "injected fault: exiting at startup");
        return ExitCode::FAILURE;
    }

    tracing::info!(pid = %pid, "renderer online");

    if run_protocol(pid).is_err() {
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn parse_pid(args: impl Iterator<Item = String>) -> Option<ProcessId> {
    let mut args = args;
    match (args.next().as_deref(), args.next()) {
        (Some("--pid"), Some(value)) => value.parse::<u32>().ok().map(ProcessId),
        _ => None,
    }
}

fn run_protocol(pid: ProcessId) -> Result<(), ModuleError> {
    let stdin = std::io::stdin();
    let mut stdin = stdin.lock();
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    let mut decoder = FrameDecoder::new();
    let mut chunk = [0u8; 4096];
    loop {
        let read = stdin
            .read(&mut chunk)
            .map_err(|e| ModuleError::new("renderer-stdin", e.to_string()))?;
        if read == 0 {
            // Browser closed the pipe without Shutdown: treat as clean.
            return Ok(());
        }
        decoder
            .push(&chunk[..read])
            .map_err(|e| ModuleError::new("renderer-stdin", e.to_string()))?;
        while let Some(payload) = decoder.pop() {
            let envelope: IpcEnvelope = serde_json::from_slice(&payload)
                .map_err(|e| ModuleError::new("renderer-stdin", e.to_string()))?;
            match envelope.payload {
                IpcMessage::Initialize => {
                    let reply =
                        IpcEnvelope::new(pid, ProcessId::BROWSER, IpcMessage::Ready { pid });
                    write_envelope(&mut stdout, &reply)?;
                }
                IpcMessage::Ping { nonce } => {
                    let reply =
                        IpcEnvelope::new(pid, ProcessId::BROWSER, IpcMessage::Pong { nonce });
                    write_envelope(&mut stdout, &reply)?;
                }
                IpcMessage::Shutdown => {
                    tracing::info!(pid = %pid, "renderer shutting down");
                    return Ok(());
                }
                _ => {
                    // v1 ignores messages a renderer cannot handle.
                }
            }
        }
    }
}

fn write_envelope<W: Write>(sink: &mut W, envelope: &IpcEnvelope) -> Result<(), ModuleError> {
    let payload = serde_json::to_vec(envelope)
        .map_err(|e| ModuleError::new("renderer-stdout", e.to_string()))?;
    sink.write_all(&encode_frame(&payload))
        .map_err(|e| ModuleError::new("renderer-stdout", e.to_string()))?;
    sink.flush()
        .map_err(|e| ModuleError::new("renderer-stdout", e.to_string()))?;
    Ok(())
}
