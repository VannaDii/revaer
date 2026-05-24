//! Inspection adapter interfaces.

use revaer_media_core::model::MediaGraph;
use thiserror::Error;

/// Error emitted by inspect adapters.
#[derive(Debug, Error)]
pub enum InspectError {
    /// Adapter failed with details.
    #[error("inspect adapter failure: {0}")]
    Adapter(String),
}

/// Inspect media and return a parsed graph.
pub trait InspectAdapter {
    /// Inspect the path and return a deterministic media graph.
    ///
    /// # Errors
    ///
    /// Returns [`InspectError`] when the adapter cannot inspect or parse the source media.
    fn inspect(&self, source_path: &str) -> Result<MediaGraph, InspectError>;
}
