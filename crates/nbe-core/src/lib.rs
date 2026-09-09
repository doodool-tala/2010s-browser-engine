//! # nbe-core
//!
//! The engine substrate. Every other crate in the workspace may depend on
//! this one; this crate depends on no workspace-internal crates.
//!
//! Contents:
//! - [`error`]: the engine-wide failure taxonomy and the `invariant!` macro
//! - [`tick`]: the deterministic engine time primitive
//! - [`logging`]: the deterministic log subscriber installer

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

pub mod clock;
pub mod error;
pub mod logging;
pub mod rng;
pub mod snapshot;
pub mod tick;

#[cfg(test)]
mod tests {
    use crate::error::ModuleError;
    use crate::tick::Tick;

    #[test]
    fn tick_origin_is_zero_and_advances() {
        assert_eq!(Tick::ORIGIN.0, 0);
        assert_eq!(Tick::ORIGIN.next().0, 1);
        assert!(Tick::ORIGIN.next() > Tick::ORIGIN);
    }

    #[test]
    fn tick_display_is_t_prefixed() {
        assert_eq!(Tick(7).to_string(), "t7");
    }

    #[test]
    fn module_error_display_format() {
        let e = ModuleError::new("net", "connection reset");
        assert_eq!(e.to_string(), "[net] connection reset");
    }

    #[test]
    fn invariant_macro_panics_on_false_condition() {
        let caught = std::panic::catch_unwind(|| {
            crate::invariant!(1 == 2, "one must equal two");
        });
        assert!(caught.is_err());
    }

    #[test]
    fn invariant_macro_passes_on_true_condition() {
        crate::invariant!(1 == 1, "must not panic");
    }
}
