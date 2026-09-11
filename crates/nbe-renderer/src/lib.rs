//! # nbe-renderer
//!
//! The renderer process: composes the web-side crates per tab and
//! owns the event loop. [`eventloop`] is the deterministic scheduling
//! core — urgent and normal lanes, virtual-time delayed tasks, one
//! "update the rendering" per worked turn; the binary drives it from
//! the IPC protocol. The render pipeline itself (layout, paint,
//! display lists) arrives with later packages.

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

pub mod eventloop;
