# Packaged root-catalog source and evidence contract

- Status: Accepted
- Date: 2026-08-16
- Operator approval: Explicitly approved by the operator on 2026-08-16:
  "I approve the ADRs as they are now and I'm resuming your goal."

## Problem

- Accepted ADR 523 authorizes an injected, deployment-authoritative
  `RootCatalogSource` as the only component that may introduce absolute root
  paths. It does not select the packaged configuration encoding, source-loading
  lifecycle, or closed durability and sole-writer classes that make those paths
  usable.
- The accepted contract requires each slot to declare durability and
  sole-writer state, but a declaration alone is not evidence. Without an exact
  mapping from package state and Linux filesystem observations to those classes,
  writable or persistent-looking storage could incorrectly authorize
  destructive work.
- The current image owns immutable application files and writable `/data` and
  `/config` volumes. Helm optionally maps `/data` to a persistent volume claim,
  otherwise uses `emptyDir`, and permits generic environment injection. It has
  no dedicated, read-only root-catalog artifact. A service-writable file under
  `/config` cannot be the deployment authority for its own filesystem access.
- Docker, Helm, and native local development need one representation with the
  same parser and identity semantics. Package-specific environment tuples or
  independently implemented adapters would create divergent behavior and make
  portable logical-key resolution depend on the launch mechanism.
- Root paths are deployment configuration, not credentials or portable profile
  state. They must not be accepted from the API, UI, database, profile YAML,
  discovery requests, jobs, or worker messages. They also must not be mixed
  into database or encryption Secrets.
- Revaer's release images currently target only Linux `amd64` and Linux `arm64`.
  No root-attestation or destructive-work claim has been proven for macOS,
  Windows, Docker Desktop host filesystems, or another operating system.

## Options

1. **Numbered environment variables or one inline JSON environment value.**
   Docker, Helm, and local launchers can supply this without another mounted
   file. However, root paths become process-environment data, shell and chart
   escaping become part of the contract, practical environment limits vary,
   and a 256-slot catalog is difficult to inspect and attest as one immutable
   input.
2. **One versioned, read-only JSON document loaded by the injected source.**
   Every package produces the same bounded document, bootstrap opens one stable
   file descriptor, and typed parsing yields one canonical semantic digest.
   This adds a mounted configuration artifact but keeps path authority outside
   runtime state and reuses the existing `serde_json` dependency.
3. **Separate native representations for Helm values, Docker flags, and local
   configuration.** Each adapter would directly build typed slots. This can
   provide package-specific ergonomics, but parser, bounds, enum, ordering, and
   identity behavior can drift, and no single artifact proves what the process
   admitted.
4. **Database-authored or API-managed catalog rows.** This would simplify remote
   administration, but it directly violates ADR 523 by allowing remote runtime
   state to expand host filesystem authority.

## Recommendation

- Adopt option 2.
- `RootCatalogSource` loads exactly one versioned UTF-8 JSON document at process
  bootstrap. It returns typed slots and source evidence to the root resolver;
  domain, API, profile, discovery, and worker code receive only normalized root
  identities and cannot construct this source or read its location.
- The packaged default location is
  `/etc/revaer/media-root-catalog.json`. Native Linux development may override
  only the document location with `REVAER_MEDIA_ROOT_CATALOG_FILE`; that value is
  an absolute UTF-8 path of at most 4,096 bytes and contains no catalog content.
  The environment must never contain slot tuples or root paths.
- A missing document means an empty catalog and media-root remediation state,
  not an implicit `/data` root. Invalid or untrusted input degrades the media
  subsystem and blocks all root-bound work while preserving the control-plane
  availability selected by ADR 514.
- The document is loaded once per process. A semantic catalog change requires a
  process restart, produces a new source digest, and is reconciled as a new ADR
  523 attestation generation. There is no file watcher, remote reload command,
  silent fallback, or in-place mutation of the active catalog.

### Exact Version 1 Document

- The top-level object contains exactly `format_version` and `slots`.
  `format_version` is the integer `1`; `slots` is an array containing zero
  through 256 objects.
- Each slot contains exactly these required fields:
  - `key`;
  - `allowed_kinds`;
  - `path`;
  - `durability_class`;
  - `durability_evidence`;
  - `sole_writer_class`; and
  - `sole_writer_evidence`.
- `key` preserves ADR 523's 1-64 byte normalized key of lowercase ASCII letters,
  digits, and hyphens. Keys are unique across the complete document.
- `allowed_kinds` is a nonempty, duplicate-free subset of the exact closed order
  `source`, `output`, `workspace`, `backup`, and `quarantine`. Unknown kinds are
  rejected. One slot may include both `source` and `output` only under ADR 523's
  same-slot rules; the document does not authorize any other overlap.
- `path` preserves ADR 523's absolute, valid UTF-8, maximum 4,096-byte path.
  NUL is rejected. Parsing a path does not prove it: the root resolver must open
  every component descriptor-relatively without following symlinks and must
  apply all accepted identity, ancestry, overlap, bind-mount, capability, and
  attestation rules before the slot becomes available.
- The raw document is limited to 8,388,608 bytes, read with one additional byte
  to prove the bound. This bounds whitespace and escape amplification without
  weakening ADR 523's decoded field limits. Empty input, multiple JSON values,
  trailing non-whitespace, duplicate object fields, duplicate slot keys,
  duplicate kinds, unknown fields, unknown enum values, non-integer versions,
  and values outside any accepted bound fail the complete document.
- JSON object order, slot order, kind order, and insignificant whitespace do not
  carry identity. After typed validation, slots are sorted by key and kinds use
  the fixed order above. The source digest is SHA-256 over a canonical frame
  containing the ASCII domain `revaer-media-root-catalog`, one zero byte, the
  big-endian unsigned 32-bit format version and slot count, then each sorted
  slot's decoded key and path as a big-endian unsigned 32-bit byte length
  followed by exact bytes, one kind-mask byte, and four enum bytes. Kind-mask
  bits zero through four are `source`, `output`, `workspace`, `backup`, and
  `quarantine`. Enum byte assignments are: durability class `disposable = 0`
  and `restart_persistent = 1`; durability evidence `none = 0`,
  `linux_dedicated_mount = 1`, and
  `kubernetes_persistent_volume_claim = 2`; sole-writer class
  `uncontrolled = 0` and `revaer_exclusive = 1`; and sole-writer evidence
  `none = 0`, `linux_dedicated_service = 1`, and
  `kubernetes_read_write_once_pod = 2`. Raw JSON bytes are never the semantic
  identity.
- The document contains no credentials, tokens, database URLs, Secret names,
  profile identifiers, discovery paths, retention settings, native commands, or
  arbitrary extension fields. A future format requires a separately proposed
  and approved contract version; version 1 never ignores an unknown field.

Example shape only; the values do not grant readiness until their evidence is
validated:

```json
{
  "format_version": 1,
  "slots": [
    {
      "key": "media-library",
      "allowed_kinds": ["source", "output"],
      "path": "/data/media-library",
      "durability_class": "restart_persistent",
      "durability_evidence": "linux_dedicated_mount",
      "sole_writer_class": "revaer_exclusive",
      "sole_writer_evidence": "linux_dedicated_service"
    }
  ]
}
```

### Source Authority And Loading

- Bootstrap opens the configured document from a trusted directory descriptor
  with no-follow semantics before media tasks start. It rejects symlinked final
  entries, files with additional hard links, non-regular files, mutable
  replacement during the read, files larger than the bound, and source
  locations writable by an untrusted identity.
- In the release container, the source must be mounted read-only and must not be
  under the service-writable `/config` or `/data` trees. The service must be
  unable to replace, unlink, chmod, or rewrite it. Root paths are mounted
  separately and are never inferred from the catalog file's parent directory.
- The Helm chart renders the version 1 document into a dedicated ConfigMap or
  mounts one explicitly named existing ConfigMap. Exactly one mode is allowed.
  The selected key is mounted read-only as the regular file at the packaged
  location; projected symlink traversal is not accepted by the loader. Generic
  `extraEnv` or `extraEnvFrom` entries cannot set reserved catalog variables or
  supply inline slot state.
- Docker and other native Linux container launches bind-mount the document as a
  read-only regular file at the packaged location and mount every declared root
  independently. The image does not bake in host paths and the existing writable
  `/config` volume is not a catalog source.
- Native Linux development may use the location override. The file must be
  owned by the launching operator, must not be group- or world-writable, and is
  opened before runtime tasks start. This supports parser, catalog, and
  read-only development flows; destructive readiness still requires one of the
  exact evidence combinations below.
- The database stores normalized attestations and source digests only after the
  resolver proves the slots. Database rows, API input, portable YAML, and a
  previous attestation can never add a slot missing from the current source.

### Closed Durability Classes

- `durability_class` is exactly one of:
  - `disposable`: no survival guarantee across process, container, pod, host,
    or volume replacement; or
  - `restart_persistent`: the deployment asserts that data survives ordinary
    Revaer process and container or pod replacement. This class does not claim
    host, storage-device, region, backup, or disaster survival.
- `durability_evidence` is exactly one of:
  - `none`;
  - `linux_dedicated_mount`; or
  - `kubernetes_persistent_volume_claim`.
- The only valid mappings are:

| Durability class | Evidence | Required result |
| --- | --- | --- |
| `disposable` | `none` | Record disposable state; never grant destructive readiness. |
| `restart_persistent` | `linux_dedicated_mount` | On native Linux, prove the root is on a dedicated externally mounted filesystem, record mount and filesystem identities, reject volatile/image-root placement, pass the ADR 523 create, fsync, rename, delete, and capacity probes, and prove restart persistence in package tests. |
| `restart_persistent` | `kubernetes_persistent_volume_claim` | Prove the chart maps the slot to a PVC rather than `emptyDir`, record the observed mount identity, pass the same filesystem probes, and prove data survives pod deletion and recreation in package tests. |

- Any other class/evidence pair is invalid. Writability, directory creation,
  free-space reporting, a familiar filesystem type, a Docker volume name, or a
  Helm persistence boolean alone never upgrades `disposable` to
  `restart_persistent`.
- Helm `emptyDir`, native temporary directories, image-layer directories, and
  unclassified mounts map only to `disposable` plus `none`. A PVC can establish
  restart-persistence evidence but does not by itself establish sole-writer
  control.

### Closed Sole-Writer Classes

- `sole_writer_class` is exactly one of:
  - `uncontrolled`: the deployment supplies no sufficient evidence that Revaer
    exclusively owns writes to the slot; or
  - `revaer_exclusive`: during the service lifetime, the managed namespace is
    dedicated to one Revaer instance and every Revaer process participating in
    that deployment uses the same held root lock. This is not protection against
    a privileged host administrator or storage-system failure.
- `sole_writer_evidence` is exactly one of:
  - `none`;
  - `linux_dedicated_service`; or
  - `kubernetes_read_write_once_pod`.
- The only valid mappings are:

| Sole-writer class | Evidence | Required result |
| --- | --- | --- |
| `uncontrolled` | `none` | Permit read-only source use when all other checks pass; block every write-capable root use. |
| `revaer_exclusive` | `linux_dedicated_service` | On native Linux, require a deployment-dedicated mount or subtree, protected ownership and ancestry, no other declared slot overlap, and a process-lifetime descriptor-bound exclusive Revaer lock. Docker or service-manager configuration must explicitly assert that no other container or service mounts the namespace for writing. |
| `revaer_exclusive` | `kubernetes_read_write_once_pod` | Require exactly one Revaer replica, a PVC whose effective access mode is exactly `ReadWriteOncePod`, no second writable mount of the namespace in the pod specification, protected ownership and ancestry, and the same process-lifetime root lock. Existing claims whose effective mode cannot be proven remain uncontrolled. |

- Any other class/evidence pair is invalid. Kubernetes `ReadWriteOnce` and
  `ReadWriteMany`, a single configured replica without mount exclusivity, Unix
  mode bits without a held lock, and a process lock without deployment ownership
  all map to `uncontrolled`.
- Every slot used as `output`, `workspace`, `backup`, or `quarantine` requires
  `revaer_exclusive` before any write. Destructive execution additionally
  requires `restart_persistent`. A source-only slot may remain `uncontrolled`;
  the ADR 523 same-slot source/output case requires both persistent and
  exclusive evidence before replacement.
- These root controls compose with ADRs 512 and 513. They do not select worker
  lease, recovery leadership, cleanup claim, retry, or timeout values and do not
  replace any generation fence or recovery barrier.

### Platform Availability

- Version 1 root attestation and destructive readiness are available only for
  the release targets `linux/amd64` and `linux/arm64`, and only after the exact
  package/filesystem evidence matrix above passes on each architecture.
- macOS, Windows, other Unix systems, Docker Desktop host-mounted filesystems,
  network filesystems, FUSE filesystems, and filesystem types absent from the
  validated release matrix are `media_root_platform_unsupported` or
  `media_root_durability_unproven`; they do not inherit Linux support by analogy.
- Cross-platform parser tests may run, but they cannot create an attestation or
  make destructive work available. Adding a platform, filesystem, or evidence
  adapter requires separately reviewed proof and, when semantics differ, a
  proposed ADR.

### Stable Fail-Closed Outcomes

- Source loading uses bounded reasons
  `media_root_catalog_source_missing`, `media_root_catalog_source_untrusted`,
  `media_root_catalog_format_invalid`, `media_root_catalog_bound_exceeded`, and
  `media_root_platform_unsupported`.
- Evidence mismatches use ADR 523's
  `media_root_attestation_invalid`, `media_root_durability_unproven`, and
  `media_root_writer_control_unproven`. Path, overlap, binding, and identity
  failures retain ADR 523's existing stable errors.
- Health and metrics expose only source state, bounded evidence class, root kind,
  and bounded reason. They do not label paths, keys, mount identifiers, device or
  inode values, source digests, profiles, jobs, pods, or claims. Full paths remain
  restricted to the authenticated instance-root administration surface.

## Consequences

- Docker, Helm, and native Linux development share one parser, one normalized
  source digest, and one fail-closed evidence model.
- Environment variables identify only the document location, and remote product
  configuration cannot expand filesystem authority.
- Helm must add a dedicated ConfigMap and mount contract. Its default
  `emptyDir` and `ReadWriteOnce` configurations remain useful for disposable or
  read-only evaluation but cannot silently authorize destructive transcoding.
- Production Kubernetes destructive work requires `ReadWriteOncePod` and one
  replica under this recommendation. Existing claims or storage classes that
  cannot prove those semantics remain visible for remediation but unavailable
  for writes.
- Native Linux Docker and service-manager deployments retain an explicit
  operator assertion for mount exclusivity, backed by ownership, ancestry,
  mount, capability, and process-lock evidence. The contract does not pretend to
  defend against a hostile privileged host.
- Restart-only catalog reconciliation makes changes explicit and deterministic,
  at the cost of requiring a process or pod restart after deployment edits.
- Unsupported platforms and filesystems fail closed rather than inheriting a
  capability from a similar environment.

## Implementation Boundary

- Approval authorizes only:
  - the injected startup-only JSON source, exact version 1 schema and canonical
    digest;
  - the fixed packaged location and native Linux location override;
  - source descriptor, read-only mount, ownership, and mutation checks;
  - the closed durability, sole-writer, and evidence enums and mappings above;
  - Docker, Helm, and native Linux adapters that produce the same document;
  - root attestation, administration, health, and stable-error integration
    needed to expose those exact outcomes; and
  - focused parser, filesystem, package, API, UI, and E2E validation of the
    accepted ADR 523 root-catalog and profile-binding workflow.
- Approval does not authorize raw path authority in API, UI, YAML, database,
  discovery, job, or worker input; a service-writable catalog; symlink following;
  arbitrary JSON fields; live reload; a second parser; an unbounded document;
  secret material; cross-platform inference; or destructive work on disposable
  or writer-uncontrolled roots.
- ADR 523's maximum 256 slots, 1-64 byte keys, maximum 4,096-byte decoded paths,
  exact five root kinds, overlap rules, same-slot source/output restriction,
  attestation-generation behavior, descriptor-relative proof, and prohibition
  on remote path authority remain binding and are not relaxed.
- ADRs 512-516, 525, 535, and 537 retain every unresolved scheduler, lease,
  recovery, cleanup, retry, fingerprint, parser, extraction, and numeric hold.
  This accepted ADR supplies no missing value, default, activation, or workaround
  for those decisions.
- Every implementation must remain within this boundary and use the
  authoritative pre-v1 `init.sql` under ADR 522 rather than adding a migration.

## Exact Validation

- **Parser and bounds:** test empty and maximum catalogs; one below, at, and one
  above every slot, key, decoded path, override path, and raw document bound;
  every invalid UTF-8, NUL, duplicate key, duplicate field, duplicate kind,
  unknown field, unknown enum, malformed number, wrong version, trailing value,
  and escape-amplification case; and deterministic semantic digests across every
  permitted ordering and whitespace variation.
- **Source authority:** race replacement, truncation, extension, chmod, unlink,
  rename, symlink, hard-link, mount replacement, and parent-directory mutation
  before open, during read, after parse, and before attestation. Prove the active
  source is one stable regular-file descriptor and that writable `/config` and
  `/data` sources fail.
- **Root proof:** cover missing, non-directory, permission-invalid, nested,
  equal, ancestor, descendant, bind-mounted, symlinked, cross-device, unsafe-
  ancestry, and capability-incomplete slots. Open every root and child through
  descriptor-relative, no-follow operations and compare exact identities at ADR
  523's enqueue, claim, admission, and pre-mutation boundaries.
- **Evidence matrix:** exercise every valid and invalid class/evidence pair.
  Prove `emptyDir`, temporary directories, image layers, unclassified mounts,
  `ReadWriteOnce`, `ReadWriteMany`, replica counts above one, missing root locks,
  and unproven existing claims cannot become destructive-ready. Prove process or
  pod recreation preserves restart-persistent fixtures and disposable fixtures
  never gain that classification from successful writes.
- **Package integration:** render Helm inline and existing-ConfigMap modes,
  reject both/neither conflicts where a catalog is required, inspect the running
  mount and environment, and prove the service cannot rewrite the file. Run
  native Linux Docker bind-file and root-mount cases without placing host paths
  in the image or environment. Validate each case on Linux `amd64` and `arm64`.
- **Writer exclusion:** run competing Revaer processes and pods against each
  root, prove exactly one holds the descriptor-bound lock, prove
  `ReadWriteOncePod` rejects simultaneous pod ownership in the supported cluster
  test, and verify loss or replacement of any ownership evidence closes the
  write gate before mutation.
- **Outside-in workflow:** inspect the authenticated root catalog, map logical
  keys into a profile and discovery association, keep unresolved mappings
  disabled, admit the exact ADR 523 five-kind snapshot, and revalidate every root
  before filesystem effects. Prove API and YAML attempts to submit paths are
  rejected and that portable exports contain logical keys only.
- **Platform closure:** run negative macOS, Windows, Docker Desktop, network, and
  FUSE fixtures where available and prove they remain unavailable unless a
  separately accepted and tested adapter exists. Absence of a test environment
  is not positive evidence.
- **Repository gates:** run focused parser/filesystem/database/package tests,
  complete media fixtures with cleanup, generated OpenAPI/client checks,
  `just ci`, `just ui-e2e`, release-image validation on both architectures, and
  strict Sonar analysis with positive coverage before claiming implementation.

## Operator Questions

1. Do you approve option 2, including the exact version 1 JSON schema, fixed
   packaged path, 8 MiB source bound, startup-only loading, and canonical
   semantic digest?
2. Do you approve the closed durability and sole-writer classes and the strict
   evidence mappings, including treating Helm `emptyDir`, `ReadWriteOnce`, and
   `ReadWriteMany` as insufficient for destructive readiness?
3. Do you approve `linux_dedicated_service` as bounded operational evidence for
   native Linux Docker/service-manager deployments, with the explicit limitation
   that it relies on an operator exclusivity assertion plus runtime ownership,
   ancestry, mount, capability, and process-lock proof?
4. Do you approve making root attestation and destructive readiness unavailable
   outside validated native Linux `amd64` and `arm64` environments until a
   separately reviewed adapter proves equivalent semantics?

## Follow-up

- Implement the source and root-administration workflow outside
  in: package document, typed parser and resolver, attestation persistence,
  authenticated catalog API/UI, logical profile and association bindings, exact
  five-kind job snapshot, then descriptor-owned runtime revalidation.
- Keep automatic watcher and schedule activation unavailable until ADR 516's
  held values and validation receive separate operator approval.

## Task Record

- Motivation:
  - Resolve the concrete source-encoding and evidence-class gaps that block the
    accepted ADR 523 root-catalog, profile-binding, and E2E workflow.
- Design notes:
  - The recommendation uses one package-neutral, read-only typed document and a
    canonical semantic digest rather than package-specific parsers or raw
    environment content.
  - Durability and sole-writer state require both a closed deployment assertion
    and independent runtime evidence; no writable-path heuristic grants
    destructive readiness.
  - The operator accepted the recommendation as written on 2026-08-16.
- Test coverage summary:
  - This documentation-only change adds no parser, filesystem, schema, package,
    API, UI, workflow, chart, config, or media test.
  - Validation is limited to policy, instruction-drift, generated documentation,
    documentation links/build, and diff hygiene.
- Observability updates:
  - No runtime observability surface changes.
  - A future accepted implementation must expose only bounded source, class,
    evidence, lifecycle, and reason values as described above; sensitive paths
    and high-cardinality identities remain excluded from metrics and general
    logs.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, the current Dockerfile and Helm chart,
    bootstrap environment loading, and ADRs 447, 452, 512-516, 521, 523, 525,
    535, 537, and 541.
  - Current product, package, API, UI, and runtime claims remain unchanged. The
    ADR index, documentation summary, and generated documentation catalogue are
    updated to expose this accepted decision.
- Risk & rollback plan:
  - Recording acceptance changes no runtime behavior and needs no operational
    rollback. Changing the accepted contract requires a superseding Proposed
    ADR and decision-specific operator approval.
  - After implementation, rollback must disable root-bound writes when the prior
    runtime cannot parse or prove an attestation. It must preserve recorded jobs
    and attestations and must never reinterpret unknown evidence as valid.
- Dependency rationale:
  - No dependency is added. The recommendation deliberately uses the existing
    `serde_json`, SHA-256, and Linux descriptor/filesystem facilities. Any future
    dependency requires separate written rationale and supply-chain review.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/revaer-data.instructions.md`,
    `.github/instructions/revaer-ui.instructions.md`, and
    `.github/instructions/devops.instructions.md`.
  - No policy drift, contradiction, suppression, criteria relaxation, or stale
    implementation claim was found. The operator approval satisfies the
    architecture-approval hold for this exact contract.
