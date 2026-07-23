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

//! Runtime persistence facades for torrent/filesystem and media operations.
//! Layout: `runtime.rs` and `media.rs`.

pub mod media;
pub mod runtime;

pub use media::*;
pub use runtime::*;
