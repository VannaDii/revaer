//! Media transcoding persistence stored-procedure helpers.

pub mod association_jobs;
pub mod associations;
pub mod capabilities;
pub mod configuration;
mod identity;
pub mod imports;
pub mod job_roots;
pub mod jobs;
pub mod policy_snapshot;
pub mod portable;
pub mod profile_versions;
pub mod profiles;
pub mod root_catalog;
pub mod schedules;

pub use identity::{
    MediaRootIdentity, MediaRootIdentityError, MediaRootIdentityResolver,
    StdMediaRootIdentityResolver,
};

#[cfg(test)]
mod schema_tests;
#[cfg(test)]
mod tests;
