# Media verifier process boundary

- Status: Accepted
- Date: 2026-07-29
- Context:
  - Candidate verification runs process-backed FFmpeg and FFplay checks before destructive replacement.
  - The execution runner and loudness analyzer already close stdin, discard stdout, and bound diagnostics, but the verifier still retained complete stderr output.
- Decision:
  - Replace verifier stderr collection with a bounded tail reader.
  - Preserve the final diagnostics and add a truncation marker when earlier output is discarded.
  - Keep verifier process invocation synchronous and dependency-free while retaining existing cancellation and kill/reap behavior.
- Consequences:
  - Verifier failures keep actionable FFmpeg summaries without letting noisy tools allocate unbounded process memory.
  - Very early stderr lines can be discarded once the cap is exceeded, which is acceptable because failure summaries are emitted at the end of FFmpeg verifier output.
- Follow-up:
  - Apply the same process-boundary review to capability probing and any remaining media tool adapters.
  - Continue closing remaining runtime verification gaps through the stacked media PRs.

## Task Record

- Motivation:
  - Move the media service closer to production correctness by removing an unbounded verifier diagnostic path before candidate replacement.
- Design notes:
  - `SystemVerificationExecutor` still closes stdin, discards stdout, and supports cooperative cancellation.
  - Stderr is retained as a bounded tail with a truncation marker so the final failure summary survives long verifier logs.
  - No new dependencies were added.
- Test coverage summary:
  - Added unit coverage for the bounded verifier stderr reader retaining the diagnostic tail.
  - Added a system verifier regression proving a noisy failing verifier reports bounded detail containing the final summary.
  - Reran focused media runtime verifier tests with warnings denied.
- Observability updates:
  - Existing verification-check details remain the operator-facing failure surface.
  - Truncated verifier diagnostics now carry the existing machine-readable truncation marker.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/devops.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md`; no instruction drift was introduced.
- Risk & rollback plan:
  - Risk: a rare verifier failure with only early stderr context may lose that early detail after the cap.
  - Rollback: revert this ADR and the verifier stderr reader change to restore prior full-stderr behavior.
- Dependency rationale:
  - No dependencies were added.
