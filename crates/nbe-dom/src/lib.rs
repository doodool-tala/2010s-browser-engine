//! # nbe-dom
//!
//! DOM core: nodes, tree mutation, live collections. Interface
//! definitions live in [`interfaces`], GENERATED from idl/dom.webidl
//! by nbe-bindings — never hand-edited. The DOM core implementation
//! begins at PP-12; the IDL surface grows with it.

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

#[rustfmt::skip]
pub mod interfaces;
