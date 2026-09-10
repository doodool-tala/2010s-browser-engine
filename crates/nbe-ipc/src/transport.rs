//! In-process IPC transport v1. Normative reference: ADR-0005.
//!
//! Simulates the process boundary with a bounded channel: each framed
//! message is one chunk, so the exact encode, frame, chunk, decode,
//! parse path is exercised — the same path PP-04's real pipes will
//! carry. Bounded capacity provides backpressure: senders block when
//! the buffer is full.

use std::collections::VecDeque;
use std::sync::mpsc;

use nbe_core::error::ModuleError;

use crate::frame::{encode_frame, FrameDecoder};
use crate::message::IpcEnvelope;

/// Errors crossing the transport boundary. A peer that produces these
/// is broken and must be supervised, never crashed into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    /// The peer's bytes violate the framing protocol.
    Frame(String),
    /// The peer's frame is not a valid message.
    Decode(String),
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportError::Frame(message) => write!(f, "ipc frame violation: {message}"),
            TransportError::Decode(message) => write!(f, "ipc message decode failure: {message}"),
        }
    }
}

impl std::error::Error for TransportError {}

impl From<TransportError> for ModuleError {
    fn from(e: TransportError) -> ModuleError {
        ModuleError::new("ipc-transport", e.to_string())
    }
}

/// Sending half of an in-process IPC channel.
pub struct IpcSender {
    tx: mpsc::SyncSender<Vec<u8>>,
}

impl IpcSender {
    /// Serialize, frame, and transmit one envelope. Blocks under
    /// backpressure when the channel's buffer is full.
    pub fn send(&self, envelope: &IpcEnvelope) -> Result<(), ModuleError> {
        let payload =
            serde_json::to_vec(envelope).map_err(|e| TransportError::Decode(e.to_string()))?;
        self.tx
            .send(encode_frame(&payload))
            .map_err(|e| TransportError::Frame(e.to_string()))?;
        Ok(())
    }
}

/// Receiving half of an in-process IPC channel.
pub struct IpcReceiver {
    rx: mpsc::Receiver<Vec<u8>>,
    decoder: FrameDecoder,
    ready: VecDeque<IpcEnvelope>,
}

impl IpcReceiver {
    /// Receive the next envelope.
    ///
    /// `Ok(None)` means the peer closed the channel. `Err` means the
    /// peer violated the protocol: tear down the channel and supervise
    /// the peer — never crash the browser.
    pub fn recv(&mut self) -> Result<Option<IpcEnvelope>, ModuleError> {
        loop {
            if let Some(envelope) = self.ready.pop_front() {
                return Ok(Some(envelope));
            }
            let chunk = match self.rx.recv() {
                Ok(chunk) => chunk,
                Err(_) => return Ok(None),
            };
            self.decoder
                .push(&chunk)
                .map_err(|e| TransportError::Frame(e.to_string()))?;
            while let Some(payload) = self.decoder.pop() {
                let envelope: IpcEnvelope = serde_json::from_slice(&payload)
                    .map_err(|e| TransportError::Decode(e.to_string()))?;
                self.ready.push_back(envelope);
            }
        }
    }
}

/// Create a connected in-process channel pair with a bounded buffer
/// (backpressure: senders block when full).
#[must_use]
pub fn channel(capacity: usize) -> (IpcSender, IpcReceiver) {
    nbe_core::invariant!(capacity > 0, "ipc channel capacity must be positive");
    let (tx, rx) = mpsc::sync_channel(capacity);
    (
        IpcSender { tx },
        IpcReceiver {
            rx,
            decoder: FrameDecoder::new(),
            ready: VecDeque::new(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::ProcessId;
    use crate::message::IpcMessage;
    use nbe_core::snapshot::{check_golden, GoldenOutcome};

    #[test]
    fn transport_roundtrip_preserves_envelopes() {
        let (tx, mut rx) = channel(4);
        for i in 0..3u64 {
            let env = IpcEnvelope::new(
                ProcessId::BROWSER,
                ProcessId(1),
                IpcMessage::Ping { nonce: i },
            );
            tx.send(&env).unwrap();
            assert_eq!(rx.recv().unwrap(), Some(env));
        }
    }

    #[test]
    fn transport_closed_channel_reports_none() {
        let (tx, mut rx) = channel(1);
        let env = IpcEnvelope::new(
            ProcessId(1),
            ProcessId::BROWSER,
            IpcMessage::Ready { pid: ProcessId(1) },
        );
        tx.send(&env).unwrap();
        drop(tx);
        assert_eq!(rx.recv().unwrap(), Some(env));
        assert_eq!(rx.recv().unwrap(), None);
    }

    #[test]
    fn transport_sequential_messages() {
        let (tx, mut rx) = channel(4);
        tx.send(&IpcEnvelope::new(
            ProcessId::BROWSER,
            ProcessId(1),
            IpcMessage::Initialize,
        ))
        .unwrap();
        tx.send(&IpcEnvelope::new(
            ProcessId(1),
            ProcessId::BROWSER,
            IpcMessage::Pong { nonce: 9 },
        ))
        .unwrap();
        let a = rx.recv().unwrap().unwrap();
        let b = rx.recv().unwrap().unwrap();
        assert_eq!(a.payload, IpcMessage::Initialize);
        assert_eq!(b.payload, IpcMessage::Pong { nonce: 9 });
    }

    #[test]
    fn golden_ipc_wire_sequence() {
        let envelopes = vec![
            IpcEnvelope::new(ProcessId::BROWSER, ProcessId(1), IpcMessage::Initialize),
            IpcEnvelope::new(
                ProcessId(1),
                ProcessId::BROWSER,
                IpcMessage::Ready { pid: ProcessId(1) },
            ),
            IpcEnvelope::new(
                ProcessId::BROWSER,
                ProcessId(1),
                IpcMessage::Ping { nonce: 7 },
            ),
            IpcEnvelope::new(
                ProcessId(1),
                ProcessId::BROWSER,
                IpcMessage::Pong { nonce: 7 },
            ),
            IpcEnvelope::new(ProcessId::BROWSER, ProcessId(1), IpcMessage::Shutdown),
        ];
        let mut wire: Vec<u8> = Vec::new();
        for e in &envelopes {
            let payload = serde_json::to_vec(e).unwrap();
            wire.extend_from_slice(&encode_frame(&payload));
        }
        let outcome = check_golden("pp-03-ipc-wire", &wire);
        assert!(
            matches!(outcome, GoldenOutcome::Matched | GoldenOutcome::Updated),
            "golden mismatch: {outcome:?}"
        );
    }
}
