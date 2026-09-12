# C1-D Compliance Chart Binding

- Status: Blocked
- Date: 2026-09-11
- Operator approval: Not applicable: nonarchitectural task record
- Scope / owner / PR: `work/media3-compliance-chart`, unpublished local chart
  binding; only `charts/revaer/**`, `scripts/tests/compliance-chart-test.sh`
  and this record. Parent owns shared integration and worktree removal.
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

## Test Coverage Summary

Source: `18357155ce084f82eefde1fb6d06fa3a3f0fd09c` plus only this local
commit's scoped chart, test and record delta. Environment: macOS arm64,
Just `1.58.0`, Helm `v4.3.0+gbec5b06`, Ruby `4.0.6`. All accepted digests/PVC
names in the tests are synthetic renderer fixtures, not release evidence.

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
- `just helm-lint`: exit 1. The existing packaging script supplies only a
  database value to Helm, so the four now-required inputs fail validation.
  The preceding `helm-annotation-test` passes. No chart package is produced.
- `just ci`: exit 1 at the same packaging lint invocation, after successful
  formatting, policy/fixture-script tests and both workspace Clippy passes.
  Later CI gates did not run; this is not a full CI pass.
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

## Parent Integration Wiring

These shared-file edits are intentionally not included in this ownership slice:

1. In `just/release.just`, add the focused recipe:

   ```just
   compliance-chart-test:
       bash scripts/tests/compliance-chart-test.sh
   ```

   Add `compliance-chart-test` to `helm-lint`'s dependencies alongside
   `helm-annotation-test`. `scripts/tests/policy-suite.sh` already discovers
   `*-test.sh`, so policy and its coverage harness need no duplicate invocation.
2. In both signing branches of `release/scripts/helm-package.sh`, keep the
   real chart defaults empty and pass explicit lint-only inputs to `helm lint`:
   `image.digest=sha256:` plus 64 `a` characters, `image.architecture=amd64`,
   `image.tag=` (empty), `compliance.existingClaim=lint-only-not-prepared`, and
   `compliance.manifestDigest=sha256:` plus 64 `b` characters. Use quoted
   `--set-string` arguments or a private values file outside the chart copy;
   never bake fixtures into the packaged `values.yaml`. Run lint with `--strict`.
   The focused suite covers both architectures. These are syntactic packaging
   checks, not authenticated images, prepared storage or installation evidence.
3. Update scoped DevOps guidance with the approved required-input contract,
   reserved annotation, focused recipe and lint-only fixture boundary. Include
   `charts/revaer/**` and `scripts/tests/compliance-chart-test.sh` in its scope.
   Keep the instruction-drift mapping aligned if its ownership scope changes.
4. Add this record to `docs/tasks/index.md` and `docs/SUMMARY.md`, regenerate
   documentation indexes through Just, and update the completion ledger only
   to the demonstrated chart-binding boundary. D3 wrapper proof remains the
   parent's independent work; C1 installer/early-startup and E1 remain separate.
5. Rerun `just compliance-chart-test`, `just helm-lint`, `just ci`,
   `just ui-e2e`, instruction drift and the changed-line gate against the actual
   integrated base/head. No default, schema or quality-gate relaxation is needed.

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
packaging lint mismatch is recorded for parent integration, not suppressed.

Chart fixtures are private and removed at exit. Both CI runs finished at the
recorded lint failure; all UI attempts exited. Removed the exact owned CI
database container `0aa3ff4d012d` with volume cleanup, its worktree-bound
`.server_root/postgres-data`, and the empty local UI fixture directory. The
exited API PID was independently checked. No owned process, container or
acquired test media remains; build outputs and failed-run logs are local ignored
evidence. Operator settings, the unrelated port-8080 service and other Docker
resources were preserved. Parent will integrate the local commit and remove
this worktree.
