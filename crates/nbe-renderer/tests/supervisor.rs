//! End-to-end supervision tests: the real nbe-renderer binary, the
//! real supervisor, real pipes, real crashes. This is the
//! one-crash-one-tab architecture exercised for the first time.

use std::process::{Command, Stdio};

use nbe_ipc::ids::ProcessId;
use nbe_ipc::message::{IpcEnvelope, IpcMessage};
use nbe_ipc::supervisor::{
    should_restart, RendererEvent, RendererExitStatus, RestartPolicy, Supervisor,
};

fn renderer_command(pid: u32) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_nbe-renderer"));
    command.arg("--pid").arg(pid.to_string());
    command
}

#[test]
fn renderer_full_lifecycle() {
    let mut supervisor = Supervisor::new(renderer_command);
    let pid = supervisor.spawn_renderer().unwrap();
    assert_eq!(pid, ProcessId(1));

    supervisor
        .send(
            pid,
            &IpcEnvelope::new(ProcessId::BROWSER, pid, IpcMessage::Initialize),
        )
        .unwrap();
    let reply = supervisor.recv(pid).unwrap().unwrap();
    assert_eq!(
        reply,
        IpcEnvelope::new(pid, ProcessId::BROWSER, IpcMessage::Ready { pid })
    );

    supervisor
        .send(
            pid,
            &IpcEnvelope::new(ProcessId::BROWSER, pid, IpcMessage::Ping { nonce: 7 }),
        )
        .unwrap();
    let reply = supervisor.recv(pid).unwrap().unwrap();
    assert_eq!(
        reply,
        IpcEnvelope::new(pid, ProcessId::BROWSER, IpcMessage::Pong { nonce: 7 })
    );

    supervisor
        .send(
            pid,
            &IpcEnvelope::new(ProcessId::BROWSER, pid, IpcMessage::Shutdown),
        )
        .unwrap();
    let status = supervisor.wait_exit(pid).unwrap();
    assert_eq!(status, RendererExitStatus::Clean);
    assert!(supervisor.is_empty());
}

#[test]
fn renderer_crash_detected_and_restarted() {
    let mut supervisor = Supervisor::new(|pid: u32| {
        let mut command = renderer_command(pid);
        if pid == 1 {
            command.env("NBE_RENDERER_FAULT", "exit-early");
        }
        command
    });
    let policy = RestartPolicy::DEFAULT;

    let first = supervisor.spawn_renderer().unwrap();
    let reply = supervisor.recv(first).unwrap();
    assert_eq!(reply, None);
    let events = supervisor.poll_exits().unwrap();
    assert_eq!(
        events,
        vec![RendererEvent::Exited {
            pid: first,
            status: RendererExitStatus::Failure(1),
        }]
    );
    assert!(supervisor.is_empty());

    assert!(should_restart(policy, 0));
    let second = supervisor.spawn_renderer().unwrap();
    supervisor
        .send(
            second,
            &IpcEnvelope::new(ProcessId::BROWSER, second, IpcMessage::Initialize),
        )
        .unwrap();
    let reply = supervisor.recv(second).unwrap().unwrap();
    assert_eq!(
        reply,
        IpcEnvelope::new(
            second,
            ProcessId::BROWSER,
            IpcMessage::Ready { pid: second }
        )
    );
    supervisor
        .send(
            second,
            &IpcEnvelope::new(ProcessId::BROWSER, second, IpcMessage::Shutdown),
        )
        .unwrap();
    assert_eq!(
        supervisor.wait_exit(second).unwrap(),
        RendererExitStatus::Clean
    );
}

#[test]
fn renderer_exits_clean_on_stdin_eof() {
    let mut child = renderer_command(42)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdin.take());
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
}

#[test]
fn ping_replies_preserve_fifo_order_through_the_loop() {
    let mut supervisor = Supervisor::new(renderer_command);
    let pid = supervisor.spawn_renderer().unwrap();
    supervisor
        .send(
            pid,
            &IpcEnvelope::new(ProcessId::BROWSER, pid, IpcMessage::Initialize),
        )
        .unwrap();
    let ready = supervisor.recv(pid).unwrap().unwrap();
    assert_eq!(ready.payload, IpcMessage::Ready { pid });

    for nonce in 1..=3u64 {
        supervisor
            .send(
                pid,
                &IpcEnvelope::new(ProcessId::BROWSER, pid, IpcMessage::Ping { nonce }),
            )
            .unwrap();
    }
    for nonce in 1..=3u64 {
        let reply = supervisor.recv(pid).unwrap().unwrap();
        assert_eq!(reply.payload, IpcMessage::Pong { nonce });
    }

    supervisor
        .send(
            pid,
            &IpcEnvelope::new(ProcessId::BROWSER, pid, IpcMessage::Shutdown),
        )
        .unwrap();
    let status = supervisor.wait_exit(pid).unwrap();
    assert_eq!(status, RendererExitStatus::Clean);
}
