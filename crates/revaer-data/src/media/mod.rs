//! Media transcoding persistence stored-procedure helpers.

pub mod capabilities;
mod identity;
pub mod imports;
pub mod policy_snapshot;
pub mod profile_versions;
pub mod profiles;
pub mod root_catalog;

pub use identity::{
    MediaRootIdentity, MediaRootIdentityError, MediaRootIdentityResolver,
    StdMediaRootIdentityResolver,
};

#[cfg(test)]
mod schema_tests;
