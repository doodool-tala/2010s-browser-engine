//! Process supervision for renderer processes. Normative reference:
//! ADR-0006. Renderers are real OS processes; a crash is a non-zero
//! exit or a closed stdout — detected, reported, and restartable. The
//! browser never crashes with them.

use std::collections::BTreeMap;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use nbe_core::error::ModuleError;

use crate::ids::ProcessId;
use crate::message::IpcEnvelope;
use crate::pipe::{send_envelope, FrameReader};

/// When a supervised renderer should be restarted after a crash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestartPolicy {
    /// Maximum restart attempts before giving up (the sad tab).
    pub max_restarts: u32,
}

impl RestartPolicy {
    /// The v1 default, Chrome-2011-style: a crashed renderer restarts.
    pub const DEFAULT: RestartPolicy = RestartPolicy { max_restarts: 3 };
}

/// Decide whether one more restart attempt is permitted.
#[must_use]
pub fn should_restart(policy: RestartPolicy, restarts_so_far: u32) -> bool {
    restarts_so_far < policy.max_restarts
}

/// How a supervised process ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RendererExitStatus {
    /// Exit code 0.
    Clean,
    /// Non-zero exit code.
    Failure(i32),
    /// Terminated by a signal or no exit code is available.
    Abnormal,
}

/// An event observed by the supervisor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RendererEvent {
    /// The process exited and has been removed from supervision.
    Exited {
        /// The process that exited.
        pid: ProcessId,
        /// How it ended.
        status: RendererExitStatus,
    },
}

struct ChildIo {
    child: Child,
    stdin: ChildStdin,
    reader: FrameReader<ChildStdout>,
    /// The child's stdout hit EOF. ADR-0006 counts a closed stdout as a
    /// crash; exit bookkeeping may lag the EOF by a moment, so exit
    /// detection reaps via `wait()` once EOF was observed.
    saw_eof: bool,
}

/// Spawns and supervises renderer processes. Renderers are addressed
/// by `ProcessId`, numbered from 1 upward; the supervisor owns their
/// stdio pipes and their exit detection.
pub struct Supervisor {
    next_pid: u32,
    command_builder: Box<dyn FnMut(u32) -> Command>,
    children: BTreeMap<ProcessId, ChildIo>,
}

impl Supervisor {
    /// Create a supervisor. `command_builder` receives the allocated
    /// pid number for each spawn (e.g. to pass `--pid N`); stdio
    /// piping is forced by the supervisor itself.
    pub fn new(command_builder: impl FnMut(u32) -> Command + 'static) -> Self {
        Self {
            next_pid: 1,
            command_builder: Box::new(command_builder),
            children: BTreeMap::new(),
        }
    }

    /// Number of live supervised processes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.children.len()
    }

    /// True when nothing is supervised.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// Spawn a renderer; returns its allocated pid.
    pub fn spawn_renderer(&mut self) -> Result<ProcessId, ModuleError> {
        let pid_number = self.next_pid;
        self.next_pid += 1;
        let pid = ProcessId(pid_number);
        let mut command = (self.command_builder)(pid_number);
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        let mut child = command
            .spawn()
            .map_err(|e| ModuleError::new("ipc-supervisor", e.to_string()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| ModuleError::new("ipc-supervisor", "stdin pipe missing"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ModuleError::new("ipc-supervisor", "stdout pipe missing"))?;
        self.children.insert(
            pid,
            ChildIo {
                child,
                stdin,
                reader: FrameReader::new(stdout),
                saw_eof: false,
            },
        );
        Ok(pid)
    }

    /// Send one envelope to a supervised process.
    pub fn send(&mut self, pid: ProcessId, envelope: &IpcEnvelope) -> Result<(), ModuleError> {
        let child_io = self
            .children
            .get_mut(&pid)
            .ok_or_else(|| ModuleError::new("ipc-supervisor", "unknown pid"))?;
        send_envelope(&mut child_io.stdin, envelope)
    }

    /// Receive the next envelope from a supervised process.
    /// `Ok(None)` = the process closed its stdout (usually: it died).
    pub fn recv(&mut self, pid: ProcessId) -> Result<Option<IpcEnvelope>, ModuleError> {
        let child_io = self
            .children
            .get_mut(&pid)
            .ok_or_else(|| ModuleError::new("ipc-supervisor", "unknown pid"))?;
        child_io.reader.next_envelope().inspect(|got| {
            if got.is_none() {
                child_io.saw_eof = true;
            }
        })
    }

    /// Non-blocking: detect and reap every process that has exited.
    pub fn poll_exits(&mut self) -> Result<Vec<RendererEvent>, ModuleError> {
        let mut events = Vec::new();
        let mut exited = Vec::new();
        for (pid, child_io) in &mut self.children {
            match child_io.child.try_wait() {
                Ok(Some(status)) => {
                    exited.push(*pid);
                    events.push(RendererEvent::Exited {
                        pid: *pid,
                        status: exit_status(&status),
                    });
                }
                Ok(None) if child_io.saw_eof => {
                    // stdout EOF: the renderer has stopped running code
                    // (it closed the pipe while dying), so reap now; the
                    // wait is bounded and recovers the real exit status.
                    match child_io.child.wait() {
                        Ok(status) => {
                            exited.push(*pid);
                            events.push(RendererEvent::Exited {
                                pid: *pid,
                                status: exit_status(&status),
                            });
                        }
                        Err(e) => {
                            return Err(ModuleError::new("ipc-supervisor", e.to_string()));
                        }
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    return Err(ModuleError::new("ipc-supervisor", e.to_string()));
                }
            }
        }
        for pid in exited {
            self.children.remove(&pid);
        }
        Ok(events)
    }

    /// Blocking: wait for one process to exit; returns how it ended.
    pub fn wait_exit(&mut self, pid: ProcessId) -> Result<RendererExitStatus, ModuleError> {
        let child_io = self
            .children
            .get_mut(&pid)
            .ok_or_else(|| ModuleError::new("ipc-supervisor", "unknown pid"))?;
        match child_io.child.wait() {
            Ok(status) => {
                let status = exit_status(&status);
                self.children.remove(&pid);
                Ok(status)
            }
            Err(e) => Err(ModuleError::new("ipc-supervisor", e.to_string())),
        }
    }
}

fn exit_status(status: &std::process::ExitStatus) -> RendererExitStatus {
    match status.code() {
        Some(0) => RendererExitStatus::Clean,
        Some(code) => RendererExitStatus::Failure(code),
        None => RendererExitStatus::Abnormal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restart_policy_bounds() {
        let policy = RestartPolicy { max_restarts: 2 };
        assert!(should_restart(policy, 0));
        assert!(should_restart(policy, 1));
        assert!(!should_restart(policy, 2));
        assert!(!should_restart(policy, 9));
    }
}
