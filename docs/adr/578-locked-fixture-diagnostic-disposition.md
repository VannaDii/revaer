# Locked fixture diagnostic disposition

- Status: Accepted
- Date: 2026-09-10
- Operator approval: 2026-09-10: "Approve **D1, D2, conditional D3, S1, and narrowly scoped F1**. Keep **S2 held** pending a defensible shutdown bound."
- Context:
  - ADRs 575-576 establish that strict preparation rejects one locked upstream
    fixture before real Rust conversion execution. Host FFprobe 9.0.1 and the
    historical Linux arm64 image's 8.0.1 emit the same element-ID error.
  - ADR 559 G1 reserves every quality-criterion change for explicit consent.
    Accepting this known error would change preparation admission, even with
    stronger assertions around it. It is not an internal implementation detail.
- Decision: F1 accepted only within the exact test-only identity, diagnostic,
  tool-profile, evidence-retention and expiry conditions below.
- Consequences:
  - F1 would admit this particular recoverable probe observation into subsequent
    validation without declaring the whole input valid or harmless.
  - Additional diagnostics, failed decode/conversion, missing evidence, or other
    fixtures would remain failures. F1 alone cannot establish a conversion pass.
- Follow-up: Implement only F1's exact scope and rerun real
  preparation/conversion and the complete quality gates.

## Approval Resolution

The operator accepted the narrowly scoped F1 proposal at commit `1d62d087`.
This explicitly authorizes the one preparation-criterion exception, not broad
diagnostic allowance, a changed fixture, omitted evidence, a new tool profile,
production error suppression or any Sonar/GitHub relaxation. The reviewed
proposal wording below records the scope and prior hold; it does not leave F1
pending. Tool/source drift still expires the exception. No preparation,
conversion, current-package or remote-check pass follows from approval.

## Evidence And Requested Scope

The exact source is the raw `test_files/test4.mkv` from
`ietf-wg-cellar/matroska-test-files` commit
`e6965e5ca666322ed93e2748a10a4f132309e005`. The repository fixture ID is
`mkv-theora-vorbis-live-style`, SHA-256
`43df750a2a01a37949791b717051b41522081a266b71d113be4b713063843699`, exactly
21,313,902 bytes. The locked source and reviewed probe snapshot must not change.

[ADR 576](576-pinned-ffprobe-fixture-diagnostic.md) retains same-byte probes,
versions, raw stderr, container isolation and upstream source interpretation.
Both probes exit zero and exactly match the reviewed Theora/Vorbis projection,
while emitting `AV_LOG_ERROR` for a five-byte element ID at offset 35. Those
observations are not complete decode, playback, current-package, or amd64 proof.

## F1: Exact Test-Only Diagnostic Contract

Recommendation, not authorization: preserve this locked input as a recovery
fixture and permit only the observed single diagnostic during fixture
preparation, with all of the following conditions:

1. Run the existing immutable acquisition checks first. Require the exact
   fixture ID, path, revision, SHA-256 and byte count above, and the unchanged
   reviewed canonical stream snapshot. Wrong or missing identity remains fatal.
2. Keep the existing successful-process-status and 4,096-byte overall stderr
   ceiling. No exception applies to a nonzero process exit, missing/invalid JSON,
   stream mismatch, missing file, or any other preparation error.
3. Empty stderr keeps its existing strict acceptance behavior. Otherwise require
   exactly one LF-terminated ASCII line, no preceding/following bytes, and at
   most 131 bytes. Only the context pointer may vary: `0x` followed by 1-16
   lowercase hexadecimal digits. The rest must match exactly:

   ```text
   [matroska,webm @ <pointer>] Length 5 indicated by an EBML number's first byte 0x0a at pos 35 (0x23) exceeds max length 4.
   ```

4. Bind nonempty acceptance to the evidenced test-tool version scopes: FFprobe
   8.0.1 from the historical Alpine 8.0.1-r1 package or the observed Homebrew
   9.0.1 build, with the exact version-report profiles below. Capture the full
   tool/build report each time. Other versions
   receive no new exception; their empty-stderr path remains unchanged. This
    does not select or approve the service's E1 production environment.
5. Retain the original stderr and tool identity as nonempty evidence whenever
   this exception is used, and report its explicit fixture classification/count.
   Do not remove the log, discard its address-bearing original, relabel the
   native error as a warning, or imply the diagnostic never occurred.
6. Do not set the existing broad `allowProbeDiagnostics: true` flag for this
   fixture. Implement a closed, identity-bound contract, not a manifest-supplied
   arbitrary regular expression or permission for any bounded stderr. Reject
   conflicting policies and unknown contract identifiers. Existing allowances
   on other fixtures are outside F1 and are not broadened by this decision.
7. Keep every fixture and real conversion test enabled. The complete Rust
   fixture binary, ignored-suite execution, fresh-report checks, positive audio
   and video conversion counts, zero suite/pipeline failures, and cleanup still
   apply. Further failures require correction or their own explicit decision.

The exact unmodified `ffprobe -version` report profiles retained in ADR 576's
external evidence directory are:

| Profile | Report File | SHA-256 Of Full Report Bytes |
| --- | --- | --- |
| Historical Linux 8.0.1 | `linux-version.txt` | `8d4ba0f1aaef40839cba09f51dbcc0efbb58d1c65dc490385e46115bc8e0217c` |
| Local Homebrew 9.0.1 | `host-version.txt` | `abd50a4468578ece7323bce6d220a2c909c5a38ce7328929947dda86a8bfab41` |

These profiles constrain the observed build reports; they are not executable
attestations or complete native dependency-closure proof. F1 does not discharge
ADR 519's separate identity requirements or certify a current release image.

This knowingly broadens one preparation criterion. Approval is fixture/test
specific, not permission for the runtime inspector, broker, transcoder, output
verifier, Sonar scanner, or general application logs to ignore errors. It does
not authorize rewriting media, replacing the source, updating snapshots,
skipping cases, changing required checks, or claiming a whole-service pass.

### Expiry And Validation

F1 expires for any change to the locked source identity/bytes, reviewed probe
snapshot, accepted diagnostic text/framing, accepted tool/build identity, or
the use/scope of this exception. Additional versions or changed distributions
require new evidence and renewed explicit operator approval. An upstream
corrected source also requires a separately reviewed fixture update; it is not
silently substituted under this identity.

Before accepting a resulting patch, require regression cases rejecting wrong
identity, unknown/conflicting policy, changed offset/width/message, uppercase
or overlong pointer text, extra lines, missing final LF, embedded NUL/CR, nonzero
exit, malformed JSON, stream drift, lost raw evidence, and excess bytes. Verify
that every other fixture still rejects unapproved stderr. Then rerun real
source acquisition, preparation and conversion with each approved tool scope.
This proposal contains no new test or runtime implementation.

The current PR fixture job installs distribution `ffmpeg` on `ubuntu-latest`
and records versions in its cache key. Its live runner version was not obtained
for this proposal. F1 does not assume that job uses either evidenced build or
promise that it will pass. A different emitting tool version remains held;
changing CI's tool provisioning needs its own reviewed scope and evidence.

## Alternatives

- Keep the current zero-unapproved-diagnostic rule and remain blocked on this
  source. This preserves criteria and is the default pending a decision.
- Replace or derive a clean positive live-stream sample and retain this locked
  source as a separately asserted negative/recovery case. This may separate
  conformance from recovery more clearly, but changes fixture semantics and
  requires its own exact generation/provenance and failure-path contract.
- Set broad diagnostic allowance, drop this fixture, or suppress stderr. These
  lose specificity/evidence and are not recommended or authorized.

## Task Record

- Motivation: Give the operator the exact scope, retained guarantees, expiry
  and remaining uncertainty for the observed conversion-preparation blocker.
- Design notes: F1 is a proposal to change one test admission criterion. It is
  deliberately not implemented, generalized, or represented as prior consent.
- Test coverage summary: Reuses ADR 576's retained observations; no source was
  reacquired for F1 and no F1-enabled test was executed. Restored-tree validation
  is recorded separately in ADR 577 and does not validate this proposal.
  New regression and full-gate requirements are listed above, not claimed met.
- Observability updates: None activated. Proposed acceptance must retain native
  stderr and explicit classification rather than manufacture quiet output.
- Status-doc validation: Update ADR index, SUMMARY, catalog and completion
  ledger with the hold. No public capability or production-readiness change.
- Risk & rollback plan: An exception could normalize an unexpected error or
  conceal a regression. Pin identity/text/tool scope, fail closed on drift,
  retain raw evidence and require new approval when it changes. Default remains
  the existing failing rule; no runtime rollback is needed for this proposal.
- Dependency rationale: No dependency or implementation added or approved.
- Stale-policy check: Reviewed root/devops instructions, ADR 559 G1, ADRs
  575-576, fixture lock/manifest, strict probe verifier and its regressions,
  the Rust suite, and PR fixture provisioning. No existing criterion, lock,
  snapshot, test selection, runtime policy or GitHub rule was changed.

## F1 Implementation Evidence (2026-09-10)

This section supersedes the preapproval task record's statements that F1 is
unimplemented. It records only the explicitly approved F1 exception, implemented
from integration base `826a8f8e8668a3e6a91e99ef84c12b0e31df5ac2` in the assigned
`work/media3-approved-fixture-diagnostic-20260910` branch. It grants no additional
approval and does not satisfy the remaining integrated handoff requirements.

- Design: `probeDiagnosticContract: "adr578-f1"` selects a closed shell helper.
  It checks the exact manifest/lock source identity and actual source bytes,
  pinned reviewed-snapshot digest, exact diagnostic framing and pointer bounds,
  and the two full tool-report hashes above. Unknown/null/conflicting policies
  fail; even deleting the contract cannot enable the broad allowance on this
  fixture. The snapshot-update path also refuses to replace the F1 snapshot.
- Observability: Successful F1 use retains byte-compared raw stderr and the
  entire version report with hash-bound classification/count JSON in a private
  unique evidence directory. Both original stderr and the full version report
  are also printed to the retained run log. Lost, empty, changed or unwritable
  evidence fails admission. Existing broad Chromium diagnostic handling is
  unchanged and its count remains separate from the F1 count.
- Dependencies: No new dependency. The implementation uses the existing Bash,
  jq, hash and core filesystem tools. Regression doubles support either of the
  existing SHA-256 command variants; they are not native executable attestation.
- Focused proof: `bash scripts/with-node.sh just test-fixture-scripts` passes
  70 F1 adversarial/acceptance cases plus existing acquisition/probe tests.
  Both complete reviewed tool-report profiles and minimum/maximum pointer
  lengths are exercised. Rejections cover identity/source/snapshot drift,
  unknown/conflicting policies, malformed/non-ASCII/NUL/CR/multiline/no-LF
  diagnostics, global byte limits, failed process status, invalid/empty JSON,
  stream drift, unapproved tool reports, and missing/corrupted raw evidence.
  These isolated plumbing cases use an explicitly synthetic source/hash and
  FFprobe double; they are not the real-media proof below.
- Real acquisition/generation: `bash scripts/with-node.sh just
  download-test-fixtures` acquired and verified all 22 immutable sources;
  `bash scripts/with-node.sh just generate-test-fixtures` passed. No source,
  lock, or reviewed snapshot was replaced. The initial sandbox-only acquisition
  failed DNS resolution; the authorized network-enabled retry passed.
- Real host conversion: The final unchanged-code command was
  `GNUPGHOME=<private-test-keyring> bash scripts/with-node.sh just policy
  test-media-conversion`, exit 0. Preparation matched all 30 snapshots and
  retained exactly one F1 diagnostic under the exact Homebrew 9.0.1 profile.
  The unfiltered all-feature Rust binary ran with `--include-ignored`:
  6 passed, 0 failed, 0 ignored, 0 filtered out. Its fresh report passed the
  unchanged recipe assertions: 30 pipeline actions, 8 video transcodes,
  6 audio transcodes, 0 pipeline failures and 0 suite failures. This includes
  the Theora/Vorbis-to-H.264/AAC conversion case.
- Real Linux preparation: `just verify-test-fixtures`, through `with-node` and
  a test-only FFprobe wrapper, passed all 30 snapshots using historical arm64
  image `sha256:da350429de6619c4172e0d3a7be5173fb9a0ee904028b0d9f1408924749631a7`.
  Temporary containers had no network, read-only root/media mounts and no
  capabilities, and were automatically removed. Its exact Alpine 8.0.1-r1
  report hash matched and one original F1 diagnostic was retained. This was
  Linux probe preparation, not Linux Rust conversion or current-package proof.
- Intermediate failures: The initial policy invocation could not access the
  host GPG trust database under the sandbox. An isolated test keyring resolved
  that access issue without changing policy. A preliminary retry overlapped
  with a regression-script edit and failed parsing; it is not passing evidence.
  The final unchanged-source policy and conversion rerun passed.
- Evidence: Non-media logs, host conversion/preparation reports and raw
  per-admission evidence are retained under
  `/private/tmp/revaer-approved-fixture-diagnostic-evidence-20260910`.
  `approved-f1-final-gates.log` records the final combined local gates;
  `linux-preparation.log` and `linux-preparation.md.probe-evidence` record the
  historical Linux preparation. Downloaded/generated media and task build
  output are removed after copying the non-media evidence.
- Remaining proof: The parent owns integrated `just ci`, `just ui-e2e`, strict
  Sonar and stack/package validation. No changed-file Sonar upload was made;
  source-upload consent is held by the parent. No GitHub mutation, push, current
  amd64/arm64 package pass, or all-service completion is claimed here.
- Risk and rollback: The approved expiry conditions above are unchanged. A
  different emitting build, source, snapshot or diagnostic must fail and return
  for operator review. Reverting this bounded fixture-only patch restores the
  previous strict preparation failure; it changes no production behavior.
- Stale-policy check: Reviewed root and devops instructions, this ADR, fixture
  acquisition/probe tooling and Rust-suite/Just boundaries. Added only the
  coordinated F1 devops bullet and fixture documentation. No shared Just recipe,
  frozen source/snapshot, production, Sonar, workflow, ADR 569/564, index or
  catalog file was changed. Parent-owned D1/D2 instruction changes remain a
  separate integration responsibility.
