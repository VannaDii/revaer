//! Deterministic, bounded media inspection through injected collaborators.

mod adapter;
mod ffprobe;
mod model;
mod parse;
mod system;

pub use adapter::{FfprobeInspectAdapter, InspectAdapter};
pub use model::{
    ChapterInspection, ContainerInspection, InspectCancellation, InspectCancellationToken,
    InspectError, InspectProbeExecutor, InspectProbeOutput, InspectProbeRequest, InspectionLimits,
    MediaInspection, MetadataEntry, NeverCancelled, ProbeGraph, ProbeStream, SideDataInspection,
    StreamInspection,
};
pub use parse::normalize_probe_graph;
pub use system::SystemInspectProbeExecutor;

#[cfg(test)]
mod tests;
