//! The engine-wide failure taxonomy. Normative reference: ADR-0002.
//!
//! Three classes:
//!
//! | Class                 | Meaning                                    | Mechanism                              |
//! |-----------------------|--------------------------------------------|----------------------------------------|
//! | `RecoverableInput`    | malformed HTML/CSS/URL/JS source           | recorded as diagnostics, never `Err`   |
//! | `Module`              | infrastructure failure (net, IO, IPC)      | `Result::Err`, handled by callers      |
//! | `InvariantViolation`  | the engine's own logic is broken           | `invariant!` macro, process dies       |

/// Classification of every failure path in the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FailureClass {
    /// Malformed input. Browsers recover per spec; never abort rendering.
    RecoverableInput,
    /// Infrastructure failure, handled by the calling module.
    Module,
    /// The engine's own logic is broken. The process must die.
    InvariantViolation,
}

/// The universal module-level error for infrastructure failures.
#[derive(Debug, Clone)]
pub struct ModuleError {
    /// The subsystem that failed, e.g. `"net"`.
    pub module: &'static str,
    /// Human-readable failure description.
    pub message: String,
}

impl ModuleError {
    /// Construct a module error.
    pub fn new(module: &'static str, message: impl Into<String>) -> Self {
        Self {
            module,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ModuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.module, self.message)
    }
}

impl std::error::Error for ModuleError {}

/// The only function in the entire workspace allowed to panic.
/// Called by the [`invariant!`](crate::invariant) macro. Never call directly.
#[doc(hidden)]
#[cold]
#[allow(clippy::panic)]
pub fn invariant_violation(location: &str, message: String) -> ! {
    panic!("INVARIANT VIOLATION {location}: {message}");
}

/// Assert an engine invariant. If the condition is false the process dies
/// with a classified, greppable message. This is the ONLY sanctioned way
/// to panic anywhere in the workspace.
///
/// ```
/// # use nbe_core::invariant;
/// invariant!(2 > 1, "math is broken");
/// ```
#[macro_export]
macro_rules! invariant {
    ($cond:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {
        if !$cond {
            $crate::error::invariant_violation(
                concat!(file!(), ":", line!(), ":", column!()),
                format!($fmt $(, $arg)*),
            );
        }
    };
}
