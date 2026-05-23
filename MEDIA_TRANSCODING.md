# Revaer Media Transcoding Implementation Plan

> For agentic workers: required implementation workflow is
> `superpowers:subagent-driven-development` or
> `superpowers:executing-plans`. Implement this plan task-by-task, keep the
> root repository policy in `AGENTS.md` authoritative, add the required ADR for
> each implementation slice, and run `just ci` plus `just ui-e2e` before
> handoff.

Status: consolidated executable plan.

Date: 2026-05-23.

Goal: build Revaer's full media transcoding subsystem as a deterministic
desired-state reconciliation engine for media files.

Architecture: media profiles bind one non-overlapping path to one target and one
policy. Revaer discovers files, inspects actual media state, compiles desired
media state, selects the least expensive safe reconciliation plan, executes in a
managed workspace, verifies output, then replaces source files only after
successful verification. Configuration is relational at runtime, with Revaer
YAML used only for versioned import/export.

Tech stack: Rust 2024, PostgreSQL stored procedures through `revaer-data`,
Axum/OpenAPI in `revaer-api`, Yew UI in `revaer-ui`, typed SSE events through
`revaer-events`, managed filesystem workspaces, ffmpeg-compatible and
ffprobe-compatible injected adapters, and a full-capability Docker image with
open-source media libraries.

---

## Non-Negotiable Repository Rules

This subsystem must follow the root repository contract:

- Rust 2024 only.
- No authored production or bootstrap `panic!`, `unwrap()`, `expect()`,
  `unreachable!()`, `todo!()`, or `unimplemented!()`.
- No dead code, future stubs, or parking-lot code.
- No source-level lint suppressions such as `#[allow(...)]` or
  `#[expect(...)]`.
- Minimal Rust dependencies. Every new Rust dependency needs ADR rationale.
- Runtime database access goes through stored procedures. Raw SQL belongs only
  in migrations, stored procedure definitions, and scoped operational bootstrap
  scripts.
- No JSONB or other conglomerate persistence for application state. Store media
  configuration and job state in normalized tables.
- Runtime logic receives dependencies from callers. Only bootstrap/wiring code
  constructs concrete adapters or reads environment.
- All local and CI gates run through `just`.
- Implementation handoff requires `just ci` and `just ui-e2e`.
- Each implementation slice adds an ADR under `docs/adr/`, updates
  `docs/adr/index.md`, and updates `docs/SUMMARY.md`.

Existing repository patterns to reuse:

- `crates/revaer-config`: typed configuration models and semantic validation.
- `crates/revaer-data`: stored-procedure-backed persistence.
- `crates/revaer-runtime`: narrow persistence facade over runtime state.
- `crates/revaer-events`: typed event payloads surfaced through SSE.
- `crates/revaer-api`: Axum handlers, OpenAPI export, problem details.
- `crates/revaer-ui`: Yew feature slices, transport services, stable selectors,
  settings/config UX patterns.
- `crates/revaer-fsops`: safe filesystem processing, operational step tracking,
  managed workspaces, and event emission patterns.

---

## System Identity

Revaer media transcoding is a declarative media reconciliation engine.

It is not an ffmpeg preset runner.

Core loop:

```text
discover -> inspect -> normalize -> compare -> plan -> execute -> verify -> replace
```

Core invariant:

```text
match = what files are in scope
target = immutable desired output media graph
policy = operational behavior when actual media differs from target
```

Targets own final state:

- Output container format.
- Desired stream count.
- Desired stream order.
- Desired codecs and formats.
- Desired channel layouts.
- Desired subtitle placement.
- Desired default and forced dispositions.
- Desired metadata shape.

Policies own behavior:

- What to do when source media is incompatible.
- What to do with unmatched streams.
- Whether unsupported streams fail, preserve, or remove.
- Operation cost weights and ranking.
- Runtime limits and discovery automation.
- Verification strictness.
- Backup, quarantine, workspace, and replacement behavior.
- Dry-run behavior.

Policies must not own or mutate:

- Stream ordering.
- Desired codecs.
- Desired default streams.
- Desired channel layouts.
- Desired subtitle target placement.

Breaking this separation causes planner ambiguity, non-deterministic compliance,
hidden data loss, unexplainable output changes, and irreproducible behavior.

Determinism requirement:

```text
same input media
+ same target
+ same policy
+ same runtime capabilities
= same reconciliation plan
```

Explainability requirement:

- Every stream selection and rejection is explainable.
- Every transformation is explainable.
- Every avoided transformation is explainable.
- Every selected plan and rejected plan is explainable.
- Every compliant, non-compliant, unsupported, failed, and skipped state is
  represented as data.

No hidden mutation:

- Every transformation appears in the selected plan.
- Every destructive intent is persisted before execution.
- Every command is generated from an explicit operation.
- Every output is reinspected before replacement.
- Nothing mutates source files as a side effect of inspection, planning, import,
  export, preview, or dry-run jobs.

---

## First Release Scope

The first release is the full system, implemented in slices. It is not a
dry-run-only release. It remains safe by default because every profile defaults
to dry-run and source mutation requires explicit configuration.

Included:

- Profile CRUD for match, target, policy, compatibility, discovery, retention,
  and output settings.
- Revaer YAML import/export using Revaer's own versioned format.
- One non-overlapping path-to-profile association per configured media path.
- Manual discovery by default.
- Disabled-by-default filesystem watchers per path-to-profile association.
- Disabled-by-default scheduled scans per path-to-profile association with one
  interval in minutes or hours.
- Immediate job enqueueing from enabled watchers and scheduled scans.
- Capability discovery at startup and on demand.
- Full media inspection through injected ffprobe-compatible adapters.
- Full open-source media toolchain in the Docker image.
- Normalization of containers, streams, codecs, languages, dispositions, labels,
  roles, HDR metadata, sidecars, chapters, attachments, and metadata.
- Multi-video source management. Profiles can preserve, remove, reorder, copy,
  remux, or transform video streams, including removing all but one output
  video stream.
- Ordered audio targets.
- Ordered subtitle targets.
- Retention rules for every unmatched stream family.
- Diff-based compliance reports and scoring.
- Costed plan generation with selected and rejected plan explanations.
- Execution DAGs.
- Managed workspaces.
- Disk-impact estimation and reserve enforcement.
- Remux, metadata/disposition rewrite, subtitle extract/embed, audio transcode,
  video transcode, backup, quarantine, verification, and atomic replacement.
- In-place source replacement only after successful output verification.
- API, OpenAPI, UI, SSE events, metrics, logs, and E2E coverage.

Excluded:

- OCR.
- Subtitle acquisition from external services.
- Subtitle generation.
- Foreign preset import.
- Multiple output files from one source file.
- Client-specific output variants from one source file.
- Distributed workers.
- Perceptual hashes in the first release.
- Automatic destructive replacement by default.
- Playback certification beyond configured compatibility targets.
- Proprietary, closed-source, or non-redistributable media components in the
  default Docker image.

---

## Resolved Product Decisions

Dry-run and source mutation:

- New profiles are always dry-run.
- Imported profiles are always forced to dry-run.
- Profile-level dry-run controls automatic jobs from watchers and schedules.
- Manual discovery over a dry-run profile creates only plan and audit jobs.
- Manual discovery cannot use the `replace` confirmation phrase.
- Explicit manual job execution may override a dry-run profile for that run
  only.
- A destructive manual override of a dry-run profile requires the exact typed
  phrase `replace`.
- A non-dry-run saved profile requires no typed confirmation phrase beyond
  normal authorization.
- A manual override never mutates the saved profile dry-run setting.

Discovery:

- Discovery uses a 1:1 path-to-profile association.
- Each configured media path maps to exactly one profile.
- Configured discovery paths must be non-overlapping. Revaer rejects nested or
  otherwise overlapping configured discovery paths.
- Manual discovery is the only active behavior by default.
- Watchers are disabled by default and may be enabled per path-to-profile
  association.
- Scheduled scans are disabled by default and may be enabled per
  path-to-profile association.
- Scheduled scans use one interval expressed in minutes or hours.
- There is no default scheduled scan cadence.
- Watchers and scheduled scans enqueue jobs immediately when enabled.
- Jobs enqueued by watchers and scheduled scans inherit the associated profile
  dry-run mode.
- Scheduled scans remain available when watchers are enabled to recover from
  missed events, restarts, and offline storage.

Replacement:

- Non-dry-run execution replaces source files in place only after successful
  output verification.
- Replacement uses a managed workspace and atomic replacement flow.
- Completed outputs are not left beside originals as normal behavior.
- Replacement failure must leave the original source intact or recoverable.
- Replacement failure must clean managed transient artifacts according to policy.

Backups:

- Backup is optional for destructive execution.
- Default backup behavior is no backup.
- If backup is enabled, the operator must configure a backup root.
- Backup roots pass the same canonical path, free-space, cleanup, and retention
  validation as other managed roots.
- If backup is required and cannot satisfy path or space validation, the job
  fails before execution.

Subtitles:

- The first release manages subtitles that already exist as embedded streams or
  sidecar files.
- Supported subtitle operations are embedding existing sidecars, extracting
  existing embedded subtitles, copying existing sidecars when configured, and
  preserving/removing/failing according to policy.
- No subtitle acquisition.
- No subtitle generation.
- No OCR.

Job history:

- Completed job-history retention is disabled until configured.
- Completed job-history retention may be configured as one age/time window or
  one retained-job count.
- Failed terminal jobs use a separate longer diagnostic retention policy.
- Failed-job diagnostic retention defaults to a 30-day age window.
- Failed-job diagnostic retention may be overridden independently by age/time
  window or retained-job count.
- Retention cleanup preserves compact audit facts needed to explain destructive
  source mutations.
- Retention cleanup may expire bulky job details and diagnostics according to
  policy.
- Retention cleanup never deletes active, running, failed-with-retry, or
  non-terminal jobs.

Capabilities:

- Capability refresh runs at startup and on demand.
- Startup refresh inspects the installed media toolchain and persists detected
  capabilities before media execution.
- Startup capability refresh failure blocks media execution until a valid
  snapshot exists.
- Startup capability refresh failure does not prevent the rest of Revaer from
  starting.
- On-demand refresh is available from API and UI.
- Refresh failures are visible through health, events, metrics, and capability
  APIs.

Disk preservation:

- Default disk reserve is 20% of the filesystem capacity that contains the
  managed workspace root.
- Operators may configure a percentage reserve or a fixed amount with explicit
  units such as MB, GB, or GiB.
- If both percentage and fixed reserve are introduced later, the larger effective
  byte value wins.
- Execution refuses to start when estimated peak usage cannot fit above the
  configured reserve.
- Disk amplification is part of plan cost and risk.

Compatibility targets:

- Compatibility targets are user-configured.
- Revaer ships seed targets for general Plex direct play and Plex on Apple TV.
- Seed targets are conveniences, not hardcoded planner behavior.
- Operators may create, edit, disable, or ignore compatibility targets.

---

## End-To-End Pipeline

```text
load config
-> validate config
-> refresh or read runtime capabilities
-> compile profile associations
-> discover candidate files
-> inspect media
-> normalize media graph
-> discover existing sidecars
-> classify streams
-> compile desired media graph
-> diff actual vs desired
-> generate candidate plans
-> prune invalid and dominated plans
-> select least expensive safe plan
-> estimate disk impact
-> execute plan in managed workspace when non-dry-run
-> inspect output
-> verify compliance
-> backup when configured
-> replace source atomically
-> final verification
-> cleanup workspace
-> persist compact audit and job outcome
```

Dry-run jobs stop after planning, disk-impact estimation, and audit persistence.
They do not create source-adjacent files, backups, sidecars, replacement files,
or unmanaged temporary artifacts.

---

## Domain Model

Core abstractions:

```text
MediaMap
DesiredMap
PolicySet
Diff
Violation
Operation
ExecutionGraph
Plan
PlanExplanation
ComplianceResult
VerificationResult
CapabilitySet
Workspace
Job
AuditEvent
```

Rust type sketch:

```rust
struct MediaMap {
    container: ContainerState,
    streams: Vec<StreamNode>,
    chapters: Vec<Chapter>,
    attachments: Vec<Attachment>,
    metadata: Metadata,
    sidecars: Vec<SidecarSubtitle>,
}

struct DesiredMap {
    container: ContainerTarget,
    streams: Vec<DesiredStream>,
    metadata: MetadataTarget,
}

struct PolicySet {
    planning: PlanningPolicy,
    compatibility: CompatibilityPolicy,
    retention: RetentionPolicy,
    verification: VerificationPolicy,
    runtime: RuntimePolicy,
    output: OutputPolicy,
    workspace: WorkspacePolicy,
    backup: BackupPolicy,
}

struct Plan {
    operations: Vec<Operation>,
    estimated_cost: Cost,
    disk_impact: DiskImpact,
    risk: Risk,
    explanation: PlanExplanation,
}

struct ComplianceResult {
    status: ComplianceStatus,
    score: f32,
    violations: Vec<Violation>,
}

struct VerificationResult {
    passed: bool,
    checks: Vec<VerificationCheck>,
}
```

Stream identity:

```rust
struct StreamIdentity {
    stream_kind: StreamKind,
    language: Option<Language>,
    role: StreamRole,
    codec: Codec,
    channels: Option<ChannelLayout>,
    placement: Placement,
    source_quality: Option<QualityTier>,
}
```

Stream indexes are unstable and must never be durable identity. Use stream
identity, source path, normalized metadata, and fingerprints for planning and
audit references.

Stream roles:

- Audio: `main`, `commentary`, `descriptive`, `karaoke`, `alternate`,
  `unknown`.
- Subtitle: `forced`, `full`, `sdh`, `commentary`, `signs_songs`, `karaoke`,
  `unknown`.
- Video: `main`, `alternate`, `angle`, `unknown`.

Stream ordering:

- Desired stream arrays define final mux order.
- Ordering implies priority.
- Target stream rows use `sort_order`.

Recommended audio ordering for Plex and Apple TV:

1. Best primary-language main stereo-compatible track.
2. Best primary-language surround track if different.
3. Other main-language tracks.
4. Original-language tracks.
5. Dub tracks.
6. Descriptive audio tracks.
7. Commentary tracks last.

Recommended subtitle ordering:

1. Forced subtitles matching default audio language.
2. Forced subtitles for original language.
3. Full subtitles for preferred language.
4. SDH/caption subtitles.
5. Other languages.
6. Commentary subtitles last.

Disposition rules:

- Exactly one default audio stream unless target explicitly allows otherwise.
- At most one default subtitle stream unless target explicitly allows otherwise.
- Commentary must not become default unless explicitly configured.
- Forced subtitle preservation where detected.
- Conditional subtitle default behavior for foreign-language audio.

Labeling:

- Labels are presentation metadata unless explicitly overridden.
- Runtime-generated labels are the default.
- Resolution hierarchy: custom per-stream label -> profile template -> runtime
  best-practice label.
- Example labels: `English Stereo AAC`, `English 5.1 EAC3`,
  `Japanese Stereo AAC`, `English Forced SRT`, `English Full ASS`,
  `HEVC 1080p HDR10`.

---

## Media Normalization And Classification

Normalization converts inconsistent tool output into canonical media state.

Normalize:

- Codec aliases, such as `dca` -> `dts` and `subrip` -> `srt`.
- Language tags, such as `eng-US` -> `eng`.
- Channel layouts.
- HDR metadata.
- Dolby Vision and HDR10 metadata.
- Color space and color range.
- Stream dispositions.
- Titles and labels.
- Container metadata.
- Chapter and attachment presence.
- Sidecar subtitle filenames.

Disposition cleanup examples:

- Multiple default audio streams -> normalize to target disposition.
- Commentary marked default -> clear unless target explicitly allows.
- Missing forced subtitle flag -> infer from title heuristics when policy
  permits inference.

Classification pipeline:

```text
raw ffprobe data
-> codec normalization
-> language normalization
-> disposition normalization
-> title and metadata heuristics
-> configured classification rules
-> semantic stream roles
```

Classification rules support:

- Stream kind.
- Role.
- Match type, such as title contains, title regex, disposition, language,
  codec, filename pattern.
- Pattern.
- Confidence.
- Sort order.
- Enabled state.

Ranking heuristics must be deterministic and configurable.

Example audio source quality order:

```text
TrueHD
DTS-HD MA
DTS
EAC3
AC3
AAC
MP3
```

Example subtitle quality order:

```text
ASS
SRT
VTT
PGS
VobSub
```

Ranking inputs:

- Language priority.
- Role priority.
- Codec quality.
- Bitrate.
- Channel layout.
- Source quality.
- Placement preference.
- Compatibility impact.
- Retention policy.

---

## Configuration Model

Runtime configuration is relational. YAML is only a Revaer-owned exchange format
for import/export.

Primary objects:

- `media_profile`: profile key, name, target profile reference, policy profile
  reference, enabled state.
- `media_profile_root`: non-overlapping root path, media type, enabled state,
  sort order, profile reference.
- `media_profile_file_rule`: include/exclude rule, glob or extension, sort
  order.
- `media_profile_filter`: min/max size, min/max duration, sample handling,
  trailer handling, trash/quarantine exclusion.
- `media_discovery_schedule`: disabled-by-default scan interval, interval unit,
  path-to-profile association, enable state.
- `media_discovery_watcher`: disabled-by-default watcher, debounce,
  path-to-profile association, enable state.
- `media_target_profile`: target key, name, version, enabled state.
- `media_target_container`: format, allowed formats, chapters, attachments,
  metadata mode, title policy, faststart behavior.
- `media_target_stream`: stream kind, stream key, role, language, optional,
  sort order, codec/format, default/forced disposition, label mode/template.
- `media_target_video_stream`: encoder preference, max width/height, max FPS,
  max bitrate, pixel format, bit depth, HDR mode, Dolby Vision policy, HDR10
  policy, tonemap policy, color space/range, deinterlace, crop, scale, quality.
- `media_target_audio_stream`: codec, channels, min channels, LFE policy,
  channel layout, bitrate per channel, passthrough formats, loudness profile,
  downmix behavior, dynamic range behavior.
- `media_target_subtitle_stream`: placement, subtitle format, role, language,
  sidecar behavior.
- `media_policy_profile`: policy key, name, version, enabled state.
- `media_policy_retention_rule`: stream kind, role, language, codec/format,
  action, placement, enabled state, sort order.
- `media_policy_unmatched_stream_behavior`: video, audio, subtitle, attachment,
  and data fallback actions.
- `media_policy_compatibility_rule`: unsupported format behavior and
  compatibility handling.
- `media_compatibility_target`: user-configured playback target definitions.
- `media_policy_compatibility_target`: ordered selected compatibility targets
  for a policy.
- `media_policy_operation_cost`: operation kind and cost weight.
- `media_policy_runtime_limit`: concurrency, maintenance window, retry, power,
  thermal, IO, pause, and disk-reserve settings.
- `media_policy_output`: dry-run, replacement mode, quarantine behavior,
  permissions/ownership behavior.
- `media_policy_workspace`: workspace root, retention, diagnostic artifacts,
  stale cleanup behavior.
- `media_policy_backup`: optional backup root, retention, and free-space
  behavior.
- `media_policy_verification`: strictness, duration tolerance, mux validation,
  playback probe choices.
- `media_job_retention_policy`: disabled-by-default completed-job retention mode
  and limit, failed-terminal diagnostic retention mode defaulting to 30 days,
  failed-terminal limit, compact audit retention behavior.
- `media_capability_snapshot`: last detected tools, codecs, encoders, decoders,
  muxers, demuxers, subtitle support, hardware acceleration, filesystem
  capabilities, and utility capabilities.
- `media_stream_classification_rule`: classification rules and confidence.
- `media_subtitle_discovery_rule`: sidecar discovery pattern, precedence, enabled
  state.
- `media_job`: job public id, profile/root association, source path, status,
  phase, dry-run effective value, selected target/policy versions, timestamps,
  compliance score, output disposition.
- `media_job_phase`: normalized job phase events.
- `media_job_operation`: selected operations and execution status.
- `media_job_violation`: normalized compliance violations.
- `media_job_plan_reason`: selected-plan reasons and rejected-plan reasons.
- `media_job_verification_check`: normalized verification checks.
- `media_job_artifact`: bounded diagnostic artifact references.
- `media_job_compact_audit`: compact audit facts retained after detail pruning.

Application state must not use JSONB. Planner explanations, audit facts, and
verification details should be normalized into rows. If a compact textual
diagnostic is useful, store it as bounded text or as a managed artifact reference
that is not mutable application state.

---

## Revaer YAML Exchange Format

Revaer supports import/export of media configurations in its own versioned YAML
format so operators can share ideal configurations.

Rules:

- Import only Revaer-authored bundles with supported `format_version`.
- Reject foreign preset formats.
- Runtime database remains the source of truth after import.
- Import reuses the same parser, compiler, semantic validator, and path mapper
  used by API/UI profile creation.
- Imports default to dry-run.
- Imports with unresolved local roots, workspace roots, backup roots, or
  quarantine roots remain draft/disabled until mapped.
- Imports never create files, source-adjacent sidecars, workspaces, backups, or
  replacement files.
- Portable exports omit local paths by default.
- Operators may explicitly choose a local-backup export that includes paths.

Format sketch:

```yaml
format_version: 1
kind: revaer.media.profile_bundle
metadata:
    name: general-plex-library
    description: General Plex direct-play library policy
compatibility_targets:
    - key: plex-direct-play
targets:
    - key: hevc-1080p-stereo-surround
policies:
    - key: dry-run-safe-replace
profiles:
    - key: movies-plex-default
      dry_run: true
      match:
          roots: []
      target_key: hevc-1080p-stereo-surround
      policy_key: dry-run-safe-replace
```

If YAML support adds a Rust dependency, the ADR must explain why the dependency
is needed and why hand-rolled parsing is not acceptable.

---

## Semantic Validation

Validation must reject:

- Multiple default audio streams unless target explicitly allows them.
- Multiple video streams with no explicit desired target or retention behavior.
- Required subtitle placement unsupported by the target container.
- Required codec unavailable under allowed capabilities.
- Required codec, profile, level, subtitle placement, or audio layout outside
  all configured compatibility targets.
- Retention rules that conflict at the same priority.
- Duplicate output stream identities.
- A destructive profile with no disk reserve.
- A policy that requires backup without a configured backup root.
- A policy that enables external subtitle acquisition or OCR.
- Diagnostic artifact retention that is unbounded by size or age.
- Enabled completed-job retention without exactly one age/time-window or count
  limit.
- Enabled failed-terminal diagnostic retention without exactly one
  age/time-window or count limit.
- Discovery paths associated with more than one profile.
- Overlapping discovery paths, including nested paths.
- Discovery schedules or watchers that do not resolve to one enabled
  path-to-profile association.
- Enabled discovery schedules without a positive minute/hour interval.
- Roots, workspace paths, backup paths, or quarantine paths that are missing, not
  directories, or outside the configured server policy.
- Unsupported sidecar discovery patterns.
- Invalid stream ordering rules.
- Required stream impossible under compatibility policy.
- Required conversion unsupported by runtime capabilities.

Validation must produce problem-detail pointers for API and UI remediation.

---

## Match And Discovery

Match scope determines which files are eligible.

Supported match properties:

- Non-overlapping root path.
- Media type.
- Include rules.
- Exclude rules.
- Recursive behavior.
- Minimum and maximum file size.
- Minimum and maximum duration.
- Sample detection.
- Trailer and extras handling.
- Trash and quarantine exclusion.

Example shape:

```yaml
match:
    roots:
        - path: /media/movies
          media_type: movie
    include:
        - "**/*.mkv"
        - "**/*.mp4"
    exclude:
        - "**/sample/**"
        - "**/.trash/**"
    filters:
        min_size_mb: 100
        min_duration_seconds: 300
```

Manual discovery:

- Manual discovery is the only active discovery mode by default.
- Runs against a selected path-to-profile association.
- Dry-run profiles create only plan/audit jobs.
- Cannot use `replace` confirmation.

Watcher discovery:

- Disabled by default.
- Enabled per path-to-profile association.
- Uses a debounce setting.
- Enqueues jobs immediately when enabled.
- Inherits profile dry-run setting.

Scheduled discovery:

- Disabled by default.
- Enabled per path-to-profile association.
- Uses one interval in minutes or hours.
- No default cadence.
- Enqueues jobs immediately when enabled.
- Inherits profile dry-run setting.

---

## Target Profiles

Targets describe final desired media graph, not procedure.

Container target:

- Preferred container.
- Allowed containers.
- Chapter preservation.
- Attachment preservation.
- Metadata mode.
- Title cleanup.
- Stream title normalization.
- MP4 faststart behavior.
- Sidecar behavior.
- Atomic replacement behavior.

Video target:

- Multiple source video streams are supported.
- Profiles may preserve, remove, reorder, copy, remux, or transform video
  streams.
- Common policy: remove all but one output video stream.
- Codec.
- Encoder preference.
- Max resolution.
- Max bitrate.
- Max FPS.
- Pixel format.
- Bit depth.
- HDR preservation.
- Dolby Vision preservation.
- HDR10 preservation.
- HDR-to-SDR tonemapping behavior.
- Color space and range.
- Deinterlacing.
- Crop rules.
- Scale rules.
- Quality mode and value.
- Anime exceptions.
- Archival exceptions.
- Disposition.

Audio target:

- Preferred codec and fallback codec.
- Channel count and channel layout.
- Minimum channels.
- Bitrate per channel.
- Downmix rules.
- Passthrough formats.
- Language priority.
- Commentary exclusion or retention.
- Descriptive audio handling.
- LFE preserve/drop behavior.
- Loudness normalization.
- Dynamic range compression.
- Stereo compatibility track generation.
- Surround preservation.
- Default track rules.

Subtitle target:

- Existing embedded subtitle streams.
- Existing sidecar subtitle files.
- Placement: embedded, sidecar, both, none.
- Format: preserve or configured format where conversion is supported without
  OCR.
- Role.
- Language.
- Optional/required state.
- Default and forced disposition.
- Sidecar discovery patterns:
  - `{stem}.{lang}.{role}.{ext}`
  - `{stem}.{lang}.{ext}`
  - `{stem}.{role}.{ext}`
- Supported extensions include `srt`, `ass`, `vtt`, `sup`, `sub`, and `idx`.
- Image-based subtitles such as PGS and VobSub are policy-controlled.
- Image subtitle actions: preserve, remove, fail.
- Default recommendation for ambiguous image subtitles is fail.
- OCR is unsupported.

Declarative stream targets:

```yaml
audio:
    desired:
        - id: primary-stereo
          match:
              language: eng
              role: main
          transform:
              codec: aac
              channels: 2
              loudness: dialog-normalized
          disposition:
              default: true

        - id: primary-surround
          match:
              language: eng
              role: main
              channels: ">=6"
          optional: true
          transform:
              codec: eac3
              channels: preserve
              lfe: preserve
          disposition:
              default: false
```

Avoid boolean-heavy ambiguity:

```yaml
preserve_surround: true
stereo_track_required: true
commentary_last: true
forced_first: true
extract_sidecar: true
embed: true
```

Use ordered `desired` declarations where multiplicity matters. Singleton domains
such as `container` do not need `desired`.

---

## Policy Profiles

Policy composition:

```text
media_profile
-> match/path association
-> target_profile
-> policy_profile
```

Reusable combinations:

- Apple TV target + archival-safe policy.
- Apple TV target + aggressive-space-saving policy.
- Plex 4K target + remux-first policy.
- Anime-preservation target + fail-closed policy.

Retention rules define what happens to source streams not matched by target
streams.

Actions:

- `preserve`
- `remove`
- `fail`

Fallback unmatched behavior:

```yaml
unmatched_video_action: fail
unmatched_audio_action: preserve
unmatched_subtitle_action: preserve
unmatched_attachment_action: preserve
unmatched_data_action: remove
```

Planning policy:

- No-op if already compliant.
- Remux before transcode.
- Transcode only offending streams.
- Preserve original when uncertain.
- Dry-run by default.
- Quality regression guards.
- Checksums and fingerprints.
- Rollback and cleanup behavior.

Quality guards:

- Prevent upscale by default.
- Prevent fake audio channel upmix by default.
- Prevent lossy-to-lossy video reencode unless explicitly enabled.
- Preserve higher-quality sources when already compliant.
- Prevent runaway remux loops.
- Prevent recursive reconversion.
- Enforce maximum transcode size growth.
- Enforce minimum acceptable quality.
- Do not transcode output generated by the same profile within a configured
  guard window unless explicitly forced.

Runtime policy:

- Max parallel jobs.
- CPU/GPU preference.
- Hardware acceleration.
- CPU fallback.
- Thermal limits.
- Power limits.
- IO throttling.
- Maintenance windows.
- Pause windows.
- Resume behavior.
- Pause when active streams are detected.
- Failed-job retries.
- Event emission.
- Metrics.

Output policy:

- Dry-run.
- Replace in place after verify.
- Write all transient files under managed workspace.
- Optional backup to configured backup root.
- Quarantine output on verification failure.
- Preserve permissions.
- Preserve ownership.
- Atomic move.
- Naming convention for managed artifacts.
- Reflink/hardlink behavior where safe.

---

## Media Intents

Media intent affects heuristics without mutating target semantics.

Supported intent families:

- General.
- Anime.
- Audiobook.
- Archival.

Anime intent:

- Preserve ASS subtitles.
- Preserve fonts and attachments.
- Preserve grain.
- Protect line art and gradients.
- Avoid naive hardware encodes where policy marks them unsafe.
- Prefer subtitle styling retention.
- Avoid destructive debanding choices unless configured.

Audiobook intent:

- Allow mono where appropriate.
- Prioritize chapters.
- Prioritize speech loudness normalization.
- Support speech-focused dynamic range and bitrate decisions.

Archival intent:

- Prefer stream preservation.
- Prefer remux over transcode.
- Preserve attachments.
- Preserve original subtitles.
- Preserve metadata unless policy says otherwise.

---

## Compatibility And Capabilities

Compatibility targets:

- User-configured.
- Seed target: general Plex direct play.
- Seed target: Plex on Apple TV.
- Include codec support, profiles, levels, max bitrate, subtitle format support,
  audio passthrough support, container support, remote streaming caps, and known
  client limitations.

Runtime capability discovery is separate from targets to avoid profile
explosion.

Bad:

```text
apple-tv-nvenc
apple-tv-qsv
apple-tv-cpu
```

Good:

```text
apple-tv target + runtime capability selection
```

Capability discovery categories:

- Encoders: `nvenc`, `qsv`, `vaapi`, `software`, and other detected
  open-source encoders.
- Decoders: hardware decode and software decode.
- Containers: muxers and demuxers.
- Subtitles: renderers, mux support, extract support.
- Filters.
- Probes.
- Filesystem: atomic rename, reflink, hardlink, free-space query.
- Utilities: ffmpeg-compatible encoder/decoder/probe tools and
  container/subtitle utilities.

Capability fallback order:

```text
copy
-> remux
-> nvenc
-> qsv
-> vaapi
-> software
```

Intent-specific exceptions are allowed, such as anime avoiding `nvenc`.

---

## Docker Runtime Image

The default Docker image is a full media-processing runtime, not a minimal
ffmpeg-only image.

Requirement:

```text
All available open-source codecs and libraries that facilitate transcoding must
be included in the Docker image by default.
```

Operational definition:

- Include open-source, redistributable codec libraries, container libraries,
  demuxers, muxers, subtitle renderers, filters, probe tools, and media
  utilities available through selected build sources.
- Include tools needed to expose those libraries to Revaer, such as
  ffmpeg-compatible encode/decode/probe tooling and container/subtitle
  utilities.
- Prefer distro-supported packages when they provide required coverage.
- If a required open-source library is unavailable from base package sources,
  add an auditable build path or record a specific ADR exclusion.
- Exclude proprietary, closed-source, and non-redistributable components.
- Generate a package and codec inventory artifact in CI/release outputs.
- Surface installed codecs, encoders, decoders, muxers, demuxers, subtitle
  support, hardware acceleration, and utilities through the capability API.

Published runtime image distribution policy:

- Published runtime images must contain compiled executables, shared libraries,
  required runtime data such as fonts and plugin assets, license notices, and
  generated inventory/SBOM artifacts only.
- Published runtime images must not contain source archives, source checkouts,
  VCS metadata, build directories, package-manager caches, object files, static
  archives, headers, compilers, assemblers, linkers, build systems, or
  intermediate build outputs.
- Source downloads, source checkouts, compilation, and assembly happen only in
  builder stages.
- The final Docker stage must copy only runtime artifacts from builder stages.
- The release pipeline must inspect the final image and fail if third-party
  media component source trees, build inputs, or builder-only tools are present.
- GPL/LGPL and similar source-availability obligations are satisfied by a
  release source-compliance artifact or source-offer link generated from the
  same build manifest, not by embedding source code in the runtime image.
- Interpreted utilities are source-form programs even when installed as
  executables. Any interpreted media utility requires an ADR-backed exception or
  must be replaced by compiled/runtime-library functionality.
- The release artifact set is incomplete and must not be published until
  corresponding-source, license, attribution, notice, and patent-review evidence
  exists for every shipped third-party media component.

Current-version policy:

- The versions below are the current upstream targets as of 2026-05-23.
- Implementation may use distro packages only when they provide the same
  component and an acceptable security-support posture.
- If the base image package lags behind the listed current version, the Docker
  image task must either build from the official source or record an ADR-backed
  exclusion.
- Components with no stable release cadence, such as x264, must be pinned to an
  exact upstream commit in the generated image inventory.
- The generated inventory must include executable paths, library versions,
  `ffmpeg -version`, `ffmpeg -buildconf`, `ffmpeg -codecs`, `ffmpeg -encoders`,
  `ffmpeg -decoders`, `ffmpeg -formats`, `ffmpeg -filters`, `ffmpeg -bsfs`, and
  `ffprobe -version`.

Required runtime packaged components:

| Component | Official site or repository | Purpose in image | Current version target | Dependency relationships |
| --- | --- | --- | --- | --- |
| FFmpeg compiled runtime distribution | [ffmpeg.org](https://ffmpeg.org/download.html) | Provides the compiled media-processing suite and shared libraries used by Revaer adapters. | 8.1.1 | Built from the FFmpeg source archive in a builder stage; depends on the codec, filter, subtitle, hardware, protocol, and utility libraries below; Revaer runtime depends on `ffmpeg`, `ffprobe`, and the detected `libav*` capabilities. |
| `ffmpeg` CLI | [FFmpeg tool docs](https://ffmpeg.org/ffmpeg.html) | Executes remux, stream copy, metadata rewrite, audio transcode, video transcode, filter, and verification operations. | 8.1.1 | Built from FFmpeg; depends on `libavcodec`, `libavformat`, `libavfilter`, `libswscale`, `libswresample`, and enabled external libraries; Revaer execution adapters depend on it. |
| `ffprobe` CLI | [FFprobe docs](https://ffmpeg.org/ffprobe.html) | Produces machine-readable container, stream, codec, disposition, chapter, metadata, and sidecar-adjacent inspection input. | 8.1.1 | Built from FFmpeg; depends on `libavformat`, `libavcodec`, and `libavutil`; Revaer inspection adapters depend on it. |
| `ffplay` CLI | [FFplay docs](https://ffmpeg.org/ffplay.html) | Optional playback-smoke probe for verification profiles that enable it. | 8.1.1 | Built from FFmpeg; depends on SDL2 when enabled; verification depends on it only when a playback probe policy is selected. |
| `libavutil` | [FFmpeg libraries docs](https://ffmpeg.org/documentation.html) | Shared FFmpeg utility layer for media metadata, rational values, hashes, frames, and option handling. | 60.26.101 | FFmpeg tools and all other `libav*` libraries depend on it. |
| `libavcodec` | [FFmpeg codec docs](https://ffmpeg.org/ffmpeg-codecs.html) | Native codec implementation and wrapper layer for external codec libraries. | 62.28.101 | Depends on enabled external codec libraries such as libx264, x265, libaom, dav1d, libopus, libvpx, and libass-related subtitle decoders; `ffmpeg` and `ffprobe` depend on it. |
| `libavformat` | [FFmpeg formats docs](https://ffmpeg.org/ffmpeg-formats.html) | Native demuxer and muxer layer for containers, chapters, attachments, streams, and protocol IO. | 62.12.101 | Depends on compression/protocol libraries where enabled; `ffmpeg`, `ffprobe`, remux planning, and verification depend on it. |
| `libavfilter` | [FFmpeg filters docs](https://ffmpeg.org/ffmpeg-filters.html) | Filtergraph support for scale, crop, deinterlace, tone map, subtitle render, loudness, quality analysis, and validation filters. | 11.14.101 | Depends on filter libraries such as libass, zimg, libplacebo, libvmaf, rubberband, soxr, frei0r, LADSPA, and font libraries; `ffmpeg` filter execution depends on it. |
| `libavdevice` | [FFmpeg device docs](https://ffmpeg.org/ffmpeg-devices.html) | Device abstraction for optional local capture/playback probe support. | 62.3.101 | Depends on platform device libraries when enabled; Revaer does not use it for source discovery. |
| `libswscale` | [FFmpeg scaler docs](https://ffmpeg.org/libswscale.html) | Pixel-format conversion and software scaling fallback. | 9.5.101 | FFmpeg video filters and transcode operations depend on it; zimg/libplacebo may replace it for configured high-quality paths. |
| `libswresample` | [FFmpeg resampler docs](https://ffmpeg.org/libswresample.html) | Audio sample-format, channel-layout, and resampling fallback. | 6.3.101 | FFmpeg audio transcode and downmix operations depend on it; soxr may replace it for configured high-quality resampling. |
| MKVToolNix CLI (`mkvmerge`, `mkvinfo`, `mkvextract`, `mkvpropedit`) | [mkvtoolnix.download](https://mkvtoolnix.download/) | Lossless Matroska inspect, mux, extract, track-order, chapter, attachment, language, disposition, and metadata operations. | 98.0 | Complements FFmpeg for MKV-specific container-only work; Revaer planner may prefer it for low-cost Matroska edits. |
| GPAC / `MP4Box` | [gpac.io](https://gpac.io/) | ISO BMFF/MP4 inspect, packaging, faststart/interleaving, track import/export, subtitle packaging, DASH/HLS packaging support. | 26.02.0 | Complements FFmpeg for MP4-specific container-only work; Revaer planner may prefer it for low-cost MP4 edits and validation. |
| Bento4 CLI (`mp4dump`, `mp4info`, `mp4edit`, `mp4extract`, `mp4fragment`, `mp4compact`) | [bento4.com](https://www.bento4.com/downloads/) | Secondary MP4 atom inspection, structural validation, extraction, and packaging diagnostics. | 1.6.0-641 | Complements GPAC and FFmpeg for verification and MP4 diagnostics; Revaer verification may use it for strict MP4 profiles. |
| MediaInfo CLI / libmediainfo | [mediaarea.net](https://mediaarea.net/en/MediaInfo) | Independent media metadata, technical stream, and tag inspection for cross-checking `ffprobe`. | 26.05 | Depends on MediaArea libraries such as ZenLib; Revaer verification may use it as an independent inspector. |
| CCExtractor | [ccextractor.org](https://ccextractor.org/) | Existing closed-caption and subtitle extraction from media files where FFmpeg support is incomplete. | 0.96.6 | Complements FFmpeg subtitle extraction; Revaer may use it only for existing embedded captions, never for acquisition or OCR. |
| libaom | [AOMedia libaom](https://aomedia.googlesource.com/aom) | AV1 encode/decode support through FFmpeg `libaom-av1`. | 3.14.1 | FFmpeg `libavcodec` depends on it for `--enable-libaom`; libavif/libheif may also use it. |
| SVT-AV1 | [AOMediaCodec/SVT-AV1](https://gitlab.com/AOMediaCodec/SVT-AV1) | High-performance AV1 encoding through FFmpeg `libsvtav1`. | 4.1.0 | FFmpeg `libavcodec` depends on it for `--enable-libsvtav1`; libheif may use it for AVIF encoding plugins. |
| dav1d | [VideoLAN dav1d](https://code.videolan.org/videolan/dav1d) | Fast AV1 decoding through FFmpeg `libdav1d`. | 1.5.3 | FFmpeg `libavcodec` depends on it for `--enable-libdav1d`; libavif/libheif may also use it. |
| rav1e | [xiph/rav1e](https://github.com/xiph/rav1e) | Alternative AV1 encoder for quality/comparison profiles. | 0.8.1 | FFmpeg `libavcodec` depends on it for `--enable-librav1e`. |
| libvpx | [webmproject/libvpx](https://chromium.googlesource.com/webm/libvpx) | VP8/VP9 encode/decode support. | 1.16.0 | FFmpeg `libavcodec` depends on it for `--enable-libvpx`; WebM compatibility targets depend on it. |
| x264 | [VideoLAN x264](https://www.videolan.org/developers/x264.html) | High-quality H.264/AVC encoding. | Rolling Git master; pin exact commit in image inventory | FFmpeg `libavcodec` depends on it for `--enable-libx264`; enables GPL FFmpeg build mode. |
| x265 | [x265 documentation](https://x265.readthedocs.io/en/master/releasenotes.html) | HEVC/H.265 encoding with HDR-related options. | 4.2 | FFmpeg `libavcodec` depends on it for `--enable-libx265`; enables GPL FFmpeg build mode. |
| OpenH264 | [cisco/openh264](https://github.com/cisco/openh264) | Open-source H.264 codec path for constrained baseline and Cisco binary-license workflows. | 2.6.0 | FFmpeg `libavcodec` depends on it for `--enable-libopenh264`; does not replace x264 for high-quality H.264 encoding. |
| Xvid | [xvid.com](https://www.xvid.com/) | MPEG-4 ASP encode/decode compatibility for older libraries. | 1.3.7 | FFmpeg `libavcodec` depends on it for `--enable-libxvid`; legacy compatibility targets may depend on it. |
| libtheora | [theora.org](https://www.theora.org/) | Theora video encoding for Ogg/Theora compatibility. | 1.2.0 | FFmpeg `libavcodec` depends on it for `--enable-libtheora`; depends on libogg. |
| OpenJPEG | [openjpeg.org](https://www.openjpeg.org/) | JPEG 2000 encode/decode support. | 2.5.4 | FFmpeg `libavcodec` depends on it for `--enable-libopenjpeg`; image and archival profiles may depend on it. |
| libjxl | [libjxl/libjxl](https://github.com/libjxl/libjxl) | JPEG XL image encode/decode support for attachments, covers, and still-image media paths. | 0.11.2 | FFmpeg `libavcodec` depends on it for `--enable-libjxl`; image metadata tooling may also use it. |
| libwebp | [webmproject/libwebp](https://chromium.googlesource.com/webm/libwebp) | WebP image encode/decode support for artwork, thumbnails, and image streams. | 1.6.0 | FFmpeg `libavcodec` depends on it for `--enable-libwebp`; libheif/libavif-adjacent image workflows may use it. |
| libheif | [strukturag/libheif](https://github.com/strukturag/libheif) | HEIF/HEIC/AVIF image container support for covers, attachments, and image streams. | 1.21.2 | Depends on codec plugins such as libaom, dav1d, SVT-AV1, x265, and libde265 depending on build; complements FFmpeg, MediaInfo, and image validation tooling. |
| libavif | [AOMediaCodec/libavif](https://github.com/AOMediaCodec/libavif) | AVIF encode/decode tooling and validation. | 1.4.1 | Depends on libaom/dav1d/SVT-AV1/rav1e depending on build; complements FFmpeg for AVIF verification. |
| LAME / libmp3lame | [lame.sourceforge.io](https://lame.sourceforge.io/) | MP3 audio encoding for compatibility targets that require MP3. | 3.100 | FFmpeg `libavcodec` depends on it for `--enable-libmp3lame`. |
| libopus | [Xiph Opus](https://opus-codec.org/) | Opus audio encode/decode support. | 1.5.2 | FFmpeg `libavcodec` depends on it for `--enable-libopus`; WebM/Ogg compatibility targets depend on it. |
| libvorbis | [Xiph Vorbis](https://xiph.org/vorbis/) | Vorbis audio encode/decode support. | 1.3.7 | FFmpeg `libavcodec` depends on it for `--enable-libvorbis`; depends on libogg. |
| FLAC | [Xiph FLAC](https://xiph.org/flac/) | Lossless FLAC encode/decode support and `flac` CLI validation. | 1.5.0 | FFmpeg has native FLAC support, but libFLAC and CLI provide independent validation and compatibility. |
| Speex / SpeexDSP | [speex.org](https://www.speex.org/) | Legacy speech codec and speech DSP support. | 1.2.1 | FFmpeg `libavcodec` depends on it for `--enable-libspeex`; speech/audiobook profiles may use SpeexDSP functions indirectly. |
| TwoLAME | [twolame.org](https://www.twolame.org/) | MP2 audio encoding for legacy compatibility. | 0.4.0 | FFmpeg `libavcodec` depends on it for `--enable-libtwolame`. |
| libsoxr | [SoX Resampler](https://sourceforge.net/projects/soxr/) | High-quality audio resampling. | 0.1.3 | FFmpeg depends on it for `--enable-libsoxr`; can replace libswresample in high-quality audio paths. |
| Rubber Band Library | [breakfastquay.com/rubberband](https://breakfastquay.com/rubberband/) | Audio time-stretching and pitch-shift support for repair/normalization workflows. | 4.0.0 | FFmpeg `libavfilter` depends on it for `--enable-librubberband`. |
| libsndfile | [libsndfile.github.io](https://libsndfile.github.io/libsndfile/) | Independent audio file read/write support for validation and auxiliary tooling. | 1.2.2 | Depends on codec libraries such as FLAC/Ogg/Vorbis where enabled; complements FFmpeg for audio diagnostics. |
| Chromaprint | [acoustid.org/chromaprint](https://acoustid.org/chromaprint) | Audio fingerprinting for duplicate detection and media identity. | 1.6.0 | FFmpeg depends on it for `--enable-chromaprint`; Revaer fingerprinting may depend on it. |
| libass | [libass/libass](https://github.com/libass/libass) | ASS/SSA subtitle rendering and validation without OCR. | 0.17.4 | FFmpeg `libavfilter` depends on it for `ass`/`subtitles`; depends on FreeType, HarfBuzz, FriBidi, and fontconfig. |
| FreeType | [freetype.org](https://freetype.org/) | Font rasterization for subtitle rendering. | 2.14.3 | Required by libass and FFmpeg drawtext/subtitle filters. |
| HarfBuzz | [harfbuzz.github.io](https://harfbuzz.github.io/) | Text shaping for subtitles and complex scripts. | 14.2.0 | Required by libass; depends on FreeType in common builds. |
| FriBidi | [fribidi.org](https://fribidi.org/) | Bidirectional text support for subtitle rendering. | 1.0.16 | Required by libass for RTL subtitle text. |
| fontconfig | [freedesktop fontconfig](https://www.freedesktop.org/wiki/Software/fontconfig/) | Font discovery and matching for subtitle rendering. | 2.18.0 | Required by libass/FFmpeg subtitle rendering paths; depends on packaged fonts. |
| packaged open fonts | [Noto fonts](https://notofonts.github.io/) | Reliable subtitle glyph coverage across common languages. | Versioned per installed font package; image inventory must pin exact Noto package versions | Used by fontconfig/libass; image inventory must list installed font packages. |
| libzvbi | [ZVBI](https://zapping.sourceforge.net/ZVBI/) | DVB teletext and teletext subtitle decoding. | 0.2.44 | FFmpeg `libavcodec` depends on it for `--enable-libzvbi`; subtitle extraction depends on it for teletext sources. |
| libaribb24 | [libaribb24](https://github.com/nkoriyama/aribb24) | ARIB STD-B24 caption decoding. | 1.0.4 | FFmpeg subtitle decoders depend on it for `--enable-libaribb24`; Japanese broadcast subtitle handling depends on it. |
| libaribcaption | [libaribcaption](https://github.com/xqq/libaribcaption) | ARIB caption decoding/rendering alternative. | 1.1.1 | FFmpeg subtitle decoders depend on it for `--enable-libaribcaption`; complements libaribb24. |
| zimg | [sekrit-twc/zimg](https://github.com/sekrit-twc/zimg) | High-quality resize, colorspace conversion, dithering, and bit-depth conversion through FFmpeg `zscale`. | 3.0.6 | FFmpeg `libavfilter` depends on it for `--enable-libzimg`; HDR/SDR conversion depends on it. |
| libplacebo | [libplacebo.org](https://libplacebo.org/) | GPU-capable color management, scaling, tone mapping, debanding, dithering, and shader filters. | 7.360.1 | FFmpeg `libavfilter` depends on it for `--enable-libplacebo`; depends on Vulkan/libdrm stack where enabled. |
| libvmaf | [Netflix/vmaf](https://github.com/Netflix/vmaf) | Objective quality measurement for verification and regression analysis. | 3.1.0 | FFmpeg `libavfilter` depends on it for `--enable-libvmaf`; strict verification may depend on it. |
| vid.stab | [georgmartius/vid.stab](https://github.com/georgmartius/vid.stab) | Optional stabilization filter support for repair profiles. | 1.1.1 | FFmpeg `libavfilter` depends on it for `--enable-libvidstab`. |
| frei0r plugins | [frei0r.dyne.org](https://frei0r.dyne.org/) | Open video filter plugin collection. | 2.3.3 | FFmpeg `libavfilter` depends on it for `--enable-frei0r`; only allowlisted filters may be exposed to profiles. |
| LADSPA plugins | [ladspa.org](https://www.ladspa.org/) | Open audio filter plugin interface and plugin set. | 1.17 API plus packaged plugin set | FFmpeg `libavfilter` depends on it for `--enable-ladspa`; only allowlisted plugins may be exposed to profiles. |
| FFmpeg bitstream filters | [FFmpeg bitstream-filter docs](https://ffmpeg.org/ffmpeg-bitstream-filters.html) | Lossless stream-level fixes, metadata edits, Annex B/MP4 conversions, and codec-specific stream transforms. | Bundled in FFmpeg 8.1.1 | Built into FFmpeg; planner depends on detected `ffmpeg -bsfs` output. |
| FFmpeg NVENC/NVDEC runtime support built with ffnvcodec | [FFmpeg/nv-codec-headers](https://github.com/FFmpeg/nv-codec-headers) | Compiled NVENC/NVDEC support in FFmpeg for compatible host NVIDIA drivers. | ffnvcodec n13.0.19.0 at build time | FFmpeg depends on ffnvcodec headers only in the builder stage; runtime must not retain the headers and still depends on host NVIDIA driver availability. Enable only when the selected FFmpeg release can build this support without `--enable-nonfree`; otherwise report NVENC/NVDEC as absent capability. |
| oneVPL | [intel/libvpl](https://github.com/intel/libvpl) | Intel Quick Sync / oneVPL hardware encode/decode dispatch. | 2.16.0 | FFmpeg depends on it for QSV/oneVPL support; runtime depends on host Intel GPU/media driver availability. |
| Intel media driver | [intel/media-driver](https://github.com/intel/media-driver) | VAAPI/QSV media driver for Intel GPUs. | 26.1.5 | Used by FFmpeg hardware acceleration through VAAPI/QSV; depends on host device mounts. |
| libva / VAAPI runtime | [intel/libva](https://github.com/intel/libva) | Vendor-neutral Linux video acceleration API. | 2.23.0 | FFmpeg VAAPI filters/encoders depend on it; GPU profiles depend on host device access. |
| Vulkan loader | [Khronos Vulkan-Loader](https://github.com/KhronosGroup/Vulkan-Loader) | Runtime loader for Vulkan-backed filters/codecs and libplacebo. | 1.4.350.0 | libplacebo and FFmpeg Vulkan filters depend on it; GPU profiles depend on host Vulkan driver availability. Vulkan headers are builder-stage-only. |
| libdrm and Mesa VA/Vulkan userspace | [libdrm](https://gitlab.freedesktop.org/mesa/drm), [Mesa3D](https://www.mesa3d.org/) | Open userspace graphics/media stack for VAAPI/Vulkan on supported GPUs. | libdrm 2.4.133; Mesa 26.1.1 | VAAPI/Vulkan/libplacebo paths depend on it; runtime depends on host devices. |
| libbluray | [videolan/libbluray](https://code.videolan.org/videolan/libbluray) | Blu-ray playlist, chapter, and subtitle container handling for files already present in a library. | 1.3.4 | FFmpeg `libavformat` may depend on it for `--enable-libbluray`; Revaer must not perform disc acquisition. |
| libdvdread/libdvdnav | [libdvdread](https://code.videolan.org/videolan/libdvdread), [libdvdnav](https://code.videolan.org/videolan/libdvdnav) | DVD structure navigation for files already present in a library. | libdvdread 7.0.1; libdvdnav 7.0.0 | FFmpeg may use these through distro builds; Revaer must not perform disc acquisition or ripping workflows. |
| libsrt | [Haivision/srt](https://github.com/Haivision/srt) | Secure Reliable Transport protocol support for media IO and future stream validation. | 1.5.4 | FFmpeg `libavformat` depends on it for `--enable-libsrt`; local-file plans should not depend on it. |
| librist | [rist.tech](https://code.videolan.org/rist/librist) | RIST protocol support for media IO and future stream validation. | 0.2.11 | FFmpeg `libavformat` depends on it for `--enable-librist`; local-file plans should not depend on it. |
| libssh | [libssh.org](https://www.libssh.org/) | SFTP/SCP protocol support for configured remote roots if later enabled. | 0.12.0 | FFmpeg protocol layer depends on it for `--enable-libssh`; disabled unless remote roots are explicitly supported. |
| GnuTLS TLS library | [GnuTLS](https://www.gnutls.org/) | TLS protocol support for FFmpeg network protocols and release fetch verification helpers. | 3.8.13 | FFmpeg protocol layer depends on it; image build tooling may also depend on it. The default full GPL media image must not link FFmpeg against OpenSSL unless a later legal ADR proves the exact build is redistributable. |
| compression libraries (`zlib`, `bzip2`, `xz/lzma`, `snappy`) | [zlib](https://zlib.net/), [xz](https://tukaani.org/xz/), [bzip2](https://sourceware.org/bzip2/), [Snappy](https://github.com/google/snappy) | Container compression, subtitles, archives, and codec/container metadata support. | zlib 1.3.2; bzip2 1.0.8; xz 5.8.3; Snappy 1.2.2 | FFmpeg, MediaInfo, MKVToolNix, GPAC, and Bento4 may depend on these libraries. |

Builder-stage-only components:

| Component | Official site or repository | Purpose in build | Current version target | Dependency relationships |
| --- | --- | --- | --- | --- |
| FFmpeg source archive | [ffmpeg.org](https://ffmpeg.org/download.html) | Builder-stage input used to produce the compiled FFmpeg runtime distribution. | 8.1.1 | Depends on builder-stage source archives, headers, and libraries for enabled external components; must not be copied into the final runtime image. |
| ffnvcodec headers | [FFmpeg/nv-codec-headers](https://github.com/FFmpeg/nv-codec-headers) | Build-time headers for NVENC/NVDEC support. | n13.0.19.0 | Required only while compiling FFmpeg; must not be copied into the final runtime image. |
| source archives/checkouts for runtime libraries | Official sites and repositories listed in the runtime table | Builder-stage inputs for components unavailable or too stale in base package sources. | Match the runtime table version targets | Required only to build compiled runtime artifacts; source trees, archives, and VCS metadata must not be copied into the final runtime image. |
| build and assembly helpers (`nasm`, `yasm`, `pkg-config`, C/C++ toolchain, Meson, CMake, Ninja) | [NASM](https://www.nasm.us/), [Yasm](https://yasm.tortall.net/), [pkgconf](https://github.com/pkgconf/pkgconf), [Meson](https://mesonbuild.com/), [CMake](https://cmake.org/), [Ninja](https://ninja-build.org/) | Build-from-source support for codec libraries and optimized assembly paths. | NASM 3.01; Yasm 1.3.0; pkgconf 2.5.1; Meson 1.11.1; CMake 4.3.3; Ninja 1.13.2; C/C++ toolchain pinned by builder base image | Build stage depends on them; final runtime image must not retain them unless a specific runtime executable is intentionally promoted through an ADR. |

Default FFmpeg build posture:

- Build or package only redistributable FFmpeg binaries, never an
  `--enable-nonfree` FFmpeg.
- The default full-capability image includes GPL-triggering components such as
  x264, x265, Xvid, frei0r, vid.stab, and Rubber Band. It also includes
  version-3-triggering components such as VMAF and libaribb24. Treat the
  default FFmpeg binary as GPLv3-or-later unless the generated license evidence
  proves a narrower obligation.
- Enable `--enable-gpl` when required for x264, x265, Xvid, frei0r, vid.stab,
  Rubber Band, or other GPL components.
- Enable `--enable-version3` when required by selected redistributable
  libraries.
- Enable all required libraries above when legally compatible and available for
  the target architecture.
- Expose the exact configure flags in the generated inventory.
- The capability API must report absent components separately from unsupported
  profile requirements so operators can distinguish image gaps from profile
  mistakes.
- Provide an optional future LGPL-only media image only through a separate ADR.
  That image would intentionally exclude GPL-triggering components and is not
  the first-release default.

Excluded from the default image:

| Component | Official site or repository | Reason excluded |
| --- | --- | --- |
| `--enable-nonfree` FFmpeg builds | [FFmpeg legal](https://ffmpeg.org/legal.html) | The default image must be redistributable. FFmpeg documents `--enable-nonfree` as producing a nonfree/nonredistributable build for incompatible combinations. |
| Fraunhofer FDK-AAC / libfdk-aac | [mstorsjo/fdk-aac](https://github.com/mstorsjo/fdk-aac) | FFmpeg marks this path as requiring `--enable-nonfree` in incompatible builds. Revaer must not ship a nonfree/non-redistributable FFmpeg image by default. |
| OpenSSL-linked GPL FFmpeg builds | [FFmpeg license notes](https://www.ffmpeg.org/doxygen/7.0/md_LICENSE.html) | FFmpeg's license notes call out OpenSSL incompatibility with GPLv2/GPLv3. The default full media image uses GnuTLS instead. |
| ExifTool | [exiftool.org](https://exiftool.org/) | ExifTool is an interpreted Perl utility. Under the binary-only runtime policy, it is excluded unless a later ADR explicitly allows source-form utility distribution and records license-notice handling. |
| Tesseract or other OCR engines | [tesseract-ocr](https://github.com/tesseract-ocr/tesseract) | First release explicitly excludes OCR and subtitle generation. |
| Subtitle acquisition tools or online subtitle clients | n/a | First release manages only existing embedded and sidecar subtitles. |
| VapourSynth and AviSynth script execution | [VapourSynth](https://www.vapoursynth.com/), [AviSynth+](https://github.com/AviSynth/AviSynthPlus) | Script execution is unnecessary for the first release and creates a broad code-execution surface. Add only through a later threat model and ADR. |
| Proprietary GPU drivers or proprietary codec SDK binaries | Vendor-specific | Builder stages may include redistributable open headers when legally compatible, and the runtime image may include redistributable userspace dispatch libraries. Host drivers remain operator-provided and must be detected at runtime. |

Media runtime license-compliance artifacts:

Every release that publishes a media runtime image must produce these artifacts
from the same immutable build manifest and attach them wherever the image is
published:

- `media-runtime-inventory.json`: component name, canonical project URL,
  upstream source URL, source tag or commit, source archive checksum, shipped
  binary paths, shipped shared-library paths, runtime data paths, version,
  detected license expression, declared license expression, concluded license
  expression, copyright notice path, attribution text, dependency edges,
  FFmpeg configure flags, patch list, modification status, patent-review status,
  and source-compliance bundle path.
- `THIRD_PARTY_NOTICES.md` and `THIRD_PARTY_NOTICES.html`: human-readable
  attribution for every shipped media component, including project name,
  official link, copyright notices, license name, license text or bundled
  license path, source-offer link, modification summary, and trademark notices.
- `source-compliance.tar.zst`: exact corresponding source for every GPL/LGPL
  and source-availability component, including original source archives or
  checkouts, applied patches, `changes.diff` where applicable, build scripts,
  Dockerfiles, lockfiles, configure command lines, build logs, generated
  inventories, and checksum manifests.
- `media-runtime.spdx.json`: SPDX SBOM with package checksums, package
  verification codes where available, declared licenses, concluded licenses,
  license texts or references, copyright text, external references, and
  relationship edges.
- `SOURCE_OFFER.md`: stable source-offer text for download pages, image labels,
  release notes, and the in-app About/API surfaces. It must point to the exact
  source-compliance bundle for the image digest, not only to upstream project
  home pages.
- `ffmpeg-buildconf.txt`, `ffmpeg-codecs.txt`, `ffmpeg-encoders.txt`,
  `ffmpeg-decoders.txt`, `ffmpeg-formats.txt`, `ffmpeg-filters.txt`,
  `ffmpeg-bsfs.txt`, and `ffprobe-version.txt`: command output captured from
  the final runtime image.

Required runtime notice locations:

- `/usr/share/revaer/media-runtime-inventory.json`
- `/usr/share/doc/revaer-media/THIRD_PARTY_NOTICES.md`
- `/usr/share/doc/revaer-media/SOURCE_OFFER.md`
- `/usr/share/licenses/revaer-media/<component>/LICENSE`
- `/usr/share/licenses/revaer-media/<component>/NOTICE` when upstream provides
  one

The image must also expose OCI annotations for source URL, revision, license
expression, documentation URL, SBOM digest, source-compliance bundle digest, and
third-party notice digest.

License obligation matrix:

| Obligation | Applies to | Required behavior | Release evidence |
| --- | --- | --- | --- |
| Corresponding source availability | GPL/LGPL components, including the default GPL FFmpeg build and GPL-linked external media libraries | Provide exact corresponding source, patches, build scripts, configure flags, and checksums for the distributed binaries. Do not satisfy this by placing source in the runtime image. | `source-compliance.tar.zst`, source-offer URL, checksum manifest, build logs |
| GPL build-mode disclosure | FFmpeg built with GPL components such as x264, x265, Xvid, frei0r, vid.stab, or Rubber Band | Treat the FFmpeg binary as GPL. When version-3 components are also present, treat the full FFmpeg binary as GPLv3-or-later. | `ffmpeg-buildconf.txt`, `media-runtime-inventory.json`, `THIRD_PARTY_NOTICES.md` |
| LGPL linking obligations | LGPL media libraries and any direct Revaer linkage to LGPL libraries | Prefer separate CLI execution or dynamic linking. If Revaer ever directly links LGPL libraries, provide required notices, license text, source, and a relinking/debugging path for modified LGPL libraries. | Linkage scan, ldd/readelf output, source bundle, notice files |
| Revaer/GPL separation | GPL media tools and the Revaer application | Revaer must invoke GPL media tools through process boundaries and files, not link GPL media libraries into Revaer crates. Any direct link to GPL media libraries requires an ADR and legal review before implementation. | Revaer binary linkage scan, dependency inventory, ADR if direct linking is introduced |
| Permissive-license notices | MIT, BSD, ISC, zlib, and similar components | Preserve copyright notices, license text, warranty disclaimers, and required attribution in the notices bundle and image license directory. | Component license files, `THIRD_PARTY_NOTICES.md`, SBOM license fields |
| Apache-2.0 notices | Apache-2.0 components such as VMAF and any other Apache-licensed runtime dependency | Preserve license text and NOTICE content, record modifications, and ensure FFmpeg license mode is compatible by enabling version-3 where required. | NOTICE files, patch list, `ffmpeg-buildconf.txt`, SBOM license fields |
| Font license notices | Packaged fonts, especially Noto/OFL-family fonts | Ship original font license text and copyright notices. Do not rename or modify fonts unless the font license permits the exact change. | Font package inventory, font license files, notices bundle |
| Trademark attribution | FFmpeg and other projects with trademark guidance | Spell project names correctly, do not imply endorsement, and include official project links in notices and download surfaces. | Notices bundle, release page text, UI/API About text |
| Patent-risk review | Patent-encumbered codec areas such as H.264, H.265/HEVC, AAC, MPEG-4, and OpenH264 | Record that copyright/license compliance does not grant patent rights. If using Cisco-provided OpenH264 binaries, include Cisco binary-license notices and satisfy its distribution terms; if source-building OpenH264, record separate patent-risk review. | Patent-review field in inventory, OpenH264 license notice when applicable, ADR for chosen OpenH264 distribution mode |
| No reverse-engineering prohibition conflict | LGPL components and any terms presented to users | Release terms, EULA text, UI text, and documentation must not prohibit reverse engineering where LGPL debugging/modification rights require it. | Release checklist, docs review, notice bundle |
| Nonfree exclusion | Components requiring `--enable-nonfree`, proprietary codec SDKs, proprietary drivers, incompatible OpenSSL/GPL FFmpeg combinations | Exclude from default image and expose absent capability instead of silently shipping a nonredistributable binary. | Build gate output, excluded-component report, capability API result |

Compliance gates:

- Fail the build if `ffmpeg -buildconf` contains `--enable-nonfree`.
- Fail the default full media image if `ffmpeg -buildconf` contains
  `--enable-openssl`.
- Fail the build if GPL-triggering components are present without
  `--enable-gpl`.
- Fail the build if version-3-triggering components are present without
  `--enable-version3`.
- Fail the build if the final image contains source archives, source checkouts,
  `.git` directories, build directories, object files, static archives, headers,
  compilers, assemblers, build systems, package-manager caches, or
  builder-only tools.
- Fail the build if any shipped media component lacks a detected license,
  official project link, source URL, source checksum, copyright notice, license
  text, and source-compliance artifact entry.
- Fail the build if Revaer's own binaries link directly to GPL media libraries.
- Fail the build if the final image lacks runtime license and notice files at
  the required locations.
- Fail publication if the release page, image annotations, API About endpoint,
  or UI About surface lacks the source-offer and third-party-notice links.
- Fail publication if source-compliance bundle checksums do not match the image
  digest and manifest that produced the binaries.

Compliance API and UX requirements:

- Capability API responses must include image license mode, FFmpeg license mode,
  `--enable-gpl`, `--enable-version3`, `--enable-nonfree` status, source-offer
  URL, third-party-notice URL, SBOM URL, source-compliance bundle digest, and
  a list of absent capabilities caused by license exclusions.
- The UI capability/settings surface must show license mode, source-offer link,
  third-party-notice link, SBOM link, and warnings for patent-sensitive codecs.
- YAML export must include profile behavior only. It must not export bundled
  third-party license texts, source bundles, or notices; those belong to the
  image release artifacts.

Compliance reference links:

- [FFmpeg License and Legal Considerations](https://ffmpeg.org/legal.html)
- [FFmpeg license notes](https://www.ffmpeg.org/doxygen/7.0/md_LICENSE.html)
- [GNU GPL FAQ](https://www.gnu.org/licenses/gpl-faq.en.html)
- [GNU GPLv2](https://www.gnu.org/licenses/old-licenses/gpl-2.0.en.html)
- [GNU GPLv3](https://www.gnu.org/licenses/gpl-3.0.en.html)
- [GNU LGPLv2.1](https://www.gnu.org/licenses/old-licenses/lgpl-2.1.en.html)
- [GNU LGPLv3](https://www.gnu.org/licenses/lgpl-3.0.en.html)
- [SPDX License List](https://spdx.org/licenses/)
- [SPDX specification](https://spdx.github.io/spdx-spec/)
- [OpenChain ISO/IEC 5230 license-compliance program](https://openchainproject.org/license-compliance)
- [OpenH264 project](https://www.openh264.org/)
- [OpenH264 binary license](http://www.openh264.org/BINARY_LICENSE.txt)

This runtime-image requirement does not weaken Rust dependency policy. Rust
crates remain minimal; image packages are broad because transcoding capability is
product behavior.

---

## Planner

Planner responsibilities:

- Resolve desired streams.
- Determine source reuse.
- Support source-to-output fanout.
- Compute diffs.
- Generate candidate operations.
- Expand valid plans.
- Prune dominated plans.
- Rank by least expensive safe cost.
- Include disk amplification in cost.
- Handle compatibility policy.
- Generate execution DAGs.
- Generate verification expectations.
- Explain selected and rejected plans.

Planning pipeline:

```text
actual media graph
-> desired media graph
-> diff
-> stream resolution
-> operation generation
-> plan expansion
-> cost and risk optimization
-> execution DAG
-> verification expectations
```

Resolution outcomes:

```rust
enum StreamResolution {
    ExistingCopy,
    ExistingRemux,
    Transform,
    Derive,
    Preserve,
    Remove,
    MissingOptional,
    MissingRequired,
    Unsupported,
}
```

Fanout example:

```text
source: English DTS 5.1
outputs:
  - English AAC stereo
  - English EAC3 5.1
```

Operation taxonomy:

- NoOp.
- RewriteMetadata.
- RewriteDisposition.
- RewriteLabels.
- ReorderStreams.
- CopyStream.
- ExtractSubtitle.
- CopySidecarSubtitle.
- EmbedSubtitle.
- RemuxContainer.
- TranscodeAudio.
- TranscodeVideo.
- BackupOriginal.
- AtomicReplace.
- VerifyOutput.
- QuarantineOutput.

Weighted cost defaults:

```text
no-op: 0
rewrite_metadata: 1
rewrite_disposition: 1
rewrite_labels: 1
stream_reorder: 2
copy_sidecar_subtitle: 2
extract_subtitle: 3
embed_subtitle: 4
remux: 5
audio_transcode: 20
subtitle_conversion_without_ocr: 80
video_transcode: 1000
full_rewrite: 1200
```

Compute-minimal planning order:

1. No-op.
2. Metadata/disposition/label rewrite.
3. Remux only.
4. Stream copy and reorder.
5. Subtitle extraction or copy.
6. Audio transcode only.
7. Video transcode.
8. Full rewrite.

Subtitle operation order:

```text
no-op: 0
label/disposition update: 1
copy existing sidecar: 2
extract existing embedded subtitle: 3
embed existing sidecar subtitle: 4
remux for subtitle changes: 5
unsupported conversion: fail or quarantine
```

Candidate pruning:

- Eliminate dominated plans.
- Stop exploring plans exceeding known best cost.
- Eliminate equivalent output plans.
- Prefer stream reuse before transcode generation.
- Discard a plan if another plan costs less and produces identical output.

Plan explanation shape:

```yaml
plan:
    selected: true
    estimated_cost: 25
    risk: low
    reasons:
        - video already compliant, stream copy selected
        - audio codec mismatch requires audio transcode
        - subtitle placement mismatch requires extraction
        - remux required to update stream ordering and dispositions
    rejected_plans:
        - id: full-transcode
          estimated_cost: 1020
          reason: higher cost with no compliance benefit
```

---

## Compliance

Compliance is diff-based, not pass/fail only.

Violation shape:

```yaml
violations:
    - path: audio[0].codec
      severity: high
      issue: codec_mismatch
      expected: aac
      actual: dts
```

Compliance result:

```yaml
compliance:
    status: non_compliant
    score: 0.87
    violations:
        - path: video[0].codec
          severity: high
          issue: codec_mismatch
          expected: hevc
          actual: h264
        - path: audio[0].channels
          severity: medium
          issue: channel_layout_mismatch
          expected: 2
          actual: 6
        - path: subtitles[0].placement
          severity: low
          issue: placement_mismatch
          expected: sidecar
          actual: embedded
```

Compliance scoring must support:

- Compliant.
- Non-compliant.
- Unsupported.
- Dry-run planned.
- Failed validation.
- Failed execution.
- Failed verification.

---

## Execution

Execution is DAG-based, not a sequential shell script.

Benefits:

- Deterministic ordering.
- Retry support.
- Resumability.
- Intermediate reuse.
- Failure isolation.
- Future distributed execution.

Example DAG:

```text
extract subtitle
-> transcode audio
-> copy video
-> final mux
-> verify output
-> backup if configured
-> atomic replace
-> final verification
-> cleanup
```

External tools:

- Use injected adapters.
- Build argument vectors only.
- Never build shell command strings.
- Profile configuration must not inject arbitrary raw tool arguments.
- Command construction must be unit tested without spawning tools.
- Execution adapters may spawn configured tool binaries only through bootstrap
  wiring.

Pseudocode:

```rust
fn process_file(candidate, profile, capabilities, adapters) -> Result<JobOutcome> {
    let actual = adapters.inspect(candidate.path)?;
    let normalized = normalize(actual)?;
    let desired = compile_desired(profile.target, profile.policy, normalized, capabilities)?;
    let diff = diff(normalized, desired)?;
    let plans = plan(normalized, desired, diff, profile.policy, capabilities)?;
    let selected = choose_least_expensive_safe(plans)?;
    let disk = estimate_disk_impact(selected, candidate.path, profile.policy)?;

    if selected.effective_dry_run {
        persist_plan_and_audit(selected, disk)?;
        return Ok(JobOutcome::DryRunPlanned);
    }

    enforce_disk_reserve(disk)?;
    let workspace = create_workspace(candidate.path, profile.policy.workspace)?;
    execute_dag(selected.graph, workspace)?;
    let output = adapters.inspect(workspace.output_path)?;
    verify(output, desired, profile.policy.verification)?;
    backup_if_configured(candidate.path, profile.policy.backup)?;
    replace_atomically(candidate.path, workspace.output_path)?;
    let final_state = adapters.inspect(candidate.path)?;
    verify(final_state, desired, profile.policy.verification)?;
    cleanup_workspace(workspace)?;
    persist_success_audit(selected, final_state)?;
    Ok(JobOutcome::Completed)
}
```

---

## Workspace, Disk, Backup, And Cleanup

Managed workspace layout:

```text
workspace/
  input/
  intermediate/
  subtitles/
  output/
  logs/
  plan/
  compliance/
  verification/
```

Required behavior:

- All transient files live under a configured managed workspace root.
- No temporary files, logs, probes, sidecars, partial outputs, or recovery files
  beside source media.
- Workspaces are deterministic, job-scoped, and safe to delete after terminal
  state.
- Successful jobs delete transient workspace contents after final verification
  and audit persistence.
- Failed jobs delete transient clutter by default.
- Failed jobs may retain bounded diagnostics when explicitly configured.
- Interrupted jobs are recoverable by job id.
- Startup or scheduled janitor removes stale workspaces not attached to active
  jobs.
- Quarantine is bounded by size and retention duration.
- Audit records persist compact facts and references, not large unbounded logs.

Disk-impact estimate:

- Scratch bytes.
- Output bytes.
- Backup bytes.
- Quarantine bytes.
- Safety reserve bytes.
- Estimated peak workspace bytes.
- Free bytes.
- Effective reserve bytes.

Disk-preserving preferences:

- No-op.
- Metadata-only rewrite.
- Stream copy.
- Reflink where safe and available.
- Hardlink where safe and available.
- Remux in a managed workspace.
- Avoid full-file duplication where verifiable.

Backup:

- Disabled by default.
- Optional configured backup root.
- Backup must happen before replacement when required.
- Backup root must satisfy path validation and free-space validation.
- Backup failure fails the job before replacement.

Atomic replacement:

- Stage replacement inside managed workspace.
- Preserve permissions when configured.
- Preserve ownership when configured.
- Atomically rename into place on the same filesystem when possible.
- Verify final source path after replacement.
- If replacement fails, leave original intact or recoverable.

---

## Verification

Successful tool exit is never sufficient.

Verification pipeline:

```text
produced output
-> reinspect media
-> normalize graph
-> verify desired state
-> compliance result
```

Structural checks:

- Stream count.
- Stream ordering.
- Codec validation.
- Format validation.
- Channel validation.
- Subtitle placement.
- Container format.
- Disposition validation.

Safety checks:

- Duration delta.
- Corruption checks.
- Mux validation.
- Decode validation when configured.
- Playback probe when configured.
- Keyframe seek validation when configured.
- Stream decode checks when configured.

Metadata checks:

- Labels.
- Stream titles.
- Language tags.
- Chapters.
- Attachments.
- Provider IDs.
- Edition tags.
- Version tags.
- Source quality tags.
- Original path/fingerprint metadata where configured.

Verification strictness profiles:

- `strict`: full reinspect, playback probe, mux validation, duration tolerance
  0.1s, stream-by-stream validation.
- `balanced`: full reinspect, mux validation, normal duration tolerance, no
  playback probe unless policy enables it.
- `fast`: metadata validation with relaxed duration tolerance and no playback
  probe.

---

## Fingerprinting And Caching

Fingerprinting supports:

- Duplicate detection.
- Planner caching.
- Resumability.
- Change detection.
- Compliance caching.
- Reconversion prevention.

Fingerprint types:

- Container structural hash.
- Stream fingerprint.
- Media graph hash.
- Container-independent identity hash.
- Perceptual hash is excluded from first release.

Planner cache key:

```text
source fingerprint
+ target profile version
+ policy profile version
+ capability fingerprint
```

Cache layers:

- Capability cache.
- Inspection cache.
- Normalization cache.
- Compliance cache.
- Planner cache.
- Verification cache.

Cache entries must be invalidated by source fingerprint, target version, policy
version, and capability fingerprint changes.

---

## API Surface

Initial endpoints:

```text
GET    /v1/media/profiles
POST   /v1/media/profiles
GET    /v1/media/profiles/{profile_public_id}
PATCH  /v1/media/profiles/{profile_public_id}
POST   /v1/media/profiles/{profile_public_id}/validate

GET    /v1/media/targets
POST   /v1/media/targets
GET    /v1/media/policies
POST   /v1/media/policies
GET    /v1/media/compatibility-targets
POST   /v1/media/compatibility-targets
POST   /v1/media/imports/validate
POST   /v1/media/imports
POST   /v1/media/exports

GET    /v1/media/capabilities
POST   /v1/media/capabilities/refresh
GET    /v1/media/job-retention
PATCH  /v1/media/job-retention

POST   /v1/media/discovery/preview
GET    /v1/media/discovery/schedules
POST   /v1/media/discovery/schedules
GET    /v1/media/discovery/watchers
POST   /v1/media/discovery/watchers
POST   /v1/media/plan
POST   /v1/media/jobs
GET    /v1/media/jobs
GET    /v1/media/jobs/{job_public_id}
POST   /v1/media/jobs/{job_public_id}/cancel
POST   /v1/media/jobs/{job_public_id}/retry
```

Response families:

- Profile summaries and details.
- Target summaries and details.
- Policy summaries and details.
- Compatibility target summaries and details.
- Validation reports with problem-detail pointers.
- Import validation reports.
- YAML export bundles.
- Capability reports.
- Job-retention reports.
- Discovery previews.
- Discovery schedule and watcher summaries.
- Compliance reports.
- Plan previews with explanations and rejected plans.
- Disk-impact reports.
- Job details.

Job creation dry-run behavior:

- Automatic watcher/scheduled jobs inherit profile dry-run setting.
- Manual jobs may override profile dry-run for that run only.
- Manual destructive override of a dry-run profile must include exact
  confirmation phrase `replace`.
- Manual destructive run for a non-dry-run profile requires no typed phrase.

Handlers stay thin and map typed errors into existing problem responses. Runtime
mutations call stored procedures through `revaer-data`.

---

## Configuration UX

Top-level UI:

- Add `crates/revaer-ui/src/features/media`.
- Add a route once route-ready.
- Tabs: Profiles, Targets, Policies, Compatibility, Capabilities, Jobs.

Primary workflows:

- Create/edit a profile by selecting one non-overlapping path, target, and
  policy.
- Import Revaer YAML, review validation, map local paths, save as dry-run.
- Export portable Revaer YAML with local paths omitted by default.
- Build ordered video, audio, and subtitle target rows.
- Configure video preserve/remove/reorder/transform behavior.
- Edit retention rules, compatibility actions, operation costs, output behavior,
  and verification strictness.
- Manage compatibility targets.
- View startup capability status and refresh on demand.
- Configure completed-job retention.
- Configure failed-terminal diagnostic retention.
- Run manual discovery.
- Enable watchers per path-to-profile association.
- Enable schedules per path-to-profile association.
- Preview discovery.
- Preview plans.
- Review disk-impact estimates.
- Keep dry-run visible in profile lists, job previews, and job details.
- Override dry-run for manual jobs only with `replace` confirmation.
- Inspect job timeline, selected plan, rejected plans, verification result, and
  final disposition.

UX constraints:

- Follow existing Yew feature-slice conventions.
- Keep transport DTOs in `models.rs` or generated API models.
- Convert transport DTOs into feature state before views.
- Keep browser globals, storage, router, and EventSource in `app/*`.
- Keep shared components free of persistence and API side effects.
- Stable selectors are required for E2E coverage.
- Destructive actions must be explicit and easy to audit.
- UI must not allow unbounded diagnostic retention.
- UI must not allow disk reserves to be disabled for destructive profiles.

---

## Observability

Events:

```text
media_profile_changed
media_capabilities_refreshed
media_capabilities_refresh_failed
media_discovery_previewed
media_job_queued
media_job_inspected
media_job_planned
media_job_execution_started
media_job_verification_failed
media_job_completed
media_job_failed
media_job_history_pruned
```

Metrics:

- Discovery candidates and exclusions.
- Inspection failures.
- Compliance status counts.
- Selected operation counts.
- Estimated and actual operation cost.
- Disk amplification.
- Transcode counts by codec and encoder family.
- Verification failures.
- Replacement failures.
- Cleanup failures.
- Refused executions due to free-space reserve protection.
- Job duration by phase.
- Capability refresh failures.
- Completed job history pruned by age/count.
- Failed terminal diagnostic history pruned by age/count.
- Runtime capability usage.
- Planner fallback frequency.

Logs:

- Include job id, profile id, root/path association id, phase, operation kind,
  and failure category.
- Log errors once at origin, then propagate as data.
- Avoid logging secrets and secret-like values.
- Redact path and metadata where existing logging policy requires it.

---

## Failure Philosophy

Fail closed.

Preferred state:

```yaml
status: unsupported
reason: image_subtitle_conversion_not_allowed
```

Never silently drop subtitles, audio, attachments, video streams, or metadata.

Failure categories:

- Inspection failure.
- Normalization failure.
- Planning failure.
- Capability failure.
- Disk-reserve failure.
- Execution failure.
- Verification failure.
- Replacement failure.
- Cleanup failure.

Each category produces explicit job state, audit facts, and events where
operator-visible.

Recovery capabilities:

- Resume DAG nodes where safe.
- Reuse verified intermediates where safe.
- Cleanup stale workspace.
- Recover interrupted replacement.
- Retry retryable operations.
- Quarantine failed output.
- Preserve compact audit facts after retention pruning.

---

## Crate And File Structure

Create:

```text
crates/revaer-media-core
  src/lib.rs
  src/model/
  src/normalize/
  src/classify/
  src/compile/
  src/diff/
  src/plan/
  src/verify/
  src/explain/

crates/revaer-media-runtime
  src/lib.rs
  src/capabilities/
  src/inspect/
  src/workspace/
  src/execute/
  src/jobs/
```

Responsibilities:

- `revaer-media-core`: pure deterministic domain logic. No filesystem, no
  environment reads, no command spawning, no database access.
- `revaer-media-runtime`: injected runtime adapters, tool execution, workspace,
  replacement, capability discovery, disk probing, job orchestration.
- `revaer-data`: migrations and stored-procedure callers for media persistence.
- `revaer-runtime`: narrow facade over media runtime persistence.
- `revaer-api`: handlers, models, OpenAPI, problem mapping, events.
- `revaer-ui`: media feature slice, services, state, logic, views.
- `revaer-events`: typed media events.
- Docker/release files: full redistributable open-source media runtime image,
  capability inventory, SBOM, third-party notices, source offer, and exact
  source-compliance bundle.

---

## Implementation Slices

Every slice adds an ADR and keeps docs in sync.

1. Domain model and normalization
   - Add `revaer-media-core`.
   - Define media graph, desired graph, policy, compliance, operation, plan,
     capability, workspace, and verification types.
   - Add deterministic codec, language, disposition, title, stream identity,
     sidecar, HDR, chapter, attachment, and metadata normalization.
   - Add pure unit tests.

2. Classification and ranking
   - Add classification rules.
   - Add semantic role inference.
   - Add deterministic stream ranking.
   - Add tests for commentary, descriptive audio, forced, SDH, signs/songs,
     karaoke, unknown, codec aliases, language aliases, and ranking ties.

3. Profile compilation and semantic validation
   - Compile match, target, policy, compatibility, retention, discovery, output,
     backup, workspace, and verification data.
   - Validate all contradictions listed in this plan.
   - Validate non-overlapping paths.
   - Validate multi-video target and retention behavior.
   - Add tests for valid and invalid configurations.

4. Diff, compliance, and explanation
   - Add graph diff.
   - Add compliance scoring.
   - Add structured violations.
   - Add selected and rejected plan explanations.
   - Add tests for video, audio, subtitle, metadata, disposition, placement, and
     retention violations.

5. Planner and cost model
   - Add stream resolution.
   - Add operation generation.
   - Add fanout.
   - Add plan expansion.
   - Add pruning.
   - Add disk amplification to cost/risk.
   - Add tests for no-op, remux, audio transcode, video transcode, subtitle
     extract/embed, unmatched streams, and unsupported streams.

6. Persistence schema and stored procedures
   - Add normalized media configuration tables.
   - Add discovery schedule and watcher tables.
   - Add compatibility target tables.
   - Add job, phase, operation, violation, explanation, verification,
     capability snapshot, backup policy, retention policy, artifact, and compact
     audit tables.
   - Add stored procedures for all runtime reads/writes.
   - Avoid JSONB application-state storage.
   - Add migration and procedure tests.

7. Revaer YAML import/export
   - Define versioned YAML schema.
   - Export portable bundles from relational configuration.
   - Import with parsing, semantic validation, conflict detection, dry-run
     coercion, and unresolved-path reporting.
   - Reject foreign formats and unsupported versions.
   - Add API, UI, and unit tests.

8. Runtime adapters and capability discovery
   - Add injected ffprobe-compatible inspection adapter.
   - Add ffmpeg-compatible command builders.
   - Add capability discovery for installed tools.
   - Add startup and on-demand refresh.
   - Persist snapshots through stored procedures.
   - Block media execution without valid capabilities.
   - Add tests for argument construction without spawning shell commands.

9. Docker image codec, toolchain, and license-compliance coverage
   - Install full redistributable open-source media-processing toolchain by
     default.
   - Build the default FFmpeg runtime as a redistributable GPL/version-3 image
     when GPL and version-3-triggering components are enabled.
   - Exclude nonfree, proprietary, source-form, or license-incompatible
     components from the default image.
   - Generate package, codec, library, utility, license, source, notice, SBOM,
     and dependency inventory artifacts.
   - Generate exact source-compliance bundle and source-offer text for the image
     digest.
   - Add CI/release gates for required tools, source-free final runtime images,
     nonfree configure flags, license detection, notices, source bundle
     completeness, Revaer-to-GPL direct linkage, and publication metadata.
   - Add API and UI exposure for license mode, source offer, third-party
     notices, SBOM, and license-excluded capabilities.
   - Update devops instructions with Docker/release compliance policy changes.

10. API and OpenAPI
    - Add routes and handlers.
    - Add DTOs and problem mapping.
    - Add OpenAPI export.
    - Add handler tests for validation, import/export, capability refresh,
      retention, preview, planning, job queueing, cancellation, retry, dry-run
      override, and permissions.

11. UI configuration surface
    - Add media feature slice.
    - Add profile, target, policy, compatibility, capability, and jobs tabs.
    - Add forms and validation displays.
    - Add import/export UX.
    - Add discovery watcher/schedule UX.
    - Add retention UX.
    - Add dry-run and `replace` override UX.
    - Add E2E coverage for core workflows.

12. Workspace, cleanup, disk reserve, and retention
    - Add managed workspace creation and teardown.
    - Add stale workspace janitor.
    - Add disk-impact estimates and reserve enforcement.
    - Add optional backup-root validation.
    - Add quarantine policy.
    - Add bounded diagnostic artifact retention.
    - Add completed-job retention cleanup.
    - Add failed-terminal diagnostic retention cleanup with 30-day default.
    - Add cancellation and failure cleanup tests.

13. Discovery execution
    - Add manual discovery.
    - Add path-to-profile associations.
    - Reject overlapping paths.
    - Add disabled-by-default scheduled scans.
    - Add disabled-by-default watchers.
    - Route all discovery through same pipeline.
    - Enqueue jobs immediately from enabled watchers and schedules.
    - Keep manual discovery over dry-run profiles plan/audit-only.
    - Add missed-event recovery tests.

14. Execution operations
    - Add remux.
    - Add metadata rewrite.
    - Add disposition rewrite.
    - Add label rewrite.
    - Add stream reorder.
    - Add existing sidecar copy/embed.
    - Add existing embedded subtitle extract.
    - Add audio transcode.
    - Add backup.
    - Add quarantine.
    - Add atomic replacement.
    - Add final verification.
    - Add failure recovery tests.

15. Video transcode
    - Add video transcode planning and execution.
    - Add quality guards.
    - Add HDR and color policy handling.
    - Add hardware/software fallback.
    - Add intent-specific safeguards.
    - Add strict regression tests.

---

## Verification Gates

For every implementation slice:

- Unit tests for pure domain logic.
- Procedure tests for persistence behavior.
- Handler tests for API behavior.
- UI tests or E2E tests for user-facing behavior.
- Policy guardrail compliance.
- `just fmt`.
- `just lint`.
- Relevant focused tests.
- Before handoff: `just ci` and `just ui-e2e`.

Additional media-specific gates:

- Tool argument construction tests.
- No shell-command string construction.
- No raw runtime SQL outside data layer.
- No JSONB application-state storage.
- No source-adjacent transient file creation.
- Disk-reserve refusal tests.
- Workspace cleanup tests for success, failure, cancellation, and startup
  janitor.
- Verification failure prevents replacement.
- Replacement failure preserves or recovers original.
- Dry-run jobs do not mutate filesystem.
- Manual discovery over dry-run profiles remains plan/audit-only.
- Destructive manual override requires exact `replace` phrase only when saved
  profile is dry-run.
- Final runtime image contains no source archives, source checkouts, VCS
  metadata, headers, build tools, static archives, object files, or package
  caches.
- FFmpeg build never contains `--enable-nonfree`.
- Default full media image does not link FFmpeg against OpenSSL.
- GPL/version-3-triggering FFmpeg components have matching configure flags and
  license evidence.
- Every shipped third-party media component has license text, attribution,
  source URL, exact source checksum, source-compliance bundle entry, and SBOM
  entry.
- Revaer binaries do not directly link GPL media libraries.
- Published image metadata, release notes, API About surface, and UI About
  surface expose source-offer, third-party-notice, and SBOM links.

---

## Open Questions

No unresolved operator decisions remain in this plan. Future implementation may
discover engineering tradeoffs, but those should be handled through ADRs rather
than by reintroducing ambiguous behavior into this document.
