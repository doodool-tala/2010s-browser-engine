//! Length-prefixed frame codec. Normative reference: ADR-0005.
//!
//! A frame is: 4-byte big-endian payload length, then exactly that
//! many payload bytes. A zero-length declaration or a declaration over
//! the hard cap is a protocol violation that poisons the decoder.

use nbe_core::error::ModuleError;

/// Hard cap on a single frame's declared payload length.
pub const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;

/// Frame codec error. Malformed input from a peer: the peer is broken
/// and the channel must be torn down — never a browser crash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameError {
    /// Declared length exceeds the hard cap.
    Oversize {
        /// The declared length read off the wire.
        declared: usize,
    },
    /// Declared length is zero. Frames always carry a payload.
    ZeroLength,
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Oversize { declared } => {
                write!(
                    f,
                    "frame declared {declared} bytes, over cap {MAX_FRAME_BYTES}"
                )
            }
            FrameError::ZeroLength => {
                write!(f, "frame declared zero-length payload")
            }
        }
    }
}

impl std::error::Error for FrameError {}

impl From<FrameError> for ModuleError {
    fn from(e: FrameError) -> ModuleError {
        ModuleError::new("ipc-frame", e.to_string())
    }
}

/// Encode one payload as a frame.
#[must_use]
pub fn encode_frame(payload: &[u8]) -> Vec<u8> {
    nbe_core::invariant!(
        !payload.is_empty() && payload.len() <= MAX_FRAME_BYTES,
        "frame payload must be 1..=MAX_FRAME_BYTES bytes"
    );
    let mut out = Vec::with_capacity(payload.len() + 4);
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    out.extend_from_slice(payload);
    out
}

/// Incremental frame decoder. Feed it raw chunks; it yields complete
/// payloads. Any protocol violation poisons it permanently.
#[derive(Debug, Default)]
pub struct FrameDecoder {
    buffer: Vec<u8>,
    poisoned: bool,
}

impl FrameDecoder {
    /// A fresh, empty decoder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed one chunk of bytes. Returns Err on protocol violation;
    /// after an error the decoder is poisoned and yields nothing.
    pub fn push(&mut self, chunk: &[u8]) -> Result<(), FrameError> {
        if self.poisoned {
            return Ok(());
        }
        self.buffer.extend_from_slice(chunk);
        if self.buffer.len() >= 4 {
            let declared = u32::from_be_bytes([
                self.buffer[0],
                self.buffer[1],
                self.buffer[2],
                self.buffer[3],
            ]) as usize;
            if declared == 0 {
                self.poisoned = true;
                return Err(FrameError::ZeroLength);
            }
            if declared > MAX_FRAME_BYTES {
                self.poisoned = true;
                return Err(FrameError::Oversize { declared });
            }
        }
        Ok(())
    }

    /// Pop the next complete payload, if one is fully buffered.
    #[must_use]
    pub fn pop(&mut self) -> Option<Vec<u8>> {
        if self.poisoned || self.buffer.len() < 4 {
            return None;
        }
        let declared = u32::from_be_bytes([
            self.buffer[0],
            self.buffer[1],
            self.buffer[2],
            self.buffer[3],
        ]) as usize;
        if declared == 0 || declared > MAX_FRAME_BYTES {
            // Defensive: push() validates any header it has seen.
            return None;
        }
        let end = 4 + declared;
        if self.buffer.len() < end {
            return None;
        }
        let payload = self.buffer[4..end].to_vec();
        self.buffer.drain(..end);
        Some(payload)
    }

    /// True once a protocol violation has occurred.
    #[must_use]
    pub fn is_poisoned(&self) -> bool {
        self.poisoned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_roundtrip() {
        let payload = b"frame-payload";
        let frame = encode_frame(payload);
        assert_eq!(frame.len(), payload.len() + 4);
        let mut d = FrameDecoder::new();
        d.push(&frame).unwrap();
        assert_eq!(d.pop().as_deref(), Some(payload.as_slice()));
        assert_eq!(d.pop(), None);
    }

    #[test]
    fn frame_chunked_delivery() {
        let frame = encode_frame(b"hello-ipc");
        let mut d = FrameDecoder::new();
        for byte in &frame {
            d.push(&[*byte]).unwrap();
        }
        assert_eq!(d.pop(), Some(b"hello-ipc".to_vec()));
        assert_eq!(d.pop(), None);
    }

    #[test]
    fn frame_multiple_frames_in_one_chunk() {
        let a = encode_frame(b"first");
        let b = encode_frame(b"second");
        let mut chunk = a.clone();
        chunk.extend_from_slice(&b);
        let mut d = FrameDecoder::new();
        d.push(&chunk).unwrap();
        assert_eq!(d.pop(), Some(b"first".to_vec()));
        assert_eq!(d.pop(), Some(b"second".to_vec()));
        assert_eq!(d.pop(), None);
    }

    #[test]
    fn frame_rejects_zero_length() {
        let mut d = FrameDecoder::new();
        assert_eq!(d.push(&[0, 0, 0, 0]), Err(FrameError::ZeroLength));
        assert!(d.is_poisoned());
        assert_eq!(d.pop(), None);
    }

    #[test]
    fn frame_rejects_oversize() {
        let header = ((MAX_FRAME_BYTES + 1) as u32).to_be_bytes();
        let mut d = FrameDecoder::new();
        assert_eq!(
            d.push(&header),
            Err(FrameError::Oversize {
                declared: MAX_FRAME_BYTES + 1
            })
        );
        assert!(d.is_poisoned());
    }
}
