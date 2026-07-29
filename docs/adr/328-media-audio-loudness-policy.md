# Media Audio Loudness Policy

- Status: Accepted
- Date: 2026-07-22

## Motivation

Desired audio targets could already pin bitrate and sample rate, but they could not express explicit loudness or dynamic-range processing. That left the desired-target graph unable to carry an operator's dialog-normalization intent through API, persistence, job snapshots, YAML bundles, and runtime command construction.

## Design Notes

- Add normalized `audio_loudness_profile` and `audio_dynamic_range` fields to desired audio target rows and immutable job snapshots.
- Permit only `dialog-normalized` for loudness and `preserve` or `speech` for dynamic range. API validation, YAML import validation, core target compilation, stored procedures, and relational constraints reject invalid values and reject audio-only fields on non-audio streams.
- Move data access to v5 stored procedures while preserving v4 append compatibility by forwarding missing policy fields as `NULL`.
- Force a concrete audio encoder when the desired codec otherwise matches the source but the policy requires a filter. A copy codec cannot apply `loudnorm` or compression.
- Emit `loudnorm=I=-16:TP=-1.5:LRA=11` for `dialog-normalized` and `acompressor=threshold=-18dB:ratio=2:attack=20:release=250` for `speech`.
- Do not claim measured loudness verification in this slice. Candidate and final verification still inspect bitrate and sample rate here; follow-up ADR 329 adds measured verification for the implemented `dialog-normalized` and `speech` policies.

## Test Coverage Summary

- `cargo check -p revaer-media-core -p revaer-media-runtime -p revaer-data -p revaer-app -p revaer-api -p revaer-api-models --all-targets`
- `cargo test -p revaer-media-runtime desired_graph_audio_policy_filters_force_audio_encoder -- --nocapture`
- `cargo test -p revaer-api desired_target_audio_constraints_are_normalized_and_mapped -- --nocapture`
- `REVAER_TEST_DATABASE_URL=postgres://revaer:revaer@localhost:5432/revaer DATABASE_URL=postgres://revaer:revaer@localhost:5432/revaer cargo test -p revaer-data desired_target_versions_are_ordered_immutable_job_snapshots -- --nocapture`
- `just policy`
- Sonar read-only checks for PR 31: `sonar list issues --project VannaDii_Revaer --statuses OPEN,CONFIRMED --pull-request 31 --format table --page-size 500`, `sonar api get "/api/qualitygates/project_status?projectKey=VannaDii_Revaer&pullRequest=31"`, `sonar api get "/api/measures/component?component=VannaDii_Revaer&pullRequest=31&metricKeys=coverage,line_coverage,lines_to_cover,uncovered_lines"`, `sonar api get "/api/issues/search?componentKeys=VannaDii_Revaer&pullRequest=31&resolved=false&inNewCodePeriod=true&ps=1"`, and `sonar api get "/api/hotspots/search?projectKey=VannaDii_Revaer&pullRequest=31&status=TO_REVIEW&inNewCodePeriod=true&ps=1"`.
- Local `sonar verify --file ... --project VannaDii_Revaer` was not run because the escalation reviewer rejected uploading private repository file contents to SonarCloud. This is a tooling/privacy blocker, not a clean file-scoped analysis result.

## Observability Updates

- No new metric labels were added. The policy affects deterministic runtime argv construction and existing media job execution/verification evidence.
- Sonar PR 31 read-only checks reported an OK quality gate, coverage metrics present, no unresolved new-code issues, and no unreviewed new-code hotspots at the time of this task.

## Risk And Rollback Plan

- Roll back by reverting migration `0159` and the v5 data-access wiring before any deployed database has policy rows depending on the new columns.
- If a deployed database has policy rows, first create a replacement desired-target version without loudness or dynamic-range fields, repin profiles to that version, drain queued jobs that snapshotted the fields, and then remove the columns and v5 procedures.
- Runtime rollback is low blast radius because the new filters are emitted only when the immutable target explicitly requests the policy.

## Dependency Rationale

- No new dependency was added. The implementation uses existing FFmpeg filter support and existing Rust/database/API crates.

## Stale-Policy Check

- Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, `.github/instructions/devops.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md`.
- No policy relaxation was introduced. Sonar scope, issue-ignore, coverage, duplication, analyzer, SCA, SCM, and quality-gate criteria remain strict. File-scoped Sonar upload was blocked by environment policy and was not bypassed.
