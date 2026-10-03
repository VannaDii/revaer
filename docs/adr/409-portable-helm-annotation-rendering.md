# Portable Helm Annotation Rendering

- Status: Accepted
- Date: 2026-08-04
- Context:
  - Helm packaging injects multiline Artifact Hub image and signing annotations into a copied `Chart.yaml` before linting and packaging.
  - Passing the complete multiline annotation body through `awk -v` fails with the macOS system awk because the value is parsed as awk program data containing literal newlines.
  - Signed and unsigned package output must retain the same annotations, metadata handling, package options, and verification behavior without adding a release dependency.
- Decision:
  - Write generated release annotations to a temporary file outside the copied chart directory and pass only that file path to a POSIX awk renderer.
  - Quote generated YAML scalar values before writing them and reject embedded line endings through the existing YAML scalar helper.
  - Exercise the renderer with a realistic multiline image and signing annotation fixture through `just helm-annotation-test`, and make that focused regression a prerequisite of `just helm-lint`.
  - Keep GPG import, signing, unsigned packaging, Helm lint, provenance verification, repository metadata, and owner metadata behavior unchanged.
- Consequences:
  - macOS and Linux awk implementations receive only a single-line file path through `-v`, avoiding implementation-specific multiline assignment parsing.
  - Annotation rendering has one focused executable seam and fails if its template marker is missing, duplicated, or its annotation input is empty.
  - The extra temporary annotations file is removed by the existing package cleanup trap and is never included in the chart archive.
- Follow-up:
  - Keep annotation additions in the file-generation block and extend the focused expected output when the release metadata contract changes.
  - Continue exercising both unsigned Helm lint packaging and signed release verification through their existing release paths.

## Task Record

- Motivation:
  - Restore release packaging on macOS without weakening release validation or changing the published chart contract.
- Design notes:
  - The renderer uses only Bash and POSIX awk already required by the repository; multiline data is read with `getline` from a temporary file rather than parsed as an awk variable assignment.
  - The system awk is placed first on `PATH` by the focused test so macOS runs exercise `/usr/bin/awk` semantics directly.
- Test coverage summary:
  - `just helm-annotation-test` passed with the macOS system `awk` first on `PATH`.
  - `just helm-lint` passed and packaged a chart containing the expected multiline annotations.
  - `just instruction-drift` passed.
  - `just lint` reached native compilation and was blocked by the base branch's libtorrent 2.1 incompatibility; the immediately stacked libtorrent compatibility task owns that independent failure.
- Observability updates:
  - No runtime observability surface changed.
  - Renderer failures now identify missing input, empty input, unreadable annotation data, or an invalid marker count.
- Status-doc validation:
  - Re-checked `.github/instructions/devops.instructions.md`, `docs/adr/index.md`, and `docs/SUMMARY.md`; updated them to include the portable rendering rule and task record.
- Risk & rollback plan:
  - The primary risk is byte-level chart metadata drift from file-based insertion. The focused fixture comparison and `just helm-lint` validate placement and YAML parsing before handoff.
  - Roll back by reverting this commit; no persisted data, runtime state, or published artifact is mutated by the implementation itself.
- Dependency rationale:
  - No dependencies were added. Bash, awk, cmp, and diff are existing system tools used by repository automation.
- Stale-policy check:
  - Reviewed `AGENTS.md` and `.github/instructions/devops.instructions.md`.
  - Drift found: release automation policy did not explicitly prohibit passing multiline YAML through `awk -v` or require focused coverage for the annotation renderer.
  - Removed that drift by adding the portable renderer rule and wiring the focused test into the canonical Helm lint recipe.
