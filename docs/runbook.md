# Phase One Runbook

This runbook exercises the end-to-end control plane, validating FsOps, telemetry, and guard rails.
Use `just ui-e2e` for the repeatable API/browser workflow. The retained
`just runbook` command wraps that workflow for deployments that explicitly need
its archive output; do not use it to create a separate proof package for routine
work. Current media scenarios are part of the existing API/UI suite, with focused
recovery selections available through `just test-media-recovery` and
`just test-media-service-recovery` (the latter requires Linux and its managed-root
fixture). Use the task help and test README for exact prerequisites.

## Automated Checks

```sh
just ui-e2e
```

Use focused regression tests for a demonstrated failure. Do not repeat completed
checks or add independent verification frameworks merely to generate evidence.
The deployment-specific drills below concern actual mounts, permissions and
restart boundaries; they do not replace required CI or release checks.

## Prerequisites

-   Docker image `revaer:ci` (built via `just docker-build`) or a local `revaer-app` binary (`just build-release`).
-   PostgreSQL instance accessible to the application.
-   API key with a conservative rate limit (e.g., burst `5`, period `60s`).
-   CLI configured with `REVAER_API_URL`, `REVAER_API_KEY`, and optional `REVAER_TELEMETRY_ENDPOINT`.

## Scenario

1. **Bootstrap**

    - Issue a setup token: `revaer setup start --issued-by runbook`.
    - Complete configuration with CLI secrets and directories: `revaer setup complete --instance runbook --bind 127.0.0.1 --resume-dir .server_root/resume --download-root .server_root/downloads --library-root .server_root/library --api-key-label runbook --passphrase <pass>`.
    - Inspect the committed snapshot via `revaer config get --output table` and confirm `/health/full` returns `status=ok` with `guardrail_violations_total=0`.

2. **Add Torrent & Observe FsOps**

    - Add a torrent: `revaer torrent add <magnet> --name runbook`.
    - Tail events: `revaer tail --event torrent_added,progress,state_changed --resume-file .server_root/revaer.tail`.
    - Verify FsOps emits `fsops_started`, `fsops_completed`, and Prometheus counters `fsops_steps_total` increase.

3. **Restart & Resume**

    - Stop the application, restart it, and ensure the torrent catalog repopulates.
    - Confirm `SelectionReconciled` (if metadata diverges) and `HealthChanged` clears once resume succeeds.

4. **Rate Limit Guard-Rail**

    - Apply a tight API key limit (burst `1` / `per_seconds 60`) via `revaer config set --file rate-limit.json` (using a JSON patch that updates the relevant key).
    - Execute three rapid CLI calls (e.g., `revaer status <id>`). The third should exit with code `3`, displaying a `429` Problem+JSON response.
    - Inspect `/metrics` to verify `api_rate_limit_throttled_total` incremented and `/health/full` reflects `degraded=["api_rate_limit_guard"]`.

5. **Recovery**

    - Restore the API key limit to an acceptable value through another `revaer config set ...` invocation.
    - Re-run `revaer status <id>` to confirm success, `guardrail_violations_total` stops increasing, and `degraded` returns to `[]`.

6. **FsOps Failure Simulation**
    - Temporarily revoke write permissions on the library directory and re-run a completion.
    - Observe `fsops_failed` events, `HealthChanged` with `["fsops"]`, and guard-rail telemetry.
    - Restore permissions and confirm recovery events.

Manual-only rationale:

- Permission failures and restart/resume drills depend on the actual runtime mount layout, writable volumes, and supervisor behavior of the target deployment.
- The checked-in automation covers the repeatable control-plane baseline; these remaining drills intentionally stay manual so operators can validate their real environment rather than a simulated local-only shell.

## Results and Cleanup

Report failures and relevant revision information briefly. Keep ordinary command
output temporary; do not archive telemetry, screenshots or health/metrics
snapshots solely to substantiate routine work. Application audit/checkpoint
records and required release contracts remain intact.

Stop or remove only owned disposable resources after checking whether another
active task still needs them. Preserve persistent databases, original media and
unfinished replacement/rollback artifacts. This runbook does not itself establish
passing CI, a published release or production readiness; follow AGENTS.md and
the existing release checks for the revision being operated.
