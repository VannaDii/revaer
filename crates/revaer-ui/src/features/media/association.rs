//! Operator-authored association state, with no physical root paths.

use revaer_api_models::media_root_contract::{DiscoveryAssociationRequest, DiscoveryModes};
use uuid::Uuid;

use crate::models::media_configuration::{AssociationCreated, ProfileHeadPage};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProfileChoices {
    pub active: Vec<ActiveProfile>,
    pub next_cursor: Option<String>,
}

impl TryFrom<ProfileHeadPage> for ProfileChoices {
    type Error = &'static str;

    fn try_from(page: ProfileHeadPage) -> Result<Self, Self::Error> {
        let mut seen = std::collections::HashSet::new();
        if page.profiles.len() > 200 {
            return Err("The server returned an oversized profile page.");
        }
        let mut active = Vec::new();
        for profile in page.profiles {
            if profile.latest_version <= 0 || !seen.insert(profile.media_profile_public_id) {
                return Err("The server returned invalid profile version information.");
            }
            if let Some(version) = profile.active_version {
                if version <= 0 || version > profile.latest_version {
                    return Err("The server returned invalid profile version information.");
                }
                active.push(ActiveProfile {
                    id: profile.media_profile_public_id,
                    key: profile.profile_key,
                    version,
                });
            }
        }
        Ok(Self {
            active,
            next_cursor: page.next_cursor,
        })
    }
}

pub(crate) fn confirm_creation(
    response: &AssociationCreated,
    etag: &str,
    request: &DiscoveryAssociationRequest,
) -> Result<String, &'static str> {
    let tag = etag
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'));
    if response.association_key != request.association_key()
        || response.media_profile_public_id != request.media_profile_public_id()
        || response.profile_version != request.profile_version()
        || response.source_root_key != request.source_root_key()
        || response.root_relative_path != request.root_relative_path()
        || response.manual_enabled != request.modes().manual_enabled
        || response.watcher_enabled != request.modes().watcher_enabled
        || response.schedule_enabled != request.modes().schedule_enabled
        || response.latest_version != 1
        || response.active_version.is_some_and(|version| version != 1)
        || tag.is_none_or(|value| {
            value
                .chars()
                .any(|character| character == '"' || character.is_control())
        })
    {
        return Err(
            "Creation returned inconsistent confirmation. Check existing associations before resubmitting.",
        );
    }
    let lifecycle = if response.active_version.is_some() {
        "active"
    } else {
        "draft"
    };
    Ok(format!(
        "Association {} created as {lifecycle} version 1.",
        response.media_discovery_association_public_id
    ))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ActiveProfile {
    pub id: Uuid,
    pub key: String,
    pub version: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SourceChoice {
    pub key: String,
    pub binding_ready: bool,
    pub destructive_ready: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum SourceScope {
    #[default]
    Unselected,
    WholeRoot,
    Prefix,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct AssociationDraft {
    pub key: String,
    pub profile: Option<(Uuid, i32)>,
    pub source: String,
    pub scope: SourceScope,
    pub prefix: String,
    pub modes: DiscoveryModes,
}

impl Default for AssociationDraft {
    fn default() -> Self {
        Self {
            key: String::new(),
            profile: None,
            source: String::new(),
            scope: SourceScope::Unselected,
            prefix: String::new(),
            modes: DiscoveryModes {
                manual_enabled: false,
                watcher_enabled: false,
                schedule_enabled: false,
            },
        }
    }
}

impl AssociationDraft {
    pub(crate) fn request(
        &self,
        profiles: &[ActiveProfile],
        sources: &[SourceChoice],
    ) -> Result<DiscoveryAssociationRequest, &'static str> {
        let (id, version) = self.profile.ok_or("Select an active profile version.")?;
        if !profiles
            .iter()
            .any(|profile| profile.id == id && profile.version == version)
        {
            return Err(
                "The selected profile version is no longer active. Reload and review the draft.",
            );
        }
        if !sources
            .iter()
            .any(|source| source.key == self.source && source.binding_ready)
        {
            return Err("Select a binding-ready source root from the current catalog.");
        }
        let prefix = match self.scope {
            SourceScope::Unselected => return Err("Select whole-root or relative-prefix scope."),
            SourceScope::WholeRoot => "",
            SourceScope::Prefix if self.prefix.is_empty() => {
                return Err("Enter a nonempty relative prefix.");
            }
            SourceScope::Prefix => &self.prefix,
        };
        DiscoveryAssociationRequest::new(&self.key, id, version, &self.source, prefix, self.modes)
            .map_err(|_| "Check the association key and relative prefix. Absolute paths and parent traversal are not allowed.")
    }
}

pub(crate) const fn mutation_failure(status: u16) -> &'static str {
    match status {
        401 => "Authentication required. The unsaved association has been preserved.",
        403 => "Access denied. The unsaved association has been preserved.",
        404 | 405 => {
            "Association creation is unavailable on this server. The draft has been preserved."
        }
        409 => {
            "The association conflicts with current configuration or readiness. Reload and review before resubmitting."
        }
        412 => {
            "The configuration changed. Reload and review the preserved draft before resubmitting."
        }
        428 => "The server rejected the create precondition. The draft has been preserved.",
        400 | 413 => {
            "The server rejected the association configuration. Review the preserved draft."
        }
        _ => {
            "Creation could not be confirmed. Check existing associations before resubmitting; the draft has been preserved."
        }
    }
}

#[cfg(test)]
mod tests;
