//! Path-free profile root selection. Catalog readiness is not write authority.

use revaer_api_models::media_root_contract::RootKind;

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct RootChoice {
    pub key: String,
    pub kind: RootKind,
    pub binding_ready: bool,
    pub destructive_ready: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProfileRootKind {
    Output,
    Workspace,
    Backup,
    Quarantine,
}

impl ProfileRootKind {
    pub(crate) const ALL: [Self; 4] = [
        Self::Output,
        Self::Workspace,
        Self::Backup,
        Self::Quarantine,
    ];

    pub(crate) const fn kind(self) -> RootKind {
        match self {
            Self::Output => RootKind::Output,
            Self::Workspace => RootKind::Workspace,
            Self::Backup => RootKind::Backup,
            Self::Quarantine => RootKind::Quarantine,
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Output => "Output root",
            Self::Workspace => "Workspace root",
            Self::Backup => "Backup root",
            Self::Quarantine => "Quarantine root",
        }
    }

    pub(crate) const fn required(self) -> bool {
        matches!(self, Self::Output | Self::Workspace)
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
pub(crate) struct ProfileRootDraft {
    output: String,
    workspace: String,
    backup: String,
    quarantine: String,
}

impl ProfileRootDraft {
    pub(crate) fn from_request(
        request: &revaer_api_models::media_root_contract::ProfileVersionRequest,
    ) -> Self {
        let fields = request.fields();
        Self {
            output: fields.output_root_key.clone(),
            workspace: fields.workspace_root_key.clone(),
            backup: fields.backup_root_key.clone().unwrap_or_default(),
            quarantine: fields.quarantine_root_key.clone().unwrap_or_default(),
        }
    }
    pub(crate) fn selected(&self, kind: ProfileRootKind) -> &str {
        match kind {
            ProfileRootKind::Output => &self.output,
            ProfileRootKind::Workspace => &self.workspace,
            ProfileRootKind::Backup => &self.backup,
            ProfileRootKind::Quarantine => &self.quarantine,
        }
    }

    pub(crate) fn select(
        &mut self,
        kind: ProfileRootKind,
        key: &str,
        choices: &[RootChoice],
    ) -> Result<(), &'static str> {
        if !key.is_empty()
            && !choices.iter().any(|choice| {
                choice.kind == kind.kind() && choice.key == key && choice.binding_ready
            })
        {
            return Err("Select a binding-ready root of the required kind.");
        }
        let selected = match kind {
            ProfileRootKind::Output => &mut self.output,
            ProfileRootKind::Workspace => &mut self.workspace,
            ProfileRootKind::Backup => &mut self.backup,
            ProfileRootKind::Quarantine => &mut self.quarantine,
        };
        key.clone_into(selected);
        Ok(())
    }

    pub(crate) fn readiness(
        &self,
        kind: ProfileRootKind,
        choices: &[RootChoice],
    ) -> SelectionReadiness {
        let key = self.selected(kind);
        if key.is_empty() {
            return if kind.required() {
                SelectionReadiness::Required
            } else {
                SelectionReadiness::Absent
            };
        }
        match choices
            .iter()
            .find(|choice| choice.key == key && choice.kind == kind.kind())
        {
            Some(choice) if choice.binding_ready => SelectionReadiness::Ready {
                destructive: choice.destructive_ready,
            },
            Some(_) => SelectionReadiness::NotReady,
            None => SelectionReadiness::Unavailable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectionReadiness {
    Required,
    Absent,
    Unavailable,
    NotReady,
    Ready { destructive: bool },
}

impl SelectionReadiness {
    pub(crate) const fn message(self) -> &'static str {
        match self {
            Self::Required => "Required root not selected.",
            Self::Absent => "No optional root selected.",
            Self::Unavailable => {
                "Selected key is unavailable for this kind. Unsaved selection retained."
            }
            Self::NotReady => {
                "Selected root is no longer binding-ready. Unsaved selection retained."
            }
            Self::Ready { destructive: false } => {
                "Binding ready. Destructive operations are not ready."
            }
            Self::Ready { destructive: true } => {
                "Binding ready. Destructive root checks are ready."
            }
        }
    }
}

#[cfg(test)]
mod tests;
