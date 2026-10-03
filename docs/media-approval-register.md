# Media Approval Register

Decision navigation as of 2026-10-02, including the operator's explicit approval
of ADRs 589-591 on 2026-09-15 and ADR 588 at reviewed commit `9575c077`. Linked ADRs and their exact
operator evidence remain authoritative. This register grants no consent and
does not certify implementation. Read the complete linked contract before work;
conditions and expiries are not discarded by a summary row.

## Current Simplification Resolution

ADRs [593](adr/593-simple-checkpoint-recovery.md) and
[594](adr/594-simple-discovery-worker-cleanup.md) replace the custom supervisor
and registration contract (588 S2/LIFE-1 and 589), distributed worker recovery
(512), cleanup ownership/obsolete-media retention (513), and the named discovery
frontier/quota/lease requirements. Revaer must execute the full workflow locally,
budget scratch, restart unfinished steps and promptly clean disposable media.
Distributed workers are future work, not approved scope. Historical rows below
do not restore superseded requirements. Unrelated approvals and quality gates
remain binding; removal of the detour is not release qualification.

## Recorded Approvals

| Decision | Status And Evidence | Scope / Conditions | Affected Work |
| --- | --- | --- | --- |
| R1-R4, B1-B3, narrow G1 | Approved 2026-09-09, [559](adr/559-media-approval-delta.md#approval-resolution), resolution `e8be2459`. | Exact root and broker contracts in [557](adr/557-root-persistence-contract.md)/[558](adr/558-rvb1-native-process-broker-wire-contract.md); G1 is internal semantics-preserving work only. E1 and other enumerated holds were not released. | L1-L10, especially root binding and native execution. |
| D1, D2 | Approved 2026-09-10, [569](adr/569-init-privilege-and-timeout-resolution.md#approval-resolution), reviewed `1d62d087`. | Exact stock-extension boundary/expiry and function-scoped reset timeout. Final-init proof and cutover gates remain required. | L1. |
| D3 | Conditionally approved in the same [569 resolution](adr/569-init-privilege-and-timeout-resolution.md#approval-resolution). | Independent cold/warm/helper ingestion proof required; not satisfied by shared legacy failures or partial parity. Does not approve D4/D5. | L1. |
| S1 | Approved 2026-09-10, [577](adr/577-runtime-shutdown-event-classification.md#approval-resolution). | Only requested cancellation classification. Unexpected cancellation/panic warnings remain; no watcher termination policy supplied. | Runtime observability. |
| F1 | Narrowly approved 2026-09-10, [578](adr/578-locked-fixture-diagnostic-disposition.md#approval-resolution). | Exact fixture identity, diagnostic and test-preparation contract. Drift expires it; original evidence remains. No production or Sonar exception. | L7/L10 fixture preparation. |
| Evidence-led delivery and record separation | Approved 2026-09-11: "Agreed, make it so", [587](adr/587-evidence-led-delivery-governance.md). | Eight preceding delivery recommendations; new routine records separated from architectural ADRs. All existing quality gates and decision holds remain. | Delivery process and verification planning, not runtime policy. |
| Isolated decision-evidence experiments | Authorized 2026-09-11 in the operator-activated revised goal; exact boundary in [588](adr/support/588-decision-details.md#research-authority-and-scope). | Bounded, disposable, unpublished, nonproduction experiments only. Resulting architecture and external uploads still require separate consent. | Consolidated resolution of known holds; no production activation. |

Earlier accepted architectural authority remains linked from
[559's retained dependencies](adr/559-media-approval-delta.md#remaining-holds)
and the [completion ledger](adr/564-media-completion-ledger.md). This is not a
retroactive reapproval or an exhaustive historical ADR re-audit.

## Accepted Implementation Scope

The operator explicitly approved choices 1-6 and separately authorized transfers
in 7 on 2026-09-11. The [dated resolution](adr/588-first-release-decision-package.md#approval-resolution)
pins the exact reviewed contracts and evidence. Older approval rows above record
what those earlier decisions did or did not authorize; ADR 588 now resolves
the named holds below without certifying implementation.

| Choice / Scope | Approval | Conditions Still Required |
| --- | --- | --- |
| 1: D4/D5 ingestion family | Approved; [exact changes](adr/support/588-decision-details.md#exact-recommendation-for-review) | Final-init-only scratch-table lifetime and IMDb/TMDB/TVDB predicate fixes. No canonical merge or frozen-reference change. Complete conditional D3 before cutover. |
| 2: C1/C1-D and E1 | Approved; E1 activation conditional; [startup contract](adr/support/588-decision-details.md#package-environment-and-compliance-startup) | Whole-service failure on invalid compliance metadata, verified immutable read-only bundle and exact 214-byte environment. Real current-source native amd64/arm64 closure and startup evidence before activation. |
| 3: S2/LIFE-1 | Superseded by accepted [593](adr/593-simple-checkpoint-recovery.md) and [594](adr/594-simple-discovery-worker-cleanup.md) | Ordinary cancellation/shutdown, checkpoint replay and interrupted final-replacement recovery require real service evidence; no custom owner protocol. |
| 4: DISC-1 through DISC-7 | Retained except exact deltas replaced by accepted [594](adr/594-simple-discovery-worker-cleanup.md) | Bounded rescans, duplicate prevention, identities and cancellation/recovery evidence; no distributed frontier/quota/lease qualification requirement. |
| 5: AUDIO-1 | Approved for implementation; [audio contract](adr/support/588-decision-details.md#audio-1-full-versioned-audio-contract) | Three exact versioned intents, music scalar delta and bitrate/cap semantics. Full codec/layout/rate, meter, short-form and operator listening acceptance. |
| 6: ASSET-1 | Narrow exception approved; [exact boundary](adr/support/588-decision-details.md#asset-1-content-bound-replay-and-permanent-expiry) | Only 213 content-bound binary deletions on PR 130; count/review all text normally. Fresh provider identity/history, exact content, permanent first-close/merge expiry and coordinated consent/guard/instruction implementation. The current guard is unchanged. |
| 7: External transfers | Separately authorized; [exact scope](adr/support/588-decision-details.md#separately-scoped-upload-request) | Reviewed GitHub implementation/evidence, exact Sonar organization/project, and gated existing GHCR Revaer release destinations through merge or earlier revocation. No secrets, test media, unpublished experiments, unrelated files, criteria/server/billing changes or deployment. |

## Remaining Gates

[ADR 591](adr/591-outside-in-first-release-path.md#approval-resolution) was
approved on 2026-09-15. It replaces pre-cutover feature-schema and legacy-parity
prerequisites with single-init application, security, transaction and workflow
qualification. Earlier rows retain historical evidence, not superseded release
prerequisites. Exclude unqualified D3 compiler substitutions; do not restart
legacy-equivalence or migration work for this unreleased product.

[ADR 590](adr/590-profile-version-wire-completion.md#approval-resolution) and
[ADR 589](adr/589-hash-helper-registration-deadline.md#approval-resolution) were
approved on the same date under their recommendations. Profile wire format and
enabled-state behavior remain approved, not qualified by that approval. ADR 589's
fingerprint registration contract is superseded by ADR 593.

On 2026-09-21 the operator approved the next milestone: save configuration through
the UI, restart the real service, and run a dry-run plan without modifying source
media. Work is single-agent only unless the operator explicitly authorizes a
change. The complete first-release scope and all quality gates remain required.

No named design choice in ADR 588 awaits another vote. Its conditional
qualification, original-preservation, full CI/UI, strict Sonar with positive
coverage, package, size, review and merge requirements remain unmet until
exact-revision evidence proves them. Approval of ASSET-1 alone does not enable
the exception or waive any unrelated guard.

Only a newly discovered or materially changed architectural choice returns for
decision-specific approval. Existing accepted work continues while independent
execution or verification problems are resolved. Record actual implementation,
local verification, package qualification and merge separately in the
[completion ledger](adr/564-media-completion-ledger.md).

ADR 592 records the operator's 2026-09-21 direction: do not prove root
exclusivity; lock or check active files, abandon missing inputs and refuse
replacement of changed inputs. The operator affirmed that approach with
"Great!". On 2026-10-01 its status was corrected to Accepted, removing the
agent-created approval hold. It replaces only ADR 550's native external-writer
assertion clauses. Notifications are optional; the residual external-writer race
is documented. Implementation and qualification remain outstanding.
See [ADR 592](adr/592-native-root-writer-assertion.md).
