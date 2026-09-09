//! # nbe-dom
//!
//! DOM core: nodes, tree mutation, live collections
//!
//! Status: STUB. Implementation begins at PP-12. Nothing here is frozen.

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
