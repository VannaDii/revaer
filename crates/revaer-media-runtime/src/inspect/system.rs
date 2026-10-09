use std::sync::Arc;

use crate::process::{
    NativeProcessControl, NativeProcessPrimaryError, NativeProcessRequest, NativeProcessStopReason,
    NativeProcessSupervisor,
};

use super::model::{
    InspectCancellation, InspectError, InspectProbeExecutor, InspectProbeOutput,
    InspectProbeRequest,
};

/// `FFprobe` executor using the process supervisor shared by runtime wiring.
#[derive(Clone)]
pub struct SupervisedInspectProbeExecutor {
    supervisor: Arc<dyn NativeProcessSupervisor>,
}

impl std::fmt::Debug for SupervisedInspectProbeExecutor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SupervisedInspectProbeExecutor")
            .finish_non_exhaustive()
    }
}

impl SupervisedInspectProbeExecutor {
    /// Construct an inspector from the process supervisor shared by runtime wiring.
    #[must_use]
    pub fn new(supervisor: Arc<dyn NativeProcessSupervisor>) -> Self {
        Self { supervisor }
    }
}

impl InspectProbeExecutor for SupervisedInspectProbeExecutor {
    fn run(
        &self,
        request: &InspectProbeRequest,
        cancellation: &dyn InspectCancellation,
    ) -> Result<InspectProbeOutput, InspectError> {
        let native_request = NativeProcessRequest::inspection(
            request.program.clone(),
            request.args.iter().cloned(),
            request.timeout,
            request.max_stdout_bytes,
            request.max_stderr_bytes,
        );
        match self.supervisor.run(
            &native_request,
            &InspectionNativeProcessControl(cancellation),
        ) {
            Ok(output) => {
                let (stdout, stderr) = output.into_streams();
                Ok(InspectProbeOutput { stdout, stderr })
            }
            Err(error) => {
                let (primary, secondary_evidence) = error.into_parts();
                match primary {
                    NativeProcessPrimaryError::Stopped(NativeProcessStopReason::Cancelled) => {
                        Err(InspectError::Cancelled { secondary_evidence })
                    }
                    NativeProcessPrimaryError::DeadlineExceeded(timeout) => {
                        Err(InspectError::DeadlineExceeded {
                            timeout,
                            secondary_evidence,
                        })
                    }
                    NativeProcessPrimaryError::OutputLimitExceeded {
                        stream,
                        maximum_bytes,
                    } => Err(InspectError::ProcessOutputLimitExceeded {
                        stream,
                        maximum_bytes,
                        secondary_evidence,
                    }),
                    primary => Err(InspectError::ProbeFailed {
                        message: primary.to_string(),
                        secondary_evidence,
                    }),
                }
            }
        }
    }
}

struct InspectionNativeProcessControl<'a>(&'a dyn InspectCancellation);

impl NativeProcessControl for InspectionNativeProcessControl<'_> {
    fn stop_reason(&self) -> Option<NativeProcessStopReason> {
        self.0
            .is_cancelled()
            .then_some(NativeProcessStopReason::Cancelled)
    }
}
