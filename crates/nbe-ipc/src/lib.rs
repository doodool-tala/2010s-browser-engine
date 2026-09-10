//! # nbe-ipc
//!
//! The process boundary: message schemas, length-prefixed framing, the
//! v1 in-process transport, pipe transport for real child processes,
//! and the renderer supervisor. The browser process (nbe-shell) and
//! renderer processes (nbe-renderer) speak ONLY through this crate —
//! never by linking to each other.
//!
//! Wire format locked by the golden test.

#![deny(missing_docs)]
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::todo,
        clippy::unimplemented,
        clippy::dbg_macro,
        clippy::print_stdout,
        clippy::print_stderr
    )
)]

pub mod frame;
pub mod ids;
pub mod message;
pub mod pipe;
pub mod supervisor;
pub mod transport;
