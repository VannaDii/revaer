# Descriptor-bound media inspection inputs

- Status: Accepted
- Date: 2026-08-16
- Operator approval: Explicitly approved by the operator on 2026-08-16:
  "I approve the ADRs as they are now and I'm resuming your goal."

## Problem

- Current inspection snapshots pathname metadata, passes the pathname to
  `ffprobe`, and checks pathname metadata afterward.
- A concurrent actor can replace source `A` with `B`, let `ffprobe` open `B`,
  and restore `A` before the final pathname check. The accepted probe output can
  then describe different bytes than the identity retained for planning.
- Source media, sidecars, and companion files all cross this boundary. A
  production planner must bind evidence to the exact opened objects instead of
  trusting a pathname to retain one meaning across process launch.
- ADRs 447, 450, 452, 501, 519, 523, and 524 define managed-root ownership,
  sidecar policy, dependency injection, process supervision, execution closure,
  root binding, and token grammar. They do not decide how native inspection is
  bound to an opened filesystem identity.

## Options

1. **Keep pre/post pathname metadata checks.** This is portable and cheap, but
   it retains the swap-and-restore race.
2. **Add advisory locks around path inspection.** This can coordinate with
   cooperating writers, but it does not constrain non-cooperating writers and
   still lets the child resolve a pathname independently.
3. **Open once and inspect inherited read-only handles.** Resolve each input
   relative to its attested managed-root handle, retain the opened object for the
   complete inspection, and expose only private child-local aliases for those
   handles. This removes child pathname re-resolution and keeps paired inputs
   representable.
4. **Copy every input into the attempt workspace before inspection.** This
   provides private pathnames but doubles source IO and storage, can exceed ADR
   501's inspection deadline for large files, and still needs a consistent-copy
   contract.

## Recommendation

- Adopt option 3.
- Add an injected `MediaInspectionInputOpener` boundary. Production bootstrap is
  solely responsible for constructing its platform implementation.
- Resolve every source, sidecar, and companion relative to the already-attested
  managed-root directory handle. Resolution must reject absolute escapes,
  `..`, symlinks, magic links, non-regular files, mount-boundary escapes, and
  path components outside the bound root.
- Open each accepted object exactly once as a read-only, close-on-exec handle.
  The opener returns an owned handle plus bounded identity evidence; it does not
  return a second pathname for native execution.
- Extend the approved ADR 501 supervisor request with an explicit collection of
  read-only media handles. The supervisor closes every descriptor not required
  by the attested executable closure, stdio capture, cancellation, or this
  collection.
- The child receives deterministic private aliases within its sandbox, such as
  `/revaer-input/source` and `/revaer-input/sidecar-0001.srt`. The aliases refer
  to the retained handles, never to the host pathname. Related VobSub handles
  receive one private `.idx`/`.sub` pair in the same child-only directory.
- Capture descriptor metadata before process launch and after the final parse.
  Device, inode or platform file id, type, size, modification time, and change
  time must remain identical. A change is terminal even when the pathname still
  resolves to the same object.
- Re-resolve the claimed host pathname after inspection and require it to name
  the retained identity. A moved, removed, exchanged, or replaced path is
  terminal. Restoring the original path does not affect what the child read,
  because the child used the retained handle.
- The supported threat boundary excludes a compromised kernel and an actor with
  privileges sufficient to falsify kernel-maintained identity metadata. Those
  conditions are host-compromise events, not recoverable media-job races.
- A platform that cannot prove equivalent beneath-root, no-follow, retained-
  handle semantics reports media inspection unavailable. It must not fall back
  to pathname execution or mark media readiness healthy.

## Consequences

- Inspection evidence is bound to the same opened objects throughout native
  execution and parsing.
- Ordinary rename, exchange, symlink, and swap-and-restore races fail closed or
  become irrelevant to what the child reads.
- The process supervisor and inspection request gain a typed handle contract,
  but no generic arbitrary-descriptor inheritance surface.
- Platform support is explicit. A platform without the required primitive can
  run injected unit tests but cannot claim production media readiness.
- This decision does not itself prevent a demuxer from requesting auxiliary
  resources. ADR 527 separately decides native inspection confinement.

## Implementation Boundary

- This accepted ADR authorizes only the injected opener, typed retained-handle
  request, child-private aliases, before/after descriptor validation, final path
  reconciliation, readiness reporting, and tests described here.
- It does not authorize new input formats, remote media, arbitrary child
  descriptor inheritance, source mutation, pathname fallback, or weaker root
  validation.
- Persistence changes, if any become necessary, belong in the approved pre-v1
  `init.sql` transition under ADR 522.

## Validation

- Add deterministic tests that exchange `A` and `B` before child open, during
  inspection, and before final validation; accepted output must always describe
  the retained `A` or the operation must fail.
- Test symlinks, magic links, `..`, absolute paths, mount escapes, directories,
  devices, FIFOs, deleted paths, moved paths, in-place size/time changes, and
  duplicate identities.
- Test source plus every sidecar form, including paired VobSub handles, with
  stable child aliases and no host pathname in the native arguments.
- Prove that no unlisted descriptor reaches the child and that unsupported
  platform implementations fail readiness before queue admission.
- Run `just ci`, `just ui-e2e`, the media fixture suite, the security scan, and
  the strict Sonar gate after implementation.

## Follow-up

- Implement ADR 527 with this record so retained handles are not weakened by an
  unconstrained native parser.
- Reconcile ADR 501's request model, ADR 519's execution closure, ADR 523's root
  attestation, and the operator readiness surface in the implementation change.

## Task Record

- Motivation:
  - Record a high-severity identity race found during independent review of the
    reconstructed inspection foundation before that foundation is pushed.
- Design notes:
  - The recommendation preserves large-file performance by retaining handles
    instead of copying complete media into the workspace.
  - Private aliases support native tools that need filename relationships while
    keeping host path resolution outside the child.
- Test coverage summary:
  - No runtime test or media conversion was added or run by this proposal.
  - Proposal validation is documentation links, instruction drift, policy, and
    diff hygiene.
- Observability updates:
  - No telemetry changes are made by this proposal.
  - A future implementation must expose bounded reason codes for unsafe path,
    identity changed, path changed, handle unavailable, and platform unsupported.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md` and ADRs 447, 450, 452, 501, 519, 523, and
    524. No product-status claim is changed by this proposal.
- Risk & rollback plan:
  - This proposal changes no production behavior. If accepted implementation
    regresses inspection, rollback must disable media readiness; it must not
    restore pathname execution.
- Dependency rationale:
  - No dependency is added by this proposal. An implementation must prefer
    existing standard/platform APIs and separately justify any new safe wrapper
    required to avoid hand-written unsafe filesystem code.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/revaer-data.instructions.md`,
    `.github/instructions/ffi.instructions.md`, and
    `.github/instructions/devops.instructions.md`.
  - No policy drift or contradiction was found.
