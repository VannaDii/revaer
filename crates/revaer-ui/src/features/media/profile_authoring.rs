//! Complete profile drafts and exact create confirmation; no implicit versions.

use revaer_api_models::media_root_contract::{
    ProfileVersionRequest, ProfileVersionRequestFields, ProfileVersionResponse,
};

use super::profile_roots::{ProfileRootDraft, ProfileRootKind, RootChoice, SelectionReadiness};

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ProfileDraft {
    pub key: String,
    pub name: String,
    pub description: String,
    pub target_key: String,
    pub target_version: String,
    pub policy_key: String,
    pub policy_version: String,
    pub enabled: bool,
    pub dry_run_only: bool,
}

impl Default for ProfileDraft {
    fn default() -> Self {
        Self {
            key: String::new(),
            name: String::new(),
            description: String::new(),
            target_key: String::new(),
            target_version: String::new(),
            policy_key: String::new(),
            policy_version: String::new(),
            enabled: false,
            dry_run_only: true,
        }
    }
}

impl ProfileDraft {
    pub(crate) fn from_request(request: &ProfileVersionRequest) -> Self {
        let fields = request.fields();
        Self {
            key: fields.profile_key.clone(),
            name: fields.display_name.clone(),
            description: fields.description.clone(),
            target_key: fields.desired_target_key.clone(),
            target_version: fields.desired_target_version.to_string(),
            policy_key: fields.policy_key.clone(),
            policy_version: fields.policy_version.to_string(),
            enabled: fields.enabled,
            dry_run_only: fields.dry_run_only,
        }
    }
    pub(crate) fn request(
        &self,
        roots: &ProfileRootDraft,
        choices: &[RootChoice],
    ) -> Result<ProfileVersionRequest, &'static str> {
        for kind in ProfileRootKind::ALL {
            match roots.readiness(kind, choices) {
                SelectionReadiness::Ready { .. } | SelectionReadiness::Absent => {}
                _ => {
                    return Err(
                        "Select binding-ready roots before saving. Your draft is preserved.",
                    );
                }
            }
        }
        let optional = |kind| {
            let key = roots.selected(kind);
            (!key.is_empty()).then(|| key.to_owned())
        };
        ProfileVersionRequest::new(ProfileVersionRequestFields {
            profile_key: self.key.clone(),
            display_name: self.name.clone(),
            description: self.description.clone(),
            enabled: self.enabled,
            dry_run_only: self.dry_run_only,
            desired_target_key: self.target_key.clone(),
            desired_target_version: self
                .target_version
                .parse()
                .map_err(|_| "Select an explicit positive target version.")?,
            policy_key: self.policy_key.clone(),
            policy_version: self
                .policy_version
                .parse()
                .map_err(|_| "Select an explicit positive policy version.")?,
            output_root_key: roots.selected(ProfileRootKind::Output).to_owned(),
            workspace_root_key: roots.selected(ProfileRootKind::Workspace).to_owned(),
            backup_root_key: optional(ProfileRootKind::Backup),
            quarantine_root_key: optional(ProfileRootKind::Quarantine),
        })
        .map_err(|_| "Check the profile fields and explicit versions. Your draft is preserved.")
    }
}

pub(crate) fn confirm_creation(
    response: &ProfileVersionResponse,
    etag: &str,
    request: &ProfileVersionRequest,
) -> Result<(), &'static str> {
    let fields = response.fields();
    let expected_tag = format!("\"media-profile:{}:v1\"", fields.media_profile_public_id);
    if &fields.profile != request
        || fields.latest_version != 1
        || fields.active_version != Some(1)
        || etag != expected_tag
    {
        return Err(
            "Save returned inconsistent confirmation. Check existing profiles before resubmitting. Your draft is preserved.",
        );
    }
    Ok(())
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ProfileEdit {
    pub id: uuid::Uuid,
    pub version: i32,
    pub etag: String,
    pub profile: ProfileVersionRequest,
}

impl ProfileEdit {
    pub(crate) fn loaded(
        id: uuid::Uuid,
        response: &ProfileVersionResponse,
        etag: String,
    ) -> Result<Self, &'static str> {
        let fields = response.fields();
        if fields.media_profile_public_id != id
            || etag != format!("\"media-profile:{id}:v{}\"", fields.latest_version)
        {
            return Err(
                "The server returned inconsistent profile version information. The draft is preserved.",
            );
        }
        Ok(Self {
            id,
            version: fields.latest_version,
            etag,
            profile: fields.profile.clone(),
        })
    }

    pub(crate) fn confirm_replacement(
        &self,
        request: &ProfileVersionRequest,
        response: &ProfileVersionResponse,
        etag: &str,
    ) -> Result<i32, &'static str> {
        let next = self
            .version
            .checked_add(1)
            .ok_or("Profile version limit reached. The draft is preserved.")?;
        let fields = response.fields();
        if fields.media_profile_public_id != self.id
            || fields.latest_version != next
            || fields.active_version != Some(next)
            || &fields.profile != request
            || etag != format!("\"media-profile:{}:v{next}\"", self.id)
        {
            return Err(
                "Save returned inconsistent confirmation. Check existing profiles before resubmitting. The draft is preserved.",
            );
        }
        Ok(next)
    }
}

#[cfg(test)]
mod tests;

pub(crate) const fn mutation_failure(status: u16) -> &'static str {
    match status {
        401 => "Authentication required. The unsaved profile is preserved.",
        403 => "Access denied. The unsaved profile is preserved.",
        404 | 405 => "Profile version creation is unavailable. The draft is preserved.",
        400 | 413 => "The server rejected the profile configuration. Review the preserved draft.",
        409 | 412 => {
            "Configuration changed or conflicts with this profile. Reload and review the preserved draft before resubmitting."
        }
        428 => "The server rejected the create precondition. The draft is preserved.",
        _ => {
            "Save could not be confirmed. Check existing profiles before resubmitting; the draft is preserved."
        }
    }
}
