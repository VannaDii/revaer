//! Feature-owned presentation of validated immutable profile pages.

use revaer_api_models::media_root_contract::{
    ProfileLifecycle, ProfileVersionPageResponse, RootKind,
};
use uuid::Uuid;

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ProfilePage {
    pub profiles: Vec<ProfileSummary>,
    pub next_cursor: Option<String>,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ProfileSummary {
    pub id: Uuid,
    pub key: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub dry_run_only: bool,
    pub latest: i32,
    pub active: Option<i32>,
    pub lifecycle: &'static str,
    pub target: String,
    pub policy: String,
    pub roots: Vec<ProfileRootSummary>,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ProfileRootSummary {
    pub kind: &'static str,
    pub key: String,
    pub binding: bool,
    pub destructive: bool,
    pub reason: Option<String>,
}

impl From<ProfileVersionPageResponse> for ProfilePage {
    fn from(page: ProfileVersionPageResponse) -> Self {
        let (profiles, next_cursor) = page.into_parts();
        Self {
            profiles: profiles
                .into_iter()
                .map(|profile| {
                    let fields = profile.fields();
                    let input = fields.profile.fields();
                    ProfileSummary {
                        id: fields.media_profile_public_id,
                        key: input.profile_key.clone(),
                        name: input.display_name.clone(),
                        description: input.description.clone(),
                        enabled: input.enabled,
                        dry_run_only: input.dry_run_only,
                        latest: fields.latest_version,
                        active: fields.active_version,
                        lifecycle: match fields.lifecycle_state {
                            ProfileLifecycle::Draft => "Draft",
                            ProfileLifecycle::Active => "Active",
                            ProfileLifecycle::Archived => "Archived",
                        },
                        target: format!(
                            "{} v{}",
                            input.desired_target_key, input.desired_target_version
                        ),
                        policy: format!("{} v{}", input.policy_key, input.policy_version),
                        roots: fields
                            .root_bindings
                            .iter()
                            .map(|root| ProfileRootSummary {
                                kind: match root.kind {
                                    RootKind::Source => "Source",
                                    RootKind::Output => "Output",
                                    RootKind::Workspace => "Workspace",
                                    RootKind::Backup => "Backup",
                                    RootKind::Quarantine => "Quarantine",
                                },
                                key: root.logical_key.clone(),
                                binding: root.binding_ready,
                                destructive: root.destructive_ready,
                                reason: root
                                    .binding_reason
                                    .clone()
                                    .or_else(|| root.destructive_reason.clone()),
                            })
                            .collect(),
                    }
                })
                .collect(),
            next_cursor,
        }
    }
}

#[cfg(test)]
mod tests;
