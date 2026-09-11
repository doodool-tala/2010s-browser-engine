//! # nbe-bindings
//!
//! The IDL-to-Rust code generator and, later (PP-43), the JS-DOM
//! bridge. PP-05 delivers the machinery: [`idl`] parses a WebIDL
//! subset, [`gen`] emits Rust interface definitions. Generated output
//! is committed, reviewable, and regenerated with
//! `NBE_UPDATE_GOLDENS=1 cargo test -p nbe-bindings` — never
//! hand-edited.

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

pub mod gen;
pub mod idl;
