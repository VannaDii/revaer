# Locked fixture diagnostic disposition

- Status: Proposed
- Date: 2026-09-10
- Operator approval: Pending. No diagnostic acceptance or fixture replacement
  is authorized by ADR 557-559 or by this proposal.
- Context:
  - ADRs 575-576 establish that strict preparation rejects one locked upstream
    fixture before real Rust conversion execution. Host FFprobe 9.0.1 and the
    historical Linux arm64 image's 8.0.1 emit the same element-ID error.
  - ADR 559 G1 reserves every quality-criterion change for explicit consent.
    Accepting this known error would change preparation admission, even with
    stronger assertions around it. It is not an internal implementation detail.
- Decision: None made. Present F1's bounded test-only exception for approval.
  Keep the current diagnostic gate failing until the operator decides.
- Consequences:
  - F1 would admit this particular recoverable probe observation into subsequent
    validation without declaring the whole input valid or harmless.
  - Additional diagnostics, failed decode/conversion, missing evidence, or other
    fixtures would remain failures. F1 alone cannot establish a conversion pass.
- Follow-up: Obtain an explicit F1 decision, implement only its exact scope if
  accepted, and rerun real preparation/conversion and the complete quality gates.

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
