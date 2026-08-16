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
    let mut evidence = NativeProcessSecondaryEvidence::default();
    let mut monitor = CleanupMonitor::new(monitor);
    let mut streams = streams;
    monitor.capture(streams.as_deref());
    let initial = observe(
        child,
        process_group,
        leader_reaped,
        streams.as_deref_mut(),
        &mut evidence,
    );
    monitor.capture(streams.as_deref());
    if initial.complete() {
        return monitor.into_outcome(evidence);
    }

    record_signal(
        signal_process_group(process_group, Signal::TERM),
        "process-group termination failed",
        &mut evidence,
    );
    if !initial.leader_reaped {
        record_signal(
            signal_process(Pid::from_child(child), Signal::TERM),
            "process-leader termination failed",
            &mut evidence,
        );
    }

    let graceful = wait_for_state(
        child,
        process_group,
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
            signal_process_group(process_group, Signal::KILL),
            "process-group force kill failed",
            &mut evidence,
        );
    }
    if !graceful.leader_reaped {
        match child.kill() {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::InvalidInput => {}
            Err(error) => push_unique(
                &mut evidence,
                format!("process-leader force kill failed: {error}"),
            ),
        }
    }

    let forced = wait_for_state(
        child,
        process_group,
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
}

impl CleanupState {
    const fn complete(self) -> bool {
        self.leader_reaped && !self.group_present
    }
}

fn wait_for_state<F>(
    child: &mut Child,
    process_group: Pid,
    leader_reaped: bool,
    mut streams: Option<&mut ProcessStreams>,
    evidence: &mut NativeProcessSecondaryEvidence,
    monitor: &mut CleanupMonitor<F>,
    policy: WaitPolicy,
) -> CleanupState
where
    F: FnMut(Option<&ProcessStreams>) -> Option<NativeProcessError>,
{
    let started = Instant::now();
    let mut leader_reaped = leader_reaped;
    loop {
        monitor.capture(streams.as_deref());
        let state = observe(
            child,
            process_group,
            leader_reaped,
            streams.as_deref_mut(),
            evidence,
        );
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

fn observe(
    child: &mut Child,
    process_group: Pid,
    leader_reaped: bool,
    streams: Option<&mut ProcessStreams>,
    evidence: &mut NativeProcessSecondaryEvidence,
) -> CleanupState {
    if let Some(streams) = streams {
        streams.drain();
    }
    let leader_reaped = if leader_reaped {
        true
    } else {
        match child.try_wait() {
            Ok(Some(_status)) => true,
            Ok(None) => false,
            Err(error) => {
                push_unique(
                    evidence,
                    format!("process-leader reap probe failed: {error}"),
                );
                false
            }
        }
    };
    let group_present = match process_group_exists(process_group) {
        Ok(present) => present,
        Err(error) => {
            push_unique(evidence, error);
            true
        }
    };
    CleanupState {
        leader_reaped,
        group_present,
    }
}

fn process_group_exists(process_group: Pid) -> Result<bool, String> {
    match rustix::process::test_kill_process_group(process_group) {
        Ok(()) | Err(rustix::io::Errno::PERM) => Ok(true),
        Err(rustix::io::Errno::SRCH) => Ok(false),
        Err(error) => Err(format!("process-group existence probe failed: {error}")),
    }
}

fn signal_process_group(process_group: Pid, signal: Signal) -> Result<(), String> {
    match rustix::process::kill_process_group(process_group, signal) {
        Ok(()) | Err(rustix::io::Errno::SRCH) => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

fn signal_process(process: Pid, signal: Signal) -> Result<(), String> {
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
