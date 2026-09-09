//! # nbe-ipc
//!
//! Process-boundary message schemas, channels, and supervision protocol
//!
//! Status: STUB. Implementation begins at PP-03. Nothing here is frozen.

#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unimplemented,
        clippy::dbg_macro,
        clippy::print_stdout,
        clippy::print_stderr
    )
)]
