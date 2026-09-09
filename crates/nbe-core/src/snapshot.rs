//! Golden snapshot framework. Normative reference: ADR-0004.
//!
//! Deterministic subsystems are regression-tested against byte-exact
//! golden files in test-goldens/ at the workspace root. Byte comparison
//! is the verification mechanism; FNV-1a hashes appear only in failure
//! messages, never as the check itself.
//!
//! Update mode: NBE_UPDATE_GOLDENS=1 rewrites goldens instead of
//! comparing. Golden updates land as reviewed diffs like any other
//! change.

use std::fs;
use std::path::PathBuf;

/// Outcome of a golden check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoldenOutcome {
    /// Bytes match the golden exactly.
    Matched,
    /// Golden was (re)written in update mode.
    Updated,
    /// Golden file does not exist and update mode is off.
    Missing,
    /// Bytes differ from the golden.
    Mismatch {
        /// FNV-1a hash of the golden bytes.
        golden_hash: u64,
        /// FNV-1a hash of the actual bytes.
        actual_hash: u64,
        /// Length of the golden bytes.
        golden_len: usize,
        /// Length of the actual bytes.
        actual_len: usize,
    },
    /// Filesystem error, rendered as a string.
    IoError(String),
}

/// Workspace-root golden directory.
fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-goldens")
}

/// Path of the golden file for `name`.
#[must_use]
pub fn golden_path(name: &str) -> PathBuf {
    golden_dir().join(format!("{name}.golden"))
}

/// Check `bytes` against `test-goldens/<name>.golden`.
#[must_use]
pub fn check_golden(name: &str, bytes: &[u8]) -> GoldenOutcome {
    crate::invariant!(!name.is_empty(), "golden name must be non-empty");
    for c in name.chars() {
        crate::invariant!(
            c.is_ascii_alphanumeric() || c == '-' || c == '_',
            "golden name must be [a-zA-Z0-9_-]: {name}"
        );
    }
    let path = golden_path(name);
    let update = std::env::var("NBE_UPDATE_GOLDENS")
        .map(|v| v == "1")
        .unwrap_or(false);
    let golden = match fs::read(&path) {
        Ok(g) => g,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if update {
                return write_golden(&path, bytes);
            }
            return GoldenOutcome::Missing;
        }
        Err(e) => return GoldenOutcome::IoError(e.to_string()),
    };
    if golden == bytes {
        return GoldenOutcome::Matched;
    }
    if update {
        return write_golden(&path, bytes);
    }
    GoldenOutcome::Mismatch {
        golden_hash: fnv1a64(&golden),
        actual_hash: fnv1a64(bytes),
        golden_len: golden.len(),
        actual_len: bytes.len(),
    }
}

/// Write `bytes` to `path` (update mode). Creates parent directories.
fn write_golden(path: &std::path::Path, bytes: &[u8]) -> GoldenOutcome {
    if let Some(dir) = path.parent() {
        if let Err(e) = fs::create_dir_all(dir) {
            return GoldenOutcome::IoError(e.to_string());
        }
    }
    match fs::write(path, bytes) {
        Ok(()) => GoldenOutcome::Updated,
        Err(e) => GoldenOutcome::IoError(e.to_string()),
    }
}

/// FNV-1a 64-bit hash. Reporting only; never a verification mechanism.
#[must_use]
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a64_reference_vectors() {
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a64(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn golden_path_layout() {
        let p = golden_path("abc");
        assert!(p.ends_with("test-goldens/abc.golden"));
    }

    #[test]
    fn bad_golden_names_are_rejected() {
        let caught = std::panic::catch_unwind(|| {
            let _ = check_golden("bad name!", b"x");
        });
        assert!(caught.is_err());
    }

    #[test]
    fn golden_determinism_sequence_matches() {
        let mut clock = crate::clock::VirtualClock::new();
        let mut rng = crate::rng::SeededRng::new(0x2011_2011);
        let mut bytes: Vec<u8> = Vec::new();
        for i in 0..64u64 {
            let t = clock.advance_by(i % 7);
            let r = rng.next_bounded(1_000_003);
            let f = rng.next_f64();
            bytes.extend_from_slice(format!("i={i} t={t} r={r} f={f:.6}\n").as_bytes());
        }
        let outcome = check_golden("pp-02-determinism", &bytes);
        assert!(
            matches!(outcome, GoldenOutcome::Matched | GoldenOutcome::Updated),
            "golden mismatch: {outcome:?}"
        );
    }
}
