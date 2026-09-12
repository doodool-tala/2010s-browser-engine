//! # nbe-net
//!
//! Networking for the engine: the URL layer (parsing, serialization,
//! origin), the resource loader (priority-ordered fetching over a
//! transport abstraction, deterministic LRU cache), and the HTTP/1.1
//! transport that implements it over real TCP sockets. Cookies, MIME
//! sniffing, TLS, and disk persistence arrive in later packages.
//!
//! URL parsing is the engine's first real use of the RecoverableInput
//! error class: malformed input returns `None` — never an error,
//! never a crash (ADR-0002). The loader and transport add the Module
//! branch: network failures are `Err`.

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

pub mod http;
pub mod loader;
pub mod origin;
pub mod url;
