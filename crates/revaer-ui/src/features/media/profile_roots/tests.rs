use super::*;

fn choices() -> Vec<RootChoice> {
    [
        RootKind::Source,
        RootKind::Output,
        RootKind::Workspace,
        RootKind::Backup,
        RootKind::Quarantine,
    ]
    .map(|kind| RootChoice {
        key: format!("{kind:?}").to_ascii_lowercase(),
        kind,
        binding_ready: true,
        destructive_ready: false,
    })
    .to_vec()
}

#[test]
fn defaults_never_choose_roots_or_require_optional_bindings() {
    let draft = ProfileRootDraft::default();
    for kind in ProfileRootKind::ALL {
        assert!(draft.selected(kind).is_empty());
        assert_eq!(
            draft.readiness(kind, &choices()),
            if kind.required() {
                SelectionReadiness::Required
            } else {
                SelectionReadiness::Absent
            }
        );
        assert!(!kind.label().is_empty());
    }
}

#[test]
fn selections_are_exact_kind_scoped_and_do_not_accept_paths() -> Result<(), &'static str> {
    let mut draft = ProfileRootDraft::default();
    let roots = choices();
    for kind in ProfileRootKind::ALL {
        let key = format!("{:?}", kind.kind()).to_ascii_lowercase();
        draft.select(kind, &key, &roots)?;
        assert_eq!(draft.selected(kind), key);
        for invalid in [
            "source",
            "/private/library",
            "../output",
            "output ",
            "unknown",
        ] {
            assert!(draft.select(kind, invalid, &roots).is_err());
            assert_eq!(draft.selected(kind), key);
        }
        draft.select(kind, "", &roots)?;
        assert!(draft.selected(kind).is_empty());
    }
    Ok(())
}

#[test]
fn readiness_updates_preserve_unsaved_selection_without_claiming_authority()
-> Result<(), &'static str> {
    let mut draft = ProfileRootDraft::default();
    let mut roots = choices();
    draft.select(ProfileRootKind::Output, "output", &roots)?;
    assert_eq!(
        draft.readiness(ProfileRootKind::Output, &roots),
        SelectionReadiness::Ready { destructive: false }
    );
    for root in &mut roots {
        root.destructive_ready = true;
    }
    assert_eq!(
        draft.readiness(ProfileRootKind::Output, &roots),
        SelectionReadiness::Ready { destructive: true }
    );
    for root in &mut roots {
        root.binding_ready = false;
    }
    assert_eq!(
        draft.readiness(ProfileRootKind::Output, &roots),
        SelectionReadiness::NotReady
    );
    assert!(
        draft
            .select(ProfileRootKind::Output, "output", &roots)
            .is_err()
    );
    assert_eq!(
        draft.readiness(ProfileRootKind::Output, &[]),
        SelectionReadiness::Unavailable
    );
    assert_eq!(draft.selected(ProfileRootKind::Output), "output");
    Ok(())
}

#[test]
fn all_readiness_messages_are_bounded_and_value_free() {
    for state in [
        SelectionReadiness::Required,
        SelectionReadiness::Absent,
        SelectionReadiness::Unavailable,
        SelectionReadiness::NotReady,
        SelectionReadiness::Ready { destructive: false },
        SelectionReadiness::Ready { destructive: true },
    ] {
        assert!(!state.message().is_empty());
        assert!(state.message().len() < 128);
        assert!(!state.message().contains('/'));
    }
}
