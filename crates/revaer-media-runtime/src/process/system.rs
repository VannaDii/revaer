#[cfg(unix)]
use super::{NATIVE_PROCESS_TERMINATION_GRACE, NativeProcessSecondaryEvidence};
use super::{
    NativeProcessControl, NativeProcessError, NativeProcessOutput, NativeProcessRequest,
    NativeProcessSupervisor,
};

/// Unix process-group implementation of [`NativeProcessSupervisor`].
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemNativeProcessSupervisor;

impl NativeProcessSupervisor for SystemNativeProcessSupervisor {
    fn run(
        &self,
        request: &NativeProcessRequest,
        control: &dyn NativeProcessControl,
    ) -> Result<NativeProcessOutput, NativeProcessError> {
        run_platform(request, control)
    }
}

#[cfg(not(unix))]
fn run_platform(
    _request: &NativeProcessRequest,
    _control: &dyn NativeProcessControl,
) -> Result<NativeProcessOutput, NativeProcessError> {
    Err(NativeProcessError::unsupported())
}

#[cfg(unix)]
mod unix {
    use std::io;
    use std::os::fd::BorrowedFd;
    use std::os::unix::process::CommandExt;
    use std::process::{Child, Command, ExitStatus, Stdio};
    use std::thread;
    use std::time::{Duration, Instant};

    use rustix::process::Pid;

    use super::{
        NATIVE_PROCESS_TERMINATION_GRACE, NativeProcessControl, NativeProcessError,
        NativeProcessOutput, NativeProcessRequest, NativeProcessSecondaryEvidence,
    };
    use crate::process::cleanup::terminate_and_verify;
    use crate::process::stream::{ProcessStreams, configure_nonblocking};

    const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(5);

    pub(super) fn run(
        request: &NativeProcessRequest,
        control: &dyn NativeProcessControl,
    ) -> Result<NativeProcessOutput, NativeProcessError> {
        run_with(
            request,
            control,
            NATIVE_PROCESS_TERMINATION_GRACE,
            configure_nonblocking,
        )
    }

    fn run_with<C>(
        request: &NativeProcessRequest,
        control: &dyn NativeProcessControl,
        termination_grace: Duration,
        configure: C,
    ) -> Result<NativeProcessOutput, NativeProcessError>
    where
        C: FnMut(BorrowedFd<'_>) -> Result<(), io::Error>,
    {
        let started = Instant::now();
        let context = RequestContext::new(started, request, control, termination_grace);
        if let Some(error) = context.boundary(None) {
            return Err(error);
        }

        let mut child = spawn_child(request)?;
        let process_group = Pid::from_child(&child);
        let Some(stdout) = child.stdout.take() else {
            return abort_without_streams(
                &mut child,
                process_group,
                NativeProcessError::supervision("stdout pipe unavailable".to_string()),
                context,
            );
        };
        let Some(stderr) = child.stderr.take() else {
            return abort_without_streams(
                &mut child,
                process_group,
                NativeProcessError::supervision("stderr pipe unavailable".to_string()),
                context,
            );
        };
        let mut streams = match ProcessStreams::new(stdout, stderr, request, configure) {
            Ok(streams) => streams,
            Err(error) => {
                return abort_without_streams(
                    &mut child,
                    process_group,
                    NativeProcessError::supervision(error),
                    context,
                );
            }
        };

        loop {
            streams.drain();
            if let Some(error) = context.boundary(Some(&streams)) {
                return stop_with_error(&mut child, process_group, &mut streams, error, context);
            }
            match child.try_wait() {
                Ok(Some(status)) => {
                    return finish_after_exit(&mut child, process_group, streams, status, context);
                }
                Ok(None) => thread::sleep(PROCESS_POLL_INTERVAL),
                Err(error) => {
                    return stop_with_error(
                        &mut child,
                        process_group,
                        &mut streams,
                        NativeProcessError::supervision(format!(
                            "process-leader wait failed: {error}"
                        )),
                        context,
                    );
                }
            }
        }
    }

    fn spawn_child(request: &NativeProcessRequest) -> Result<Child, NativeProcessError> {
        let mut command = Command::new(&request.program);
        command
            .args(&request.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0);
        command
            .spawn()
            .map_err(|error| NativeProcessError::spawn(error.to_string()))
    }

    #[derive(Clone, Copy)]
    struct RequestContext<'a> {
        started: Instant,
        request: &'a NativeProcessRequest,
        control: &'a dyn NativeProcessControl,
        grace: Duration,
    }

    impl<'a> RequestContext<'a> {
        const fn new(
            started: Instant,
            request: &'a NativeProcessRequest,
            control: &'a dyn NativeProcessControl,
            grace: Duration,
        ) -> Self {
            Self {
                started,
                request,
                control,
                grace,
            }
        }

        fn boundary(self, streams: Option<&ProcessStreams>) -> Option<NativeProcessError> {
            self.control
                .stop_reason()
                .map(NativeProcessError::stopped)
                .or_else(|| {
                    (self.started.elapsed() >= self.request.timeout)
                        .then(|| NativeProcessError::deadline_exceeded(self.request.timeout))
                })
                .or_else(|| streams.and_then(|streams| streams.output_limit_error(self.request)))
                .or_else(|| {
                    streams.and_then(|streams| {
                        streams.has_read_failure().then(|| {
                            NativeProcessError::supervision(
                                "native process output read failed".to_string(),
                            )
                        })
                    })
                })
        }

        fn deadline_remaining(self) -> Duration {
            self.request.timeout.saturating_sub(self.started.elapsed())
        }

        fn cleanup_boundary(self, streams: Option<&ProcessStreams>) -> Option<NativeProcessError> {
            if self.deadline_remaining().is_zero() {
                Some(NativeProcessError::deadline_exceeded(self.request.timeout))
            } else {
                self.boundary(streams)
            }
        }
    }

    fn stop_with_error(
        child: &mut Child,
        process_group: Pid,
        streams: &mut ProcessStreams,
        primary: NativeProcessError,
        context: RequestContext<'_>,
    ) -> Result<NativeProcessOutput, NativeProcessError> {
        let cleanup = terminate_and_verify(
            child,
            process_group,
            false,
            Some(streams),
            context.grace,
            |streams| context.cleanup_boundary(streams),
        );
        let settlement = settle_streams(streams, context);
        Err(combine_error(
            primary,
            streams,
            cleanup.evidence,
            [cleanup.boundary, settlement],
        ))
    }

    fn abort_without_streams(
        child: &mut Child,
        process_group: Pid,
        primary: NativeProcessError,
        context: RequestContext<'_>,
    ) -> Result<NativeProcessOutput, NativeProcessError> {
        let cleanup = terminate_and_verify(
            child,
            process_group,
            false,
            None,
            context.grace,
            |streams| context.cleanup_boundary(streams),
        );
        let mut evidence = cleanup.evidence;
        append_secondary_errors(&primary, [cleanup.boundary], &mut evidence);
        Err(combine_messages(primary, evidence))
    }

    fn finish_after_exit(
        child: &mut Child,
        process_group: Pid,
        mut streams: ProcessStreams,
        status: ExitStatus,
        context: RequestContext<'_>,
    ) -> Result<NativeProcessOutput, NativeProcessError> {
        let cleanup = terminate_and_verify(
            child,
            process_group,
            true,
            Some(&mut streams),
            context.grace,
            |streams| context.cleanup_boundary(streams),
        );
        let settlement = settle_streams(&mut streams, context);
        let stream_primary = streams.output_limit_error_from_capture().or_else(|| {
            streams.has_read_failure().then(|| {
                NativeProcessError::supervision("native process output read failed".to_string())
            })
        });
        let mut evidence = stream_evidence(&streams);
        evidence.extend(cleanup.evidence);
        let mut primary = None;
        select_primary(&mut primary, cleanup.boundary, &mut evidence);
        select_primary(&mut primary, settlement, &mut evidence);
        select_primary(&mut primary, stream_primary, &mut evidence);
        let output = streams.into_output();
        let exit_failure = (!status.success())
            .then(|| NativeProcessError::exit(status.to_string(), output.stderr()));
        select_primary(&mut primary, exit_failure, &mut evidence);
        if let Some(primary) = primary {
            return Err(combine_messages(primary, evidence));
        }
        if evidence.is_empty() {
            Ok(output)
        } else {
            Err(combine_messages(
                NativeProcessError::supervision("native process cleanup failed".to_string()),
                evidence,
            ))
        }
    }

    fn combine_error(
        primary: NativeProcessError,
        streams: &ProcessStreams,
        mut evidence: NativeProcessSecondaryEvidence,
        secondary_errors: [Option<NativeProcessError>; 2],
    ) -> NativeProcessError {
        evidence.extend(stream_evidence(streams));
        if let Some(limit) = streams.output_limit_error_from_capture()
            && limit != primary
        {
            evidence.push(limit.to_string());
        }
        append_secondary_errors(&primary, secondary_errors, &mut evidence);
        combine_messages(primary, evidence)
    }

    fn select_primary(
        primary: &mut Option<NativeProcessError>,
        candidate: Option<NativeProcessError>,
        evidence: &mut NativeProcessSecondaryEvidence,
    ) {
        let Some(candidate) = candidate else {
            return;
        };
        if let Some(existing) = primary {
            if *existing != candidate {
                evidence.push(candidate.to_string());
            }
        } else {
            *primary = Some(candidate);
        }
    }

    fn append_secondary_errors<const N: usize>(
        primary: &NativeProcessError,
        secondary_errors: [Option<NativeProcessError>; N],
        evidence: &mut NativeProcessSecondaryEvidence,
    ) {
        for error in secondary_errors.into_iter().flatten() {
            if &error != primary {
                evidence.push(error.to_string());
            }
        }
    }

    fn stream_evidence(streams: &ProcessStreams) -> NativeProcessSecondaryEvidence {
        let mut evidence = streams.evidence();
        if !streams.all_closed() {
            evidence.push("output pipes remained open after process-group cleanup".to_string());
        }
        evidence
    }

    fn settle_streams(
        streams: &mut ProcessStreams,
        context: RequestContext<'_>,
    ) -> Option<NativeProcessError> {
        let settlement_started = Instant::now();
        loop {
            streams.drain();
            if let Some(error) = context.boundary(Some(streams)) {
                return Some(error);
            }
            if streams.all_closed()
                || streams.has_read_failure()
                || settlement_started.elapsed() >= context.grace
            {
                return None;
            }
            let grace_remaining = context.grace.saturating_sub(settlement_started.elapsed());
            let sleep_for = PROCESS_POLL_INTERVAL
                .min(grace_remaining)
                .min(context.deadline_remaining());
            if sleep_for.is_zero() {
                continue;
            }
            thread::sleep(sleep_for);
        }
    }

    pub(in crate::process) fn combine_messages(
        primary: NativeProcessError,
        evidence: NativeProcessSecondaryEvidence,
    ) -> NativeProcessError {
        primary.with_secondary_evidence(evidence)
    }

    #[cfg(test)]
    pub(in crate::process) fn run_with_grace(
        request: &NativeProcessRequest,
        control: &dyn NativeProcessControl,
        grace: Duration,
    ) -> Result<NativeProcessOutput, NativeProcessError> {
        run_with(request, control, grace, configure_nonblocking)
    }

    #[cfg(test)]
    pub(in crate::process) fn run_with_setup_delay(
        request: &NativeProcessRequest,
        control: &dyn NativeProcessControl,
        delay: Duration,
        grace: Duration,
    ) -> Result<NativeProcessOutput, NativeProcessError> {
        let mut delayed = false;
        run_with(request, control, grace, |descriptor| {
            if !delayed {
                delayed = true;
                thread::sleep(delay);
            }
            configure_nonblocking(descriptor)
        })
    }
}

#[cfg(unix)]
fn run_platform(
    request: &NativeProcessRequest,
    control: &dyn NativeProcessControl,
) -> Result<NativeProcessOutput, NativeProcessError> {
    unix::run(request, control)
}

#[cfg(all(test, unix))]
pub(super) use unix::{combine_messages, run_with_grace, run_with_setup_delay};
