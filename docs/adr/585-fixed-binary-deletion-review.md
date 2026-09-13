# Fixed binary deletion review

> Historical proposal. The revised, content-bound ASSET-1 contract and explicit
> approval in [ADR 588](588-first-release-decision-package.md#approval-resolution)
> supersede this proposal's exception scope, not its retained evidence.

- Status: Superseded
- Superseded by: ADR 588 ASSET-1
- Date: 2026-09-10
- Operator approval: 2026-09-11 approval applies only to ADR 588's revised
  ASSET-1 scope at reviewed commit `9575c077`, not this earlier proposal wholesale.
- Implementation status: Local guard implementation is under validation below;
  no published checked head yet contains it and no canonical exception pass is claimed.

The pre-approval investigation below remains historical. Approval alone neither
changes the canonical guard nor establishes asset, review or merge acceptance.

## Problem And Existing Authority

The canonical [changed-line guard](../../scripts/database_rebaseline/contract.rb)
uses `git diff --no-renames --numstat BASE HEAD --`. Its `totals` method rejects
any nonnumeric addition/deletion field before calculating a total. A binary
deletion is `-`, `-`, path, not zero text lines. The
[stack contract](../../config/database-rebaseline.env) permits at most **9,999
text additions plus deletions**; 10,000 fails. The unchanged current guard also
rejects binary entries, independently of that numeric ceiling.

[Accepted ADR 522](522-pre-v1-single-init-script-transition.md) explicitly
requires binary/uncountable diffs to fail closed.
[Recorded ADR 528](528-pre-v1-init-freeze-guard.md) implements that rule.
[ADR 581](581-stack-boundary-reconciliation.md) explicitly records PR 130 as
blocked with **no approved or implemented binary exception**.
[Accepted ADR 379](379-ui-runtime-image-canonicalization.md) authorizes unused
raster deletion and deterministic SVG replacement, subject to equivalent
validated runtime imagery. That asset approval does not authorize different
review-size accounting. [ADR 378](378-ui-vendor-image-input-pruning.md) records
the earlier unused-vendor cleanup, not a guard exception. No authorized bypass
was found in these decisions or the current root/UI/Rust/DevOps/Sonar rules.

## Exact Boundaries

Read-only authenticated GitHub CLI inspection on 2026-09-10 confirms
[PR 130](https://github.com/VannaDii/revaer/pull/130) is open with one commit:

| Representation | Immediate Parent | Head |
| --- | --- | --- |
| PR 130 original | `6471074ea4da6a41b1c59b4450a2f0249a7dc467` | `a379717455aae39f005e4120fb59b1f5247bbc95` |
| Isolated asset replay | `945e81d610c320bdb62e3f3eb728e6f0a218e0da` | `2fdd81edf749c51d3689545b2deeb1e11224d9c0` |

These are two representations of **one asset deliverable**, not two independent
permissions to delete assets. The API base equals the original commit's parent;
the replay base equals its parent. No moving branch name or GitHub summary is
used as the diff authority. The PR title/body still describe only vendor input
pruning, but the actual commit also replaces runtime imagery. That narrower
description is not evidence of the full deletion scope; no remote text was edited.

The [machine-readable manifest](support/585-binary-deletions.json) records full
base/head/tree identities, every binary deletion's prior mode, Git blob OID,
SHA-256 and byte size, and every text path's exact no-rename numstat and old/new
blob identity. Its shared binary inventory is identical at both boundaries.
The inventory's defined canonical-JSON SHA-256 is
`3e1e4af169880c3aee57e967a01c58f9d4f368bb7ac9b72666622e7ce1918d7a`.
No binary payload is copied into this proposal or its private evidence.

| Kind, At Each Boundary | Added Paths | Modified Paths | Deleted Paths |
| --- | ---: | ---: | ---: |
| Binary | 0 | 0 | 213 |
| Text | 27 | 13 | 56 |
| Total | 27 | 13 | 269 |

There are **309 changed paths**, **411 added text lines**, **2,070 deleted text
lines**, and **2,481 changed text lines** at each boundary. The 56 text deletions
are vendor SVG files and remain fully counted. The 213 binary deletions total
**21,064,113 original bytes**; these are per-path bytes, including duplicated
vendor/runtime copies, not unique-content bytes or a text-line equivalent.
The historical ADR 581 figures therefore remain exact at the refreshed SHAs.

| Binary Group | Paths | Original Bytes |
| --- | ---: | ---: |
| Branding/icons, including both root and runtime raster logos | 27 | 2,814,198 |
| Runtime `static/nexus/images` | 62 | 6,083,305 |
| Vendor `html/images` | 62 | 6,083,305 |
| Vendor `public/images` | 62 | 6,083,305 |

Both canonical guard invocations exit 1 at the same first entry:
`Git reported a binary or uncountable changed-line entry:` followed by
`-`, `-`, `crates/revaer-ui/favicon.ico`. **2,481 is a text subtotal, not a
passing current guard result or a measure of total binary review effort.**

## Asset Preservation Evidence And Limits

Static inspection covers 285 tracked text files under the UI, its vendor and
generator paths, chart consumers, and the UI recipe path. Original/replay UI,
chart and replacement root-logo trees are byte-identical. The per-path private
reference inventory includes positive matches, not merely successful searches.

- Only two UI runtime Rust files change: `src/components/shell.rs` changes logo
  extensions (2 added/2 deleted lines); `src/features/dashboard/logic.rs` changes
  queue/event extensions and their existing assertions (4/4). No runtime source,
  route, component, translation, or feature module is deleted by this boundary.
- All 20 dynamically selected avatar/product SVG targets exist. All seven
  distinct served-CSS relative image targets exist, including retained SVGs.
  Root/chart and shell logo references switch to SVG; favicon, manifest and tile
  definitions switch to the committed application SVG icon.
- `asset_sync` stops copying `html/images`, requires the committed runtime image
  directory, and still copies vendor CSS/JS. Served CSS equals vendor CSS;
  the served DataTables file exactly equals the vendor file plus the generator's
  declared row-mapping transform. No image generator or external fetch replaces
  the removed vendor directories. The 56 removed vendor SVGs are not omitted
  from review by calling them binaries.
- The 50 remaining served DataTables raster literals are recognized generator
  inputs. At these exact heads the inserted transform changes `.png` to `.svg`
  but leaves `/images/avatars/` instead of `/static/nexus/images/avatars/`.
  Icon/manifest/tile URLs similarly omit `/static`, while Trunk copies `static`
  as a directory. Thus static file presence is **not** proof those emitted URLs
  resolve. Two vendor DataTables input files also retain the legacy literals;
  vendor HTML JS is not the copy source, while public JS is canonicalized.
- Existing successor `4681b5c4`, replayed as
  `e978793b030a635263adb0127449bf90990839ae`, corrects those URL forms and
  strengthens asset validation. Its static DataTables and manifest corrections
  were checked read-only. Later success cannot certify the earlier boundary,
  and this proposal neither replays it nor changes either asset commit.

There is no hidden runtime-source deletion in the inspected diff, but an
unconditional claim of **no asset/feature loss is not established**. Photography
and unused demonstrations are deliberately removed; SVG illustrations are not
pixel-equivalent. ADR 379 still requires validated replacements, branding and
licensing preservation, desktop/mobile visual review, and release-output URL
checks. Platform-specific icon support is also not proven by source inspection.
The known intermediate URL defects and those acceptance obligations remain
blocking; this task does not run a compiler, synchronizer, browser or media test.

## Corrected Candidate Checkpoint

The later local reconstruction `3892b126` combines the asset conversion with
its existing hardening successor and brings forward the exact approved
`1462d18e` release-logo correction. Its parent is `945e81d6`; its complete
text subtotal is 3,123 lines across 317 changed paths. Independent blob checks
prove the same 213 binary deletions and the same inventory digest above.
The UI runtime tree matches `e978793b` exactly, while the repository-root logo
matches the canonical runtime logo and the approved release correction.

The candidate's Trunk output serves all 22 checked SVG URLs over HTTP with
correct MIME type and exact source bytes; icon, manifest, tile and DataTables
references pass. Full UI validation passes 101 tests. Follow-up `0d725dce` adds
three malformed-asset regression tests without changing production code. Full
CI then passes, including all 16 package coverage gates, but 40 shutdown WARN
records remain. Mobile/platform-icon acceptance is not claimed. This improves
the implementation evidence but does not authorize a
binary exception or establish all visual/feature-preservation requirements.

The original/replay identities in the proposal manifest remain historical and
are not silently replaced. Neither `3892b126` nor `0d725dce` is covered by the
proposed exception:
its complete boundary must be added to a revised manifest and the exact consent
request reviewed before any exception can apply. No approval has been received
for any representation, and the unchanged guard still rejects this candidate.

## Options And Recommendation

1. **Preserve the current block.** Make no policy change. Both exact asset
   boundaries remain rejected regardless of the small text subtotal. Further
   splitting the same binary deletions does not solve a guard that rejects even
   one such entry. This is the operative policy unless explicit consent arrives.
2. **Approve a fixed deletion-only exception for this asset deliverable.**
   Recommended for review-size handling only, subject to the conditions below.
   Review the binary removals by exact identity and asset-preservation evidence,
   separately from the unchanged numeric text gate. This is a narrow relaxation
   of binary rejection, not a finding that binaries have zero review cost.

Do not invent bytes-per-line, file-count-to-line, or another arbitrary numeric
conversion. The actual byte inventory is evidence, not a replacement ceiling.
Restoring rasters, changing formats again, removing features, or loosening Sonar
to avoid this blocker is not authorized by either option.

## Proposed Exception Boundary

Only after explicit decision-specific operator approval, a separately reviewed
implementation could recognize the selected exact representation above and:

- Require the complete 213-entry manifest set, each exact path, regular-file
  mode, prior blob OID, SHA-256 and byte length, and absence at the head.
  Reject additions, modifications, renames, symlinks, unknown paths, missing
  entries, partial matches, differing blobs, and any other uncountable entry.
- Keep `--no-renames` and the **9,999 additions-plus-deletions text maximum**
  unchanged across the complete deliverable, including all text replacements,
  deleted SVGs, documentation, manifest and any later approved guard changes.
  Do not skip text rows or report this as the old all-entry gate passing.
- Limit use to the one selected original/replay asset deliverable, not an
  ancestor-to-tip aggregate, unrelated PR, or future deletion. A changed
  base/head or changed manifest requires renewed exact-boundary review and
  explicit approval before use; matching names, totals or a subset cannot qualify.
- Expire on that selected deliverable's merge (or abandonment, if earlier).
  The unselected representation grants no second use. Remove any temporary
  implementation through reviewed cleanup after the merge; no future general
  binary bypass survives, and stale identities always fail closed.
- Preserve every other required check and all Sonar/workflow/security criteria.
  Consent to this exception would not approve unresolved asset defects, substitute
  for exact-boundary full gates, authorize a push/merge, or release D4, D5 or S2.

This commit changes **no executable policy or approval state**. The current
guard must still reject both asset boundaries. Any later implementation and
its negative tests require their own bounded review; this proposal is not a
manual instruction to override a failed required check.

## Exact Approval Question

Do you approve a one-deliverable, deletion-only exception to binary rejection
for PR 130's original or its listed replay boundary, limited to the exact 213
path/blob/SHA-256/size identities in manifest inventory
`3e1e4af169880c3aee57e967a01c58f9d4f368bb7ac9b72666622e7ce1918d7a`,
with the complete 9,999-text-line gate unchanged, expiry on the selected
deliverable's merge or abandonment, no future binary bypass, and no waiver of
asset correctness, required checks, Sonar criteria, or D4/D5/S2 holds?

## Task Record

- Motivation: make the inherited binary-deletion blocker reviewable without
  treating existing asset approval as permission to relax a different rule.
- Design notes: work starts at `b9198e14240550202dae64faa03b721cdd9a843f` in
  the isolated binary-deletion review branch. The clean replay candidate
  `27130e90ceb0ac90355ff45a659096596242b0ce` and original refs are preserved.
- Test coverage summary: exact parent/tree/numstat inventory and prior blob
  hashes are checked with Git and Node 24.19.0 through Just; the two unchanged
  canonical guards reproduce the expected failure. Structured manifest, scoped
  references/generator transform, documentation links, instruction drift and
  whitespace checks cover this documentation-only proposal. Full Linux CI/UI,
  native compilation, visual/media and Sonar validation remain parent-owned
  and are not claimed here.
- Bounded results: independent manifest validation passes for all 309 paths at
  both boundaries, including all 213 prior binary identities. Offline Lychee
  0.24.2 reports 1,037 OK, zero errors and one excluded external link across the
  proposal/index/summary. Instruction drift and whitespace checks pass. The
  existing prebuilt documentation indexer runs through Just without compilation;
  generated catalogue comparison proves only ADR 585 and its generation timestamp
  were added/updated, with all previous entries unchanged. These scoped checks
  are not substitutes for canonical full documentation/CI/UI gates.
- Observability updates: private CLI snapshot, raw numstats, complete text diffs,
  guard logs, reference hits and validation output are retained under
  `/private/tmp/revaer-binary-deletion-evidence-20260910`. No production telemetry
  or remote state changes.
- Status-doc validation: update the ADR index, book summary and generated
  catalogues without reclassifying any existing ADR or approval.
- Risk & rollback plan: fixed blob identity proves deletion identity, not
  semantic safety. Keep both boundaries blocked pending consent, implementation
  review and their remaining asset/gate evidence. Reject this proposal without
  source rollback; revert only its documentation commit if it is withdrawn.
- Dependency rationale: no dependency, generator, alias, replacement or version
  change; use existing Git, authenticated read-only GitHub CLI, Node and Just.
- Stale-policy check: reviewed `AGENTS.md`, Rust/UI/DevOps/Sonar scoped rules,
  the task template, ADRs 378/379/522/528/581, guard source/configuration and
  documentation recipes. No criteria edit is made. The stale PR scope claim and
  the distinction between approved asset work and unapproved size handling are
  recorded above, not silently corrected remotely. D4/D5 approval has been
  requested by the parent but not received; D4, D5 and S2 remain held.

## Approved Guard Implementation (2026-09-12)

This continuation implements only the ASSET-1 scope actually approved in
[ADR 588](588-first-release-decision-package.md#approval-resolution), at reviewed
`9575c077`. The historical proposal and inventory above are not broader consent.

- Motivation and design: unblock one review-accounting boundary without hiding
  text or authorizing different deletions. The ordinary canonical command uses
  NUL-delimited, no-rename Git numstat. Its only binary exception pins all 213
  inventory entries, checks raw deletion metadata and immutable blob bytes, and
  cross-checks complete raw/numstat path sets. Init assembly remains ineligible.
- Provider and expiry: the guard reads the exact repository/PR/head identity
  and complete unfiltered timeline through GitHub CLI. It validates pagination,
  counts, unique event identities and unchanged PR metadata across pages. Any
  close, merge or reopen permanently expires this exception. No cached receipt
  or caller-supplied provider payload is a canonical input.
- Coordinated consent: the checked head must contain the evaluated guard,
  its entry point/helpers, the actual ADR 588 consent/contract, this fixed
  inventory and the scoped instruction update. Their text remains counted at
  its owning review boundary. A diagnostic for an unpublished candidate is not
  a canonical pass, and a new base/head pair must match fresh provider state.
- Test coverage: 45 existing rebaseline assertions and 93 new guard assertions
  pass. New tests exercise
  exact inventory bytes from immutable Git objects, synthetic provider/head
  controls, paginated expiry, changed/malformed/duplicate/missing evidence,
  raw/numstat disagreement, and the 9,999/10,000 text boundary. The first legacy
  test exposed an unclassified missing-manifest error; it is corrected without
  accepting an unknown binary entry. A disposable real Git repository verifies
  all 213 deletions, immutable blobs, checked-head source and subsequent text
  corrections against an explicitly synthetic provider. Its files are removed
  on exit. `just policy` passed before that final real-Git case was added; the
  updated focused test passed afterward. Combined CI/UI results remain pending.
- Live evidence: PR 130 remains open at `6471074e..a3797174`. The new reader
  successfully reads all 26 unfiltered timeline entries with no expiry event.
  The filtered API returned an unfiltered total of 26 with zero selected nodes;
  that is not sufficient completeness proof and is not used by the guard.
- Sonar: full-file MAIN analysis of four changed Ruby files reported zero
  issues before the final real-Git test addition; that changed test source still
  needs reanalysis. The eight-file secrets upload was rejected by auto-review
  before process creation, including after the actual ADR 588 transfer record
  was checked. A fresh exact-payload permission request is outstanding; no
  alternate upload was used. The canonical scanner still lacks its required
  token. Auxiliary results are not canonical analysis, published coverage,
  full CI/UI, native-package or exact-restacked-PR acceptance.
- Observability: successful canonical use prints the full provider receipt,
  exact checked SHAs, inventory identity and original binary byte/count totals
  before the ordinary text subtotal. No runtime telemetry or external mutation.
- Risk and rollback: accounting does not prove asset correctness. Remove this
  single exception after close/merge, or revert the guard increment to restore
  universal binary rejection. Before any merge, re-read current refs/state and
  require the remaining asset, stack, review and quality gates.
- Dependency rationale: existing Git/GitHub CLI and Ruby standard libraries only.
- Stale-policy check: root, DevOps and Sonar rules and ADRs 522/528/588 reviewed.
  DevOps now names the exact approved exception and keeps init assembly strict;
  no unrelated criterion, approval, source visibility or threshold is changed.
