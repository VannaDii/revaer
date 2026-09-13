---
applyTo:
  - "crates/revaer-data/**"
  - "crates/**/migrations/**"
  - "config/database-rebaseline.env"
  - "scripts/dev-seed.sql"
---

`AGENTS.md` and `rust.instructions.md` apply first. This file specializes database and migration work.

# Database Rules

- Runtime application code must call stored procedures for database behavior. Do not embed inline business SQL in Rust.
- `just lint` mechanically enforces that `sqlx::query*` usage stays confined to `crates/revaer-data/src`, except for the disposable test-database provisioning helper at `crates/revaer-test-support/src/postgres.rs` and its integration test. Inline DDL/DML text must not appear in authored Rust.
- Raw DDL, DML, stored procedure bodies, and seed SQL belong in migrations or tightly scoped operational bootstrap scripts only.
- `JSONB` and related conglomerate persistence formats are banned for application state.
- Shared behavior lives in shared stored procedures. Do not duplicate the same database behavior across multiple crates.
- Use named bind parameters and explicit transactions where a multi-step change must be atomic.

# Migration Rules

- During ADR 522's freeze and assembly phases, the exact numbered migration
  corpus pinned by `config/database-rebaseline.env` remains the sole bootstrap
  authority. Do not add, edit, rename, or remove a migration after the freeze.
- `crates/revaer-data/init.sql` must remain absent during the freeze phase. A
  later assembly-phase prefix is review evidence only and must not be selected
  by runtime, development, or ordinary tests before the accepted cutover step.
- Run the canonical database rebaseline guard before schema handoff. Candidate
  output belongs only under the ignored `target/database-rebaseline/` evidence
  directory and must match the pinned digest and statement count.
- ADR 551 finalization retains the exact frozen 1,624-statement candidate as
  transition evidence. Final SQL may differ only by its approved header,
  three dump timeout-reset removals, ADR 569's exact approved D2/D3 routine
  substitutions, ADR 588's exact approved D4/D5 ingestion-family corrections,
  and reviewed lifecycle/security/grant
  sections. `just db-init-final-proof` must compare the exact legacy bytes and
  independently pinned final bytes, then test constrained owner/runtime roles
  in disposable PostgreSQL. Finalization is not runtime cutover: ordinary
  application and test bootstrap must still use the frozen migrations.
- Finalization proof must distinguish authorized parity exceptions from local
  prototypes. The `plpgsql.variable_conflict` substitution recorded in ADR 569
  D3 was conditionally approved on 2026-09-10, not certified by G1 or a
  normalized-reference comparison. Do not publish, embed, or activate that
  candidate before the required independent cold/warm semantic proof is
  recorded. ADR 569 D1's exact pinned stock-extension inventory/definition/ACL
  boundary and D2's function-scoped five-second reset bound are also approved;
  no additional extension grant, timeout increase or frozen-migration edit is
  authorized. Preserve their mutation, failure-path and expiry requirements.
- ADR 588 choice 1 explicitly approved D4/D5 on 2026-09-11: add `ON COMMIT DROP`
  only to ingestion's `tmp_policy_rules` and the existing non-null partial-index
  predicates only to IMDb/TMDB/TVDB ingestion conflicts. Update exact generator,
  final digest and mutation guards together. Do not change canonical merge,
  indexes, frozen migrations, callers or same-transaction semantics. D3's
  conditional proof stays fail-closed and retains the frozen counterexamples;
  approval is not permission to normalize those failures into a passing proof.
- Before the ADR 522 cutover, persisted-state behavior changes are blocked from
  the frozen corpus. Complete them before candidate freeze or defer them to a
  direct pre-v1 `init.sql` edit after cutover; do not create another migration.
- If runtime database behavior changes after cutover, update the authoritative
  stored procedure layer and the Rust caller in the same change. A post-v1
  migration system requires a separately approved decision.

# Testing

- ADR 569/588 transition qualification must exercise the actual Rust ingestion
  wrapper through an explicitly provisioned disposable `PgPool`, retaining
  backend/role identity, cold and restored compiler settings, typed SQLSTATE,
  message/detail, committed reuse, error recovery and persisted identities.
  The dedicated pool-probe recipe requires an explicit input file and a new
  report; ordinary Rust runs exercise its launch guard and must never select
  the final candidate as application or ordinary-test bootstrap authority.
  Preserve the frozen D4 failure separately from the approved final success.
  Passing this bounded pool case does not close D3, cancellation or native scope.
- Server-cancellation qualification must use the actual Rust wrapper and an
  observed owned lock wait, not an idle-backend cancellation. Require exact
  SQLSTATE/message/detail, all 18 unchanged rollback images, restored caller
  settings, and successful same-pool recovery. Publish the checkpoint atomically
  before releasing the lock. This does not prove client-future cancellation,
  every interruption site or complete conditional D3.
- The cancellation probe's explicit cache state is mandatory. Retain cold
  coverage and a real committed warm-up on the same single-connection pool,
  with distinct source identity. Publish readiness before controller locking,
  accept only its exact backend/database/cache-state start signal, and retain
  the prepared state through cancellation. Frozen warm recovery keeps D4's
  exact `42P07` and rollback; final recovery success is a named correction,
  not equivalent frozen behavior or full D3 acceptance.
- Runtime trust-rank qualification must reuse the ingestion backend while a
  separate direct writer commits the rank changes. Cover both directions across
  the existing confidence thresholds, preserve all unrelated read/write images,
  reject duplicate writer JSON fields and retain the exact frozen D4 failure.
  Setup-only rank variation is not runtime-change evidence.
- Exercise database behavior through the same stored procedure entry points that production uses.
- Keep migration and procedure tests representative of runtime call patterns.
- If a migration or procedure change affects API or CLI behavior, update the relevant docs and task record in the same change.
