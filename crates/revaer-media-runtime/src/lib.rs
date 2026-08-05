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
//! `FFmpeg` toolchain, adjacent subtitle sidecars, managed workspace lifecycle,
//! and normalized media inspection. Runtime collaborators are injected; bootstrap
//! code may select the concrete system adapters.

pub mod capabilities;
pub mod inspect;
mod process;
pub mod sidecar;
pub mod workspace;
