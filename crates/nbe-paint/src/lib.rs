//! # nbe-paint
//!
//! Display-list IR, Skia backend, compositor
//!
//! Status: STUB. Implementation begins at PP-29. Nothing here is frozen.

#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::unimplemented,
        clippy::dbg_macro,
        clippy::print_stdout,
        clippy::print_stderr
    )
)]
