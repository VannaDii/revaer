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
//! The runtime provides deterministic, bounded discovery of the deployed
//! `FFmpeg` toolchain, adjacent subtitle sidecars, normalized media inspection
//! and trusted root catalogs with retained descriptor and mount checks. Command
//! execution observes cancellation and resource limits; managed scratch workspaces
//! enforce capacity, retention and cleanup policy.
//! Runtime collaborators are injected; bootstrap
//! code may select the concrete system adapters.

pub mod capabilities;
pub mod execute;
pub mod inspect;
pub mod process;
pub mod root_catalog;
pub mod sidecar;
pub mod workspace;
