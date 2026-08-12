# Media Fixture Cleanup

- Status: Accepted
- Date: 2026-08-04

## Motivation

The required PR media-conversion job downloads and generates ignored binary
fixtures. Report upload ran under `if: always()`, but the job did not guarantee
fixture cleanup after success or failure.

## Design Notes

- Run `just clean-test-fixtures` after report upload with `if: always()`.
- Keep cleanup on the canonical Justfile surface.
- Preserve the uploaded report before deleting generated fixture directories.

## Test Coverage Summary

- `just instruction-drift`
- `just lint`
- `just ci`
- `just ui-e2e`
- Remote PR media-conversion execution proves the always-run cleanup step.

## Observability Updates

The existing media-conversion report remains available in the job summary and
artifact upload before cleanup runs.

## Risk And Rollback Plan

The change can reduce cache-save usefulness after a cache miss because ignored
fixture directories are removed before job completion. Correct artifact hygiene
takes precedence. Roll back the workflow, instruction, and specification edits
together if fixture caching is redesigned around an explicit archive artifact.

## Dependency Rationale

No dependencies were added.

## Stale-Policy Check

Reviewed `AGENTS.md`, `.github/instructions/devops.instructions.md`,
`.github/workflows/pr.yml`, `justfile`, and `MEDIA_TRANSCODING.md`. The missing
always-run cleanup requirement was the only drift found and is corrected here.
