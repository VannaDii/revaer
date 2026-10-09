//! Read projections for the immutable configuration workflow, never legacy aliases.

use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub(crate) struct ProfileHeadPage {
    pub profiles: Vec<ProfileHead>,
    pub next_cursor: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct ProfileHead {
    pub media_profile_public_id: Uuid,
    pub profile_key: String,
    pub latest_version: i32,
    pub active_version: Option<i32>,
}

#[derive(Clone, Deserialize)]
pub(crate) struct AssociationCreated {
    pub media_discovery_association_public_id: Uuid,
    pub association_key: String,
    pub latest_version: i32,
    pub active_version: Option<i32>,
    pub media_profile_public_id: Uuid,
    pub profile_version: i32,
    pub source_root_key: String,
    pub root_relative_path: String,
    pub manual_enabled: bool,
    pub watcher_enabled: bool,
    pub schedule_enabled: bool,
}
