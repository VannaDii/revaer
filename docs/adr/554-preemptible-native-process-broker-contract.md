# Preemptible native process broker contract

- Status: Accepted
- Date: 2026-08-16
- Operator approval: Approved by Vanna on 2026-08-16

## Context

- Accepted ADR 549 selects a long-lived, independently supervised native-process
  broker instead of unsafe or platform-specific process creation. It does not
  select the broker count, IPC representation, lifecycle, startup and recovery
  limits, executable authority, containment proof, or fatal-failure behavior.
- Accepted ADR 501 requires one absolute deadline around every shipped native
  media invocation. The current in-process supervisor starts that clock before
  `std::process::Command::spawn`, but a synchronous spawn or pipe setup can block
  before the application receives a controllable child handle.
- A useful broker must already have a controllable identity before a media
  request begins. A broker created per request merely moves the same spawn gap
  and does not satisfy ADR 549.
- Revaer currently has one serial media-job runtime, but control-plane capability
  probes and 30-second inspections must not queue behind a transcode that may run
  for 24 hours. An unbounded or configurable broker pool would add concurrency
  and resource behavior that ADR 501 did not authorize.
- The release image contains one root-owned, read-only `revaer-app` executable
  and supports Linux `amd64` and Linux `arm64`. Docker and Helm currently launch
  it directly, without a second binary or privileged sidecar.
- Native media requests contain operating-system byte strings and potentially
  sensitive absolute paths. IPC must remain local, private, bounded, shell-free,
  and absent from logs, metrics, command lines, and the network.
- A media child can deliberately create a new session after exec and leave its
  inherited process group. ADR 501 already requires deployment containment for
  that residual case. The broker must preserve, not obscure, that requirement.

## Options

1. **One same-binary broker with one serial queue.** This is simple and bounded,
   but a long job blocks capability refresh and inspection for its full lifetime.
2. **Two same-binary brokers with fixed control and job lanes.** This preserves
   bounded concurrency without allowing a configurable pool. Each lane is
   independently replaceable and has a pre-existing process-group handle before
   its request begins.
3. **A network or Unix-socket sidecar service.** This can be independently
   deployed, but requires socket ownership and discovery, cross-container process
   control, sidecar versioning, and privileges or cgroup APIs that the current
   package does not possess.
4. **A broker per request.** Reject this because the request would again wait on
   an uninterruptible broker spawn before obtaining a control handle.

## Recommendation

- Adopt option 2 with every exact boundary below.
- Run the existing immutable application executable in a private internal mode
  named `media-process-broker-v1`; do not add a second executable, shell wrapper,
  network listener, dynamic plugin, or new runtime dependency.
- Construct exactly two broker lanes during bootstrap:
  - `control` accepts capability detection and inspection requests whose complete
    ADR 501 deadline is at most 30 seconds;
  - `job` accepts transcode, audio-analysis, playback, and output-verification
    requests with ADR 501's existing duration-derived ceilings.
- Each lane accepts exactly one request at a time. Queue acquisition begins
  inside the caller's absolute deadline. The pool size, lane names, routing, and
  queue discipline are not configurable.

### Process And Containment Model

- A broker manager owns each broker child, its stdin/stdout/stderr pipes, process
  group, protocol state, and readiness state. Callers receive only an injected
  `NativeProcessSupervisor` and cannot construct or restart a broker.
- The manager starts each broker on a dedicated blocking thread before media
  services become ready. `Command::spawn` is used only for this pre-request
  broker lifecycle, never after a media request starts without an existing
  broker handle.
- The complete application deployment is the outer containment unit. Supported
  production deployment must prove all of these properties:
  - the application, both brokers, and every media child share one container,
    cgroup, or service-manager control group;
  - application startup and broker recovery are terminated after 30 seconds;
  - stopping the application kills every process in that containment unit;
  - the service restarts after a nonzero broker-lifecycle exit; and
  - the service identity cannot move a process outside that containment unit.
- Helm must express the proof in one container and one pod cgroup without a
  privileged sidecar or shared host PID namespace. Native Linux service-manager
  support requires an equivalent control-group kill mode, 30-second startup
  timeout, restart-on-failure policy, and `NoNewPrivileges`. Plain background
  launch, Docker without a restart/health supervisor, Docker Desktop, macOS,
  Windows, network filesystems, and an uncontained shell are not production
  media environments.
- A broker is a process-group leader. Every media child remains in that broker's
  process group from fork through exec. The application retains the broker child
  handle and process-group identifier before submitting a request.
- On parent loss, broker IPC EOF, protocol corruption, request cancellation,
  absolute deadline, frame limit, or fatal broker supervision failure, the
  manager sends `SIGTERM` to the complete broker process group, waits ADR 501's
  exact five-second grace while draining bounded diagnostics, sends `SIGKILL`,
  reaps the broker, and verifies that the group no longer exists before reporting
  the request outcome.
- A process that creates a new session after exec remains outside the process-
  group proof. Detection of inherited output after group cleanup is secondary
  evidence, and the outer deployment containment remains mandatory.

### Startup, Recovery, And Readiness

- The application starts both broker managers before database-backed media
  workers, capability refresh, discovery, retention, API mutation, or media
  readiness. Non-media control-plane startup may continue only after both lanes
  are either ready or explicitly unavailable.
- After `spawn` returns, the broker must emit a complete handshake within five
  seconds. The parent verifies protocol version, lane, child PID, process-group
  ID, executable identity, and a parent-generated 128-bit startup nonce returned
  over the inherited pipe. A mismatch kills the complete group and fails the
  lane.
- The outer startup deadline is exactly 30 seconds from manager-thread creation,
  including synchronous spawn. If it expires, application bootstrap returns a
  stable nonzero `media_broker_startup_deadline` outcome and relies on the proven
  deployment containment to terminate a manager thread blocked inside spawn.
- A lane destroyed while handling a request becomes unavailable immediately.
  Existing requests on the other lane may settle, but no new media work is
  admitted while either lane is unavailable.
- The manager makes one replacement attempt on a fresh blocking thread. The
  replacement uses the same 30-second outer deadline and five-second handshake.
  Successful replacement restores media readiness. Failure or timeout requests
  cooperative application shutdown and a nonzero exit; there is no retry loop,
  backoff setting, partially ready media mode, or fallback to in-process spawn.
- Health exposes one closed state per lane: `starting`, `ready`, `busy`,
  `recovering`, or `failed`. Media readiness requires both lanes `ready` or
  `busy`; liveness fails only after recovery fails or expires. State transitions
  use bounded reason codes and never expose PIDs, paths, arguments, raw protocol
  bytes, or native diagnostics.

### Executable And Environment Authority

- Every request program is an absolute, valid Unix byte path with no NUL and at
  most 4,096 bytes. Bootstrap capability discovery resolves the packaged native
  tools to descriptor-verified, root-owned, non-writable regular files. A request
  names only one verified executable identity from the bound capability
  snapshot; PATH search, shell interpretation, aliases, and request-provided
  environment variables are forbidden.
- Broker startup clears the application environment and restores only a closed,
  bounded operational allowlist: `PATH`, `LD_LIBRARY_PATH`, `SSL_CERT_FILE`,
  `SSL_CERT_DIR`, `LANG`, `LC_ALL`, `TZ`, `HOME`, and `TMPDIR`. Each value is at
  most 4,096 bytes, contains no NUL, and is snapshotted at bootstrap. Database
  URLs, encryption material, tokens, tracing exporters, arbitrary `REVAER_*`
  values, and deployment secrets never enter the broker environment.
- Media children inherit that same closed snapshot. Requests cannot add, remove,
  or override environment values. The exact release defaults remain versioned in
  package configuration and native development must pass the same validation.

### Version 1 Pipe Protocol

- IPC uses only the broker's inherited stdin and stdout pipes. The broker never
  creates or opens a socket, FIFO, shared-memory object, lock file, or network
  endpoint. Stderr is a separate lifecycle-diagnostic pipe capped at 65,536
  bytes per broker lifetime.
- Every frame begins with ASCII `RVB1`, one byte frame kind, three reserved zero
  bytes, and one big-endian unsigned 32-bit payload length. Unknown kinds,
  nonzero reserved bytes, duplicate terminal frames, trailing payload bytes,
  out-of-order request identifiers, premature EOF, and a payload over its bound
  are fatal protocol errors.
- The parent sends one handshake nonce and then serial request frames. Request
  identifiers are unsigned 64-bit integers beginning at 1 and increasing by one
  without wrap. A broker response must echo the active identifier exactly.
- One request payload is at most 1,048,576 bytes and contains, in fixed order:
  protocol version `1`, lane, request identifier, remaining deadline in whole
  milliseconds, stdout and stderr byte limits, executable identity digest,
  program bytes, argument count, and length-prefixed argument bytes.
- A request has at most 4,096 arguments. Each argument is at most 4,096 bytes;
  total program and argument bytes are at most 1,000,000 bytes. Empty arguments
  are preserved, but NUL is rejected. Text decoding, JSON, shell quoting, and
  lossy Unicode conversion are forbidden.
- A response uses a closed status enum matching ADR 501's typed outcomes:
  `success`, `spawn_failed`, `cancelled`, `deadline_exceeded`,
  `stdout_limit_exceeded`, `stderr_limit_exceeded`, `exit_failed`, and
  `supervision_failed`. Broker lifecycle adds only `broker_unavailable`,
  `broker_protocol_failed`, and `broker_recovery_failed` at the application
  boundary.
- Success carries complete stdout and stderr within the request limits. Failure
  carries the stable status, bounded stderr detail, and normalized secondary
  evidence. Each output and evidence field is at most 16,777,216 bytes; the
  complete response payload is at most 50,335,744 bytes. No response may contain
  the program, arguments, environment, PID, raw OS error beyond the existing
  bounded diagnostic, or a path copied solely for logging.
- Parent and broker independently enforce the frame, field, stream, and absolute
  deadline bounds. The parent is authoritative: a broker response cannot extend
  a deadline, change a limit, reclassify cancellation, or make cleanup optional.

### Request Execution

- The caller records the absolute deadline before acquiring its lane. While
  queued and while exchanging frames, it polls cancellation at the existing
  five-millisecond process interval. Expiry or cancellation before request write
  returns the typed outcome without touching the broker.
- After a request begins, cancellation or expiry destroys the broker group. The
  caller does not wait for a cooperative broker acknowledgement as a substitute
  for process-group cleanup.
- The broker validates the complete request before native spawn. It creates
  stdin as null, captures stdout and stderr, keeps the child in the broker group,
  drains both streams concurrently, and emits no terminal frame until the child
  is reaped and all non-broker members of the group are absent.
- A normal child exit may keep the broker only when output settlement and group
  membership are proven. Any ambiguous wait, read, frame, descendant, or cleanup
  state is fatal to the lane and causes parent-owned group destruction.
- Primary-error precedence and bounded secondary evidence remain exactly those
  accepted by ADR 501. Moving process creation behind IPC must not convert typed
  failures to strings, discard concurrent cleanup evidence, or log the same
  error twice.

## Consequences

- Every media request begins with a pre-existing broker process-group handle, so
  blocked spawn and pipe setup are preemptible by killing that known group.
- Two fixed lanes preserve control-plane responsiveness without introducing a
  configurable worker pool or unbounded native concurrency.
- Cancellation and hard deadlines may replace one broker. Media work pauses
  during the single bounded replacement attempt, but the API can expose exact
  remediation state instead of silently falling back.
- The same executable and inherited pipes keep packaging and dependency growth
  small, but require a reviewed binary protocol and explicit broker mode.
- Supported production media operation becomes Linux-container or equivalent
  Linux control-group only. Local unsupported platforms can run injected tests
  but cannot claim the hard startup guarantee.
- A process deliberately escaping its process group is still handled by outer
  deployment containment, not falsely claimed as an application-only guarantee.

## Implementation Boundary

- Approval would authorize only the two fixed lanes, same-binary broker mode,
  exact process/containment model, startup and recovery lifecycle, environment
  allowlist, version 1 pipe protocol, bounds, stable states, and validation
  described above.
- Approval would not authorize a sidecar, socket, network service, configurable
  broker count, additional retry, shell, request environment, broader platform
  claim, unsafe code, new dependency, higher media-job concurrency, weaker
  deployment containment, or fallback to direct in-process spawning.
- Any change to a constant, field, state, lane, environment name, protocol
  version, retry count, containment claim, or support matrix requires a later
  Proposed ADR and decision-specific operator approval before code.
- This proposal implements no Rust, package, workflow, chart, API, UI, health,
  or runtime behavior.

## Exact Validation

- Prove frame parsing with every empty, exact-bound, one-over-bound, truncated,
  duplicated, reordered, unknown, nonzero-reserved, NUL, and integer-overflow
  case. Fuzz the parser under bounded input without adding a persistent runtime
  dependency.
- Deterministically hold broker startup and media-child spawn before and after
  process-group setup. Prove the 30-second outer lifecycle and every request
  deadline terminate the complete known group and retain cleanup evidence.
- Exercise control and job requests concurrently. Prove same-lane FIFO behavior,
  queue time inclusion, cancellation while queued, no cross-lane response, and
  no third native process.
- Prove success, every typed failure, malformed protocol, broker crash, parent
  loss, inherited-pipe hold, output flood, descendant hold, replacement success,
  replacement failure, cooperative application shutdown, and restart recovery.
- Inspect `/proc`, container cgroups, Helm pod state, and native service-manager
  state on Linux `amd64` and `arm64`. Prove the service identity cannot escape
  containment and outer termination removes a session-escaping fixture.
- Prove broker and child environments contain exactly the allowlist and never
  contain database, encryption, API, tracing, or arbitrary deployment secrets.
- Run focused process/broker/bootstrap/package/Helm tests, media fixtures with
  cleanup, release-image validation on both architectures, `just ci`,
  `just ui-e2e`, strict Sonar with positive coverage and retained evidence, all
  required GitHub checks, and a security diff scan before implementation claims.

## Operator Questions

1. Do you approve exactly two serial same-binary broker lanes, `control` and
   `job`, with the fixed routing and no configurable pool?
2. Do you approve the 30-second outer startup/recovery deadline, five-second
   handshake, one replacement attempt, fail-closed media readiness, and nonzero
   application exit after failed recovery?
3. Do you approve the exact inherited-pipe protocol, request/argument/response
   bounds, stable status set, and closed environment allowlist?
4. Do you approve production media support only in validated Linux `amd64` and
   `arm64` deployment containment, with process-group escape handled by the
   mandatory outer container or service control group?

## Follow-up

- Present this Proposed ADR for decision-specific operator approval. Do not
  implement or push a broker prototype before approval.
- If accepted, implement outside in: package containment and lifecycle tests,
  protocol model and parser, broker child mode, manager/readiness boundary,
  supervisor routing, replacement behavior, then complete native-call cutover.
- Keep the current elapsed-time supervisor as reviewed transition code until the
  broker passes the complete matrix; remove it atomically rather than retaining
  an unapproved fallback.

## Task Record

- Motivation:
  - Convert the accepted broker direction into a closed implementation contract
    that can actually prove preemptible startup without making new decisions in
    code.
- Design notes:
  - Two fixed lanes separate short control work from long job work while keeping
    native concurrency bounded. Parent-owned process-group destruction remains
    authoritative even when the broker reports a typed failure.
  - The protocol carries raw Unix bytes and closed enums so it does not depend on
    shell quoting, lossy Unicode, or free-form JSON.
- Test coverage summary:
  - This documentation-only proposal adds no runtime tests. Applicable checks are
    policy, instruction drift, documentation generation/build/links, and diff
    hygiene. The exact implementation matrix is specified above.
- Observability updates:
  - The proposed health surface uses closed lane states and bounded reason codes.
    PIDs, paths, arguments, environment, protocol bytes, and native diagnostics
    remain excluded from metrics and general logs.
- Status-doc validation:
  - Reviewed accepted ADRs 501 and 549, the native supervisor model, bootstrap,
    release Dockerfile, Helm deployment and values, and supported architectures.
    Product guides remain unchanged because no broker behavior is implemented.
- Risk & rollback plan:
  - The proposal changes no runtime behavior. Rejection removes or supersedes
    this record. After implementation, rollback disables media readiness and
    reverts the coordinated broker/package slices; it must not restore direct
    spawn as a production fallback.
- Dependency rationale:
  - No dependency is proposed. The recommendation uses `std`, existing `rustix`,
    the current executable, and existing process-group primitives. A socket,
    serialization library, sidecar, or unsafe process API was rejected.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/ffi.instructions.md`, and
    `.github/instructions/devops.instructions.md`.
  - No policy relaxation, source suppression, dependency exception, or stale
    implementation claim is introduced. Architectural implementation remains
    blocked on explicit approval of this exact proposal.
