# Warning-Free Validation Tools

- Status: Recorded
- Date: 2026-08-15
- Operator approval: Not applicable: nonarchitectural task record
- Context:
  - Documentation generation used floating mdBook `0.5.4` with `mdbook-mermaid 0.17.0`, whose preprocessor protocol is built against mdBook `0.5.0`.
  - Playwright enables colored reporting internally while the local host exports `NO_COLOR`, causing Node to warn about conflicting color settings for each project process.
  - Both gates succeeded functionally but violated the repository's warning-free completion rule.
- Decision:
  - Pin mdBook `0.5.0` alongside `mdbook-mermaid 0.17.0` and replace any mismatched installed mdBook before documentation generation.
  - Invoke both managed Cargo binaries explicitly so a host package manager cannot shadow the validated versions through PATH order.
  - Remove inherited `NO_COLOR` only at the Playwright process boundary; retain explicit no-color configuration for the Rust API and Trunk processes.
  - Preserve all documentation output and browser test coverage.
- Consequences:
  - Documentation and browser validation no longer emit the two configuration warnings.
  - Local and CI tool versions are deterministic through the canonical `just` recipes.
- Follow-up:
  - Upgrade mdBook and its preprocessor together when both use the same protocol version.
  - Keep warning output fail-visible instead of filtering stderr.

## Task Record

- Motivation:
  - Make the approved validation foundation satisfy the warning-free handoff rule on its exact local toolchain.
- Design notes:
  - No warning is suppressed or redirected. The mismatched tool and environment inputs are corrected at their sources.
- Test coverage summary:
  - `just docs-install` and `just docs-build` passed using the managed mdBook `0.5.0` binary without a preprocessor warning.
  - `just ui-e2e` reported zero npm vulnerabilities and passed all 101 tests without the Node color-environment warning.
  - The complete `just ci` gate is retained for the final documented tree.
- Observability updates:
  - No production telemetry changes.
- Status-doc validation:
  - Updated the scoped DevOps rule, ADR catalogue, and documentation summary.
- Risk & rollback plan:
  - Revert the paired tool pins only when an aligned newer mdBook and preprocessor pair has been validated.
  - Restore inherited `NO_COLOR` only if Playwright stops forcing color or its upstream warning is otherwise removed.
- Dependency rationale:
  - No product dependency is added. Existing validation tools are pinned to mutually compatible versions.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/devops.instructions.md`, and the canonical documentation and UI recipes in `justfile`.
  - Drift was found in the floating mdBook installer and conflicting Playwright color environment; both were corrected without filtering warnings.
