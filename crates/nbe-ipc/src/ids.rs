//! Process identifiers for the process boundary.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Identifies one OS process in the engine. The browser process is
/// always `ProcessId::BROWSER`; renderers are allocated from 1 upward
/// by the process supervisor (PP-04).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ProcessId(pub u32);

impl ProcessId {
    /// The browser (UI) process.
    pub const BROWSER: ProcessId = ProcessId(0);
}

impl fmt::Display for ProcessId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "p{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_is_zero_and_ids_display_prefixed() {
        assert_eq!(ProcessId::BROWSER, ProcessId(0));
        assert_eq!(ProcessId(7).to_string(), "p7");
        assert!(ProcessId(0) < ProcessId(1));
    }
}
