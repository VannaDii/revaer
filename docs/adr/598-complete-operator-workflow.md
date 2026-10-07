# Complete operator workflow qualification

- Status: Recorded
- Date: 2026-10-07
- Operator approval: Not applicable: nonarchitectural qualification under accepted ADRs 593–594
- Supersedes: None
- Implementation status: Qualification harness implemented; acceptance requires the current-source results described below.

## Context and decision

Goal 7.1 requires authenticated configuration, restart, discovery, dry-run,
processing, verified replacement, cancellation, recovery and bounded scratch
scenarios together. The candidate starts at `116891c7`, including the continuation,
checkpoint recovery and concurrency/scratch implementation. The older worktrees'
individual results do not certify this candidate.

Extend the existing real Linux service-recovery selection instead of adding a
second runner or changing the approved architecture. Every native scenario now
uses the bootstrap-issued API key, including cancellation and requests following
restart. Anonymous and invalid credentials must reject configuration access.
The added dry-run scenario persists configuration, completes a real discovered
job, reopens the process and checks unchanged source bytes and absence of adjacent
or workspace media artifacts. The scratch scenario supplies an owned sparse
file exceeding the existing 20-GiB logical workspace budget. It requires observed
capacity deferral, unchanged source and preserved caller-owned scratch, then
joined shutdown, removal of the fixture blocker and successful same-attempt
replacement after restart. Completed temporary media must be reclaimed.

Startup now detects and reports a serving child that exits before health
readiness, rather than polling until the timeout and losing its diagnostics.
The qualification setup selects the pinned PostgreSQL image and preserves the
service user's ownership of generated OpenAPI artifacts. These fixture fixes do
not change production authentication or workflow behavior.

The sparse test measures logical-byte admission and cleanup, not physical disk
exhaustion, throughput or peak RSS. Existing watcher/schedule discovery, real
FFmpeg interruption/cancellation, changed/missing sources, missing/corrupt
checkpoints and prepared/committed replacement recovery remain in the same
selection. Existing discovery mode fault fixtures remain fixture qualification;
normal operator activation is separately exercised by the full UI gate.

## Task Record

- Motivation: establish one current-source operator workflow result instead of combining historical passes from different snapshots.
- Design notes: keep the existing runtime, initializer, authentication and injected collaborators. Add real-service assertions only; no new configuration knob, coordination protocol or production dependency.
- Test coverage summary: run `just test-media-service-recovery`, `just test-media-recovery`, `just ci` and normal `just ui-e2e` on the final unchanged snapshot. The explicit service selection must execute one passing test. Retain command exit status, full logs, source file hashes/modes, commit and environment/image identities in `target/operator-qualification/`. Acceptance is established only by successful fresh results for all four commands and matching source identities before and after them; a planned command or earlier result is not a pass.
- Observability updates: assert the existing `media_job_outcomes_total` capacity-deferral signal, durable attempt identity, job disposition and filesystem cleanup. Add concise native scenario outcome lines. Do not print or persist the bootstrap key.
- Status-doc validation: reviewed the release verification matrix, completion ledger, accepted recovery/cleanup ADRs and normal E2E phases. This record claims local workflow qualification only when its current-source evidence passes; package, Sonar, merge and release acceptance remain separate.
- Risk & rollback plan: test-only changes affect native fixture authentication and scenario sequencing. Revert this qualification change if the fixture is defective; preserve all production safety checks and original media. Join service children, close restricted clients, remove generated media and owned disposable databases, then stop retained qualification containers. Do not touch unrelated containers or the main checkout's unfinished cherry-pick.
- Dependency rationale: none added; existing HTTP, filesystem, PostgreSQL, FFmpeg and runtime fixtures suffice.
- Stale-policy check: reviewed root `AGENTS.md` and scoped Rust, data, UI, Python and DevOps instructions, the Justfile compatibility aliases and canonical rv task registry. The user-supplied Justfile requirement is honored through its direct rv aliases. No quality gate, lint, coverage, Sonar setting, dependency exception or approved design was relaxed. Rust guidance now names the expanded native workflow evidence; no other instruction contradiction was identified.
