use std::io;
use std::process::Child;
use std::thread;
use std::time::{Duration, Instant};

use rustix::process::{Pid, Signal};

use super::{
    NativeProcessError, NativeProcessPrimaryError, NativeProcessSecondaryEvidence,
    stream::ProcessStreams,
};

const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(5);

trait CleanupOperations {
    type Child;

    fn signal_process_group(&mut self, process_group: Pid, signal: Signal) -> Result<(), String>;
    fn signal_process_leader(&mut self, child: &Self::Child, signal: Signal) -> Result<(), String>;
    fn force_kill_process_leader(&mut self, child: &mut Self::Child) -> Result<(), String>;
    fn probe_process_leader_reaped(&mut self, child: &mut Self::Child) -> Result<bool, String>;
    fn process_group_exists(&mut self, process_group: Pid) -> Result<bool, String>;
}

struct SystemCleanupOperations;

impl CleanupOperations for SystemCleanupOperations {
    type Child = Child;

    fn signal_process_group(&mut self, process_group: Pid, signal: Signal) -> Result<(), String> {
        send_process_group_signal(process_group, signal)
    }

    fn signal_process_leader(&mut self, child: &Self::Child, signal: Signal) -> Result<(), String> {
        send_process_signal(Pid::from_child(child), signal)
    }

    fn force_kill_process_leader(&mut self, child: &mut Self::Child) -> Result<(), String> {
        match child.kill() {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::InvalidInput => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }

    fn probe_process_leader_reaped(&mut self, child: &mut Self::Child) -> Result<bool, String> {
        child
            .try_wait()
            .map(|status| status.is_some())
            .map_err(|error| error.to_string())
    }

    fn process_group_exists(&mut self, process_group: Pid) -> Result<bool, String> {
        probe_process_group_exists(process_group)
    }
}

struct CleanupTarget<'a, O>
where
    O: CleanupOperations,
{
    child: &'a mut O::Child,
    process_group: Pid,
    operations: &'a mut O,
}

impl<O> CleanupTarget<'_, O>
where
    O: CleanupOperations,
{
    fn signal_process_group(&mut self, signal: Signal) -> Result<(), String> {
        self.operations
            .signal_process_group(self.process_group, signal)
    }

    fn signal_process_leader(&mut self, signal: Signal) -> Result<(), String> {
        self.operations.signal_process_leader(self.child, signal)
    }

    fn force_kill_process_leader(&mut self) -> Result<(), String> {
        self.operations.force_kill_process_leader(self.child)
    }

    fn probe_process_leader_reaped(&mut self) -> Result<bool, String> {
        self.operations.probe_process_leader_reaped(self.child)
    }

    fn process_group_exists(&mut self) -> Result<bool, String> {
        self.operations.process_group_exists(self.process_group)
    }
}

pub(super) struct CleanupOutcome {
    pub(super) evidence: NativeProcessSecondaryEvidence,
    pub(super) boundary: Option<NativeProcessError>,
}

pub(super) fn terminate_and_verify<F>(
    child: &mut Child,
    process_group: Pid,
    leader_reaped: bool,
    streams: Option<&mut ProcessStreams>,
    grace: Duration,
    monitor: F,
) -> CleanupOutcome
where
    F: FnMut(Option<&ProcessStreams>) -> Option<NativeProcessError>,
{
    terminate_and_verify_with(
        child,
        process_group,
        leader_reaped,
        streams,
        grace,
        &mut SystemCleanupOperations,
        monitor,
    )
}

fn terminate_and_verify_with<O, F>(
    child: &mut O::Child,
    process_group: Pid,
    leader_reaped: bool,
    streams: Option<&mut ProcessStreams>,
    grace: Duration,
    operations: &mut O,
    monitor: F,
) -> CleanupOutcome
where
    O: CleanupOperations,
    F: FnMut(Option<&ProcessStreams>) -> Option<NativeProcessError>,
{
    let mut evidence = NativeProcessSecondaryEvidence::default();
    let mut monitor = CleanupMonitor::new(monitor);
    let mut streams = streams;
    let mut target = CleanupTarget {
        child,
        process_group,
        operations,
    };
    monitor.capture(streams.as_deref());
    let initial = observe(
        &mut target,
        leader_reaped,
        streams.as_deref_mut(),
        &mut evidence,
    );
    monitor.capture(streams.as_deref());
    if initial.complete() {
        return monitor.into_outcome(evidence);
    }

    if initial.group_present {
        record_signal(
            target.signal_process_group(Signal::TERM),
            "process-group termination failed",
            &mut evidence,
        );
    }
    if !initial.leader_reaped {
        record_signal(
            target.signal_process_leader(Signal::TERM),
            "process-leader termination failed",
            &mut evidence,
        );
    }

    let graceful = wait_for_state(
        &mut target,
        initial.leader_reaped,
        streams.as_deref_mut(),
        &mut evidence,
        &mut monitor,
        WaitPolicy::graceful(grace),
    );
    if graceful.complete() {
        return monitor.into_outcome(evidence);
    }

    if graceful.group_present {
        record_signal(
            target.signal_process_group(Signal::KILL),
            "process-group force kill failed",
            &mut evidence,
        );
    }
    if !graceful.leader_reaped {
        record_signal(
            target.force_kill_process_leader(),
            "process-leader force kill failed",
            &mut evidence,
        );
    }

    let forced = wait_for_state(
        &mut target,
        graceful.leader_reaped,
        streams,
        &mut evidence,
        &mut monitor,
        WaitPolicy::forced(grace),
    );
    if forced.group_present {
        push_unique(
            &mut evidence,
            "process group remained present after force kill".to_string(),
        );
    }
    if !forced.leader_reaped {
        push_unique(
            &mut evidence,
            "process leader was not reaped after force kill".to_string(),
        );
    }
    monitor.into_outcome(evidence)
}

#[derive(Clone, Copy)]
struct CleanupState {
    leader_reaped: bool,
    group_present: bool,
    streams_closed: bool,
}

impl CleanupState {
    const fn complete(self) -> bool {
        self.leader_reaped && !self.group_present && self.streams_closed
    }
}

fn wait_for_state<O, F>(
    target: &mut CleanupTarget<'_, O>,
    leader_reaped: bool,
    mut streams: Option<&mut ProcessStreams>,
    evidence: &mut NativeProcessSecondaryEvidence,
    monitor: &mut CleanupMonitor<F>,
    policy: WaitPolicy,
) -> CleanupState
where
    O: CleanupOperations,
    F: FnMut(Option<&ProcessStreams>) -> Option<NativeProcessError>,
{
    let started = Instant::now();
    let mut leader_reaped = leader_reaped;
    loop {
        monitor.capture(streams.as_deref());
        let state = observe(target, leader_reaped, streams.as_deref_mut(), evidence);
        leader_reaped = state.leader_reaped;
        monitor.capture(streams.as_deref());
        if state.complete()
            || (policy.stop_on_deadline && monitor.deadline_observed())
            || started.elapsed() >= policy.maximum_wait
        {
            return state;
        }
        thread::sleep(
            PROCESS_POLL_INTERVAL.min(policy.maximum_wait.saturating_sub(started.elapsed())),
        );
    }
}

struct CleanupMonitor<F>
where
    F: FnMut(Option<&ProcessStreams>) -> Option<NativeProcessError>,
{
    check: F,
    boundary: Option<NativeProcessError>,
    secondary_boundaries: NativeProcessSecondaryEvidence,
    deadline_observed: bool,
}

impl<F> CleanupMonitor<F>
where
    F: FnMut(Option<&ProcessStreams>) -> Option<NativeProcessError>,
{
    const fn new(check: F) -> Self {
        Self {
            check,
            boundary: None,
            secondary_boundaries: NativeProcessSecondaryEvidence::empty(),
            deadline_observed: false,
        }
    }

    fn capture(&mut self, streams: Option<&ProcessStreams>) {
        let observed = (self.check)(streams);
        if observed.as_ref().is_some_and(|error| {
            matches!(
                error.primary(),
                NativeProcessPrimaryError::DeadlineExceeded(_)
            )
        }) {
            self.deadline_observed = true;
        }
        let Some(observed) = observed else {
            return;
        };
        if let Some(primary) = &self.boundary {
            if primary != &observed {
                self.secondary_boundaries.push(observed.to_string());
            }
        } else {
            self.boundary = Some(observed);
        }
    }

    const fn deadline_observed(&self) -> bool {
        self.deadline_observed
    }

    fn into_outcome(self, mut evidence: NativeProcessSecondaryEvidence) -> CleanupOutcome {
        evidence.extend(self.secondary_boundaries);
        CleanupOutcome {
            evidence,
            boundary: self.boundary,
        }
    }
}

#[derive(Clone, Copy)]
struct WaitPolicy {
    maximum_wait: Duration,
    stop_on_deadline: bool,
}

impl WaitPolicy {
    const fn graceful(maximum_wait: Duration) -> Self {
        Self {
            maximum_wait,
            stop_on_deadline: true,
        }
    }

    const fn forced(maximum_wait: Duration) -> Self {
        Self {
            maximum_wait,
            stop_on_deadline: false,
        }
    }
}

fn observe<O>(
    target: &mut CleanupTarget<'_, O>,
    leader_reaped: bool,
    streams: Option<&mut ProcessStreams>,
    evidence: &mut NativeProcessSecondaryEvidence,
) -> CleanupState
where
    O: CleanupOperations,
{
    let streams_closed = streams.is_none_or(|streams| {
        streams.drain();
        streams.all_closed()
    });
    let leader_reaped = if leader_reaped {
        true
    } else {
        match target.probe_process_leader_reaped() {
            Ok(reaped) => reaped,
            Err(error) => {
                push_unique(
                    evidence,
                    format!("process-leader reap probe failed: {error}"),
                );
                false
            }
        }
    };
    let group_present = match target.process_group_exists() {
        Ok(present) => present,
        Err(error) => {
            push_unique(evidence, error);
            true
        }
    };
    CleanupState {
        leader_reaped,
        group_present,
        streams_closed,
    }
}

fn probe_process_group_exists(process_group: Pid) -> Result<bool, String> {
    match rustix::process::test_kill_process_group(process_group) {
        Ok(()) | Err(rustix::io::Errno::PERM) => Ok(true),
        Err(rustix::io::Errno::SRCH) => Ok(false),
        Err(error) => Err(format!("process-group existence probe failed: {error}")),
    }
}

fn send_process_group_signal(process_group: Pid, signal: Signal) -> Result<(), String> {
    match rustix::process::kill_process_group(process_group, signal) {
        Ok(()) | Err(rustix::io::Errno::SRCH) => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

fn send_process_signal(process: Pid, signal: Signal) -> Result<(), String> {
    match rustix::process::kill_process(process, signal) {
        Ok(()) | Err(rustix::io::Errno::SRCH) => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

fn record_signal(
    result: Result<(), String>,
    label: &str,
    evidence: &mut NativeProcessSecondaryEvidence,
) {
    if let Err(error) = result {
        push_unique(evidence, format!("{label}: {error}"));
    }
}

fn push_unique(evidence: &mut NativeProcessSecondaryEvidence, message: String) {
    evidence.push(message);
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::error::Error;
    use std::io::{PipeWriter, Write};
    use std::os::fd::OwnedFd;

    use crate::process::{NativeProcessRequest, stream::configure_nonblocking};

    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum CleanupCall {
        ProbeLeaderReaped,
        ProbeGroupExists,
        GroupTerm,
        LeaderTerm,
        GroupKill,
        LeaderKill,
    }

    struct FakeChild;

    struct FakeCleanupOperations {
        calls: Vec<CleanupCall>,
        leader_reaped: VecDeque<Result<bool, String>>,
        group_exists: VecDeque<Result<bool, String>>,
        group_term: VecDeque<Result<(), String>>,
        leader_term: VecDeque<Result<(), String>>,
        group_kill: VecDeque<Result<(), String>>,
        leader_kill: VecDeque<Result<(), String>>,
    }

    impl FakeCleanupOperations {
        fn exited_group(observations: usize) -> Self {
            Self {
                calls: Vec::new(),
                leader_reaped: VecDeque::from([Ok(true)]),
                group_exists: std::iter::repeat_n(Ok(false), observations).collect(),
                group_term: VecDeque::new(),
                leader_term: VecDeque::new(),
                group_kill: VecDeque::new(),
                leader_kill: VecDeque::new(),
            }
        }

        fn successful_escalation() -> Self {
            Self {
                calls: Vec::new(),
                leader_reaped: VecDeque::from([Ok(false), Ok(false), Ok(true)]),
                group_exists: VecDeque::from([Ok(true), Ok(true), Ok(false)]),
                group_term: VecDeque::from([Ok(())]),
                leader_term: VecDeque::from([Ok(())]),
                group_kill: VecDeque::from([Ok(())]),
                leader_kill: VecDeque::from([Ok(())]),
            }
        }

        fn failing_survivor() -> Self {
            Self {
                calls: Vec::new(),
                leader_reaped: VecDeque::from([
                    Err("injected reap failure".to_string()),
                    Ok(false),
                    Ok(false),
                ]),
                group_exists: VecDeque::from([
                    Err("process-group existence probe failed: injected".to_string()),
                    Ok(true),
                    Ok(true),
                ]),
                group_term: VecDeque::from([Err("injected TERM failure".to_string())]),
                leader_term: VecDeque::from([Err("injected leader TERM failure".to_string())]),
                group_kill: VecDeque::from([Err("injected KILL failure".to_string())]),
                leader_kill: VecDeque::from([Err("injected leader kill failure".to_string())]),
            }
        }
    }

    impl CleanupOperations for FakeCleanupOperations {
        type Child = FakeChild;

        fn signal_process_group(
            &mut self,
            _process_group: Pid,
            signal: Signal,
        ) -> Result<(), String> {
            match signal {
                Signal::TERM => {
                    self.calls.push(CleanupCall::GroupTerm);
                    next_result(&mut self.group_term, "group TERM")
                }
                Signal::KILL => {
                    self.calls.push(CleanupCall::GroupKill);
                    next_result(&mut self.group_kill, "group KILL")
                }
                other => Err(format!("unexpected injected group signal: {other:?}")),
            }
        }

        fn signal_process_leader(
            &mut self,
            _child: &Self::Child,
            signal: Signal,
        ) -> Result<(), String> {
            if signal != Signal::TERM {
                return Err(format!("unexpected injected leader signal: {signal:?}"));
            }
            self.calls.push(CleanupCall::LeaderTerm);
            next_result(&mut self.leader_term, "leader TERM")
        }

        fn force_kill_process_leader(&mut self, _child: &mut Self::Child) -> Result<(), String> {
            self.calls.push(CleanupCall::LeaderKill);
            next_result(&mut self.leader_kill, "leader kill")
        }

        fn probe_process_leader_reaped(
            &mut self,
            _child: &mut Self::Child,
        ) -> Result<bool, String> {
            self.calls.push(CleanupCall::ProbeLeaderReaped);
            next_result(&mut self.leader_reaped, "leader reap probe")
        }

        fn process_group_exists(&mut self, _process_group: Pid) -> Result<bool, String> {
            self.calls.push(CleanupCall::ProbeGroupExists);
            next_result(&mut self.group_exists, "group existence probe")
        }
    }

    fn next_result<T>(
        results: &mut VecDeque<Result<T, String>>,
        operation: &str,
    ) -> Result<T, String> {
        results
            .pop_front()
            .ok_or_else(|| format!("missing injected result for {operation}"))?
    }

    fn test_process_group() -> Result<Pid, String> {
        Pid::from_raw(42).ok_or_else(|| "test process-group id must be nonzero".to_string())
    }

    fn open_capture_streams() -> Result<(ProcessStreams, [PipeWriter; 2]), Box<dyn Error>> {
        let (stdout_reader, mut stdout_writer) = io::pipe()?;
        let (stderr_reader, mut stderr_writer) = io::pipe()?;
        stdout_writer.write_all(b"stdout")?;
        stderr_writer.write_all(b"stderr")?;
        let request = NativeProcessRequest::inspection(
            "/unused",
            std::iter::empty::<String>(),
            Duration::from_secs(1),
            16,
            16,
        );
        let streams = ProcessStreams::new(
            OwnedFd::from(stdout_reader).into(),
            OwnedFd::from(stderr_reader).into(),
            &request,
            configure_nonblocking,
        )?;
        Ok((streams, [stdout_writer, stderr_writer]))
    }

    #[test]
    fn exited_group_waits_for_pipe_eof_within_forced_verification() -> Result<(), Box<dyn Error>> {
        let (mut streams, writers) = open_capture_streams()?;
        let mut writers = Some(writers);
        let mut observations = 0;
        let mut operations = FakeCleanupOperations::exited_group(4);
        let deadline = NativeProcessError::deadline_exceeded(Duration::from_millis(20));
        let outcome = terminate_and_verify_with(
            &mut FakeChild,
            test_process_group()?,
            false,
            Some(&mut streams),
            Duration::from_secs(1),
            &mut operations,
            |_| {
                observations += 1;
                if observations == 7 {
                    drop(writers.take());
                }
                Some(deadline.clone())
            },
        );

        assert!(streams.all_closed());
        assert_eq!(observations, 8);
        assert_eq!(outcome.boundary, Some(deadline));
        assert!(outcome.evidence.is_empty());
        assert_eq!(operations.calls.len(), 5);
        let output = streams.into_output();
        assert_eq!(output.stdout(), b"stdout");
        assert_eq!(output.stderr(), b"stderr");
        Ok(())
    }

    #[test]
    fn exited_group_preserves_unclosed_pipe_when_verification_budget_expires()
    -> Result<(), Box<dyn Error>> {
        let (mut streams, [stdout_writer, stderr_writer]) = open_capture_streams()?;
        drop(stdout_writer);
        let mut observations = 0;
        let mut operations = FakeCleanupOperations::exited_group(3);
        let deadline = NativeProcessError::deadline_exceeded(Duration::from_millis(20));
        let outcome = terminate_and_verify_with(
            &mut FakeChild,
            test_process_group()?,
            false,
            Some(&mut streams),
            Duration::ZERO,
            &mut operations,
            |_| {
                observations += 1;
                Some(deadline.clone())
            },
        );

        assert!(!streams.all_closed());
        assert_eq!(observations, 6);
        assert_eq!(outcome.boundary, Some(deadline));
        assert!(outcome.evidence.is_empty());
        assert_eq!(operations.calls.len(), 4);
        drop(stderr_writer);
        streams.drain();
        assert!(streams.all_closed());
        Ok(())
    }

    #[test]
    fn exited_group_with_closed_pipes_needs_no_signals_or_waits() -> Result<(), Box<dyn Error>> {
        let (mut streams, writers) = open_capture_streams()?;
        drop(writers);
        let mut operations = FakeCleanupOperations::exited_group(1);
        let outcome = terminate_and_verify_with(
            &mut FakeChild,
            test_process_group()?,
            false,
            Some(&mut streams),
            Duration::ZERO,
            &mut operations,
            |_| None,
        );

        assert!(streams.all_closed());
        assert!(outcome.boundary.is_none());
        assert!(outcome.evidence.is_empty());
        assert_eq!(
            operations.calls,
            [
                CleanupCall::ProbeLeaderReaped,
                CleanupCall::ProbeGroupExists
            ]
        );
        Ok(())
    }

    #[test]
    fn cleanup_operations_deterministically_escalate_and_verify_absence() -> Result<(), String> {
        let mut operations = FakeCleanupOperations::successful_escalation();
        let outcome = terminate_and_verify_with(
            &mut FakeChild,
            test_process_group()?,
            false,
            None,
            Duration::ZERO,
            &mut operations,
            |_| None,
        );

        assert!(outcome.boundary.is_none());
        assert!(outcome.evidence.is_empty());
        assert_eq!(
            operations.calls,
            [
                CleanupCall::ProbeLeaderReaped,
                CleanupCall::ProbeGroupExists,
                CleanupCall::GroupTerm,
                CleanupCall::LeaderTerm,
                CleanupCall::ProbeLeaderReaped,
                CleanupCall::ProbeGroupExists,
                CleanupCall::GroupKill,
                CleanupCall::LeaderKill,
                CleanupCall::ProbeLeaderReaped,
                CleanupCall::ProbeGroupExists,
            ]
        );
        Ok(())
    }

    #[test]
    fn cleanup_operations_preserve_failures_and_surviving_group_evidence() -> Result<(), String> {
        let mut operations = FakeCleanupOperations::failing_survivor();
        let outcome = terminate_and_verify_with(
            &mut FakeChild,
            test_process_group()?,
            false,
            None,
            Duration::ZERO,
            &mut operations,
            |_| None,
        );

        for expected in [
            "process-leader reap probe failed: injected reap failure",
            "process-group existence probe failed: injected",
            "process-group termination failed: injected TERM failure",
            "process-leader termination failed: injected leader TERM failure",
            "process-group force kill failed: injected KILL failure",
            "process-leader force kill failed: injected leader kill failure",
            "process group remained present after force kill",
            "process leader was not reaped after force kill",
        ] {
            assert!(
                outcome
                    .evidence
                    .messages()
                    .iter()
                    .any(|message| message == expected),
                "missing cleanup evidence: {expected}"
            );
        }
        assert_eq!(operations.calls.len(), 10);
        Ok(())
    }
}
