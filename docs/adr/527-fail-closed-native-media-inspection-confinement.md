# Fail-closed native media inspection confinement

- Status: Accepted
- Date: 2026-08-16
- Operator approval: Explicitly approved by the operator on 2026-08-16:
  "I approve the ADRs as they are now and I'm resuming your goal."

## Problem

- An untrusted local media file currently reaches `ffprobe` without a protocol
  whitelist, demuxer whitelist, or filesystem/network sandbox.
- Playlist and auxiliary-resource demuxers can request HTTP, TCP, UDP, FTP,
  Unix sockets, arbitrary local files, `concat`, `subfile`, or other resources
  before Revaer can validate the parsed result. This creates SSRF and unintended
  local-resource access at the inspection boundary.
- The first-release specification requires local-file plans for MP4, Matroska,
  WebM, MPEG-TS, MOV, AVI, and the approved subtitle sidecar formats. It does not
  authorize remote inputs, playlist expansion, or ambient host-file discovery.
- ADR 501 bounds process lifetime and output. ADR 519 binds an attested execution
  closure. Neither one decides the resources the native parser may access.

## Options

1. **Rely on post-probe validation.** This preserves maximum FFmpeg compatibility
   but allows the unwanted access before validation can reject it.
2. **Use FFmpeg protocol and format allowlists only.** This is portable and
   sharply reduces exposure, but a parser defect or future alias drift can still
   reach ambient operating-system resources.
3. **Combine closed FFmpeg allowlists with an injected OS confinement profile.**
   Restrict the parser to the attested executable closure and ADR 526 input
   aliases, deny network access, deny writes, and fail readiness when equivalent
   confinement is unavailable.
4. **Run a separate privileged inspection service.** This can centralize
   confinement but adds a network service, deployment identity, lifecycle,
   protocol, and operational failure domain that the v1 scope does not need.

## Recommendation

- Adopt option 3.
- Add a closed `MediaInspectionConfinementV1` profile to the injected ADR 501
  supervisor. Production bootstrap supplies the platform implementation and
  must attest the profile before media readiness becomes healthy.
- The child filesystem view contains only:
  - read-only files in the immutable ADR 519 executable and dynamic-library
    closure required to launch the selected tool;
  - the read-only private input aliases approved by ADR 526;
  - pre-opened stdio and the minimum read-only operating-system pseudo-files
    explicitly proven necessary by the packaged fixture suite.
- The child has no source-root path, home directory, writable working directory,
  credential files, control sockets, devices, or ambient temporary directory.
  Writes are limited to the supervisor-owned stdout and stderr pipes.
- Deny network creation and connection at the operating-system boundary. A
  process that cannot be confined this way is unavailable, not degraded.
- Invoke `ffprobe` with `-protocol_whitelist file`. Do not admit `http`, `https`,
  `tcp`, `udp`, `ftp`, `sftp`, `rtsp`, `rtp`, `srt`, `rist`, `unix`, `concat`,
  `subfile`, `data`, `crypto`, or `pipe` as protocols. The `srt` subtitle demuxer
  is distinct from the forbidden SRT network protocol.
- Constrain primary-source auto-detection to the canonical FFmpeg demuxer alias
  families required by the first-release matrix:
  - `avi`;
  - `matroska,webm`;
  - `mov,mp4,m4a,3gp,3g2,mj2`, with normalized container validation still
    rejecting formats outside the approved MP4/MOV/M4A product surface;
  - `mpegts`.
- Force sidecar demuxers from the already-typed sidecar format:
  - `Srt -> srt`;
  - `Ass -> ass`;
  - `Vtt -> webvtt`;
  - `Sup -> sup`;
  - standalone `Sub -> microdvd`;
  - paired `VobSub -> vobsub`.
- Reject every source or sidecar whose selected or reported demuxer is outside
  its closed mapping. Do not retry with unrestricted probing.
- Explicitly reject playlist, manifest, image-sequence, virtual-device, concat,
  and external-reference demuxers in v1. Supporting one later requires a new
  decision that defines its resource graph and confinement contract.
- Record the confinement profile version, executable-closure identity, protocol
  policy version, and format-policy version in bounded inspection evidence.

## Consequences

- Untrusted media cannot use normal FFmpeg behavior to initiate network access
  or read arbitrary host files during inspection.
- The accepted v1 format surface is smaller than the complete packaged FFmpeg
  capability set. Capability presence does not imply policy authorization.
- Packaged and bare-metal media readiness must prove equivalent confinement;
  development mocks cannot establish production readiness.
- New formats and remote media require explicit review instead of becoming
  enabled automatically when FFmpeg gains a demuxer or protocol.
- Platform-specific confinement code and fixture coverage increase, but remain
  behind one injected supervisor profile.

## Implementation Boundary

- This accepted ADR authorizes only the exact confinement profile, protocol policy,
  demuxer mapping, readiness proof, evidence fields, and tests described here.
- It does not authorize remote roots, playlists, arbitrary FFmpeg
  capabilities, a privileged side service, a weaker development fallback in
  production, or expansion of the process supervisor beyond typed profiles.
- Any new library or FFI boundary still requires the repository's dependency and
  unsafe-code rationale in the implementation ADR.

## Validation

- Use malicious fixtures that attempt HTTP/HTTPS, TCP/UDP, Unix-socket, local
  absolute-file, parent-relative, `concat`, `subfile`, HLS, SDP, image-sequence,
  and device access. Each must fail before the resource is opened.
- Prove the accepted MP4, MKV, WebM, MPEG-TS, MOV, AVI, SRT, ASS, WebVTT, SUP,
  MicroDVD, and VobSub fixture matrix still inspects successfully.
- Verify no writable file, credential, control socket, unmanaged path, or network
  endpoint is visible to the child on every production-supported platform.
- Mutate packaged demuxer aliases and confinement attestation in tests; readiness
  and queue admission must fail closed.
- Run `just ci`, `just ui-e2e`, the complete real-media fixture suite, security
  scanning, release-image verification, and the strict Sonar gate.

## Follow-up

- Implement ADR 526 with this record and its retained-handle contract
  before enabling this profile in production.
- Prove the exact platform mechanism and immutable executable closure in the
  implementation ADR without broadening the resource policy.

## Task Record

- Motivation:
  - Record the SSRF and ambient local-resource exposure found during independent
    review before the reconstructed inspection foundation is pushed.
- Design notes:
  - Closed application-level allowlists and OS confinement are both required;
    neither is treated as a substitute for the other.
  - The recommendation distinguishes packaged capability from approved product
    surface and keeps future remote-media support behind a new decision.
- Test coverage summary:
  - No runtime test, network request, or media conversion was added or run by
    this proposal.
  - Proposal validation is documentation links, instruction drift, policy, and
    diff hygiene.
- Observability updates:
  - No telemetry changes are made by this proposal.
  - A future implementation must expose bounded confinement-unavailable,
    protocol-denied, demuxer-denied, and sandbox-violation reason codes without
    paths, URLs, command lines, or media metadata in metric labels.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md` and ADRs 452, 501, 519, 523, 524, and 526.
    No product-status claim is changed by this proposal.
- Risk & rollback plan:
  - This proposal changes no production behavior. If an accepted implementation
    regresses, rollback disables media readiness and queue admission; it must not
    restore unconstrained probing.
- Dependency rationale:
  - No dependency is added by this proposal. The implementation ADR must justify
    any OS-confinement wrapper against standard APIs, maintained safe wrappers,
    portability, auditability, and transitive supply-chain cost.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/ffi.instructions.md`, and
    `.github/instructions/devops.instructions.md`.
  - No policy drift or contradiction was found.
