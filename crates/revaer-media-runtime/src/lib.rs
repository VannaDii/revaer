#![forbid(unsafe_code)]
#![deny(
    warnings,
    dead_code,
    unused,
    unused_imports,
    unused_must_use,
    unreachable_pub,
    clippy::all,
    clippy::pedantic,
    rustdoc::broken_intra_doc_links,
    rustdoc::bare_urls,
    missing_docs
)]

//! Runtime adapters and orchestration primitives for media processing.
//!
//! The initial surface provides deterministic, bounded discovery of the deployed
//! `FFmpeg` toolchain. Callers inject [`capabilities::CapabilityProbeExecutor`] in
//! tests and use [`capabilities::SystemCapabilityProbeExecutor`] at bootstrap.

pub mod capabilities;
mod process;
pub mod sidecar;
pub mod workspace;
