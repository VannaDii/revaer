//! Media transcoding persistence stored-procedure helpers.

pub mod capabilities;
pub mod configuration;
mod identity;
pub mod imports;
pub mod jobs;
pub mod profiles;

pub use identity::{
    MediaRootIdentity, MediaRootIdentityError, MediaRootIdentityResolver,
    StdMediaRootIdentityResolver,
};

#[cfg(test)]
mod schema_tests;
#[cfg(test)]
mod tests;
