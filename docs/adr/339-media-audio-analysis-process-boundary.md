# Media Audio Analysis Process Boundary

- Status: Accepted
- Date: 2026-07-29
- Context:
  - Audio loudness verification invokes an ffmpeg-compatible analyzer and parses the `ebur128`
    summary from stderr.
  - The general execution runner already runs noninteractively and bounds retained diagnostics, but
    the audio analyzer used a separate process path that collected complete stderr output.
  - Long media files can emit large analyzer logs before the final summary. Retaining the complete
    stream creates avoidable memory pressure in the verification path.
- Decision:
  - Run the audio analyzer with explicit null stdin, null stdout, and piped stderr.
  - Drain stderr on a reader thread while retaining only the bounded tail, preserving the final
    `ebur128` summary needed for verification.
  - Prefix truncated analyzer output with a stable marker so failure diagnostics remain honest.
  - Keep the analyzer interface unchanged and do not add dependencies.
- Consequences:
  - Positive: audio loudness verification no longer has unbounded stderr retention for long files.
  - Positive: analyzer subprocess behavior now matches the noninteractive posture of the main
    command runner.
  - Risk: a toolchain that emits the `ebur128` summary far before the end of stderr could fail
    parsing after tail truncation. The expected ffmpeg behavior emits the summary at the end.
- Follow-up:
  - Continue converging ad hoc process launch paths onto bounded, injected execution boundaries
    where practical.

## Task Record

- Motivation:
  - Move media verification closer to production readiness by eliminating unbounded diagnostic
    retention in a subprocess that can run over large media files.
- Design notes:
  - Implemented bounded tail retention with `std` only.
  - Preserved final-summary parsing by retaining the stderr tail rather than the head.
  - Did not weaken Sonar, lint, coverage, or workflow criteria.
- Test coverage summary:
  - Added focused tests proving noisy analyzer stderr remains bounded while preserving a parseable
    summary tail.
  - Added focused process coverage proving failure diagnostics use the bounded stderr path.
- Observability updates:
  - Truncated analyzer diagnostics now carry the stable `...[truncated]` marker.
- Status-doc validation:
  - `MEDIA_TRANSCODING.md` remains aligned: this is an implementation hardening of the existing
    verified audio-constraint behavior, not a product-scope change.
- Risk & rollback plan:
  - Roll back this ADR and the analyzer process-boundary change if a supported ffmpeg build places
    required `ebur128` summary lines outside the retained tail, then replace it with a streaming
    summary parser before reinstating bounded retention.
- Dependency rationale:
  - No dependencies added.
- Stale-policy check:
  - Reviewed `AGENTS.md` and `.github/instructions/rust.instructions.md`.
  - No stale references or contradictions were found.
