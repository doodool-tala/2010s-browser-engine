//! The renderer event loop. Normative reference: ADR-0008.
//!
//! Deterministic, single-threaded scheduling: an urgent (input
//! priority) lane drained fully each turn, a normal lane running one
//! task per turn, and a delayed lane keyed by virtual [`Tick`].
//! Virtual time advances only when the loop would otherwise idle,
//! jumping exactly to the earliest due task. A turn that ran any
//! task performs one "update the rendering" (counted; the render
//! pipeline arrives with layout and paint).

use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, VecDeque};

use nbe_core::clock::VirtualClock;
use nbe_core::tick::Tick;

/// A unit of schedulable work. Tasks own their state and receive
/// their turn's [`LoopHandle`] by value.
pub type Task = Box<dyn for<'a> FnOnce(LoopHandle<'a>)>;

/// The per-turn context handed to a running task: it can read the
/// current virtual tick and queue follow-up work. It cannot advance
/// time or re-enter the loop.
pub struct LoopHandle<'a> {
    now: Tick,
    seq: &'a mut u64,
    urgent: &'a mut VecDeque<Task>,
    normal: &'a mut VecDeque<Task>,
    delayed: &'a mut BinaryHeap<Reverse<DelayedTask>>,
}

impl LoopHandle<'_> {
    /// The current virtual tick (fixed for this turn).
    #[must_use]
    pub fn now(&self) -> Tick {
        self.now
    }

    /// Queue a normal-lane task (one runs per turn).
    pub fn spawn(&mut self, task: Task) {
        self.normal.push_back(task);
    }

    /// Queue an urgent (input-priority) task.
    pub fn spawn_urgent(&mut self, task: Task) {
        self.urgent.push_back(task);
    }

    /// Queue a task to run once virtual time is `delay` ticks past
    /// the current tick. Same-due tasks run in queueing order.
    pub fn spawn_after(&mut self, delay: u64, task: Task) {
        let due = Tick(self.now.0 + delay);
        *self.seq += 1;
        self.delayed.push(Reverse(DelayedTask {
            due,
            seq: *self.seq,
            task,
        }));
    }
}

/// What one turn of the event loop did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnOutcome {
    /// No urgent, no normal, and no delayed tasks anywhere.
    Idle,
    /// Ran work. A `Worked` turn always ran at least one task and
    /// performed exactly one "update the rendering".
    Worked {
        /// Tasks drained from the urgent lane.
        urgent: u64,
        /// Normal-lane tasks run (at most one per turn).
        normal: u64,
        /// Delayed tasks promoted into the normal lane this turn.
        promoted: u64,
        /// The tick virtual time advanced to this turn, if it moved.
        advanced_to: Option<Tick>,
    },
}

struct DelayedTask {
    due: Tick,
    seq: u64,
    task: Task,
}

impl PartialEq for DelayedTask {
    fn eq(&self, other: &Self) -> bool {
        self.due == other.due && self.seq == other.seq
    }
}

impl Eq for DelayedTask {}

impl PartialOrd for DelayedTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DelayedTask {
    fn cmp(&self, other: &Self) -> Ordering {
        self.due.cmp(&other.due).then(self.seq.cmp(&other.seq))
    }
}

/// The renderer's event loop: two ready lanes plus virtual-time
/// delayed work, running one turn at a time.
#[derive(Default)]
pub struct EventLoop {
    clock: VirtualClock,
    seq: u64,
    render_updates: u64,
    urgent: VecDeque<Task>,
    normal: VecDeque<Task>,
    delayed: BinaryHeap<Reverse<DelayedTask>>,
}

impl EventLoop {
    /// A loop at virtual tick t0 with empty lanes.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The current virtual tick.
    #[must_use]
    pub fn now(&self) -> Tick {
        self.clock.now()
    }

    /// Total "update the rendering" steps performed so far.
    #[must_use]
    pub fn render_updates(&self) -> u64 {
        self.render_updates
    }

    /// True while any lane holds work.
    #[must_use]
    pub fn has_pending(&self) -> bool {
        !self.urgent.is_empty() || !self.normal.is_empty() || !self.delayed.is_empty()
    }

    /// Queue a normal-lane task (one runs per turn).
    pub fn spawn(&mut self, task: Task) {
        self.normal.push_back(task);
    }

    /// Queue an urgent (input-priority) task (drained fully each
    /// turn, before any normal task).
    pub fn spawn_urgent(&mut self, task: Task) {
        self.urgent.push_back(task);
    }

    /// Queue a task to run once virtual time is `delay` ticks past
    /// now. Same-due tasks run in queueing order.
    pub fn spawn_after(&mut self, delay: u64, task: Task) {
        let due = Tick(self.clock.now().0 + delay);
        self.seq += 1;
        self.delayed.push(Reverse(DelayedTask {
            due,
            seq: self.seq,
            task,
        }));
    }

    /// Run one turn: promote due delayed tasks; if nothing is ready,
    /// advance virtual time to the earliest due task and promote;
    /// drain the urgent lane; run at most one normal task; if any
    /// task ran, perform one "update the rendering".
    pub fn run_turn(&mut self) -> TurnOutcome {
        let mut promoted = self.promote_due();
        let mut advanced_to = None;
        if self.urgent.is_empty() && self.normal.is_empty() {
            let next_due = self.delayed.peek().map(|Reverse(task)| task.due);
            let Some(next_due) = next_due else {
                return TurnOutcome::Idle;
            };
            // promote_due already took everything due; the heap
            // minimum is strictly in the future.
            nbe_core::invariant!(
                next_due.0 > self.clock.now().0,
                "delayed heap must not hold past-due tasks after promotion"
            );
            advanced_to = Some(next_due);
            self.clock.advance_by(next_due.0 - self.clock.now().0);
            promoted += self.promote_due();
        }
        let mut urgent_ran = 0u64;
        while let Some(task) = self.urgent.pop_front() {
            self.run_task(task);
            urgent_ran += 1;
        }
        let mut normal_ran = 0u64;
        if let Some(task) = self.normal.pop_front() {
            self.run_task(task);
            normal_ran += 1;
        }
        if urgent_ran + normal_ran > 0 {
            self.render_updates += 1;
        }
        TurnOutcome::Worked {
            urgent: urgent_ran,
            normal: normal_ran,
            promoted,
            advanced_to,
        }
    }

    /// Run turns until the loop is fully idle. Returns the number of
    /// turns that did work. A task that self-perpetuates in the
    /// urgent lane will starve this by design (supervision at a
    /// higher layer owns such pathologies).
    pub fn run_until_idle(&mut self) -> u64 {
        let mut worked_turns = 0;
        while !matches!(self.run_turn(), TurnOutcome::Idle) {
            worked_turns += 1;
        }
        worked_turns
    }

    fn promote_due(&mut self) -> u64 {
        let now = self.clock.now();
        let mut promoted = 0u64;
        while let Some(Reverse(next)) = self.delayed.peek() {
            if next.due > now {
                break;
            }
            if let Some(Reverse(DelayedTask { task, .. })) = self.delayed.pop() {
                self.normal.push_back(task);
                promoted += 1;
            }
        }
        promoted
    }

    fn run_task(&mut self, task: Task) {
        let EventLoop {
            clock,
            seq,
            urgent,
            normal,
            delayed,
            ..
        } = self;
        let handle = LoopHandle {
            now: clock.now(),
            seq,
            urgent,
            normal,
            delayed,
        };
        task(handle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    use nbe_core::snapshot::{check_golden, GoldenOutcome};

    type Trace = Rc<RefCell<Vec<String>>>;

    fn named_task(trace: &Trace, name: &'static str) -> Task {
        let trace = Rc::clone(trace);
        Box::new(move |handle| {
            trace
                .borrow_mut()
                .push(format!("{}@{}", name, handle.now()));
        })
    }

    #[test]
    fn idle_when_empty() {
        let mut events = EventLoop::new();
        assert_eq!(events.run_turn(), TurnOutcome::Idle);
        assert_eq!(events.render_updates(), 0);
        assert!(!events.has_pending());
        assert_eq!(events.now(), Tick(0));
    }

    #[test]
    fn urgent_lane_preempts_normal() {
        let trace: Trace = Rc::new(RefCell::new(Vec::new()));
        let mut events = EventLoop::new();
        events.spawn(named_task(&trace, "normal-a"));
        events.spawn_urgent(named_task(&trace, "urgent-b"));
        events.spawn(named_task(&trace, "normal-c"));
        assert_eq!(
            events.run_turn(),
            TurnOutcome::Worked {
                urgent: 1,
                normal: 1,
                promoted: 0,
                advanced_to: None,
            }
        );
        assert_eq!(
            events.run_turn(),
            TurnOutcome::Worked {
                urgent: 0,
                normal: 1,
                promoted: 0,
                advanced_to: None,
            }
        );
        assert_eq!(events.run_turn(), TurnOutcome::Idle);
        assert_eq!(
            *trace.borrow(),
            vec!["urgent-b@t0", "normal-a@t0", "normal-c@t0"]
        );
        assert_eq!(events.render_updates(), 2);
    }

    #[test]
    fn spawn_after_runs_in_virtual_time_order() {
        let trace: Trace = Rc::new(RefCell::new(Vec::new()));
        let mut events = EventLoop::new();
        events.spawn_after(10, named_task(&trace, "timer-10"));
        events.spawn_after(5, named_task(&trace, "timer-5"));
        events.spawn(named_task(&trace, "now"));
        events.run_until_idle();
        assert_eq!(
            *trace.borrow(),
            vec!["now@t0", "timer-5@t5", "timer-10@t10"]
        );
        assert_eq!(events.now(), Tick(10));
        assert_eq!(events.render_updates(), 3);
    }

    #[test]
    fn same_due_tasks_run_fifo() {
        let trace: Trace = Rc::new(RefCell::new(Vec::new()));
        let mut events = EventLoop::new();
        events.spawn_after(3, named_task(&trace, "first"));
        events.spawn_after(3, named_task(&trace, "second"));
        let turns = events.run_until_idle();
        assert_eq!(*trace.borrow(), vec!["first@t3", "second@t3"]);
        assert_eq!(turns, 2);
        assert_eq!(events.now(), Tick(3));
    }

    #[test]
    fn tasks_spawn_followups() {
        let trace: Trace = Rc::new(RefCell::new(Vec::new()));
        let child = named_task(&trace, "child");
        let trace2 = Rc::clone(&trace);
        let spawner: Task = Box::new(move |mut handle| {
            trace2
                .borrow_mut()
                .push(format!("spawner@{}", handle.now()));
            handle.spawn(child);
        });
        let mut events = EventLoop::new();
        events.spawn(spawner);
        events.run_until_idle();
        assert_eq!(*trace.borrow(), vec!["spawner@t0", "child@t0"]);
        assert_eq!(events.render_updates(), 2);
    }

    #[test]
    fn golden_eventloop_schedule() {
        let trace: Trace = Rc::new(RefCell::new(Vec::new()));
        let mut events = EventLoop::new();
        events.spawn(named_task(&trace, "boot"));
        events.spawn_after(10, named_task(&trace, "timer-10"));
        events.spawn_after(5, named_task(&trace, "timer-5"));
        events.spawn_urgent(named_task(&trace, "input"));
        events.spawn_after(5, named_task(&trace, "timer-5b"));
        let trace2 = Rc::clone(&trace);
        let spawner: Task = Box::new(move |mut handle| {
            trace2
                .borrow_mut()
                .push(format!("spawner@{}", handle.now()));
            handle.spawn_after(2, named_task(&trace2, "child"));
        });
        events.spawn(spawner);

        let turns = events.run_until_idle();
        trace.borrow_mut().push(format!("turns={turns}"));
        trace
            .borrow_mut()
            .push(format!("render_updates={}", events.render_updates()));
        trace.borrow_mut().push(format!("clock={}", events.now()));
        let bytes = trace.borrow().join("\n").into_bytes();
        let outcome = check_golden("pp-06-eventloop", &bytes);
        assert!(
            matches!(outcome, GoldenOutcome::Matched | GoldenOutcome::Updated),
            "golden mismatch: {outcome:?}"
        );
    }
}
