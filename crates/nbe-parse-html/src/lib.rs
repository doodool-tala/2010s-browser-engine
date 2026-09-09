//! # nbe-parse-html
//!
//! HTML5 tokenizer and tree construction, html5lib-conformant
//!
//! Status: STUB. Implementation begins at PP-11. Nothing here is frozen.

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
