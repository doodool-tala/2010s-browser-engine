//! Deterministic logging. Normative reference: ADR-0003.
//!
//! All engine output flows through `tracing` to STDERR — never stdout:
//! a process's stdout may be an IPC wire (the renderer). Log lines
//! carry no wall-clock timestamps and no ANSI escapes, so identical
//! runs produce byte-identical logs. Level is controlled by the
//! `NBE_LOG` environment variable (default: `info`).

/// Install the global engine log subscriber. Call exactly once per
/// process, at startup, before any engine code runs.
pub fn init() {
    let level = std::env::var("NBE_LOG").unwrap_or_else(|_| "info".to_string());
    let filter = tracing_subscriber::EnvFilter::try_new(&level)
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false)
        .with_writer(std::io::stderr)
        .without_time()
        .init();
}
