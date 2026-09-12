# C1-D Compliance Chart Binding

- Status: In progress
- Date: 2026-09-11
- Operator approval: Not applicable: nonarchitectural task record
- Scope / owner / PR: `work/media3-compliance-chart`, unpublished local chart
  binding. Expanded parent-assigned ownership within approved C1-D scope:
  `charts/revaer/**`,
  `just/release.just`, `release/scripts/helm-package.sh`,
  `scripts/instruction-drift-check.sh`, `scripts/tests/compliance-chart-test.sh`,
  `scripts/tests/helm-package-test.sh`, `scripts/tests/instruction-drift-test.sh`
  and this record. Parent owns DevOps instructions, documentation navigation
  and generated indexes, ledger, integration and worktree removal.
- Initial binding commit: `fd1dc9b48d6b9547f3a0245458cd1cf0d53823dd`, preserved
  unchanged. Parent reports integration as `e940e873` on base `0e5deb92` in
  `/private/tmp/revaer-approved-integration`; follow-up evidence below is local
  to this worktree, not certification of that integrated revision.
- Approved authority: [ADR 588 approval resolution](../adr/588-first-release-decision-package.md#approval-resolution),
  reviewed snapshot `9575c0770a86421857335da4ae36dc4cbbae4f6a`, exact
  [C1-D contract](../adr/support/588-decision-details.md#c1-d-exact-read-only-delivery-proposal).
- Requirement ledger: [C1/C1-D delivery](../adr/564-media-completion-ledger.md);
  no E1 activation, D3 proof, deployment or package-readiness claim.

## Motivation And Design Notes

Bind one Helm release to the explicit verified platform-image digest,
architecture, operator-prepared existing PVC and selected manifest digest.
Required inputs have empty, nonworking defaults. Preserve unrelated chart
behavior and reject bad bindings instead of manufacturing compliance metadata.

- Add the previously absent `revaer.image` helper and consume it in both the
  Deployment and release notes, rendering `repository@sha256:<hex>` without
  a tag or `appVersion` fallback.
- Require exact lowercase SHA-256 syntax for both digests and an empty image
  tag. Enforce `amd64` or `arm64` through `kubernetes.io/arch`, preserving
  unrelated selectors and rejecting conflicts.
- Mount the prepared PVC at `/app/compliance` through exactly
  `<image-hex>/<manifest-hex>`. Both the PVC source and container mount are
  read-only; no compliance PVC or runtime installer is created.
- Reserve `checksum/compliance-manifest` for the complete manifest digest.
  Reject even identical, empty or null caller definitions. Preserve other
  annotations, including the existing database-secret checksum behavior.
- Enforce the binding with JSON Schema and template guards. The schema also
  rejects unsupported compliance toggles and subPath overrides.
- Update [operator inputs and limitations](../../charts/revaer/README.md#required-operator-inputs).
  Helm cannot authenticate a digest, distinguish an index from a platform
  manifest, verify PVC contents/ownership, exclude other writers, or prove
  application startup. Those remain operator/installer and package gates.

### Packaging And Repository Follow-Up

The source-review concern was reproduced against `fd1dc9b4`: with an empty
`image.tag`, a repository ending in `:mutable` rendered a tagged-and-digested
reference, and a repository already containing `@sha256:<64 c characters>`
rendered a double-digest reference. The existing numeric-port repository
`registry.example.test:5000/team/revaer` rendered correctly.

- Add the same bounded bare-repository separator guard in schema and the real
  `revaer.image` helper. Reject embedded tags, digests, schemes, whitespace and
  invalid types while retaining numeric registry ports and unqualified names.
  This is not a new OCI grammar, registry verifier or runtime dependency.
- Add canonical `just compliance-chart-test` and `just helm-package-test`
  recipes, both dependencies of `just helm-lint` alongside the existing
  annotation test. The policy suite also discovers all three `*-test.sh` files.
- Both signing branches of the existing packaging helper use `helm lint
  --strict` with identical, explicit command-line-only synthetic bindings.
  Required source and packaged bindings remain empty; no fixtures enter either
  default.
- Exercise the real release helper, annotation renderer, strict Helm lint and
  unsigned packaging. Signing tests replace only GPG and signed package/verify
  boundaries with command-recording doubles: they produce no signed archive
  and establish no signing, signature-verification or release evidence.
- Tighten instruction-drift mapping for the chart, checker and owned tests;
  fixture repositories prove that matching DevOps updates are required.

## Test Coverage Summary

Follow-up source: `fd1dc9b48d6b9547f3a0245458cd1cf0d53823dd` plus only the
owned implementation/test delta in this follow-up commit; this record was
updated after execution. Environment: macOS arm64, Just `1.58.0`, Helm
`v4.3.0+gbec5b06`, Ruby `4.0.6`. All accepted digests/PVC names in the tests
are synthetic renderer fixtures, not release evidence.

- `just compliance-chart-test`: 100 cases, 10 accepted and 90 rejected,
  308 Helm calls and 4,702 assertions. Strict lint, ordinary rendering,
  independent schema-only charts and schema-disabled template guards are
  exercised. Parsed YAML rejects duplicate keys and checks exact bindings,
  preserved unrelated settings, both architectures and rollover. The new
  repository cases include numeric ports, unqualified names, tags, digests,
  schemes, ASCII/Unicode whitespace, NUL and invalid types.
- `just helm-package-test`: six cases and 80 assertions. Real unsigned archive
  contents equal the source values, including all five empty binding fields;
  package version arguments are verified. Both branches have exact strict
  lint-only arguments. Command models verify signing/verification arguments,
  temporary key permissions, no unsigned GPG calls and failure propagation
  for unsigned lint, signed lint, signed packaging and signed verification.
  The first harness attempt exposed macOS `/var` versus `/private/var` aliases;
  canonicalizing its private fixture root resolved that assertion failure.
- `just --command bash scripts/tests/instruction-drift-test.sh`: 11 paths,
  31 guard runs and 217 assertions. Chart/test/checker/release paths reject
  absent or unrelated instruction updates, then accept matching fixture
  DevOps updates. Dirty/staged and recursive chart paths are covered; two
  unrelated/lookalike paths remain outside the new mapping.
- `just helm-lint`: passed, including annotation, chart and packaging suites.
  Its real unsigned `dist/helm/revaer-0.0.0-dev.0.tgz` was independently read
  with `helm show values` through `just --command ruby --disable-gems`.
  All values equal source, and `image.digest`, `image.architecture`,
  `image.tag`, `compliance.existingClaim` and `compliance.manifestDigest` are
  empty. The archive from the subsequent isolated CI run has SHA-256
  `28666938b9e630dbe94212cecc4ba21c3ab400f469e4a445aeca4bd192ca7960`.
  This identifies that local unsigned archive only, not a qualified release.
- `just --command bash -n <path>` passed separately for all five changed Bash
  scripts; `git diff --check` passed. ShellCheck is not installed locally.
- Real `just instruction-drift`: exit 1, correctly requiring the parent-owned
  DevOps update for chart, test, release and lint-control changes. No matching
  instruction update was fabricated in this worktree.
- Isolated `just ci`: exit 1 at that same instruction-drift gate, after passing
  formatting, policy/fixture tests, both all-feature workspace Clippy passes
  and `helm-lint`. The focused suites and Clippy emitted no warnings or skips;
  expected negative-case diagnostics remain visible in policy fixture tests.
  Later CI gates did not run. The invocation used `CARGO_BUILD_JOBS=2`, unique
  managed container `revaer-chart-ci-053844c96620`, dynamic port `60213`,
  private storage and generated credentials. Actual database image:
  `sha256:7e7dbab8d3b431a20793a6d99cb5a6bc84e44914309917f1bf5589a7568cdefd`.
  Sanitized local log: `target/compliance-chart-ci-053844c96620.log`.
- No follow-up UI attempt in the worker: the parent kept this subtask scoped
  to chart/packaging verification after the known C1 startup failure. This is
  not operator consent to waive UI verification. No substitute witness was
  created; the complete integrated UI gate remains outstanding.

### Initial Binding Evidence

These historical results tested `18357155ce084f82eefde1fb6d06fa3a3f0fd09c`
plus the original binding delta, not the follow-up or parent integration:

- `just --command bash scripts/tests/compliance-chart-test.sh`: 79 cases,
  9 accepted and 70 rejected, 244 Helm calls and 4,151 assertions. Positive
  cases use strict lint, ordinary rendering, isolated schema-only charts and
  schema-disabled rendering. Negative cases independently exercise schema and
  template rejection; unsupported extra properties are schema-only checks.
  Parsed YAML checks reject duplicate mapping keys and inspect exact bindings,
  preserved settings, both architectures, image/manifest rollover, PVC-name
  boundaries, malformed/absent/wrong-type inputs and annotation conflicts.
- The first test run failed in the test harness's single-document YAML emitter;
  wrapping each document in a YAML stream fixed that harness error. Subsequent
  focused and policy-suite executions passed, with no Helm warnings or skips.
- `just --command bash -n scripts/tests/compliance-chart-test.sh`: passed.
- `just instruction-drift` and `git diff --check`: passed for this owned delta.
- `just helm-lint` and `just ci`: exit 1 at packaging lint because the existing
  helper supplied only a database value. This failure is resolved by the
  follow-up, but the later instruction-drift gate now blocks full CI.
- `just ui-e2e`: exit 1. The sandboxed attempt failed npm audit DNS resolution;
  the network-enabled retry passed npm audit (zero vulnerabilities) and stopped
  before tests because port 8080 was occupied by an unrelated application,
  which was preserved. A further network-enabled
  `E2E_BASE_URL=http://localhost:18080 just ui-e2e` retry passed dependency,
  audit, client-generation and TypeScript setup, then the API exited with
  `compliance_metadata_startup_failed cause=missing_file` in `tests/logs/api.log`.
  No browser cases ran; teardown reported absent API coverage. This is the
  existing early-startup prerequisite, not permission to invent a bundle or
  bypass C1. No shared E2E file was changed.
- ShellCheck is not installed locally; the Bash syntax gate and executed Ruby
  test harness passed. The port check reported an unrelated Time Machine mount
  stat warning; no listener was reported and the retry reached API startup.

No Kubernetes cluster was contacted by the chart tests. No chart installation,
signature/attestation verification, native package qualification, remote check,
GitHub push or browser monitoring is claimed. Full gates remain required after
parent integration; `Blocked` does not waive them.

## Remaining Parent Integration

Just/release wiring and the instruction-drift checker are implemented here.
Only these shared-file actions remain parent-owned:

1. Add these entries to DevOps `applyTo` (existing release/Just entries remain):

   ```yaml
   - "charts/revaer/**"
   - "scripts/instruction-drift-check.sh"
   - "scripts/tests/compliance-chart-test.sh"
   - "scripts/tests/helm-package-test.sh"
   - "scripts/tests/instruction-drift-test.sh"
   ```

   Requested DevOps paragraph for the existing Helm rules:

   > Approved ADR 588 C1-D chart bindings must retain exact platform-image and
   > manifest SHA-256 digests, an empty image tag, conflict-checked amd64/arm64
   > scheduling, an existing prepared PVC, exact read-only imagehex/manifesthex
   > subPath and the reserved manifest checksum annotation. The image repository
   > must not contain a tag, digest, scheme or whitespace; numeric registry ports
   > remain supported. Keep required source/package bindings empty by default.
   > `just helm-lint` must depend on `helm-annotation-test`,
   > `compliance-chart-test` and `helm-package-test`. Both packaging branches must
   > use strict lint with explicit synthetic bindings only on the lint command,
   > never in chart or package defaults. Preserve real unsigned-archive empty-value
   > assertions and fail-closed signing command-construction regressions; doubles
   > are not signature or release evidence. Keep chart/test ownership aligned
   > with `scripts/instruction-drift-check.sh` and its regression suite. Rendering
   > and unsigned packaging do not prove verified image identity, prepared storage,
   > startup, installation, E1 activation or package qualification.

2. Add this record to `docs/tasks/index.md` and `docs/SUMMARY.md`, regenerate
   documentation indexes through Just, and update the completion ledger only
   to the demonstrated chart-binding boundary. D3 wrapper proof remains the
   parent's independent work; C1 installer/early-startup and E1 remain separate.
3. Revalidate `just compliance-chart-test`, `just helm-package-test`,
   `just helm-lint`, `just ci`, instruction drift and the changed-line gate
   against the actual integrated base/head. Full `just ui-e2e` remains required
   after real C1 startup preparation; repeated known-failing setup or invented
   metadata is not evidence. No default, schema or quality-gate relaxation is
   needed or authorized.

## Parent Integration Checkpoint (2026-09-11)

- Both worker commits are integrated: `fd1dc9b4` as `e940e873`, and `cad195b6`
  as `e48364da`. The completed worker's scoped files were compared before its
  worktree was removed. Retained artifacts are under
  `artifacts/media-verification/2026-09-11-compliance-chart/`; historical
  worktree-relative paths above now refer to that archive, not a live checkout.
- Matching DevOps ownership/rules, instruction-drift coverage, navigation and
  the ledger are integrated. `just helm-lint` passes with all 100 chart cases
  and six packaging cases. Instruction-drift regressions now exercise 15 paths,
  43 guard runs and 301 assertions. No lint fixture enters packaged defaults.
- Move the three authored Ruby test bodies out of shell heredocs into tracked
  `.rb` files, retaining thin canonical shell entrypoints. Chart and package
  bodies are byte-identical extractions; the drift test also covers these new
  paths. This exposes real source to Ruby analysis and execution coverage,
  without exclusions, generated/test classification or dependency changes.
  Full-file MAIN Ruby analysis reports zero issues for all three files.
- Fix the scanner-installer test's access to the operator's personal GPG home:
  real public-key inspection now uses a private mode-0700 temporary homedir.
  Preserve the real fingerprint/signature/archive assertions. The focused test
  and subsequent script-coverage gate pass; the initial permission failure is
  retained, not suppressed. DevOps and drift tests cover this test boundary.
- The integrated CI command exits zero but started before the final proof and
  test-source edits. It retains eight watcher warnings and does not certify the
  final revision. Corrected managed-database UI setup reaches the real API,
  which refuses startup with `compliance_metadata_startup_failed` /
  `missing_file`; no browser cases ran. The earlier UI attempt failed database
  ownership matching and is separately retained. Neither failure is a waiver.
- ADR 586 already permits explicitly injected test-only compliance loading.
  The initial read-only audit incorrectly treated that missing E2E wiring as
  missing approval; exact contract reconciliation corrected the conclusion.
  Canonical test bootstrap is ongoing approved work. Production startup and
  C1-D authenticated image delivery remain unchanged and separately required.
- Canonical Sonar is blocked by unavailable `SONAR_TOKEN`; no published coverage
  or quality-gate pass is claimed. Installer/authenticity, PVC contents, actual
  Linux amd64/arm64 startup, stable-source CI/UI and release remain incomplete.
  Earlier worker text attributing parent task direction to the operator was
  corrected; parent delegation is not operator consent to waive a gate.
- The integrated changed-file Sonar secrets scan found one hard-coded password
  in the existing Helm lint default. Removed credentials from that rendering-only
  URL instead of suppressing the finding. The repeat exact-file scan passes.
  A new real unsigned-package case exercises the default URL separately from
  the override: strict `just helm-lint` now passes all seven package cases and
  100 assertions. The final Ruby test file again returns zero MAIN Sonar issues.
  Earlier six-case coverage predates this fix; it is retained as such and must
  not certify the new source.

## Observability And Status Docs

No runtime telemetry changes. Render errors identify the invalid input, while
the manifest checksum exposes the selected bundle in the pod template. Chart
README and notes describe required operator preparation without certifying it.
Shared indexes, instructions and ledger remain parent-owned as listed above.

## Risk, Rollback And Dependencies

The intentional compatibility break is rejection of tag/default-only installs
and installs without prepared compliance storage. A syntactically valid but
unverified binding still requires the approved external verifier and startup
validation. Kubernetes read-only declarations do not prove PVC immutability or
correct storage behavior. Retain the previous jointly verified image and set
for rollback; never mix versions or bypass C1. Reverting this code does not
authorize an unbound release.

No runtime or package dependencies added. Tests use existing Helm and Just plus
Ruby standard-library JSON/YAML parsing, subprocesses and private temporary
directories, consistent with repository policy tests. No new architecture,
environment controls, registry retrieval or sidecar behavior is introduced.

## Stale-Policy Check And Cleanup

Reviewed `AGENTS.md`, `.github/instructions/devops.instructions.md`, the task
template, Just Helm/quality/UI recipes, existing image rendering and annotation
tests, packaging helper, policy-suite discovery and instruction-drift mappings.
The current root instruction uses routine task records, consistent with the
explicit ownership request. The approved C1-D text is unchanged from the reviewed
snapshot apart from the later approval wrapper. Removed stale chart tag-fallback
instructions and install examples that lacked required operator inputs. The
follow-up removes the packaging lint mismatch and repository-reference bypass,
and tightens ownership matching. DevOps scope/guidance still needs the exact
parent update above; the real drift gate correctly blocks until integration.

All chart/package/Git fixtures are private and removed at exit. The follow-up
CI run exited at instruction drift; its exact owned container, anonymous
volumes and private storage were removed after mount-ownership verification.
No generic port or database container was used during the follow-up. The two
initial CI runs finished at the recorded lint failure; all initial UI attempts
exited. Removed the exact initial owned CI
database container `0aa3ff4d012d` with volume cleanup, its worktree-bound
`.server_root/postgres-data`, and the empty local UI fixture directory. The
exited API PID was independently checked. No owned process, container or
acquired test media remains; build outputs and failed-run logs are local ignored
evidence. Operator settings, the unrelated port-8080 service and other Docker
resources were preserved. Parent will integrate the local commit and remove
this worktree.
