# Tooling integration

The uv-managed `rv` foundation merged in PR #211; the transcoding integration
merged in PR #31 on 2026-10-15. The October 2 operator checkpoint and historical
104-PR replay plan are no longer the continuation baseline. Start new work from
current `main` or the specific active branch named by its owner.

## Current command surface

- Bootstrap with `./setup.sh`; uv manages the pinned Python and locked environment.
- Use `rv <command>` locally and `uv run --locked -- rv <command>` in automation.
- The root Justfile is a compatibility surface: its recipes delegate to `rv`.
- `tools/src/revaer_tooling/cli.py` owns the public command registry. Use `rv --help`
  for available commands and command-specific help for arguments.
- Tasks under `tools/src/revaer_tooling/tasks/` compose injected adapters rather
  than maintaining independent shell implementations.
- `rv ci` composes database baseline validation, formatting, lint, dependency,
  build, test and coverage checks, followed by the release build. `rv ui-e2e`
  runs the configured API/browser workflow. The task registry and workflows own
  the exact selections; this page does not redefine them.
- Media discovery, recovery, fixture, compliance and image commands are part of
  the same registry. Their presence is not a claim that every release artifact
  or deployment has passed its required checks.

## Retained material

Legacy shell, Ruby, Node and split Just recipes may remain as historical inputs,
comparison fixtures or release dependencies. Their presence does not make them
an alternative executor or authorize blanket deletion. Check actual callers
before changing them. Use the locked dependency graphs and existing checks.

Historical implementation notes remain in [ADR 592](../docs/adr/592-python-tooling.md).
Its dated test counts, pending items and temporary checkout paths describe those
runs, not the current backlog. They do not require recreating old proof projects,
reports or retired migration/parity machinery.

Follow [AGENTS.md](../AGENTS.md) and matching scoped instructions for quality gates
and documentation policy. Update product/tooling documentation when its contract
changes; routine work does not require a new completion record or evidence bundle.
