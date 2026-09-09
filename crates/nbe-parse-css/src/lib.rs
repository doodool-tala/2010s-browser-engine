//! # nbe-parse-css
//!
//! CSS tokenizer, stylesheet parser, cascade and computed style
//!
//! Status: STUB. Implementation begins at PP-15. Nothing here is frozen.

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
