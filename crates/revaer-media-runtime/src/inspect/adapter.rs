use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Instant, SystemTime};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use super::model::{
    InspectCancellation, InspectError, InspectProbeExecutor, InspectProbeOutput,
    InspectProbeRequest, InspectionLimits, MediaInspection, NeverCancelled,
};
use super::parse::{parse_source, validate_sidecar_output};
use crate::sidecar::{
    FilesystemSidecarDiscoverer, SidecarDiscoverer, SidecarFormat, SidecarSubtitle,
};

/// Bounded media-inspection boundary.
pub trait InspectAdapter: Send + Sync {
    /// Inspect a source under the adapter's reviewed budgets.
    ///
    /// # Errors
    ///
    /// Returns a deterministic discovery, safety, resource, execution, or parse error.
    fn inspect(&self, source_path: &Path) -> Result<MediaInspection, InspectError> {
        self.inspect_with_cancellation(source_path, &NeverCancelled)
    }

    /// Inspect a source while honoring cooperative cancellation.
    ///
    /// # Errors
    ///
    /// Returns a deterministic cancellation, discovery, safety, resource, execution, or parse
    /// error.
    fn inspect_with_cancellation(
        &self,
        source_path: &Path,
        cancellation: &dyn InspectCancellation,
    ) -> Result<MediaInspection, InspectError>;
}

/// FFprobe-backed inspector with injected process and sidecar collaborators.
pub struct FfprobeInspectAdapter {
    executor: Arc<dyn InspectProbeExecutor>,
    sidecar_discoverer: Arc<dyn SidecarDiscoverer>,
    ffprobe_bin: OsString,
    limits: InspectionLimits,
}

impl FfprobeInspectAdapter {
    /// Construct an inspector with the reviewed limits and filesystem sidecar discovery.
    #[must_use]
    pub fn new(executor: Arc<dyn InspectProbeExecutor>, ffprobe_bin: impl Into<OsString>) -> Self {
        Self::with_collaborators(
            executor,
            ffprobe_bin,
            Arc::new(FilesystemSidecarDiscoverer::default()),
            InspectionLimits::reviewed(),
        )
    }

    /// Construct an inspector with explicit collaborators and limits.
    #[must_use]
    pub fn with_collaborators(
        executor: Arc<dyn InspectProbeExecutor>,
        ffprobe_bin: impl Into<OsString>,
        sidecar_discoverer: Arc<dyn SidecarDiscoverer>,
        limits: InspectionLimits,
    ) -> Self {
        Self {
            executor,
            sidecar_discoverer,
            ffprobe_bin: ffprobe_bin.into(),
            limits,
        }
    }
}

impl InspectAdapter for FfprobeInspectAdapter {
    fn inspect_with_cancellation(
        &self,
        source_path: &Path,
        cancellation: &dyn InspectCancellation,
    ) -> Result<MediaInspection, InspectError> {
        let started = Instant::now();
        check_request_state(started, self.limits.deadline, cancellation)?;
        let mut sidecars = self.sidecar_discoverer.discover(source_path)?;
        sidecars.sort_by(|left, right| left.path.cmp(&right.path));
        validate_counts(sidecars.len(), self.limits)?;
        let inputs = capture_inputs(source_path, &sidecars)?;
        check_request_state(started, self.limits.deadline, cancellation)?;

        let mut budget = ProbeBudget::new(self.limits);
        let source_output = self.probe(
            source_probe_args(source_path),
            started,
            cancellation,
            &mut budget,
        )?;
        verify_inputs(&inputs)?;
        let mut inspection = parse_source(source_path, &source_output)?;

        for sidecar in &sidecars {
            check_request_state(started, self.limits.deadline, cancellation)?;
            let output = self.probe(
                sidecar_probe_args(&sidecar.path),
                started,
                cancellation,
                &mut budget,
            )?;
            validate_sidecar_output(&sidecar.path, sidecar.format, &output)?;
            verify_inputs(&inputs)?;
        }
        check_request_state(started, self.limits.deadline, cancellation)?;
        verify_inputs(&inputs)?;
        inspection.sidecars = sidecars;
        Ok(inspection)
    }
}

impl FfprobeInspectAdapter {
    fn probe(
        &self,
        args: Vec<OsString>,
        started: Instant,
        cancellation: &dyn InspectCancellation,
        budget: &mut ProbeBudget,
    ) -> Result<Vec<u8>, InspectError> {
        budget.reserve_invocation()?;
        let remaining = remaining_time(started, self.limits.deadline)?;
        let request = InspectProbeRequest {
            program: self.ffprobe_bin.clone(),
            args,
            timeout: remaining,
            max_stdout_bytes: self.limits.max_stdout_bytes,
            max_stderr_bytes: self.limits.max_stderr_bytes,
        };
        let output = self.executor.run(&request, cancellation)?;
        check_request_state(started, self.limits.deadline, cancellation)?;
        budget.consume(output)
    }
}

struct ProbeBudget {
    limits: InspectionLimits,
    invocations: usize,
    output_bytes: usize,
}

impl ProbeBudget {
    const fn new(limits: InspectionLimits) -> Self {
        Self {
            limits,
            invocations: 0,
            output_bytes: 0,
        }
    }

    const fn reserve_invocation(&mut self) -> Result<(), InspectError> {
        if self.invocations >= self.limits.max_invocations {
            return Err(InspectError::InvocationLimitExceeded(
                self.limits.max_invocations,
            ));
        }
        self.invocations += 1;
        Ok(())
    }

    fn consume(&mut self, output: InspectProbeOutput) -> Result<Vec<u8>, InspectError> {
        if output.stdout.len() > self.limits.max_stdout_bytes {
            return Err(InspectError::ProcessOutputLimitExceeded {
                stream: "stdout",
                maximum_bytes: self.limits.max_stdout_bytes,
            });
        }
        if output.stderr.len() > self.limits.max_stderr_bytes {
            return Err(InspectError::ProcessOutputLimitExceeded {
                stream: "stderr",
                maximum_bytes: self.limits.max_stderr_bytes,
            });
        }
        let invocation_bytes = output.stdout.len().checked_add(output.stderr.len()).ok_or(
            InspectError::TotalOutputLimitExceeded(self.limits.max_total_output_bytes),
        )?;
        self.output_bytes = self
            .output_bytes
            .checked_add(invocation_bytes)
            .filter(|total| *total <= self.limits.max_total_output_bytes)
            .ok_or(InspectError::TotalOutputLimitExceeded(
                self.limits.max_total_output_bytes,
            ))?;
        Ok(output.stdout)
    }
}

fn validate_counts(sidecar_count: usize, limits: InspectionLimits) -> Result<(), InspectError> {
    if sidecar_count > limits.max_sidecars {
        return Err(InspectError::SidecarLimitExceeded(limits.max_sidecars));
    }
    let invocation_count =
        sidecar_count
            .checked_add(1)
            .ok_or(InspectError::InvocationLimitExceeded(
                limits.max_invocations,
            ))?;
    if invocation_count > limits.max_invocations {
        return Err(InspectError::InvocationLimitExceeded(
            limits.max_invocations,
        ));
    }
    Ok(())
}

fn source_probe_args(path: &Path) -> Vec<OsString> {
    [
        "-v",
        "error",
        "-read_intervals",
        "%+#1",
        "-show_streams",
        "-show_frames",
        "-show_format",
        "-show_chapters",
        "-of",
        "json",
    ]
    .into_iter()
    .map(OsString::from)
    .chain(std::iter::once(path.as_os_str().to_os_string()))
    .collect()
}

fn sidecar_probe_args(path: &Path) -> Vec<OsString> {
    ["-v", "error", "-show_streams", "-of", "json"]
        .into_iter()
        .map(OsString::from)
        .chain(std::iter::once(path.as_os_str().to_os_string()))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InputSnapshot {
    path: PathBuf,
    identity: FileIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileIdentity {
    length: u64,
    modified: SystemTime,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(unix)]
    change_time_seconds: i64,
    #[cfg(unix)]
    change_time_nanoseconds: i64,
}

fn capture_inputs(
    source_path: &Path,
    sidecars: &[SidecarSubtitle],
) -> Result<Vec<InputSnapshot>, InspectError> {
    let mut paths = BTreeSet::from([source_path.to_path_buf()]);
    for sidecar in sidecars {
        let primary = file_identity(&sidecar.path)?;
        let physical_bytes = match (sidecar.format, &sidecar.companion_path) {
            (SidecarFormat::VobSub, Some(companion)) => {
                if !paths.insert(companion.clone()) {
                    return Err(invalid_inventory("duplicate VobSub companion path"));
                }
                primary.length.checked_add(file_identity(companion)?.length)
            }
            (SidecarFormat::VobSub, None) => {
                return Err(invalid_inventory("VobSub sidecar has no companion"));
            }
            (_, Some(_)) => {
                return Err(invalid_inventory("non-VobSub sidecar has a companion"));
            }
            (_, None) => Some(primary.length),
        }
        .ok_or_else(|| invalid_inventory("sidecar byte count overflow"))?;
        if physical_bytes != sidecar.size_bytes {
            return Err(invalid_inventory("sidecar byte count does not match files"));
        }
        if !paths.insert(sidecar.path.clone()) {
            return Err(invalid_inventory("duplicate sidecar path"));
        }
    }
    paths
        .into_iter()
        .map(|path| file_identity(&path).map(|identity| InputSnapshot { path, identity }))
        .collect()
}

fn verify_inputs(inputs: &[InputSnapshot]) -> Result<(), InspectError> {
    for input in inputs {
        if file_identity(&input.path)? != input.identity {
            return Err(InspectError::InputChanged(input.path.clone()));
        }
    }
    Ok(())
}

fn file_identity(path: &Path) -> Result<FileIdentity, InspectError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| InspectError::InputMetadata {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(InspectError::UnsafeInput(path.to_path_buf()));
    }
    let modified = metadata
        .modified()
        .map_err(|error| InspectError::InputMetadata {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
    Ok(FileIdentity {
        length: metadata.len(),
        modified,
        #[cfg(unix)]
        device: metadata.dev(),
        #[cfg(unix)]
        inode: metadata.ino(),
        #[cfg(unix)]
        change_time_seconds: metadata.ctime(),
        #[cfg(unix)]
        change_time_nanoseconds: metadata.ctime_nsec(),
    })
}

fn invalid_inventory(message: &str) -> InspectError {
    InspectError::InvalidSidecarInventory(message.to_string())
}

fn check_request_state(
    started: Instant,
    deadline: std::time::Duration,
    cancellation: &dyn InspectCancellation,
) -> Result<(), InspectError> {
    if cancellation.is_cancelled() {
        return Err(InspectError::Cancelled);
    }
    if started.elapsed() >= deadline {
        return Err(InspectError::DeadlineExceeded(deadline));
    }
    Ok(())
}

fn remaining_time(
    started: Instant,
    deadline: std::time::Duration,
) -> Result<std::time::Duration, InspectError> {
    deadline
        .checked_sub(started.elapsed())
        .filter(|remaining| !remaining.is_zero())
        .ok_or(InspectError::DeadlineExceeded(deadline))
}
