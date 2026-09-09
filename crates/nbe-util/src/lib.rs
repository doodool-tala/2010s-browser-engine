//! # nbe-util
//!
//! Generic helpers with no engine-domain meaning; must stay under 500 lines forever
//!
//! Status: STUB. Implementation begins at ad-hoc. Nothing here is frozen.

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
