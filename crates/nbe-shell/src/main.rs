//! # nbe-shell
//!
//! The browser process. Eventually the 2011-Chrome UI: tab strip, omnibox,
//! NTP, downloads shelf (Tier 7, PP-54+). At PP-01 it is a substrate smoke
//! test proving logging and tick wiring end to end.
//!
//! HARD RULE (enforced by scripts/check-g1.sh): this crate must never gain
//! a dependency on nbe-dom, nbe-layout, nbe-paint, nbe-js, nbe-text,
//! nbe-bindings, nbe-webapi, or nbe-renderer. Renderers are separate OS
//! processes (ADR-0001).

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

fn main() {
    nbe_core::logging::init();
    let tick = nbe_core::tick::Tick::ORIGIN.next();
    tracing::info!(tick = %tick, "nbe substrate online");
}
