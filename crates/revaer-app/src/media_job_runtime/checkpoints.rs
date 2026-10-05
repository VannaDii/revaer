//! Resume only required unfinished writers, validating completed output content.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use revaer_data::media::step_checkpoints::StepCheckpoint;
use revaer_media_runtime::execute::execute_step_controlled;
use revaer_media_runtime::workspace::ManagedWorkspace;
use sha2::{Digest, Sha256};

use super::{
    CancellationSignal, ClaimedMediaJobRow, ExecuteSequenceError, ExecuteStepError,
    ExecutionControl, ExecutionStep, MediaAggregateFingerprint, MediaJobRuntime,
    MediaJobRuntimeError,
};

mod files;

struct Writer {
    output: PathBuf,
    inputs: Vec<PathBuf>,
    signature: Vec<u8>,
}

fn writers(steps: &[ExecutionStep]) -> Result<Vec<Option<Writer>>, MediaJobRuntimeError> {
    let mut prefix = Sha256::new();
    let mut result = Vec::with_capacity(steps.len());
    for step in steps {
        // Exact compiled prefix, including paths and all arguments. A code-format
        // change conservatively invalidates evidence, never authorizes reuse.
        let representation = format!("{step:?}");
        prefix.update(representation.len().to_be_bytes());
        prefix.update(representation.as_bytes());
        let paths = writer_paths(step)?;
        result.push(paths.map(|(output, inputs)| Writer {
            output,
            inputs,
            signature: prefix.clone().finalize().to_vec(),
        }));
    }
    Ok(result)
}

type WriterPaths = Option<(PathBuf, Vec<PathBuf>)>;

fn writer_paths(step: &ExecutionStep) -> Result<WriterPaths, MediaJobRuntimeError> {
    let paths = match step {
        ExecutionStep::CopySidecarSubtitle {
            source_path,
            output_path,
        } => Some((PathBuf::from(output_path), vec![PathBuf::from(source_path)])),
        ExecutionStep::BackupSource {
            source_path,
            backup_path,
        } => Some((PathBuf::from(backup_path), vec![PathBuf::from(source_path)])),
        ExecutionStep::Command { bin, argv } => {
            // The execution compiler emits single-output ffmpeg commands, with
            // the output last and every file input introduced by -i.
            if bin != "ffmpeg" {
                return Err(MediaJobRuntimeError::InvalidPath(
                    "media_checkpoint_command_unsupported",
                ));
            }
            let output = argv.last().ok_or(MediaJobRuntimeError::InvalidPath(
                "media_checkpoint_command_output_missing",
            ))?;
            let inputs = argv
                .windows(2)
                .filter(|pair| pair[0] == "-i")
                .map(|pair| PathBuf::from(&pair[1]))
                .collect();
            Some((PathBuf::from(output), inputs))
        }
        ExecutionStep::VerifyOutput { .. } => None,
        ExecutionStep::AtomicReplace { .. } | ExecutionStep::QuarantineFailedOutput { .. } => {
            return Err(MediaJobRuntimeError::InvalidPath(
                "media_checkpoint_replacement_step_unexpected",
            ));
        }
    };
    Ok(paths)
}

fn required_outputs(steps: &[ExecutionStep], writers: &[Option<Writer>]) -> BTreeSet<PathBuf> {
    let mut required: BTreeSet<_> = writers
        .iter()
        .flatten()
        .map(|writer| writer.output.clone())
        .collect();
    for writer in writers.iter().flatten() {
        for input in &writer.inputs {
            required.remove(input);
        }
    }
    for step in steps {
        if let ExecutionStep::VerifyOutput { output_path } = step {
            required.insert(PathBuf::from(output_path));
        }
    }
    required
}

impl MediaJobRuntime {
    pub(super) async fn execute_checkpointed_steps(
        &self,
        job: &ClaimedMediaJobRow,
        steps: &[ExecutionStep],
        workspace: &ManagedWorkspace,
        source_fingerprint: &MediaAggregateFingerprint,
        signal: Arc<CancellationSignal>,
        shutdown: Option<&super::RuntimeShutdownReceiver>,
    ) -> Result<(), MediaJobRuntimeError> {
        let writers = writers(steps)?;
        let selected = self
            .select_unfinished_steps(job, steps, &writers, workspace, &signal)
            .await?;
        for (index, step) in steps.iter().enumerate() {
            if !selected.contains(&index) {
                continue;
            }
            self.ensure_not_cancelled(job).await?;
            self.revalidate_source_fingerprint(job, source_fingerprint, shutdown)
                .await?;
            if signal.cancellation_requested() {
                return Err(MediaJobRuntimeError::Cancelled);
            }
            if let Some(writer) = &writers[index] {
                files::discard(&writer.output, workspace)?;
            }
            let runner = Arc::clone(&self.command_runner);
            let step = step.clone();
            let control = Arc::clone(&signal);
            let executed = tokio::task::spawn_blocking(move || {
                execute_step_controlled(&step, &*runner, &*control)
            })
            .await
            .map_err(|error| MediaJobRuntimeError::Join(error.to_string()))?;
            match executed {
                Ok(()) => {}
                Err(ExecuteStepError::Cancelled) => return Err(MediaJobRuntimeError::Cancelled),
                Err(failed) => {
                    return Err(MediaJobRuntimeError::Execute(ExecuteSequenceError {
                        failed_step_index: index,
                        failed,
                        recovery: None,
                    }));
                }
            }
            if let Some(writer) = &writers[index] {
                self.record_step_output(job, index, writer, workspace, &signal)
                    .await?;
            }
        }
        Ok(())
    }

    async fn select_unfinished_steps(
        &self,
        job: &ClaimedMediaJobRow,
        steps: &[ExecutionStep],
        writers: &[Option<Writer>],
        workspace: &ManagedWorkspace,
        signal: &Arc<CancellationSignal>,
    ) -> Result<BTreeSet<usize>, MediaJobRuntimeError> {
        let mut required = required_outputs(steps, writers);
        let mut selected = BTreeSet::new();
        for (index, writer) in writers.iter().enumerate().rev() {
            let Some(writer) = writer else {
                selected.insert(index);
                continue;
            };
            if !required.remove(&writer.output) {
                continue;
            }
            let checkpoint = self
                .store
                .get_step_checkpoint(
                    job.media_job_public_id,
                    job.claim_generation,
                    super::usize_to_i32(index, "step_index")?,
                )
                .await?;
            if Self::output_reusable(writer, checkpoint.as_ref(), workspace, signal).await? {
                continue;
            }
            selected.insert(index);
            required.extend(writer.inputs.iter().cloned());
        }
        Ok(selected)
    }

    async fn output_reusable(
        writer: &Writer,
        checkpoint: Option<&StepCheckpoint>,
        workspace: &ManagedWorkspace,
        signal: &Arc<CancellationSignal>,
    ) -> Result<bool, MediaJobRuntimeError> {
        let Some(checkpoint) = checkpoint else {
            return Ok(false);
        };
        if checkpoint.step_signature != writer.signature
            || Path::new(&checkpoint.output_path) != writer.output
        {
            return Ok(false);
        }
        let observed = inspect_output(&writer.output, workspace, signal, false).await?;
        Ok(observed.is_some_and(|observed| {
            observed.size_bytes == checkpoint.size_bytes
                && observed.output_sha256 == checkpoint.output_sha256
        }))
    }

    async fn record_step_output(
        &self,
        job: &ClaimedMediaJobRow,
        index: usize,
        writer: &Writer,
        workspace: &ManagedWorkspace,
        signal: &Arc<CancellationSignal>,
    ) -> Result<(), MediaJobRuntimeError> {
        let mut checkpoint = inspect_output(&writer.output, workspace, signal, true)
            .await?
            .ok_or(MediaJobRuntimeError::InvalidPath(
                "media_checkpoint_completed_output_missing",
            ))?;
        checkpoint.step_signature.clone_from(&writer.signature);
        self.store
            .write_step_checkpoint(
                job.media_job_public_id,
                job.claim_generation,
                super::usize_to_i32(index, "step_index")?,
                &checkpoint,
            )
            .await?;
        Ok(())
    }
}

async fn inspect_output(
    output: &Path,
    workspace: &ManagedWorkspace,
    signal: &Arc<CancellationSignal>,
    synchronize: bool,
) -> Result<Option<StepCheckpoint>, MediaJobRuntimeError> {
    let path = output.to_owned();
    let workspace = workspace.clone();
    let signal = Arc::clone(signal);
    tokio::task::spawn_blocking(move || files::inspect(&path, &workspace, &signal, synchronize))
        .await
        .map_err(|error| MediaJobRuntimeError::Join(error.to_string()))?
}
