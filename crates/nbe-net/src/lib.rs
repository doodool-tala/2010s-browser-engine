//! # nbe-net
//!
//! Networking for the engine. PP-07 delivers the URL layer: parsing
//! and serialization for absolute http/https URLs (domain, IPv4, and
//! IPv6 hosts, dot-segment removal, default-port stripping) and the
//! origin model that same-origin policy is built on. HTTP fetching,
//! cookies, MIME sniffing, and the resource cache arrive in later
//! packages.
//!
//! URL parsing is the engine's first real use of the RecoverableInput
//! error class: malformed input returns `None` — never an error,
//! never a crash (ADR-0002).

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

pub mod origin;
pub mod url;
