---
applyTo:
  - "crates/revaer-data/**"
  - "crates/**/init/**"
  - "crates/**/migrations/**"
  - "scripts/dev-seed.sql"
---

`AGENTS.md` and `rust.instructions.md` apply first. This file specializes database and schema work.

# Database Rules

- Runtime application code must call stored procedures for database behavior. Do not embed inline business SQL in Rust.
- `just lint` mechanically enforces that `sqlx::query*` usage stays confined to `crates/revaer-data/src`, except for the disposable test-database provisioning helper at `crates/revaer-test-support/src/postgres.rs` and its integration test. Inline DDL/DML text must not appear in authored Rust.
- Raw DDL, DML, stored procedure bodies, and seed SQL belong in the canonical schema initialization script, post-v1 migrations, or tightly scoped operational bootstrap scripts only.
- `JSONB` and related conglomerate persistence formats are banned for application state.
- Shared behavior lives in shared stored procedures. Do not duplicate the same database behavior across multiple crates.
- Use named bind parameters and explicit transactions where a multi-step change must be atomic.

# Schema Lifecycle Rules

- Before the first stable v1 release, `crates/revaer-data/init/0001_init.sql` is the only active schema file. Every persisted-state change updates that deterministic clean-install script directly; incremental migration files are prohibited.
- The v1 release establishes the durable baseline. Only changes after that baseline may add sequential, deterministic migrations that are safe to replay from the released v1 schema.
- The initialization script and post-v1 migrations must pin authored routine search paths, remain deterministic, and fail closed in clean environments.
- If the runtime behavior changes, update the stored procedure layer and the Rust caller in the same change.

# Testing

- Exercise database behavior through the same stored procedure entry points that production uses.
- Keep schema and procedure tests representative of runtime call patterns.
- If a schema or procedure change affects API or CLI behavior, update the relevant docs and task record in the same change.
