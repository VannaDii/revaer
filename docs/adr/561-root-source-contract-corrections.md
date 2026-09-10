# Root source contract corrections

- Status: Recorded
- Date: 2026-09-09
- Operator approval: Not applicable: nonarchitectural task record
- Context:
  - Integrate the existing ADR 556 source implementation commit
    `b1311cc21e0d18d4390f08ac9f6c0dd3991d98a0` onto the assigned delivery branch.
    The mechanical integration is `cdc5896b`; ADRs 555 and 557-559 and their
    documentation entries are preserved. The parent records the reconciled
    ADR 557-559 approval in `6f2c6f90` and owns ADR 560 quality repairs.
  - ADR 557 requires alphanumeric-ended root keys and non-root absolute paths.
    The prior parser admitted leading/trailing hyphens and `/`, and its tests
    incorrectly asserted those values were valid.
  - Derived Serde structs admit positional arrays, and derived unit enums admit
    tagged objects. ADR 550 requires JSON objects and string enum values, not
    these alternate representations.
  - Source tests assumed the system temporary directory had trusted ancestry.
    With `TMPDIR=/tmp`, eleven tests failed before reaching their intended
    filesystem checks because the loader correctly rejected writable ancestry.
- Decision:
  - Correct the source parser to the already-approved key and path constraints.
    Require object-only deserialization for catalog and slot containers and
    string-only deserialization for kinds and evidence enums.
  - Preserve the single Serde parser, duplicate/unknown-field rejection, exact
    decoded bytes, canonical frame, bounds, reason codes, and evidence matrices.
    Descriptor trust and platform checks are unchanged.
  - Place private, automatically removed source-test fixtures under the operator
    home, whose ancestry must satisfy the existing loader policy. Assert that
    final-entry and parent-replacement hooks actually execute.
- Consequences:
  - Invalid source declarations now fail before any downstream reconciliation.
    Existing valid-document digest vectors remain unchanged.
  - A successfully loaded empty catalog remains distinguishable from a missing
    source even though their semantic catalog bytes and digests are identical.
  - Tests require a writable operator home with trusted ancestry. They do not
    weaken production trust to accommodate shared temporary directories.
- Follow-up:
  - The parent integrates these commits with the approval and dependency/policy
    changes, regenerates combined indexes as necessary, and runs `just ci` and
    `just ui-e2e` on the integrated revision before repository completion.
  - Linux amd64/arm64 packaged read-only mounts, privileged mount replacement,
    root attestation, persistence, binding, and broker behavior remain outside
    this source-only validation. No readiness claim follows from parser tests.

## Task Record

- Motivation:
  - Make the integrated source implementation compatible with the reconciled
    approved root contract without implementing persistence or broker work.
- Design notes:
  - Two private JSON entry-point wrappers constrain Serde's accepted input shapes
    without constructing a second JSON tree or losing duplicate-field evidence.
    No public API, source digest framing, or filesystem authority is added.
  - The whole-root discovery association prefix from ADR 557 is not a catalog
    path: accepting an explicit empty association prefix does not authorize `/`.
- Test coverage summary:
  - Expanded key/path cases, object/string shape rejection, loaded-empty versus
    missing state, and deterministic replacement-hook execution assertions.
  - Fresh execution outcomes are recorded separately below. ADR 556's earlier
    CI, E2E, security-review, and documentation results remain historical and
    are not evidence for this integration revision.
- Observability updates:
  - No telemetry changes. Invalid shapes retain format-invalid classification;
    key/path rejection remains bounded and does not expose configured values.
- Status-doc validation:
  - Reviewed root-source status and the ADR 557 validation boundary. No product
    capability is activated; README and operator capability claims are unchanged.
    Register this record in the ADR index, mdBook summary, and generated catalogs.
- Risk & rollback plan:
  - The correction deliberately rejects formerly admitted out-of-contract input.
    Retain known-good source documents and fixed digest tests. A rollback must
    not treat the prior parser's broader acceptance as approval of those inputs.
    Revert the corrective commit only with the source feature held unavailable
    until equivalent contract enforcement is restored. No database rollback is
    involved because this slice does not persist or activate a catalog.
- Dependency rationale:
  - No new package or dependency update. The imported source's existing direct
    SHA-256 dependency edge retains ADR 556's rationale. Parent-owned dependency
    updates are disjoint from this corrective patch.
- Stale-policy check:
  - Reviewed `AGENTS.md`, Rust and devops scoped instructions, the ADR template,
    canonical media/quality/docs recipes, and ADRs 523, 550, and 557-559.
  - The pre-existing Rust scoped/root precedence contradiction is recorded for
    the parent-owned ADR 560 repair; this patch does not edit that instruction.
    No operational policy, threshold, exception, or approval status is changed.

## Fresh Validation

- Regression-first `just test-media-root-catalog` failed on the new key, `/`,
  and positional catalog-array assertions before correction.
- `TMPDIR=/tmp just test-media-root-catalog` reproduced eleven fixture ancestry
  failures before the fixture fix; after correction, all 39 focused tests pass.
- The first `just lint` passed policy and then rejected an unnecessary raw-string
  delimiter in the new test. That test literal was corrected without suppression.
- Final `just test-media-root-catalog`: 39 passed, zero failed or ignored, both
  with the normal environment and with `TMPDIR=/tmp`. The other 245 library
  tests and six media-fixture tests are filtered by this focused recipe.
- `just fmt`, `just lint` (policy plus both strict workspace Clippy passes),
  and `just instruction-drift`: passed. No shared database was used.
- `just docs-index`: passed and generated 502 entries, retaining ADRs 555-559
  and adding ADR 561. `git diff --check`: passed.
- `just clean-test-fixtures`: passed. No conversion media was acquired; source
  fixture directories were automatically removed and no matching directory
  remains under the operator home.
- Full CI, database-backed tests, browser/E2E, publishing, and pushing are not
  run by this source-only task. Parent-owned integrated `just ci` and
  `just ui-e2e` remain mandatory before repository completion.
