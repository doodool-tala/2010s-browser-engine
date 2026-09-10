//! Pipe-based IPC for real child processes. Normative reference:
//! ADR-0005. The in-process channel pair (C-009) is for tests; this
//! module is the real byte carrier: child stdin and stdout.

use std::io::{Read, Write};

use nbe_core::error::ModuleError;

use crate::frame::{encode_frame, FrameDecoder};
use crate::message::IpcEnvelope;

/// Pipe transport error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipeError {
    /// Writing to the peer failed.
    Io(String),
    /// Serializing the envelope failed.
    Encode(String),
    /// The peer produced a framing or decode violation.
    Peer(String),
}

impl std::fmt::Display for PipeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipeError::Io(message) => write!(f, "ipc pipe io: {message}"),
            PipeError::Encode(message) => write!(f, "ipc pipe encode: {message}"),
            PipeError::Peer(message) => write!(f, "ipc pipe peer violation: {message}"),
        }
    }
}

impl std::error::Error for PipeError {}

impl From<PipeError> for ModuleError {
    fn from(e: PipeError) -> ModuleError {
        ModuleError::new("ipc-pipe", e.to_string())
    }
}

/// Write one framed envelope to a byte sink (a child's stdin).
pub fn send_envelope<W: Write>(sink: &mut W, envelope: &IpcEnvelope) -> Result<(), ModuleError> {
    let payload = serde_json::to_vec(envelope).map_err(|e| PipeError::Encode(e.to_string()))?;
    sink.write_all(&encode_frame(&payload))
        .map_err(|e| PipeError::Io(e.to_string()))?;
    sink.flush().map_err(|e| PipeError::Io(e.to_string()))?;
    Ok(())
}

/// Reads framed envelopes from a byte source (a child's stdout).
pub struct FrameReader<R: Read> {
    source: R,
    decoder: FrameDecoder,
    chunk: [u8; 4096],
}

impl<R: Read> FrameReader<R> {
    /// Wrap a byte source.
    pub fn new(source: R) -> Self {
        Self {
            source,
            decoder: FrameDecoder::new(),
            chunk: [0; 4096],
        }
    }

    /// Read the next envelope. `Ok(None)` = clean EOF (the peer closed
    /// its stdout, usually because it exited). A truncated trailing
    /// frame is treated as EOF: the peer is dead either way.
    pub fn next_envelope(&mut self) -> Result<Option<IpcEnvelope>, ModuleError> {
        loop {
            if let Some(payload) = self.decoder.pop() {
                let envelope: IpcEnvelope =
                    serde_json::from_slice(&payload).map_err(|e| PipeError::Peer(e.to_string()))?;
                return Ok(Some(envelope));
            }
            let read = self
                .source
                .read(&mut self.chunk)
                .map_err(|e| PipeError::Io(e.to_string()))?;
            if read == 0 {
                return Ok(None);
            }
            self.decoder
                .push(&self.chunk[..read])
                .map_err(|e| PipeError::Peer(e.to_string()))?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::ProcessId;
    use crate::message::IpcMessage;
    use std::io::Cursor;

    #[test]
    fn frame_reader_parses_stream() {
        let a = IpcEnvelope::new(ProcessId::BROWSER, ProcessId(1), IpcMessage::Initialize);
        let b = IpcEnvelope::new(
            ProcessId(1),
            ProcessId::BROWSER,
            IpcMessage::Pong { nonce: 5 },
        );
        let mut stream = Vec::new();
        for e in [&a, &b] {
            let payload = serde_json::to_vec(e).unwrap();
            stream.extend_from_slice(&encode_frame(&payload));
        }
        let mut reader = FrameReader::new(Cursor::new(stream));
        assert_eq!(reader.next_envelope().unwrap(), Some(a));
        assert_eq!(reader.next_envelope().unwrap(), Some(b));
        assert_eq!(reader.next_envelope().unwrap(), None);
    }

    #[test]
    fn send_then_read_roundtrip() {
        let env = IpcEnvelope::new(ProcessId::BROWSER, ProcessId(2), IpcMessage::Shutdown);
        let mut sink = Vec::new();
        send_envelope(&mut sink, &env).unwrap();
        let mut reader = FrameReader::new(Cursor::new(sink));
        assert_eq!(reader.next_envelope().unwrap(), Some(env));
        assert_eq!(reader.next_envelope().unwrap(), None);
    }
}
