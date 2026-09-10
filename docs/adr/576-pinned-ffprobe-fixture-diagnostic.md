# Pinned FFprobe fixture diagnostic

- Status: Recorded
- Date: 2026-09-10
- Operator approval: Not applicable: nonarchitectural task record
- Context:
  - ADR 575 restored real conversion execution, but strict fixture verification
    failed before Rust execution. The local probe was FFmpeg 9.0.1 while the
    runtime package manifest pins Alpine FFmpeg 8.0.1-r1.
- Decision:
  - Record a same-byte diagnostic comparison with the existing historical
    Linux image and an independent upstream source trace. Select no new runtime
    environment, fixture disposition, diagnostic exception, or package version.
- Consequences:
  - Both probes emit the same element-ID error. Using the pinned tool version
    does not resolve this observed failure. Stream discovery is not a clean
    probe, conversion pass, or release-package certification.
- Follow-up:
  - Keep conversion verification failing until its cause is resolved under the
    existing policy. Fixture replacement or new diagnostic acceptance requires
    separate operator approval; neither is implemented here. ADR 569 and E1
    remain held independently of the approved ADR 557-559 resolution.

## Exact Evidence

The investigation starts from clean integration
`02de83942c38ea08433f68b31feded8f86740ea0`. The independent source trace and
media probes ran in separate clean worktrees, both removed after completion.
Raw outputs, commands, status, container configuration, source links, and cleanup
evidence are outside the repository at
`/Users/vanna/Source/revaer-reviews/2026-09-10/media-fixture-pinned-probe/`.

`just download-test-fixtures` passed all 22 locked source acquisitions. The
examined source is `mkv-theora-vorbis-live-style`, upstream
`ietf-wg-cellar/matroska-test-files` commit
`e6965e5ca666322ed93e2748a10a4f132309e005`, `test_files/test4.mkv`. Its size is
21,313,902 bytes and its before/after SHA-256 is unchanged:
`43df750a2a01a37949791b717051b41522081a266b71d113be4b713063843699`.
No derived media was generated for this diagnostic comparison.

Both commands used the verifier's existing arguments:
`ffprobe -v error -show_streams -of json <same locked file>`. Host FFprobe
reported 9.0.1; Linux FFprobe reported 8.0.1, built with Alpine GCC 15.2.0.
Both exited zero and their canonical stream projections exactly matched the
committed Theora/Vorbis snapshot. Host stderr was 124 bytes; Linux stderr was
127 bytes. The only difference was the process context's address prefix. Both
reported:

```text
Length 5 indicated by an EBML number's first byte 0x0a at pos 35 (0x23) exceeds max length 4.
```

The Linux probe used immutable local image ID
`sha256:da350429de6619c4172e0d3a7be5173fb9a0ee904028b0d9f1408924749631a7`,
the historical arm64 snapshot documented in ADR 572. It is not an image built
from the current integration commit and has no immutable source attestation.
No amd64 or current-release behavior is certified by this comparison.

The container retained its existing non-root `revaer` user, never ran the app
entrypoint, and had no network, published ports, capabilities, or writable root.
Empty read-only tmpfs mounts covered its declared `/config` and `/data` volumes.
Only the public locked fixture was bind-mounted read-only; its disposable host
mode was changed to 0644 for that user to read. Limits were one CPU, 256 MiB,
64 PIDs, and a 60-second probe timeout. There was no timeout or OOM event. The
container and volumes were removed, and `just clean-test-fixtures` deleted all
acquired media. These are diagnostic probes, not acceptance-gate substitutes.

## Source Interpretation

The independent trace resolves official FFmpeg tags to commits
`894da5ca7d742e4429ffb2af534fcda0103ef593` (8.0.1) and
`bf1b838f2ab88b4f8fd83443325c782ea0e0f7fa` (9.0.1). The complete
`ebml_read_num()` function is byte-identical in
[8.0.1](https://github.com/FFmpeg/FFmpeg/blob/894da5ca7d742e4429ffb2af534fcda0103ef593/libavformat/matroskadec.c#L913-L971)
and
[9.0.1](https://github.com/FFmpeg/FFmpeg/blob/bf1b838f2ab88b4f8fd83443325c782ea0e0f7fa/libavformat/matroskadec.c#L914-L972).
It emits this message at `AV_LOG_ERROR` and returns `AVERROR_INVALIDDATA`.
Calling it merely a warning would understate this source-level classification.

The retained first 96 bytes place the EBML Header at offsets 0-29, Segment ID
at 30-33, and its unknown-size encoding at 34. Offset 35 is the first Segment
payload byte, not part of the EBML Header. Byte `0x0a` indicates a five-byte
encoded number; the element-ID reader permits four. The separate size reader
permits eight and recognizes unknown sizes. Thus this error is not evidence
that unknown-size live recordings are unsupported.

The source can resynchronize after Segment parsing errors, which is consistent
with the observed stream discovery. No dynamic call stack, complete decode,
playback, end-of-file validation, or exact resynchronization offset was captured.
The evidence does not explain the origin or intended meaning of the repeated
`0x0a` bytes, prove whole-file validity or unusability, or justify suppressing
the error. Official tag source equality also does not attest to every installed
package patch or establish equality of all FFmpeg behavior across versions.

## Validation And Delivery Boundary

- Direct probes and snapshot comparison passed their narrow assertions; the
  strict fixture diagnostic criterion remains failed. The real Rust conversion
  suite did not execute in this investigation.
- Full `just ci` exited zero on the unchanged executable checkpoint, including
  all 18 package coverage gates, workspace/all-feature and minimal-feature
  tests, Clippy, policy, audits, script coverage, and release build. This is
  recipe success, not warning-free output or complete feature validation.
- `just ui-e2e` exited one: 46 passed, one failed, and 61 did not run. Profile
  creation at `tests/specs/api/media.spec.ts:130` expected 201 and received 400.
  Teardown reported unexecuted GET job `phases` and profile `readiness` routes.
  No response-body problem code is inferred, and no assertion was relaxed.
- Current CI output includes shutdown `WARN` messages for deliberately aborted
  runtime tasks. A zero recipe status alone must not be called warning-free.
  Remote PR 194 contains related observability work that is not present in this
  local executable; this task does not adopt its implementation or UI assertions.
- A fresh CLI ruleset/check comparison still finds `Supply Chain Checks` missing
  from PR 194 at `cfed91f82a07a909ac1b96d9129e9eabeaddbf1c`. Its other reported
  results do not satisfy all 21 required contexts. VannaDii remains assigned.
- Copilot review was requested through the GitHub API tool under the user's
  standing instruction. The tool returned an empty acknowledgement, and the
  subsequent REST/GraphQL reads showed no request, event, or review. Submission
  is therefore unconfirmed, not a fulfilled review requirement. No browser was
  used. No commit was pushed, branch rebased, ruleset changed, or PR merged.

## Final Checkpoint

Full logs are `ci-root-contract-02de83942c38ea08433f68b31feded8f86740ea0.log`
and `ui-e2e-root-contract-02de83942c38ea08433f68b31feded8f86740ea0.log`
under the external 2026-09-10 evidence directory. The changes after that
checkpoint are documentation only; executable/gate paths have no diff.

- `just docs-index instruction-drift docs-link-check` passed: 518 generated
  entries and 1,103 checked links, zero link errors. `git diff --check` passed.
- `just sonar-compile-db js-release-coverage js-coverage-merge
  sonar-verify-inputs` passed after the UI run. Retained Rust LCOV has 237 source
  records, 101,312 line records, and 94,236 covered lines. JavaScript LCOV has
  60 source records, 5,276 line records, and 3,916 covered lines. JavaScript
  evidence includes executed API/release code, not successful execution of the
  skipped UI cases. Native compile inputs and generic script coverage were
  retained with these reports in this task's external `coverage/` directory.
- `just sonar-scan` failed before analysis: local `SONAR_TOKEN` is unavailable.
  Positive local reports are not published server metrics or a new quality-gate
  result. No Sonar criterion, project setting, or disposition changed.
- `just --command sonar analyze secrets docs/SUMMARY.md docs/adr/index.md
  docs/adr/564-media-completion-ledger.md
  docs/adr/576-pinned-ffprobe-fixture-diagnostic.md docs/llm/manifest.json
  docs/llm/summaries.json` completed without findings. This is secrets-only
  analysis, not an authoritative repository scan.
- The task database container was removed. Ports 5441, 7070 and 18080 were
  closed, `.server_root/library` was empty, and `just clean-test-fixtures`
  passed with all four ignored media directories absent. The primary dirty
  checkout and old prunable worktree recovery metadata were preserved.
- The bounded diagnostic/documentation task is recorded; the full service
  goal remains incomplete and active. ADR 569, E1, fixture disposition,
  warning-free CI/UI, strict published Sonar, current amd64/arm64 packages,
  stack reconciliation, required checks, and review completion remain open.

## Task Record

- Motivation: Distinguish a host-version mismatch from a fixture error before
  choosing a corrective change or asking for a diagnostic-policy exception.
- Design notes: Use the unchanged locked source, probe arguments, canonical
  snapshot, existing image, and bounded read-only isolation. Preserve all raw
  diagnostics and version/provenance limits. No runtime or gate code changed.
- Test coverage summary: Both narrow probes returned zero and matched the
  reviewed snapshot while emitting the same error; no conversion pass follows.
  Full-gate and documentation outcomes are recorded in the final checkpoint.
- Observability updates: External evidence distinguishes command success from
  strict diagnostic failure. No production severity or error handling changed.
- Status-doc validation: Update the completion ledger, ADR index, SUMMARY and
  generated catalog. No operator-facing capability or release claim is added.
- Risk & rollback plan: Historical-image or partial-probe evidence could be
  overstated. Keep its exact identity and limits attached; remove or correct
  this documentation if contradicted. No production rollback is needed.
- Dependency rationale: No dependency added or changed. Existing Just, FFprobe,
  Docker, timeout, checksum, and Ruby JSON tools produced bounded evidence.
- Stale-policy check: Reviewed root, devops and Rust instructions, ADRs 569,
  572 and 575, fixture lock/manifest and verifier. No criteria were relaxed;
  source inspection corrects the informal warning description to an error.
