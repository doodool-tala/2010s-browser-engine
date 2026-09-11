//! # nbe-net
//!
//! Networking for the engine: the URL layer (parsing, serialization,
//! origin) and the resource loader (priority-ordered fetching over a
//! transport abstraction, deterministic LRU cache). The real HTTP
//! transport, cookies, MIME sniffing, and disk persistence arrive in
//! later packages.
//!
//! URL parsing is the engine's first real use of the RecoverableInput
//! error class: malformed input returns `None` — never an error,
//! never a crash (ADR-0002). The loader adds the Module branch:
//! transport failures are `Err`.

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

pub mod loader;
pub mod origin;
pub mod url;
