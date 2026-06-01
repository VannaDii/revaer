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

//! Pure, deterministic media domain logic.
//! Layout: model, normalization, classification, compilation, diff, compliance,
//! planning, verification, and explanation modules.

pub mod classify;
pub mod compile;
pub mod compliance;
pub mod diff;
pub mod explain;
pub mod model;
pub mod normalize;
pub mod plan;
pub mod verify;
