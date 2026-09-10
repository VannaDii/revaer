# Root input contract validation

- Status: Recorded
- Date: 2026-09-10
- Operator approval: Not applicable: nonarchitectural task record
- Context: ADR 557 is accepted, and its outside-in sequence starts with API
  contracts before the coordinated root-persistence cutover. The legacy media
  key grammar allows underscores and different limits; reusing it would violate
  the accepted root-key contract.
- Decision: Implement the exact accepted key, candidate-path, association-prefix,
  page-limit, and catalog-cursor grammar in the existing API-models crate. This
  record makes no new architectural choice or change to a public JSON shape.
- Consequences: Callers can validate these inputs without silently normalizing
  them. No new route, persisted state, filesystem authority, or production
  provider is installed. SQL membership and descriptor confinement remain
  separate required checks, not guarantees furnished by this library.
- Follow-up: Integrate the helpers into the approved outside-in HTTP workflow
  during its coordinated cutover, with real stored-procedure and E2E evidence.
  ADR 569, E1, and all other retained approval/evidence holds remain unchanged.

## Task Record

- Motivation: Establish the precise request grammar for the approved operator
  workflow while final-init parity decisions prevent the database cutover.
- Design notes: Logical keys preserve the exact 1-64 ASCII byte grammar.
  Candidate paths preserve UTF-8 bytes with the specified 4,096-byte bound and
  forbidden components/separators. Only a present association prefix may be
  empty. There is no trimming, Unicode normalization, filesystem lookup, default
  prefix, or working-directory interpretation. Catalog limits default to 50 and
  otherwise accept 1-200. HTTP extraction must independently reject absent/null
  required fields, malformed numeric queries, unknown fields, and offset input.
- Cursor notes: Encode u16 big-endian key length, exact key bytes, and 16 UUID
  bytes as canonical unpadded URL-safe Base64. Decode at most 110 encoded bytes
  into an 82-byte stack buffer before allocating the bounded owned key. Reject
  malformed, padded, noncanonical, trailing, old-shape, and invalid-key tokens.
  Nil and unknown UUIDs are syntactically representable; active-catalog membership
  and resource-specific SQL rejection are not invented by the codec. Debug and
  error output contain neither token nor key/UUID.
- Test coverage summary: Independent boundary, malformed-input, and literal
  cursor-vector tests are part of this change. Focused validation runs through
  `just test-media-root-contract`; integrated gate results are recorded below
  after execution. Passing a syntax test is not HTTP, SQL, or filesystem proof.
- Observability updates: No logs, metrics, health, SSE, or HTTP error mapping
  change. The new error is a closed constant without rejected input.
- Status-doc validation: Reviewed ADR 557's outside-in sequence and ADR 564's
  completion ledger. The service remains incomplete and the legacy HTTP profile
  creation failure remains open. No operator guide or OpenAPI claim is advanced.
- Risk & rollback plan: Grammar drift could reject an approved spelling or
  admit a noncanonical one. Independent vectors and exact boundary tests check
  this surface. Revert this isolated module and its recipe/docs if necessary;
  no database, media, deployment, or remote state needs restoration.
- Dependency rationale: No new dependencies. Reuse existing `base64`, `uuid`,
  and standard-library APIs. Keep HTTP helpers in the existing shared model
  crate without making the media runtime depend on an HTTP-layer crate.
- Stale-policy check: Reviewed root, Rust, devops, data, and Sonar instructions.
  Added the focused recipe contract to devops instructions with no change to
  required gates, thresholds, exclusions, frozen SQL, or approval boundaries.
  No contradictory policy was removed. Updated both ADR indexes and generated
  documentation in the same change.

## Validation Evidence

- The independently authored tests from worker commit `74eab718` pass on the
  combined implementation: 21 tests, including five literal Ruby-derived cursor
  vectors. A parent-added cursor-debug check extends the focused suite to 22
  tests. This suite does not use the encoder to derive its known-answer values.
- A separate scoped source review found no actionable issue in the helper
  implementation. It specifically retained HTTP extraction, SQL membership,
  resource separation, and descriptor confinement as unproven integration work.
  This is not a completed repository security scan or a Copilot review.
- `just fmt` and the full two-pass `just lint` passed on the production module;
  Clippy's missing-const finding was corrected without a suppression. Both
  required full gates will run on the combined committed revision. Their
  pending results are not passes.
- `just docs-index docs-link-check instruction-drift` passed with 1,089 links
  checked and zero link errors before the final validation-result update.
- `just --command sonar verify --file
  crates/revaer-api-models/src/media_root_contract.rs --project VannaDii_Revaer`
  returned the organization-entitlement HTTP 403 for Agentic Analysis. It did
  not analyze the code. The authoritative scanner and published-coverage
  requirements remain separate, mandatory gates.
