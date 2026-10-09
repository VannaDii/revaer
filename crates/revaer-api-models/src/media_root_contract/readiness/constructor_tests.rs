use std::num::NonZeroU64;

use super::{
    ROOT_KIND_ORDER, RootAttestationFailure, RootCatalogReadinessResponse,
    RootCatalogReadinessState, RootKindReadiness, RootReadinessError, RootSourceFailure,
};

fn counts(value: u16) -> [RootKindReadiness; 5] {
    ROOT_KIND_ORDER.map(|kind| RootKindReadiness {
        kind,
        attested_slot_count: value,
        binding_ready_slot_count: value,
        destructive_ready_slot_count: value,
    })
}

#[test]
fn constructor_preserves_all_closed_states() -> Result<(), Box<dyn std::error::Error>> {
    let sources = [
        RootSourceFailure::Missing,
        RootSourceFailure::Untrusted,
        RootSourceFailure::Invalid,
        RootSourceFailure::BoundExceeded,
        RootSourceFailure::Unsupported,
    ];
    for reason in sources {
        let state = RootCatalogReadinessState::SourceUnavailable(reason);
        let response = RootCatalogReadinessResponse::new(state, counts(0))?;
        assert_eq!(response.state(), state);
        assert_eq!(response.kinds(), &counts(0));
        assert_eq!(
            serde_json::from_value::<RootCatalogReadinessResponse>(serde_json::to_value(
                &response
            )?)?,
            response
        );
    }
    let failures = [
        RootAttestationFailure::Invalid,
        RootAttestationFailure::Overlap,
        RootAttestationFailure::UnsafeAncestry,
        RootAttestationFailure::DurabilityUnproven,
        RootAttestationFailure::WriterControlUnproven,
        RootAttestationFailure::IdentityMismatch,
    ];
    for reason in failures {
        let state = RootCatalogReadinessState::AttestationInvalid(reason);
        let response = RootCatalogReadinessResponse::new(state, counts(0))?;
        assert_eq!(response.state(), state);
        assert_eq!(
            serde_json::from_value::<RootCatalogReadinessResponse>(serde_json::to_value(
                response
            )?)?
            .state(),
            state
        );
    }
    Ok(())
}

#[test]
fn constructor_preserves_generations_without_javascript_number_rounding()
-> Result<(), Box<dyn std::error::Error>> {
    for generation in [1, 9_007_199_254_740_993, 9_223_372_036_854_775_807] {
        let state = RootCatalogReadinessState::Ready {
            generation: NonZeroU64::new(generation).ok_or("zero test generation")?,
        };
        for count in [0, 1, 255, 256] {
            let response = RootCatalogReadinessResponse::new(state, counts(count))?;
            assert_eq!(response.state(), state);
            assert_eq!(response.kinds(), &counts(count));
            let json = serde_json::to_value(response)?;
            assert_eq!(json["generation"], generation.to_string());
        }
    }
    Ok(())
}

#[test]
fn constructor_rejects_out_of_database_range_generation() -> Result<(), Box<dyn std::error::Error>>
{
    for generation in [9_223_372_036_854_775_808, u64::MAX] {
        assert_eq!(
            RootCatalogReadinessResponse::new(
                RootCatalogReadinessState::Ready {
                    generation: NonZeroU64::new(generation).ok_or("zero test generation")?,
                },
                counts(0),
            ),
            Err(RootReadinessError)
        );
    }
    Ok(())
}

#[test]
fn constructor_rejects_counts_without_a_generation() {
    for state in [
        RootCatalogReadinessState::SourceUnavailable(RootSourceFailure::Missing),
        RootCatalogReadinessState::AttestationInvalid(RootAttestationFailure::Invalid),
    ] {
        assert_eq!(
            RootCatalogReadinessResponse::new(state, counts(1)),
            Err(RootReadinessError)
        );
    }
}

#[test]
fn constructor_rejects_reordered_or_incoherent_counts() {
    let state = RootCatalogReadinessState::Ready {
        generation: NonZeroU64::MIN,
    };
    let mut reordered = counts(0);
    reordered.swap(0, 1);
    let mut excessive = counts(0);
    excessive[0].attested_slot_count = 257;
    let mut invalid_binding = counts(0);
    invalid_binding[0].binding_ready_slot_count = 1;
    let mut invalid_destructive = counts(1);
    invalid_destructive[0].destructive_ready_slot_count = 2;
    for invalid in [reordered, excessive, invalid_binding, invalid_destructive] {
        assert_eq!(
            RootCatalogReadinessResponse::new(state, invalid),
            Err(RootReadinessError)
        );
    }
}

#[test]
fn readiness_error_is_bounded_and_has_no_nested_source() {
    use std::error::Error as _;

    assert_eq!(
        RootReadinessError.to_string(),
        "media root readiness is invalid"
    );
    assert_eq!(format!("{RootReadinessError:?}"), "RootReadinessError");
    assert!(RootReadinessError.source().is_none());
}
