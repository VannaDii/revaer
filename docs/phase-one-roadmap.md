# Project Roadmap

_Last reconciled: 2026-10-15_

## Implemented scope

The original Phase One control-plane remediation is recorded as closed in
[the remediation plan](https://github.com/VannaDii/revaer/blob/main/REMEDIATION_PLAN.md). Transcoding merged into `main`
in PR #31 on 2026-10-15. The old media restack and operator-checkpoint sequence
is complete as a delivery plan; it is not the starting point for new work.

| Area | Implemented surface | Reference |
| --- | --- | --- |
| Configuration | Setup, typed PostgreSQL-backed configuration, API keys and runtime updates | [Configuration](platform/configuration.md) |
| Torrents and filesystem | Native libtorrent integration, torrent controls, archive/PAR2 processing and managed transfers | [Torrent flows](platform/torrent-flows.md) |
| Public interfaces | HTTP API, CLI, SSE, health and metrics | [API](platform/api.md), [CLI](platform/cli.md) |
| Media | Relational profiles and root associations, manual/watcher/scheduled discovery, dry-run planning, local execution and verified replacement | [Transcoding design](https://github.com/VannaDii/revaer/blob/main/MEDIA_TRANSCODING.md) |
| Media lifecycle | Checkpoint restart, explicit cancellation, source-change handling, configured concurrency, bounded scratch and cleanup | [Transcoding design](https://github.com/VannaDii/revaer/blob/main/MEDIA_TRANSCODING.md#recovery-concurrency-and-scratch) |
| Tooling | uv-managed Python `rv` commands, delegated Just recipes and shared local/CI task implementations | [Tooling](https://github.com/VannaDii/revaer/blob/main/tools/README.md) |

## Continuing work

Use current source, open issues/PRs and their owners' handoffs to choose work.
Historical assertions that pagination, setup, media runtime or tooling still
need to be implemented are not an actionable backlog. Recheck a specific claim
against its owning implementation before carrying it forward.

Implementation merged to `main` is distinct from a published release or a running
deployment. Required CI, UI, security, Sonar and release checks remain controlled
by the current task registry and workflows. Consult [the release checklist](release-checklist.md)
for publication and [the runbook](runbook.md) for deployment operations.

The [original remaining-scope specification](phase-one-remaining-spec.md) is
historical. Preserve its product intent where still applicable, but do not
revive completed tasks, obsolete thresholds or artifact-collection instructions.
Future architectural changes require the operator decision described in
[AGENTS.md](https://github.com/VannaDii/revaer/blob/main/AGENTS.md).
