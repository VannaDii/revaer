# Media conversion gate restoration

- Status: Recorded
- Date: 2026-09-10
- Operator approval: Not applicable: nonarchitectural task record
- Context:
  - The parent assigned a bounded independent validation restoration at
    `32a49c4b353bd5ebf2077929e0ecf2417382b517`. ADR 574, the completion ledger,
    runtime code, fixture assertions, and remote settings are outside this task.
  - The PR workflow already calls `just test-media-conversion`, but that recipe
    only depended on `verify-test-fixtures` and had no Rust command. The
    preparation script checked locked media and probes, then wrote a passed
    preparation report to the same path used for runtime conversion evidence.
  - [ADR 477](477-stacked-pr-check-emission.md) explicitly requires strengthening
    the canonical gate with production-adapter tests when the runtime exists.
    The existing ignored `verify_prepared_fixture_suite` calls
    `run_pipeline_cases`, including real video and audio transcodes.
- Decision:
  - Restore that existing gate: remove stale conversion evidence, verify
    integrity, execute the complete `media_fixtures` test binary with all
    features, warnings denied, and `--include-ignored`, then require fresh
    successful runtime evidence. Do not use a named test filter.
  - Preserve the existing workflow and report filename. During the conversion
    recipe, preparation writes to the configured report path plus
    `.preparation`; the Rust suite alone writes the conversion report.
    Standalone `just verify-test-fixtures` keeps its existing behavior.
  - Add parsed recipe validation and regressions under the existing
    [ADR 482](482-tooling-module-boundaries.md) input-loader/required-check owners.
    No new policy owner, dependency, architecture, or exception is introduced.
- Consequences:
  - A preparation-only command, a zero-selected-test success, or a stale
    passed report cannot establish conversion proof. Missing prerequisites
    and Rust compilation/test failures remain blocking.
  - The real local run now exposes an existing unapproved probe diagnostic.
    It failed before Rust execution; this is not a conversion-suite pass.
- Follow-up:
  - The parent owns review/integration and the combined full CI/UI gates.
    Resolve the exact fixture/tool diagnostic under the existing policy before
    claiming real conversion success. Do not rewrite snapshots, allow new
    diagnostics, bypass integrity, or weaken assertions in this task.

## Execution And Evidence

The canonical recipe removes the prior conversion report before verification.
It invokes `just verify-test-fixtures` with a preparation-only report path,
then runs:

```text
cargo --config 'build.rustflags=["-Dwarnings"]' test \
    -p revaer-media-runtime --all-features --test media_fixtures -- --include-ignored
```

This is a recipe-owned command, not an ad hoc Cargo invocation. The unfiltered
binary runs both regular tests and the prepared ignored suite. Bash
`set -euo pipefail` preserves failed preparation and Cargo execution; evidence
checks never overwrite those outcomes with success.

After successful Rust execution, the recipe requires a nonempty conversion
report, a passed outcome, positive pipeline/video-transcode/audio-transcode
counts, and zero pipeline/suite failures. Those fields are emitted by the
existing Rust report. The preparation report cannot satisfy them. A failure
before the Rust report is written intentionally leaves no conversion report
for the workflow's always-upload step; absence must fail, not publish green
preparation evidence. No production report or HTTP schema changes.

Just's existing JSON dump supplies parsed recipe attributes, body fragments,
dependencies, and ordering to the current input loader. The required-check
owner validates the exact executable recipe and its integrity helper. Comments,
unrelated commands, named filters, reduced features, disabled warnings,
suppressed errors, changed report routing, and missing evidence checks cannot
satisfy that contract. Parser failure is an error, not an empty successful scan.

## Validation Record

- `just workflow-guardrails-test`: passed. Added 13 structural mutations and
  10 recipe-execution cases using isolated verifier/Cargo stubs. Cases prove
  verification precedes Rust, failed verification prevents Rust, failed Cargo
  and missing tools remain failures, old success evidence is removed, zero
  execution cannot pass, preparation-only reports fail, and failed/zero-count
  reports fail. The positive stub case accepts only fresh complete evidence;
  stub results are not real media conversion proof.
- `just stack-check-contract-test`: passed; existing workflow relationships
  and always-clean/report requirements remain unchanged.
- `just test-fixture-scripts`: passed, preserving immutable acquisition,
  corruption/size/digest rejection, fallback, strict probe diagnostics,
  read-only verification, and explicitly controlled snapshot replacement.
- `just instruction-drift` and `git diff --check`: passed.
- `just docs-index`: passed, regenerating 517 catalog entries.
- Real run, using this worktree's own target, a private job-owned temporary
  directory, and `CARGO_BUILD_JOBS=8`:
  - `just download-test-fixtures`: passed, 22 locked source fixtures.
  - `just generate-test-fixtures`: passed.
  - `just test-media-conversion`: failed with exit 1 at the unchanged
    integrity prerequisite. Rust compilation and tests did not start.
  - First failure: `probe-fixtures: unapproved diagnostics emitted for
    mkv-theora-vorbis-live-style (124 bytes)`.
  - ffprobe diagnostic: `Length 5 indicated by an EBML number's first byte
    0x0a at pos 35 (0x23) exceeds max length 4.`
  - Both `/opt/homebrew/bin/ffmpeg` and `/opt/homebrew/bin/ffprobe` report
    version `9.0.1`, Homebrew build prefix `/opt/homebrew/Cellar/ffmpeg/9.0.1_1`,
    built with Apple clang `21.0.0 (clang-2100.1.1.101)`. Full version,
    configuration, and library output is retained in `ffmpeg-version.txt` and
    `ffprobe-version.txt` in the evidence directory below.
  - No successful preparation or runtime report was produced for this failed
    run. Full command output and exit status are retained externally.
- The runner's EXIT trap executed `just clean-test-fixtures` successfully.
  It also removed only this job's `target/media-fixture-integration` and
  private temporary tree. No downloaded/generated test media is retained.
- Evidence directory:
  `/Users/vanna/Source/revaer-reviews/2026-09-10/media-fixture-gate/`.
  The runner, per-recipe logs, cleanup log, and final status preserve the actual
  failure without creating a synthetic conversion report.
  `test-media-conversion.log` retains the complete emitted ffprobe stderr and
  both recipe failure messages; `final-status.txt` records `1`.
- Full `just ci` and `just ui-e2e` were not run here; they are parent-owned
  on the combined tree. This record does not assert repository completion.
- Parent integration `4b073661` passed `just workflow-guardrails-test
  stack-check-contract-test test-fixture-scripts instruction-drift` again.
  The Sonar Ruby snippet analyzer returned zero issues for the full contents of
  both changed Ruby guardrail files in MAIN scope. This does not replace the
  authoritative scanner, coverage publication, or combined CI/UI gates.

## Failing Fixture Provenance

`mkv-theora-vorbis-live-style` is a locked upstream source, not derived media.
Its entry in [`test-fixtures/lock.json`](../../test-fixtures/lock.json) pins
`ietf-wg-cellar/matroska-test-files` revision
`e6965e5ca666322ed93e2748a10a4f132309e005`, `test_files/test4.mkv`, as raw bytes:

- Path: `test-fixtures/matroska/mkv-theora-vorbis-live-style.mkv`.
- SHA-256: `43df750a2a01a37949791b717051b41522081a266b71d113be4b713063843699`.
- Exact size: `21313902` bytes (both lock bounds).

The matching [`test-fixtures/manifest.json`](../../test-fixtures/manifest.json)
entry explicitly sets `generated: false`, `shouldDownload: true`, and
`shouldGenerate: false`; the source download passed its locked digest and size
checks. The manifest notes that some ffprobe builds emit an EBML warning, but
does not authorize that diagnostic. The unchanged strict verifier rejects it.
The derived-fixture generator does not own this source. This evidence does not
establish a generator defect, source corruption, or the precise compatibility
cause, and no media was reacquired for further investigation. Fixture
replacement or diagnostic-policy changes require separate operator approval.
The selected lock/manifest entries are retained as `failing-fixture-lock.json`
and `failing-fixture-manifest.json` alongside the stderr log. All acquired and
generated media was deleted after the failed run.

## Task Record

- Motivation:
  - Close the demonstrated gap between the workflow's conversion label and
    the canonical recipe's actual execution, as already required by ADR 477.
- Design notes:
  - Use the existing test binary, report fields, Just command surface, and five
    approved guardrail owners. Keep exact source locks, probes, diagnostics,
    runtime behavior, fixture assertions, and all-feature selection unchanged.
- Test coverage summary:
  - Structural recipe mutations and controlled execution cases above passed.
    The real-media attempt failed at integrity verification before the runtime
    suite, preserving the unresolved validation gap as a blocking failure.
- Observability updates:
  - Distinguish preparation evidence from fresh Rust conversion evidence.
    Retain failures externally; no runtime logging or telemetry changes.
- Status-doc validation:
  - Reviewed fixture setup/report guidance and the PR media workflow. Updated
    the scoped devops contract, ADR index, summary, and generated catalogs.
    ADR 574 and the parent completion ledger are unchanged. No production
    readiness claim is added.
- Risk & rollback plan:
  - The stricter recipe can reveal latent media/tool or runtime failures, as
    this attempt demonstrates. Fix the proven cause rather than reverting to
    a fixture-only pass. If the wiring is defective, correct it while retaining
    blocked conversion status; no runtime, database, deployment, or remote
    rollback is needed.
- Dependency rationale:
  - No dependencies added. Existing Just JSON metadata and Ruby standard-library
    JSON/Open3 provide structured recipe inspection; existing Bash tools check
    the existing human-readable report.
- Stale-policy check:
  - Reviewed `AGENTS.md`, devops and Rust instructions, ADRs 477 and 482, the
    ADR template, PR workflow, fixture scripts/README, and Rust fixture suite.
    Found concrete drift between the existing devops/ADR conversion obligation
    and the fixture-only recipe. Restored executable checks and clarified
    report provenance. No instruction conflict, accepted architectural value,
    fixture lock/probe, workflow, Sonar setting, or quality criterion was relaxed.
