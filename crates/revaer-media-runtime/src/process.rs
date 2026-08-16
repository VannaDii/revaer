//! Bounded native process execution shared by runtime adapters.

use std::time::Duration;

use thiserror::Error;

#[cfg(unix)]
use std::io::{self, Read};
#[cfg(unix)]
use std::os::fd::{AsFd, BorrowedFd};
#[cfg(unix)]
use std::process::{ChildStderr, ChildStdout, Command, ExitStatus, Stdio};
#[cfg(unix)]
use std::sync::Arc;
#[cfg(unix)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(unix)]
use std::thread::{self, JoinHandle};
#[cfg(unix)]
use std::time::Instant;

#[cfg(unix)]
const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(5);

#[derive(Clone, Copy)]
pub(crate) struct ProcessLimits {
    pub(crate) timeout: Duration,
    pub(crate) max_stdout_bytes: usize,
    pub(crate) max_stderr_bytes: usize,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum ProcessError {
    #[cfg(not(unix))]
    #[error("bounded process-tree execution is unavailable on this platform")]
    Unsupported,
    #[error("failed to spawn process: {0}")]
    Spawn(String),
    #[error("process deadline exceeded after {0:?}")]
    DeadlineExceeded(Duration),
    #[error("process {stream} exceeded {maximum_bytes} bytes")]
    OutputLimitExceeded {
        stream: &'static str,
        maximum_bytes: usize,
    },
    #[error("process exited with status {status}: {stderr}")]
    Exit { status: String, stderr: String },
    #[error("process execution failed: {0}")]
    Execution(String),
}

pub(crate) fn run_bounded(
    program: &str,
    args: &[&str],
    limits: ProcessLimits,
) -> Result<Vec<u8>, ProcessError> {
    run_platform(program, args, limits)
}

#[cfg(not(unix))]
fn run_platform(
    _program: &str,
    _args: &[&str],
    _limits: ProcessLimits,
) -> Result<Vec<u8>, ProcessError> {
    Err(ProcessError::Unsupported)
}

#[cfg(unix)]
fn run_platform(
    program: &str,
    args: &[&str],
    limits: ProcessLimits,
) -> Result<Vec<u8>, ProcessError> {
    run_platform_with(
        program,
        args,
        limits,
        configure_nonblocking,
        &ThreadReaderSpawner,
    )
}

#[cfg(unix)]
fn run_platform_with<C, R>(
    program: &str,
    args: &[&str],
    limits: ProcessLimits,
    mut configure: C,
    reader_spawner: &R,
) -> Result<Vec<u8>, ProcessError>
where
    C: FnMut(BorrowedFd<'_>) -> Result<(), io::Error>,
    R: ReaderSpawner,
{
    use std::os::unix::process::CommandExt;

    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    let mut child = command
        .spawn()
        .map_err(|error| ProcessError::Spawn(error.to_string()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| abort_spawned_child(&mut child, "stdout pipe unavailable".to_string()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| abort_spawned_child(&mut child, "stderr pipe unavailable".to_string()))?;
    configure(stdout.as_fd()).map_err(|error| {
        abort_spawned_child(&mut child, format!("failed to bound stdout: {error}"))
    })?;
    configure(stderr.as_fd()).map_err(|error| {
        abort_spawned_child(&mut child, format!("failed to bound stderr: {error}"))
    })?;

    let readers_should_stop = Arc::new(AtomicBool::new(false));
    let stdout_exceeded = Arc::new(AtomicBool::new(false));
    let stderr_exceeded = Arc::new(AtomicBool::new(false));
    let stdout_reader = reader_spawner
        .spawn_stdout(
            stdout,
            limits.max_stdout_bytes,
            Arc::clone(&stdout_exceeded),
            Arc::clone(&readers_should_stop),
        )
        .map_err(|error| {
            abort_spawned_child(
                &mut child,
                format!("failed to start stdout reader: {error}"),
            )
        })?;
    let stderr_reader = match reader_spawner.spawn_stderr(
        stderr,
        limits.max_stderr_bytes,
        Arc::clone(&stderr_exceeded),
        Arc::clone(&readers_should_stop),
    ) {
        Ok(reader) => reader,
        Err(error) => {
            let failure = abort_spawned_child(
                &mut child,
                format!("failed to start stderr reader: {error}"),
            );
            readers_should_stop.store(true, Ordering::Release);
            return match join_reader(stdout_reader, "stdout") {
                Ok(_) => Err(failure),
                Err(join_error) => Err(ProcessError::Execution(format!("{failure}; {join_error}"))),
            };
        }
    };

    let status = wait_bounded(&mut child, &limits, &stdout_exceeded, &stderr_exceeded);
    readers_should_stop.store(true, Ordering::Release);
    let stdout = join_reader(stdout_reader, "stdout")?;
    let stderr = join_reader(stderr_reader, "stderr")?;
    let status = status?;
    if !status.success() {
        return Err(ProcessError::Exit {
            status: status.to_string(),
            stderr: String::from_utf8_lossy(&stderr).trim().to_string(),
        });
    }
    Ok(stdout)
}

#[cfg(unix)]
fn configure_nonblocking(descriptor: BorrowedFd<'_>) -> Result<(), io::Error> {
    let flags = rustix::fs::fcntl_getfl(descriptor).map_err(io::Error::from)?;
    rustix::fs::fcntl_setfl(descriptor, flags | rustix::fs::OFlags::NONBLOCK)
        .map_err(io::Error::from)
}

#[cfg(unix)]
fn abort_spawned_child(child: &mut std::process::Child, reason: String) -> ProcessError {
    abort_spawned_child_with(child, reason, terminate_and_reap)
}

#[cfg(unix)]
fn abort_spawned_child_with<F>(
    child: &mut std::process::Child,
    reason: String,
    mut terminate: F,
) -> ProcessError
where
    F: FnMut(&mut std::process::Child) -> Result<(), ProcessError>,
{
    match terminate(child) {
        Ok(()) => ProcessError::Execution(reason),
        Err(cleanup_error) => {
            ProcessError::Execution(format!("{reason}; process cleanup failed: {cleanup_error}"))
        }
    }
}

#[cfg(unix)]
fn wait_bounded(
    child: &mut std::process::Child,
    limits: &ProcessLimits,
    stdout_exceeded: &AtomicBool,
    stderr_exceeded: &AtomicBool,
) -> Result<ExitStatus, ProcessError> {
    let started = Instant::now();
    loop {
        if started.elapsed() >= limits.timeout {
            terminate_and_reap(child)?;
            return Err(ProcessError::DeadlineExceeded(limits.timeout));
        }
        if let Err(limit_error) = check_output_limits(limits, stdout_exceeded, stderr_exceeded) {
            return match terminate_and_reap(child) {
                Ok(()) => Err(limit_error),
                Err(cleanup_error) => Err(ProcessError::Execution(format!(
                    "{limit_error}; process cleanup failed: {cleanup_error}"
                ))),
            };
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                terminate_process_group(child)?;
                check_output_limits(limits, stdout_exceeded, stderr_exceeded)?;
                return Ok(status);
            }
            Ok(None) => thread::sleep(PROCESS_POLL_INTERVAL),
            Err(error) => {
                return Err(abort_spawned_child(
                    child,
                    format!("process wait failed: {error}"),
                ));
            }
        }
    }
}

#[cfg(unix)]
fn check_output_limits(
    limits: &ProcessLimits,
    stdout_exceeded: &AtomicBool,
    stderr_exceeded: &AtomicBool,
) -> Result<(), ProcessError> {
    if stdout_exceeded.load(Ordering::Acquire) {
        return Err(ProcessError::OutputLimitExceeded {
            stream: "stdout",
            maximum_bytes: limits.max_stdout_bytes,
        });
    }
    if stderr_exceeded.load(Ordering::Acquire) {
        return Err(ProcessError::OutputLimitExceeded {
            stream: "stderr",
            maximum_bytes: limits.max_stderr_bytes,
        });
    }
    Ok(())
}

#[cfg(unix)]
fn terminate_and_reap(child: &mut std::process::Child) -> Result<(), ProcessError> {
    terminate_and_reap_with(
        child,
        terminate_process_group,
        std::process::Child::kill,
        std::process::Child::wait,
    )
}

#[cfg(unix)]
fn terminate_and_reap_with<G, K, W>(
    child: &mut std::process::Child,
    mut terminate_group: G,
    mut kill_leader: K,
    mut reap_leader: W,
) -> Result<(), ProcessError>
where
    G: FnMut(&std::process::Child) -> Result<(), ProcessError>,
    K: FnMut(&mut std::process::Child) -> Result<(), io::Error>,
    W: FnMut(&mut std::process::Child) -> Result<ExitStatus, io::Error>,
{
    let group_result = terminate_group(child);
    let leader_result = if group_result.is_err() {
        match kill_leader(child) {
            Ok(()) => None,
            Err(error) if error.kind() == io::ErrorKind::InvalidInput => None,
            Err(error) => Some(error),
        }
    } else {
        None
    };
    let reap_result = reap_leader(child).err();
    let mut failures = Vec::with_capacity(3);
    if let Err(error) = group_result {
        failures.push(format!("process-group termination failed: {error}"));
    }
    if let Some(error) = leader_result {
        failures.push(format!("process-leader termination failed: {error}"));
    }
    if let Some(error) = reap_result {
        failures.push(format!("process-leader reap failed: {error}"));
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(ProcessError::Execution(failures.join("; ")))
    }
}

#[cfg(unix)]
fn terminate_process_group(child: &std::process::Child) -> Result<(), ProcessError> {
    let process_group = rustix::process::Pid::from_child(child);
    match rustix::process::kill_process_group(process_group, rustix::process::Signal::KILL) {
        Ok(()) | Err(rustix::io::Errno::SRCH) => Ok(()),
        Err(error) => Err(ProcessError::Execution(error.to_string())),
    }
}

#[cfg(unix)]
fn spawn_reader<R>(
    reader: R,
    maximum_bytes: usize,
    exceeded: Arc<AtomicBool>,
    should_stop: Arc<AtomicBool>,
    stream: &'static str,
) -> Result<JoinHandle<Result<Vec<u8>, io::Error>>, io::Error>
where
    R: Read + Send + 'static,
{
    thread::Builder::new()
        .name(format!("media-{stream}-reader"))
        .spawn(move || read_bounded(reader, maximum_bytes, &exceeded, &should_stop))
}

#[cfg(unix)]
type ReaderHandle = JoinHandle<Result<Vec<u8>, io::Error>>;

#[cfg(unix)]
trait ReaderSpawner {
    fn spawn_stdout(
        &self,
        reader: ChildStdout,
        maximum_bytes: usize,
        exceeded: Arc<AtomicBool>,
        should_stop: Arc<AtomicBool>,
    ) -> Result<ReaderHandle, io::Error>;

    fn spawn_stderr(
        &self,
        reader: ChildStderr,
        maximum_bytes: usize,
        exceeded: Arc<AtomicBool>,
        should_stop: Arc<AtomicBool>,
    ) -> Result<ReaderHandle, io::Error>;
}

#[cfg(unix)]
struct ThreadReaderSpawner;

#[cfg(unix)]
impl ReaderSpawner for ThreadReaderSpawner {
    fn spawn_stdout(
        &self,
        reader: ChildStdout,
        maximum_bytes: usize,
        exceeded: Arc<AtomicBool>,
        should_stop: Arc<AtomicBool>,
    ) -> Result<ReaderHandle, io::Error> {
        spawn_reader(reader, maximum_bytes, exceeded, should_stop, "stdout")
    }

    fn spawn_stderr(
        &self,
        reader: ChildStderr,
        maximum_bytes: usize,
        exceeded: Arc<AtomicBool>,
        should_stop: Arc<AtomicBool>,
    ) -> Result<ReaderHandle, io::Error> {
        spawn_reader(reader, maximum_bytes, exceeded, should_stop, "stderr")
    }
}

#[cfg(unix)]
fn read_bounded(
    mut reader: impl Read,
    maximum_bytes: usize,
    exceeded: &AtomicBool,
    should_stop: &AtomicBool,
) -> Result<Vec<u8>, io::Error> {
    let mut output = Vec::with_capacity(maximum_bytes.min(64 * 1024));
    let mut buffer = [0_u8; 8 * 1024];
    loop {
        if exceeded.load(Ordering::Acquire) && should_stop.load(Ordering::Acquire) {
            return Ok(output);
        }
        match reader.read(&mut buffer) {
            Ok(0) => return Ok(output),
            Ok(count) => {
                if output.len().saturating_add(count) > maximum_bytes {
                    exceeded.store(true, Ordering::Release);
                } else if !exceeded.load(Ordering::Acquire) {
                    output.extend_from_slice(&buffer[..count]);
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                if should_stop.load(Ordering::Acquire) {
                    return Ok(output);
                }
                thread::sleep(PROCESS_POLL_INTERVAL);
            }
            Err(error) => return Err(error),
        }
    }
}

#[cfg(unix)]
fn join_reader(
    handle: JoinHandle<Result<Vec<u8>, io::Error>>,
    stream: &'static str,
) -> Result<Vec<u8>, ProcessError> {
    handle
        .join()
        .map_err(|_| ProcessError::Execution(format!("{stream} reader failed")))?
        .map_err(|error| ProcessError::Execution(error.to_string()))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::error::Error;

    #[derive(Clone, Copy)]
    enum ReaderFailure {
        StdoutSpawn,
        StderrSpawn,
        StdoutRead,
        StderrSpawnAfterStdoutRead,
    }

    struct FailingReaderSpawner(ReaderFailure);

    impl ReaderSpawner for FailingReaderSpawner {
        fn spawn_stdout(
            &self,
            reader: ChildStdout,
            maximum_bytes: usize,
            exceeded: Arc<AtomicBool>,
            should_stop: Arc<AtomicBool>,
        ) -> Result<ReaderHandle, io::Error> {
            match self.0 {
                ReaderFailure::StdoutSpawn => Err(io::Error::other("stdout spawn failed")),
                ReaderFailure::StdoutRead | ReaderFailure::StderrSpawnAfterStdoutRead => {
                    thread::Builder::new()
                        .name("failing-stdout-reader".to_string())
                        .spawn(|| Err(io::Error::other("stdout read failed")))
                }
                ReaderFailure::StderrSpawn => {
                    spawn_reader(reader, maximum_bytes, exceeded, should_stop, "stdout")
                }
            }
        }

        fn spawn_stderr(
            &self,
            reader: ChildStderr,
            maximum_bytes: usize,
            exceeded: Arc<AtomicBool>,
            should_stop: Arc<AtomicBool>,
        ) -> Result<ReaderHandle, io::Error> {
            match self.0 {
                ReaderFailure::StderrSpawn | ReaderFailure::StderrSpawnAfterStdoutRead => {
                    Err(io::Error::other("stderr spawn failed"))
                }
                ReaderFailure::StdoutRead | ReaderFailure::StdoutSpawn => {
                    spawn_reader(reader, maximum_bytes, exceeded, should_stop, "stderr")
                }
            }
        }
    }

    struct ErrorReader(io::ErrorKind);

    impl Read for ErrorReader {
        fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, io::Error> {
            Err(io::Error::from(self.0))
        }
    }

    const fn limits(timeout: Duration, maximum_bytes: usize) -> ProcessLimits {
        ProcessLimits {
            timeout,
            max_stdout_bytes: maximum_bytes,
            max_stderr_bytes: maximum_bytes,
        }
    }

    fn finished_child() -> Result<std::process::Child, io::Error> {
        Command::new("/bin/sh").args(["-c", "exit 0"]).spawn()
    }

    #[test]
    fn returns_successful_stdout() {
        let result = run_bounded(
            "/bin/sh",
            &["-c", "printf bounded"],
            limits(Duration::from_secs(1), 128),
        );
        assert_eq!(result, Ok(b"bounded".to_vec()));
    }

    #[test]
    fn rejects_nonzero_exit() {
        let result = run_bounded(
            "/bin/sh",
            &["-c", "printf failure >&2; exit 7"],
            limits(Duration::from_secs(1), 128),
        );
        assert!(matches!(
            result,
            Err(ProcessError::Exit { status, stderr })
                if status.contains('7') && stderr == "failure"
        ));
    }

    #[test]
    fn rejects_large_output() {
        let result = run_bounded(
            "/bin/sh",
            &["-c", "while :; do printf 1234567890; done"],
            limits(Duration::from_secs(1), 128),
        );
        assert!(matches!(
            result,
            Err(ProcessError::OutputLimitExceeded {
                stream: "stdout",
                maximum_bytes: 128
            })
        ));
    }

    #[test]
    fn rejects_large_stderr() {
        let result = run_bounded(
            "/bin/sh",
            &["-c", "while :; do printf 1234567890 >&2; done"],
            limits(Duration::from_secs(1), 128),
        );
        assert!(matches!(
            result,
            Err(ProcessError::OutputLimitExceeded {
                stream: "stderr",
                maximum_bytes: 128
            })
        ));
    }

    #[test]
    fn reports_spawn_failure() {
        let result = run_bounded(
            "/definitely/missing/revaer-process-test",
            &[],
            limits(Duration::from_secs(1), 128),
        );
        assert!(matches!(result, Err(ProcessError::Spawn(message)) if !message.is_empty()));
    }

    #[test]
    fn reports_each_pipe_configuration_failure() {
        for (failure_call, expected_stream) in [(0, "stdout"), (1, "stderr")] {
            let calls = Cell::new(0);
            let result = run_platform_with(
                "/bin/sh",
                &["-c", "sleep 30"],
                limits(Duration::from_secs(1), 128),
                |descriptor| {
                    let current_call = calls.get();
                    calls.set(current_call + 1);
                    if current_call == failure_call {
                        Err(io::Error::other("configuration failed"))
                    } else {
                        configure_nonblocking(descriptor)
                    }
                },
                &ThreadReaderSpawner,
            );
            assert!(matches!(
                result,
                Err(ProcessError::Execution(message))
                    if message.contains(&format!("failed to bound {expected_stream}"))
            ));
        }
    }

    #[test]
    fn reports_each_reader_spawn_failure() {
        for (failure, expected_stream) in [
            (ReaderFailure::StdoutSpawn, "stdout"),
            (ReaderFailure::StderrSpawn, "stderr"),
        ] {
            let result = run_platform_with(
                "/bin/sh",
                &["-c", "sleep 30"],
                limits(Duration::from_secs(1), 128),
                configure_nonblocking,
                &FailingReaderSpawner(failure),
            );
            assert!(matches!(
                result,
                Err(ProcessError::Execution(message))
                    if message.contains(&format!("failed to start {expected_stream} reader"))
            ));
        }
    }

    #[test]
    fn preserves_reader_error_when_second_reader_cannot_start() {
        let result = run_platform_with(
            "/bin/sh",
            &["-c", "sleep 30"],
            limits(Duration::from_secs(1), 128),
            configure_nonblocking,
            &FailingReaderSpawner(ReaderFailure::StderrSpawnAfterStdoutRead),
        );
        assert!(matches!(
            result,
            Err(ProcessError::Execution(message))
                if message.contains("failed to start stderr reader")
                    && message.contains("stdout read failed")
        ));
    }

    #[test]
    fn reports_reader_io_failure() {
        let result = run_platform_with(
            "/bin/sh",
            &["-c", "exit 0"],
            limits(Duration::from_secs(1), 128),
            configure_nonblocking,
            &FailingReaderSpawner(ReaderFailure::StdoutRead),
        );
        assert_eq!(
            result,
            Err(ProcessError::Execution("stdout read failed".to_string()))
        );
    }

    #[test]
    fn read_bounded_stops_on_would_block_and_propagates_other_errors() {
        let exceeded = AtomicBool::new(false);
        let should_stop = AtomicBool::new(true);
        let stopped = read_bounded(
            ErrorReader(io::ErrorKind::WouldBlock),
            128,
            &exceeded,
            &should_stop,
        );
        assert!(matches!(stopped, Ok(output) if output.is_empty()));

        let error = read_bounded(
            ErrorReader(io::ErrorKind::BrokenPipe),
            128,
            &exceeded,
            &AtomicBool::new(false),
        );
        assert!(matches!(error, Err(error) if error.kind() == io::ErrorKind::BrokenPipe));
    }

    #[test]
    fn abort_reports_cleanup_failure() -> Result<(), Box<dyn Error>> {
        let mut child = finished_child()?;
        let error = abort_spawned_child_with(&mut child, "abort failed".to_string(), |_| {
            Err(ProcessError::Execution("cleanup failed".to_string()))
        });
        let _status = child.wait()?;
        assert_eq!(
            error,
            ProcessError::Execution(
                "abort failed; process cleanup failed: process execution failed: cleanup failed"
                    .to_string()
            )
        );
        Ok(())
    }

    #[test]
    fn cleanup_reports_group_failure_after_invalid_leader_kill() -> Result<(), Box<dyn Error>> {
        let mut child = finished_child()?;
        let result = terminate_and_reap_with(
            &mut child,
            |_| Err(ProcessError::Execution("group failed".to_string())),
            |_| Err(io::Error::from(io::ErrorKind::InvalidInput)),
            std::process::Child::wait,
        );
        assert!(matches!(
            result,
            Err(ProcessError::Execution(message))
                if message.contains("process-group termination failed")
                    && !message.contains("process-leader termination failed")
        ));
        Ok(())
    }

    #[test]
    fn cleanup_aggregates_group_leader_and_reap_failures() -> Result<(), Box<dyn Error>> {
        let mut child = finished_child()?;
        let result = terminate_and_reap_with(
            &mut child,
            |_| Err(ProcessError::Execution("group failed".to_string())),
            |_| Err(io::Error::from(io::ErrorKind::PermissionDenied)),
            |_| Err(io::Error::from(io::ErrorKind::BrokenPipe)),
        );
        let _status = child.wait()?;
        assert!(matches!(
            result,
            Err(ProcessError::Execution(message))
                if message.contains("process-group termination failed")
                    && message.contains("process-leader termination failed")
                    && message.contains("process-leader reap failed")
        ));
        Ok(())
    }

    #[test]
    fn cleanup_accepts_successful_fallback_leader_kill() -> Result<(), Box<dyn Error>> {
        let mut child = finished_child()?;
        let result = terminate_and_reap_with(
            &mut child,
            |_| Err(ProcessError::Execution("group failed".to_string())),
            |_| Ok(()),
            std::process::Child::wait,
        );
        assert!(matches!(
            result,
            Err(ProcessError::Execution(message))
                if message.contains("process-group termination failed")
                    && !message.contains("process-leader termination failed")
        ));
        Ok(())
    }

    #[test]
    fn deadline_terminates_descendants_holding_pipes() {
        let started = Instant::now();
        let result = run_bounded(
            "/bin/sh",
            &["-c", "(sleep 30) & wait"],
            limits(Duration::from_millis(40), 128),
        );
        assert!(matches!(result, Err(ProcessError::DeadlineExceeded(_))));
        assert!(started.elapsed() < Duration::from_millis(750));
    }
}
