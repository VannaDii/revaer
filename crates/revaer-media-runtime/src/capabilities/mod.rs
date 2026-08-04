//! Tool capability models and detector adapters.

mod detect;
mod model;
mod parse;

pub use detect::{
    CapabilityDetectError, CapabilityDetector, CapabilityProbeExecutor, FfmpegCapabilityDetector,
    SystemCapabilityProbeExecutor, UnavailableCapabilityDetector,
};
pub use model::{CapabilitySnapshot, CodecCapability};

#[cfg(test)]
mod tests;
