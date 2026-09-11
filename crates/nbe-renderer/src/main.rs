//! # nbe-renderer (binary)
//!
//! The renderer process. v1 protocol placeholder (PP-04); PP-06 puts
//! its message handling on the event loop: Initialize replies via a
//! normal task, Ping replies ride the urgent (input-priority) lane,
//! Shutdown drains the loop and exits. Virtual time advances only
//! inside the loop; blocking on IPC pauses it. Speaks
//! length-prefixed JSON envelopes on stdin/stdout per ADR-0005.
//! NBE_RENDERER_FAULT=exit-early exits 1 at startup (fault injection
//! for crash-detection tests).

use std::io::Read;
use std::process::ExitCode;

use nbe_core::error::ModuleError;
use nbe_ipc::frame::FrameDecoder;
use nbe_ipc::ids::ProcessId;
use nbe_ipc::message::{IpcEnvelope, IpcMessage};
use nbe_ipc::pipe::send_envelope;
use nbe_renderer::eventloop::{EventLoop, Task};

fn main() -> ExitCode {
    nbe_core::logging::init();

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
    let mut events = EventLoop::new();
    let mut decoder = FrameDecoder::new();
    let mut chunk = [0u8; 4096];
    loop {
        let read = stdin
            .read(&mut chunk)
            .map_err(|e| ModuleError::new("renderer-stdin", e.to_string()))?;
        if read == 0 {
            // Browser closed the pipe without Shutdown: drain and
            // treat as clean.
            events.run_until_idle();
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
                    let reply_task: Task = Box::new(move |_handle| {
                        send_reply(&reply);
                    });
                    events.spawn(reply_task);
                }
                IpcMessage::Ping { nonce } => {
                    let reply =
                        IpcEnvelope::new(pid, ProcessId::BROWSER, IpcMessage::Pong { nonce });
                    let reply_task: Task = Box::new(move |_handle| {
                        send_reply(&reply);
                    });
                    events.spawn_urgent(reply_task);
                }
                IpcMessage::Shutdown => {
                    tracing::info!(pid = %pid, "renderer shutting down");
                    events.run_until_idle();
                    return Ok(());
                }
                _ => {
                    // v1 ignores messages a renderer cannot handle.
                }
            }
        }
        events.run_until_idle();
    }
}

/// Write one reply envelope to stdout. IPC failures are logged and
/// swallowed: a dead browser shows up as stdin EOF, a clean exit.
fn send_reply(reply: &IpcEnvelope) {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    if let Err(e) = send_envelope(&mut lock, reply) {
        tracing::error!(error = %e, "reply failed");
    }
}
