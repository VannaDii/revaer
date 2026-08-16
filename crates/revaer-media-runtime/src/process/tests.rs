use std::error::Error;
use std::ffi::OsString;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[cfg(unix)]
use std::time::Instant;

use crate::capabilities::{CapabilityProbeExecutor, SupervisedCapabilityProbeExecutor};
use crate::inspect::{
    InspectCancellationToken, InspectError, InspectProbeExecutor, InspectProbeRequest,
    SupervisedInspectProbeExecutor,
};

use super::*;

#[derive(Debug, Default)]
struct ActiveControl;

impl NativeProcessControl for ActiveControl {
    fn stop_reason(&self) -> Option<NativeProcessStopReason> {
        None
    }
}

#[derive(Debug, Default)]
#[cfg(unix)]
struct CancelledControl;

#[cfg(unix)]
impl NativeProcessControl for CancelledControl {
    fn stop_reason(&self) -> Option<NativeProcessStopReason> {
        Some(NativeProcessStopReason::Cancelled)
    }
}

#[derive(Debug)]
#[cfg(unix)]
struct ElapsedControl {
    started: Instant,
    delay: Duration,
}

#[cfg(unix)]
impl ElapsedControl {
    fn after(delay: Duration) -> Self {
        Self {
            started: Instant::now(),
            delay,
        }
    }
}

#[cfg(unix)]
impl NativeProcessControl for ElapsedControl {
    fn stop_reason(&self) -> Option<NativeProcessStopReason> {
        (self.started.elapsed() >= self.delay).then_some(NativeProcessStopReason::Cancelled)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct RecordedRequest {
    program: String,
    timeout: Duration,
    max_stdout_bytes: usize,
    max_stderr_bytes: usize,
    stop_reason: Option<NativeProcessStopReason>,
}

#[derive(Debug, Default)]
struct RecordingSupervisor {
    requests: Mutex<Vec<RecordedRequest>>,
}

impl NativeProcessSupervisor for RecordingSupervisor {
    fn run(
        &self,
        request: &NativeProcessRequest,
        control: &dyn NativeProcessControl,
    ) -> Result<NativeProcessOutput, NativeProcessError> {
        self.requests
            .lock()
            .map_err(|error| NativeProcessError::supervision(error.to_string()))?
            .push(RecordedRequest {
                program: request.program().to_string_lossy().into_owned(),
                timeout: request.timeout(),
                max_stdout_bytes: request.max_stdout_bytes(),
                max_stderr_bytes: request.max_stderr_bytes(),
                stop_reason: control.stop_reason(),
            });
        Ok(NativeProcessOutput::from_streams(
            b"probe".to_vec(),
            Vec::new(),
        ))
    }
}

#[test]
fn inspection_request_preserves_reviewed_bounds() {
    let request = NativeProcessRequest::inspection(
        "ffprobe",
        ["-version"],
        Duration::from_secs(90),
        usize::MAX,
        usize::MAX,
    );

    assert_eq!(request.timeout(), Duration::from_secs(30));
    assert_eq!(request.max_stdout_bytes(), 16 * 1024 * 1024);
    assert_eq!(request.max_stderr_bytes(), 16 * 1024 * 1024);
    assert_eq!(request.program(), "ffprobe");
    assert_eq!(request.arguments(), [OsString::from("-version")]);
    assert_eq!(NATIVE_PROCESS_TERMINATION_GRACE, Duration::from_secs(5));
}

#[test]
fn capability_and_inspection_share_one_injected_supervisor() -> Result<(), Box<dyn Error>> {
    let recording = Arc::new(RecordingSupervisor::default());
    let shared: Arc<dyn NativeProcessSupervisor> = recording.clone();
    let control: Arc<dyn NativeProcessControl> = Arc::new(ActiveControl);
    let capability =
        SupervisedCapabilityProbeExecutor::new(Arc::clone(&shared), Arc::clone(&control));
    let inspection = SupervisedInspectProbeExecutor::new(Arc::clone(&shared));

    assert_eq!(
        capability.run("ffmpeg", &["-version"]),
        Ok("probe".to_string())
    );
    let probe = inspection.run(
        &InspectProbeRequest {
            program: OsString::from("ffprobe"),
            args: vec![OsString::from("-version")],
            timeout: Duration::from_secs(7),
            max_stdout_bytes: 4_096,
            max_stderr_bytes: 1_024,
        },
        &InspectCancellationToken::default(),
    )?;
    assert_eq!(probe.stdout, b"probe");

    let requests = recording
        .requests
        .lock()
        .map_err(|error| error.to_string())?;
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].program, "ffmpeg");
    assert_eq!(requests[0].timeout, Duration::from_secs(10));
    assert_eq!(requests[0].max_stdout_bytes, 16 * 1024 * 1024);
    assert_eq!(requests[0].max_stderr_bytes, 1024 * 1024);
    assert_eq!(requests[1].program, "ffprobe");
    assert_eq!(requests[1].timeout, Duration::from_secs(7));
    assert_eq!(requests[1].max_stdout_bytes, 4_096);
    assert_eq!(requests[1].max_stderr_bytes, 1_024);
    assert!(requests.iter().all(|request| request.stop_reason.is_none()));
    drop(requests);
    Ok(())
}

#[test]
fn adapters_cannot_construct_or_bypass_the_shared_supervisor() {
    for (adapter, source) in [
        (
            "capability detection",
            include_str!("../capabilities/detect.rs"),
        ),
        ("inspection", include_str!("../inspect/system.rs")),
    ] {
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        assert!(
            !production.contains("SystemNativeProcessSupervisor"),
            "{adapter} must receive the shared supervisor from composition"
        );
        assert!(
            !production.contains("Command::new("),
            "{adapter} must not launch a native process directly"
        );
    }
}

#[test]
fn unix_only_supervisor_imports_are_cfg_scoped() {
    let source = include_str!("system.rs");
    assert!(source.contains(
        "#[cfg(unix)]\nuse super::{NATIVE_PROCESS_TERMINATION_GRACE, NativeProcessSecondaryEvidence};"
    ));
    let unconditional_import = source
        .split("#[cfg(not(unix))]")
        .next()
        .unwrap_or(source)
        .split("#[cfg(unix)]")
        .next()
        .unwrap_or(source);
    assert!(!unconditional_import.contains("NativeProcessSecondaryEvidence"));
}

#[cfg(unix)]
fn shell_request(script: &str, timeout: Duration, maximum_bytes: usize) -> NativeProcessRequest {
    NativeProcessRequest::inspection(
        "/bin/sh",
        ["-c", script],
        timeout,
        maximum_bytes,
        maximum_bytes,
    )
}

#[cfg(unix)]
#[test]
fn pre_cancelled_request_does_not_spawn() {
    let request = NativeProcessRequest::inspection(
        "/definitely/missing/revaer-pre-cancel",
        std::iter::empty::<&str>(),
        Duration::from_secs(1),
        16,
        16,
    );

    assert_eq!(
        SystemNativeProcessSupervisor.run(&request, &CancelledControl),
        Err(NativeProcessError::stopped(
            NativeProcessStopReason::Cancelled,
        ))
    );
}

#[cfg(unix)]
#[test]
fn supervisor_closes_stdin_and_enforces_each_output_bound() -> Result<(), NativeProcessError> {
    let exact = shell_request(
        "if read value; then exit 9; fi; printf 1234; printf 5678 >&2",
        Duration::from_secs(1),
        4,
    );
    let output = SystemNativeProcessSupervisor.run(&exact, &ActiveControl)?;
    assert_eq!(output.stdout(), b"1234");
    assert_eq!(output.stderr(), b"5678");

    for (script, stream) in [("printf 12345", "stdout"), ("printf 12345 >&2", "stderr")] {
        let result = SystemNativeProcessSupervisor.run(
            &shell_request(script, Duration::from_secs(1), 4),
            &ActiveControl,
        );
        assert!(matches!(
            result,
            Err(error) if matches!(
                error.primary(),
                NativeProcessPrimaryError::OutputLimitExceeded {
                stream: actual,
                maximum_bytes: 4,
                } if *actual == stream
            )
        ));
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn deadline_includes_pipe_setup_time() {
    let request = shell_request("sleep 30", Duration::from_millis(20), 16);
    let started = Instant::now();
    let result = super::system::run_with_setup_delay(
        &request,
        &ActiveControl,
        Duration::from_millis(40),
        Duration::from_millis(40),
    );

    assert_eq!(
        result,
        Err(NativeProcessError::deadline_exceeded(
            Duration::from_millis(20)
        ))
    );
    assert!(started.elapsed() >= Duration::from_millis(40));
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[cfg(unix)]
#[test]
fn term_grace_precedes_force_kill_for_uncooperative_process() {
    let request = shell_request(
        "trap '' TERM; while :; do sleep 1; done",
        Duration::from_secs(5),
        16,
    );
    let grace = Duration::from_millis(100);
    let started = Instant::now();
    let result = super::system::run_with_grace(
        &request,
        &ElapsedControl::after(Duration::from_millis(40)),
        grace,
    );

    assert_eq!(
        result,
        Err(NativeProcessError::stopped(
            NativeProcessStopReason::Cancelled,
        ))
    );
    assert!(started.elapsed() >= grace);
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[cfg(unix)]
#[test]
fn successful_leader_cleans_descendant_retaining_output_pipe() -> Result<(), Box<dyn Error>> {
    assert_descendant_is_cleaned(false)
}

#[cfg(unix)]
#[test]
fn successful_leader_cleans_descendant_with_closed_output_pipes() -> Result<(), Box<dyn Error>> {
    assert_descendant_is_cleaned(true)
}

#[cfg(unix)]
#[test]
fn deadline_remains_live_after_process_leader_exit() {
    let grace = Duration::from_secs(2);
    let script = synchronized_descendant_script(UNCOOPERATIVE_DESCENDANT, false);
    let request = shell_request(&script, Duration::from_millis(60), 64);
    let started = Instant::now();
    let result = super::system::run_with_grace(&request, &ActiveControl, grace);

    assert!(
        matches!(
            &result,
            Err(error)
                if error.primary()
                    == &NativeProcessPrimaryError::DeadlineExceeded(Duration::from_millis(60))
        ),
        "unexpected result: {result:?}"
    );
    assert!(started.elapsed() < grace);
}

#[cfg(unix)]
#[test]
fn cancellation_remains_live_after_process_leader_exit() {
    let grace = Duration::from_millis(200);
    let script = synchronized_descendant_script(UNCOOPERATIVE_DESCENDANT, false);
    let request = shell_request(&script, Duration::from_secs(5), 64);
    let started = Instant::now();
    let result = super::system::run_with_grace(
        &request,
        &ElapsedControl::after(Duration::from_millis(60)),
        grace,
    );

    assert!(
        matches!(
            &result,
            Err(error)
                if error.primary()
                    == &NativeProcessPrimaryError::Stopped(NativeProcessStopReason::Cancelled)
        ),
        "unexpected result: {result:?}"
    );
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[cfg(unix)]
const COOPERATIVE_DESCENDANT: &str = r#"printf "%s\n" "$$" > "$1"; exec sleep 30"#;

#[cfg(unix)]
const UNCOOPERATIVE_DESCENDANT: &str =
    r#"trap "" TERM; printf "%s\n" "$$" > "$1"; while :; do sleep 1; done"#;

#[cfg(unix)]
fn synchronized_descendant_script(descendant: &str, close_output: bool) -> String {
    let redirection = if close_output { " >/dev/null 2>&1" } else { "" };
    format!(
        "ready_dir=$(mktemp -d \"${{TMPDIR:-/tmp}}/revaer-supervisor.XXXXXX\") || exit 90; \
         ready=\"$ready_dir/ready\"; \
         mkfifo \"$ready\" || {{ rmdir \"$ready_dir\"; exit 91; }}; \
         /bin/sh -c '{descendant}' descendant \"$ready\"{redirection} & \
         child=$!; \
         IFS= read -r ready_pid < \"$ready\" || {{ rm -f \"$ready\"; rmdir \"$ready_dir\"; exit 92; }}; \
         rm -f \"$ready\"; \
         rmdir \"$ready_dir\"; \
         [ \"$ready_pid\" = \"$child\" ] || exit 93; \
         printf %s \"$child\""
    )
}

#[cfg(unix)]
fn assert_descendant_is_cleaned(close_output: bool) -> Result<(), Box<dyn Error>> {
    let script = synchronized_descendant_script(COOPERATIVE_DESCENDANT, close_output);
    let output = super::system::run_with_grace(
        &shell_request(&script, Duration::from_secs(2), 64),
        &ActiveControl,
        Duration::from_secs(1),
    )?;
    let raw_pid = String::from_utf8(output.into_streams().0)?.parse::<i32>()?;
    let pid = rustix::process::Pid::from_raw(raw_pid).ok_or("shell returned PID zero")?;
    assert!(matches!(
        rustix::process::test_kill_process(pid),
        Err(rustix::io::Errno::SRCH)
    ));
    Ok(())
}

#[cfg(unix)]
#[test]
fn read_failure_is_combined_with_boundary_and_cleanup_evidence() {
    let mut evidence = super::stream::deterministic_read_failure();
    evidence.push("process group remained present after force kill".to_string());
    let result = super::system::combine_messages(
        NativeProcessError::deadline_exceeded(Duration::from_secs(1)),
        evidence,
    );

    assert_eq!(
        result.primary(),
        &NativeProcessPrimaryError::DeadlineExceeded(Duration::from_secs(1))
    );
    assert_eq!(result.code(), "deadline_exceeded");
    assert_eq!(
        result.secondary_evidence().messages(),
        &[
            "process group remained present after force kill".to_string(),
            "stdout read failed: injected read failure".to_string(),
        ]
    );
}

#[derive(Clone)]
struct FailingSupervisor(NativeProcessError);

impl NativeProcessSupervisor for FailingSupervisor {
    fn run(
        &self,
        _request: &NativeProcessRequest,
        _control: &dyn NativeProcessControl,
    ) -> Result<NativeProcessOutput, NativeProcessError> {
        Err(self.0.clone())
    }
}

fn mapped_inspect_failure(
    primary: NativeProcessPrimaryError,
) -> Result<crate::inspect::InspectProbeOutput, InspectError> {
    let failure = NativeProcessError::from_primary(primary).with_secondary_evidence(
        NativeProcessSecondaryEvidence::from_message("cleanup verification failed".to_string()),
    );
    let executor = SupervisedInspectProbeExecutor::new(Arc::new(FailingSupervisor(failure)));
    executor.run(
        &InspectProbeRequest {
            program: OsString::from("ffprobe"),
            args: vec![OsString::from("-version")],
            timeout: Duration::from_secs(1),
            max_stdout_bytes: 16,
            max_stderr_bytes: 16,
        },
        &InspectCancellationToken::default(),
    )
}

#[test]
fn inspect_adapter_preserves_typed_primary_and_secondary_process_evidence() {
    let cancellation = mapped_inspect_failure(NativeProcessPrimaryError::Stopped(
        NativeProcessStopReason::Cancelled,
    ));
    assert!(matches!(
        cancellation,
        Err(InspectError::Cancelled { secondary_evidence })
            if secondary_evidence.messages() == ["cleanup verification failed"]
    ));

    let deadline = mapped_inspect_failure(NativeProcessPrimaryError::DeadlineExceeded(
        Duration::from_secs(1),
    ));
    assert!(matches!(
        deadline,
        Err(InspectError::DeadlineExceeded {
            timeout,
            secondary_evidence,
        }) if timeout == Duration::from_secs(1)
            && secondary_evidence.messages() == ["cleanup verification failed"]
    ));

    let output_limit = mapped_inspect_failure(NativeProcessPrimaryError::OutputLimitExceeded {
        stream: "stdout",
        maximum_bytes: 16,
    });
    assert!(matches!(
        output_limit,
        Err(InspectError::ProcessOutputLimitExceeded {
            stream: "stdout",
            maximum_bytes: 16,
            secondary_evidence,
        }) if secondary_evidence.messages() == ["cleanup verification failed"]
    ));
}
