#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum JobAction {
    Cancel,
    Retry,
}

impl JobAction {
    pub(crate) fn for_status(status: &str) -> Option<Self> {
        match status {
            "queued" | "running" | "verifying" => Some(Self::Cancel),
            "failed" | "cancelled" => Some(Self::Retry),
            _ => None,
        }
    }

    pub(crate) const fn path_suffix(self) -> &'static str {
        match self {
            Self::Cancel => "cancel",
            Self::Retry => "retry",
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Cancel => "Cancel job",
            Self::Retry => "Retry job",
        }
    }

    pub(crate) const fn confirmation(self) -> &'static str {
        match self {
            Self::Cancel => "Confirm cancellation",
            Self::Retry => "Confirm retry",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::JobAction;

    #[test]
    fn actions_match_service_transitions_and_fail_closed() {
        for status in ["queued", "running", "verifying"] {
            assert_eq!(JobAction::for_status(status), Some(JobAction::Cancel));
        }
        for status in ["failed", "cancelled"] {
            assert_eq!(JobAction::for_status(status), Some(JobAction::Retry));
        }
        for status in ["completed", "unknown", "", "FAILED"] {
            assert_eq!(JobAction::for_status(status), None);
        }
        assert_eq!(JobAction::Retry.path_suffix(), "retry");
        assert_eq!(JobAction::Cancel.path_suffix(), "cancel");
        assert_eq!(JobAction::Retry.label(), "Retry job");
        assert_eq!(JobAction::Cancel.label(), "Cancel job");
        assert_eq!(JobAction::Retry.confirmation(), "Confirm retry");
        assert_eq!(JobAction::Cancel.confirmation(), "Confirm cancellation");
    }
}
