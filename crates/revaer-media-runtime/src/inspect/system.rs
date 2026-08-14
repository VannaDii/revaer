use super::model::{
    InspectCancellation, InspectError, InspectProbeExecutor, InspectProbeOutput,
    InspectProbeRequest,
};

/// System-process `FFprobe` executor with process-tree cancellation and bounded capture.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemInspectProbeExecutor;

impl InspectProbeExecutor for SystemInspectProbeExecutor {
    fn run(
        &self,
        request: &InspectProbeRequest,
        cancellation: &dyn InspectCancellation,
    ) -> Result<InspectProbeOutput, InspectError> {
        run_platform(request, cancellation)
    }
}

#[cfg(not(unix))]
fn run_platform(
    _request: &InspectProbeRequest,
    _cancellation: &dyn InspectCancellation,
) -> Result<InspectProbeOutput, InspectError> {
    Err(InspectError::ProbeFailed(
        "cancellable process-tree inspection is unavailable on this platform".to_string(),
    ))
}

#[cfg(unix)]
fn run_platform(
    request: &InspectProbeRequest,
    cancellation: &dyn InspectCancellation,
) -> Result<InspectProbeOutput, InspectError> {
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    let mut command = Command::new(&request.program);
    command
        .args(&request.args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    let mut child = command
        .spawn()
        .map_err(|error| InspectError::ProbeFailed(format!("process spawn failed: {error}")))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| abort_child(&mut child, "stdout pipe unavailable".to_string()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| abort_child(&mut child, "stderr pipe unavailable".to_string()))?;
    let stdout_exceeded = Arc::new(AtomicBool::new(false));
    let stderr_exceeded = Arc::new(AtomicBool::new(false));
    let stdout_reader = spawn_reader(
        stdout,
        request.max_stdout_bytes,
        Arc::clone(&stdout_exceeded),
        "stdout",
    )
    .map_err(|error| abort_child(&mut child, error))?;
    let stderr_reader = match spawn_reader(
        stderr,
        request.max_stderr_bytes,
        Arc::clone(&stderr_exceeded),
        "stderr",
    ) {
        Ok(reader) => reader,
        Err(error) => {
            let failure = abort_child(&mut child, error);
            let stdout_result = join_reader(stdout_reader, "stdout");
            return match stdout_result {
                Ok(_) => Err(failure),
                Err(join_error) => Err(combine_errors(&failure, &join_error)),
            };
        }
    };

    let status = wait_for_child(
        &mut child,
        request,
        cancellation,
        &stdout_exceeded,
        &stderr_exceeded,
    );
    let stdout = join_reader(stdout_reader, "stdout");
    let stderr = join_reader(stderr_reader, "stderr");
    let (status, stdout, stderr) = merge_process_results(status, stdout, stderr)?;
    if !status.success() {
        let detail = String::from_utf8_lossy(&stderr).trim().to_string();
        let suffix = if detail.is_empty() {
            String::new()
        } else {
            format!(": {detail}")
        };
        return Err(InspectError::ProbeFailed(format!(
            "process exited with status {status}{suffix}"
        )));
    }
    Ok(InspectProbeOutput { stdout, stderr })
}

#[cfg(unix)]
fn spawn_reader<R>(
    reader: R,
    maximum_bytes: usize,
    exceeded: std::sync::Arc<std::sync::atomic::AtomicBool>,
    stream: &'static str,
) -> Result<std::thread::JoinHandle<Result<Vec<u8>, InspectError>>, String>
where
    R: std::io::Read + Send + 'static,
{
    std::thread::Builder::new()
        .name(format!("media-inspect-{stream}"))
        .spawn(move || read_bounded(reader, maximum_bytes, &exceeded, stream))
        .map_err(|error| format!("failed to start {stream} reader: {error}"))
}

#[cfg(unix)]
fn read_bounded<R>(
    mut reader: R,
    maximum_bytes: usize,
    exceeded: &std::sync::atomic::AtomicBool,
    stream: &'static str,
) -> Result<Vec<u8>, InspectError>
where
    R: std::io::Read,
{
    use std::sync::atomic::Ordering;

    let capture_limit = maximum_bytes.saturating_add(1);
    let mut output = Vec::with_capacity(capture_limit.min(64 * 1024));
    let mut buffer = [0_u8; 8 * 1024];
    while output.len() < capture_limit {
        let remaining = capture_limit - output.len();
        let read_size = remaining.min(buffer.len());
        let count = reader
            .read(&mut buffer[..read_size])
            .map_err(|error| InspectError::ProbeFailed(format!("{stream} read failed: {error}")))?;
        if count == 0 {
            break;
        }
        output.extend_from_slice(&buffer[..count]);
    }
    if output.len() > maximum_bytes {
        exceeded.store(true, Ordering::Release);
        return Err(InspectError::ProcessOutputLimitExceeded {
            stream,
            maximum_bytes,
        });
    }
    Ok(output)
}

#[cfg(unix)]
fn wait_for_child(
    child: &mut std::process::Child,
    request: &InspectProbeRequest,
    cancellation: &dyn InspectCancellation,
    stdout_exceeded: &std::sync::atomic::AtomicBool,
    stderr_exceeded: &std::sync::atomic::AtomicBool,
) -> Result<std::process::ExitStatus, InspectError> {
    use std::sync::atomic::Ordering;
    use std::thread;
    use std::time::{Duration, Instant};

    let started = Instant::now();
    loop {
        let boundary_error = if cancellation.is_cancelled() {
            Some(InspectError::Cancelled)
        } else if started.elapsed() >= request.timeout {
            Some(InspectError::DeadlineExceeded(request.timeout))
        } else if stdout_exceeded.load(Ordering::Acquire) {
            Some(InspectError::ProcessOutputLimitExceeded {
                stream: "stdout",
                maximum_bytes: request.max_stdout_bytes,
            })
        } else if stderr_exceeded.load(Ordering::Acquire) {
            Some(InspectError::ProcessOutputLimitExceeded {
                stream: "stderr",
                maximum_bytes: request.max_stderr_bytes,
            })
        } else {
            None
        };
        if let Some(error) = boundary_error {
            terminate_and_reap(child)?;
            return Err(error);
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                terminate_completed_process_group(child)?;
                return Ok(status);
            }
            Ok(None) => thread::sleep(Duration::from_millis(5)),
            Err(error) => {
                return Err(abort_child(child, format!("process wait failed: {error}")));
            }
        }
    }
}

#[cfg(unix)]
fn abort_child(child: &mut std::process::Child, reason: String) -> InspectError {
    match terminate_and_reap(child) {
        Ok(()) => InspectError::ProbeFailed(reason),
        Err(error) => InspectError::ProbeFailed(format!("{reason}; cleanup failed: {error}")),
    }
}

#[cfg(unix)]
fn terminate_and_reap(child: &mut std::process::Child) -> Result<(), InspectError> {
    let group_result = terminate_process_group(child);
    let leader_result = if group_result.is_err() {
        match child.kill() {
            Ok(()) => None,
            Err(error) if error.kind() == std::io::ErrorKind::InvalidInput => None,
            Err(error) => Some(error.to_string()),
        }
    } else {
        None
    };
    let wait_result = child.wait().map(|_| ()).map_err(|error| error.to_string());
    let mut failures = Vec::new();
    if let Err(error) = group_result {
        failures.push(error.to_string());
    }
    if let Some(error) = leader_result {
        failures.push(format!("process leader termination failed: {error}"));
    }
    if let Err(error) = wait_result {
        failures.push(format!("process leader reap failed: {error}"));
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(InspectError::ProbeFailed(failures.join("; ")))
    }
}

#[cfg(unix)]
fn terminate_process_group(child: &std::process::Child) -> Result<(), InspectError> {
    let process_group = rustix::process::Pid::from_child(child);
    match rustix::process::kill_process_group(process_group, rustix::process::Signal::KILL) {
        Ok(()) | Err(rustix::io::Errno::SRCH) => Ok(()),
        Err(error) => Err(InspectError::ProbeFailed(format!(
            "process-group termination failed: {error}"
        ))),
    }
}

#[cfg(unix)]
fn terminate_completed_process_group(child: &std::process::Child) -> Result<(), InspectError> {
    let process_group = rustix::process::Pid::from_child(child);
    match rustix::process::kill_process_group(process_group, rustix::process::Signal::KILL) {
        Ok(()) | Err(rustix::io::Errno::SRCH | rustix::io::Errno::PERM) => Ok(()),
        Err(error) => Err(InspectError::ProbeFailed(format!(
            "completed process-group cleanup failed: {error}"
        ))),
    }
}

#[cfg(unix)]
fn join_reader(
    reader: std::thread::JoinHandle<Result<Vec<u8>, InspectError>>,
    stream: &'static str,
) -> Result<Vec<u8>, InspectError> {
    reader.join().map_err(|_| {
        InspectError::ProbeFailed(format!("{stream} reader thread terminated unexpectedly"))
    })?
}

#[cfg(unix)]
fn combine_errors(primary: &InspectError, secondary: &InspectError) -> InspectError {
    InspectError::ProbeFailed(format!("{primary}; {secondary}"))
}

#[cfg(unix)]
fn merge_process_results(
    status: Result<std::process::ExitStatus, InspectError>,
    stdout: Result<Vec<u8>, InspectError>,
    stderr: Result<Vec<u8>, InspectError>,
) -> Result<(std::process::ExitStatus, Vec<u8>, Vec<u8>), InspectError> {
    let status = match status {
        Err(
            error @ (InspectError::Cancelled
            | InspectError::DeadlineExceeded(_)
            | InspectError::ProcessOutputLimitExceeded { .. }),
        ) => return Err(error),
        status => status,
    };
    match (status, stdout, stderr) {
        (Ok(status), Ok(stdout), Ok(stderr)) => Ok((status, stdout, stderr)),
        (Err(error), Ok(_), Ok(_)) | (Ok(_), Err(error), Ok(_)) | (Ok(_), Ok(_), Err(error)) => {
            Err(error)
        }
        (status, stdout, stderr) => {
            let failures = [status.err(), stdout.err(), stderr.err()]
                .into_iter()
                .flatten()
                .map(|error| error.to_string())
                .collect::<Vec<_>>();
            Err(InspectError::ProbeFailed(failures.join("; ")))
        }
    }
}
