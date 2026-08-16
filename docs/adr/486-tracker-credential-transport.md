# Tracker credential transport

- Status: Accepted
- Date: 2026-08-15
- Operator approval: Recommendation approved wholesale by the operator on
  2026-08-15: "I approve the ADRs as they are now and I'm resuming your goal."
- Context:
  - The native shim currently injects configured Basic-auth credentials into
    both `http://` and `https://` tracker URLs. Credentials sent to a plaintext
    HTTP tracker can be observed or modified in transit, and credential-bearing
    URLs can be reflected by tracker alerts.
  - HTTP and UDP trackers remain common enough that globally banning those
    schemes would be a separate compatibility policy. The security requirement
    is narrower: authentication must not silently downgrade to plaintext or an
    unsupported scheme.
  - Tracker URLs and authentication fields are separate inputs, so transport
    policy can fail before a credential-bearing URL reaches libtorrent.
- Decision:
  - Recommended option: allow unauthenticated HTTP, HTTPS, and UDP trackers, but
    permit username/password authentication only for HTTPS trackers.
  - Reject authentication for HTTP, UDP, unknown schemes, or malformed URLs with
    a stable validation error. Do not auto-upgrade HTTP to HTTPS because endpoint
    identity and capability cannot be inferred safely.
  - Reject embedded URL userinfo in configured tracker URLs. Credentials must use
    the dedicated secret input and must never be persisted or returned in a
    tracker collection.
  - If libtorrent requires temporary HTTPS userinfo internally, construct it only
    at the final call boundary. Sanitize tracker URLs from alerts, status events,
    logs, audit rows, and error messages before they leave the native shim.
  - Keep cookie or tracker-id behavior separate from HTTP Basic auth; validate its
    supported transport explicitly and never append secret values to a URL.
  - Alternative considered: ban all non-HTTPS trackers. Rejected as a broader
    compatibility change than the detected credential risk requires.
  - Alternative considered: auto-upgrade authenticated HTTP URLs. Rejected
    because the HTTPS endpoint may not exist or may identify a different service.
  - Alternative considered: retain authenticated HTTP behind a warning. Rejected
    because warnings do not protect credentials and fail the repository's
    fail-closed security posture.
- Consequences:
  - Credentials cannot be sent over plaintext tracker transport or leaked through
    emitted URLs.
  - Existing users of authenticated HTTP trackers must move those trackers to
    HTTPS or remove authentication before the configuration is accepted.
  - Unauthenticated legacy trackers continue to work.
- Follow-up:
  - Add parser tests for userinfo, schemes, malformed URLs, credential combinations,
    and sanitized alert output.
  - Add native and Rust integration tests proving rejected configuration leaves
    existing tracker state unchanged.
  - Document the compatibility change in operator configuration and release notes.

## Task Record

- Motivation:
  - Resolve the tracker transport hotspot with an explicit operator security and
    compatibility decision.
- Design notes:
  - Validation occurs before mutation and uses dedicated auth fields rather than
    treating secret-bearing URLs as configuration values.
- Test coverage summary:
  - Proposal only. Implementation requires native parser/mutation tests, Rust
    error-mapping tests, and sanitized event assertions.
- Observability updates:
  - Add a stable rejected-transport reason without logging tracker credentials.
- Status-doc validation:
  - Updated the ADR index and documentation summary. Current behavior remains
    documented until approval and implementation.
- Risk & rollback plan:
  - Roll back by reverting the validation change only after a replacement secure
    transport policy is explicitly approved; never restore plaintext credential
    injection as an emergency bypass.
- Dependency rationale:
  - No dependency is proposed. Use the existing native URL and settings surface.
- Stale-policy check:
  - Reviewed `AGENTS.md`, FFI instructions, the tracker-auth implementation, and
    the open Sonar hotspot.
  - Drift found: the current implementation treats authenticated HTTP and HTTPS
    as equivalent despite different confidentiality guarantees.
