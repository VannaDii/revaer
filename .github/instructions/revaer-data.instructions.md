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
- Before the ADR 522 cutover, persisted-state behavior changes are blocked from
  the frozen corpus. Complete them before candidate freeze or defer them to a
  direct pre-v1 `init.sql` edit after cutover; do not create another migration.
- If runtime database behavior changes after cutover, update the authoritative
  stored procedure layer and the Rust caller in the same change. A post-v1
  migration system requires a separately approved decision.

# Testing

- Exercise database behavior through the same stored procedure entry points that production uses.
- Keep migration and procedure tests representative of runtime call patterns.
- If a migration or procedure change affects API or CLI behavior, update the relevant docs and task record in the same change.
