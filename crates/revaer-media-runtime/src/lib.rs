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
//! Layout: capability snapshots, media inspection abstractions, workspace policy,
//! execution argument builders, and job state models.

pub mod capabilities;
pub mod execute;
pub mod inspect;
pub mod jobs;
pub mod workspace;
