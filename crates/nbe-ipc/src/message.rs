//! IPC message schemas. Normative reference: ADR-0005.
//!
//! The wire format is length-prefixed serde_json — our own protocol,
//! not Chrome 2011's binary IPC (see docs/DIVERGENCE-REGISTER.md).
//! Struct fields serialize in declaration order, so wire bytes are
//! deterministic and locked by the golden test.

use crate::ids::ProcessId;
use serde::{Deserialize, Serialize};

/// A message crossing the browser-renderer boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IpcMessage {
    /// Browser → renderer: begin duties.
    Initialize,
    /// Browser → renderer: shut down cleanly.
    Shutdown,
    /// Browser → renderer: supervision heartbeat probe.
    Ping {
        /// Supervision round identifier.
        nonce: u64,
    },
    /// Renderer → browser: startup complete.
    Ready {
        /// The sender's own process id.
        pid: ProcessId,
    },
    /// Renderer → browser: heartbeat reply.
    Pong {
        /// Echoes the ping's nonce.
        nonce: u64,
    },
    /// Renderer → browser: fatal condition; the renderer is dying.
    Crashed {
        /// Human-readable crash reason.
        reason: String,
    },
}

/// The unit of transmission on a channel: a payload plus routing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IpcEnvelope {
    /// Sending process.
    pub sender: ProcessId,
    /// Receiving process.
    pub recipient: ProcessId,
    /// Message payload.
    pub payload: IpcMessage,
}

impl IpcEnvelope {
    /// Construct an envelope.
    pub fn new(sender: ProcessId, recipient: ProcessId, payload: IpcMessage) -> Self {
        Self {
            sender,
            recipient,
            payload,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_json_roundtrip() {
        let e = IpcEnvelope::new(
            ProcessId::BROWSER,
            ProcessId(3),
            IpcMessage::Ping { nonce: 42 },
        );
        let json = serde_json::to_string(&e).unwrap();
        assert_eq!(
            json,
            r#"{"sender":0,"recipient":3,"payload":{"Ping":{"nonce":42}}}"#
        );
        let back: IpcEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(back, e);
    }

    #[test]
    fn all_message_variants_roundtrip() {
        let variants = vec![
            IpcMessage::Initialize,
            IpcMessage::Shutdown,
            IpcMessage::Ping { nonce: 1 },
            IpcMessage::Ready { pid: ProcessId(2) },
            IpcMessage::Pong { nonce: 1 },
            IpcMessage::Crashed {
                reason: "boom".into(),
            },
        ];
        for v in variants {
            let json = serde_json::to_string(&v).unwrap();
            let back: IpcMessage = serde_json::from_str(&json).unwrap();
            assert_eq!(v, back);
        }
    }
}
