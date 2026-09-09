//! # nbe-renderer
//!
//! Renderer process: composes the web-side crates per tab; owns the event loop
//!
//! Status: STUB. Implementation begins at PP-06. Nothing here is frozen.

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
