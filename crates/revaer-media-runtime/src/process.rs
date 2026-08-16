//! Shared, injected supervision for native media processes.

mod model;
mod system;

#[cfg(unix)]
mod cleanup;
#[cfg(unix)]
mod stream;

pub use model::{
    INSPECTION_PROCESS_TIMEOUT, NATIVE_PROCESS_STREAM_LIMIT_BYTES,
    NATIVE_PROCESS_TERMINATION_GRACE, NativeProcessControl, NativeProcessError,
    NativeProcessOutput, NativeProcessPrimaryError, NativeProcessRequest,
    NativeProcessSecondaryEvidence, NativeProcessStopReason, NativeProcessSupervisor,
    NeverStopNativeProcess,
};
pub use system::SystemNativeProcessSupervisor;

#[cfg(test)]
mod tests;
