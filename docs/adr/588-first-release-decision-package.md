# First-Release Approval Proposal

- Status: Superseded
- Superseded by: [ADR 593](593-simple-checkpoint-recovery.md). Its recovery
  decision replaces the custom supervisor machinery; unrelated approved choices
  and transfer permissions remain in force as explicitly retained there.
- Date: 2026-09-11
- Operator approval: 2026-09-11: "Approved", in direct reply to the annotated
  approval question for choices 1-6 and, separately, transfers in 7.
- Supersedes: Only the pending decisions and exact contract deltas identified
  below; unrelated approvals, conditions and holds remain binding.
- Implementation status: Approved work may proceed; release qualification incomplete.

## Approval Resolution

The operator reviewed commit `9575c0770a86421857335da4ae36dc4cbbae4f6a`
and explicitly approved both parts of the question: design choices 1-6 and the
separate transfer scope in 7. The contracts and appendices at that commit are
the approved boundary; the brief below preserves the reviewed recommendations.

- Design: D4/D5 ingestion family; C1/C1-D and conditional E1; S2/LIFE-1;
  DISC-1 through DISC-7; AUDIO-1; narrowly scoped ASSET-1.
- Transfers: Reviewed implementation/evidence to GitHub `VannaDii/revaer`,
  canonical and per-file analysis to Sonar `vannadii/VannaDii_Revaer`, and
  gated release artifacts to existing Revaer `ghcr.io/vannadii` destinations,
  through merge or earlier revocation. All stated exclusions still apply.
- Conditions: D3 proof, native qualification, listening acceptance and all
  release gates remain required. ASSET-1 may be used only with its exact
  provider/content/expiry checks and coordinated consent, guard and instruction
  implementation; this record alone does not change the size guard.

This releases the named design holds, not validation failures. No deployment,
broader criteria exception or assertion of readiness is authorized.

## Recommendation

Approve the six design choices below, and answer the upload request separately.
Together they provide an implementation path through the known holds without
reducing the first-release scope. Approval permits implementation, not release.

The linked [technical details](support/588-decision-details.md) define the exact
contracts, alternatives, evidence and rollback. This brief changes their
presentation, not their substance.

## 1. Fix Database Ingestion

**Recommend:** Make the policy scratch table disappear on commit and correct
the IMDb/TMDB/TVDB conflict predicates in the final init script.

**Tradeoff:** This fixes the reproduced failures without redesigning ingestion.
Repeated calls within one explicit transaction remain unsupported. Frozen
migrations stay unchanged; conditional D3 proof still gates the init cutover.

**Question:** Approve these scoped D4/D5 fixes?
[Exact changes](support/588-decision-details.md#exact-recommendation-for-review).

## 2. Require Verified Startup

**Recommend:** Refuse to start the entire service when compliance metadata is
missing or invalid. Use an operator-prepared, immutable, read-only bundle bound
to the exact platform image, plus the proposed fixed nine-variable environment.

**Tradeoff:** Installation needs prepared storage and architecture-specific Helm
releases. There is no degraded fallback; unrelated APIs also remain unavailable
until setup is correct.

**Question:** Approve C1/C1-D and conditional E1, with native amd64/arm64
qualification required before activation?
[Startup contract](support/588-decision-details.md#package-environment-and-compliance-startup).

## 3. Bound Shutdown And Require Safe Recovery

**Recommend:** Use an independent Linux supervisor and a 1,024-task ceiling.
Target forced termination at 28 seconds inside the 30-second application budget;
retain the 45-second external and earlier at-most-5-second native deadlines.
Couple 5-second heartbeats/40-second shared leases with bounded retries and proof
that the old owner stopped before replacement work begins.

**Tradeoff:** Clean handoffs can recover automatically. Uncertain crashes require
an authenticated operator confirmation backed by external termination evidence.
Control and logging limits can stop service under overload. Timing still needs
full native qualification; experimental success is not a universal guarantee.

**Question:** Approve S2/LIFE-1, including the operator-assisted recovery path?
[Shutdown contract](support/588-decision-details.md#s2-exact-lifecycle-and-recovery-proposal)
and [recovery values](support/588-decision-details.md#life-1-coupled-lease-recovery-and-retry-values).

## 4. Make Discovery Resumable And Capacity-Limited

**Recommend:** Persist discovery progress, identities and quotas in normalized
tables. Use bounded read-only helpers. Start with 256 GiB primary-file and
260 GiB aggregate limits; permit configured increases only within the proposed
1 TiB/1,040 GiB hard limits.

**Tradeoff:** Restart safety adds persistence complexity. Oversized work is
rejected explicitly; slow-storage performance remains to be qualified.

**Question:** Approve DISC-1 through DISC-7 and their exact admission limits?
[Discovery contract](support/588-discovery-proposal.md) and
[complete limits](support/588-discovery-values.md).

## 5. Adopt Three Audio Intents

**Recommend:** Use measured gain-only preservation for music, moderate
compression/normalization for dialog, and stronger processing for speech.
Retain configurable per-channel bitrate and an optional average-bitrate cap.

**Tradeoff:** Music explicitly changes the earlier two-pass transformation
contract. All supported codecs/layouts/rates, independent measurements and
operator listening acceptance still require qualification.

**Question:** Approve AUDIO-1's three versioned contracts, including that music
change?
[Audio contract](support/588-audio-proposal.md).

## 6. Allow One Asset-Removal Exception

**Recommend:** Permit only the exact 213 binary deletions on PR 130. Count and
review all text normally. Bind the exception to verified PR/content identity;
expire it permanently at first close or merge.

**Tradeoff:** This is a narrow review-size exception. It changes no other size,
Sonar, review or required-check rule.

**Question:** Approve ASSET-1 for that PR and those deletions only?
[Exact boundary](support/588-decision-details.md#asset-1-content-bound-replay-and-permanent-expiry).

## 7. Authorize Transfers Separately

**Recommend:** Permit approved implementation iterations to upload reviewed
source and required evidence to GitHub `VannaDii/revaer`, canonical analysis
inputs to Sonar `vannadii/VannaDii_Revaer`, and verified release artifacts to
existing Revaer destinations under `ghcr.io/vannadii`, through merge or earlier
revocation.

**Tradeoff:** This sends project material to those services. Secrets, test media,
unpublished experiments, unrelated files, criteria changes and deployment remain
excluded.

**Question:** Separately approve those three transfer scopes?
[Exact permissions](support/588-decision-details.md#separately-scoped-upload-request).

## After Approval

Keep dry-run enabled by default. First finish database/configuration and a safe
discovery-to-plan dry-run.
Parallelize approved package/lifecycle work, then integrate execution,
verification, replacement and recovery. Finish the full supported-feature matrix
and merge the reviewed linear stack only after all existing gates pass.

The [full goal](support/588-decision-details.md#updated-goal) remains unchanged:
complete the production-usable v0 service, with positive Sonar coverage, clean
CI/UI, both native packages and resolved feedback. No feature or gate is waived.
[Execution and validation record](support/588-decision-details.md#task-record).
