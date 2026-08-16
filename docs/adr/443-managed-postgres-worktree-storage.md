# Managed PostgreSQL Worktree Storage

- Status: Accepted
- Date: 2026-08-13
- Operator approval: Approved as written by the operator on 2026-08-15: "I approve the ADRs as they are now and I'm resuming your goal."
- Context:
  - Linked worktrees shared the default `revaer-db` container while deriving its bind-mounted data directory from the invoking worktree.
  - Removing a completed worktree could therefore delete the live PostgreSQL data directory and crash unrelated validation with missing relation and global control files.
- Decision:
  - Derive the default managed data directory from Git's stable common repository directory so every linked worktree selects the same non-temporary storage root.
  - Inspect an existing managed container's PostgreSQL data mount before reuse. Fail closed when a running container points elsewhere, and recreate it only when it is stopped.
  - Apply mount ownership enforcement only when managed mode is explicit; an unmanaged external database selection never authorizes container replacement.
  - Prove the container accepts PostgreSQL connections before external probes or schema commands; a bounded readiness timeout is terminal.
  - Preserve `PG_CONTAINER`, the local database port contract, and `REVAER_DB_DATA_DIR` for explicitly isolated validation environments.
- Consequences:
  - Removing a temporary worktree no longer removes the shared managed database's live storage.
  - Existing stopped containers with worktree-local mounts are recreated on their next managed startup.
  - Parallel worktrees that need independent databases must continue to select independent container names, ports, and data directories.
- Follow-up:
  - Apply the operator approval recorded on 2026-08-15 when advancing and
    pushing the implementation.
  - Keep the mount-ownership check aligned with the managed database lifecycle recipe.
  - Retain explicit isolated database settings for genuinely concurrent database-backed gates.

## Task Record

- Motivation:
  - Restore deterministic local validation after a completed worktree cleanup invalidated the shared PostgreSQL container's bind mount.
- Design notes:
  - The Git common directory identifies the primary repository independently of the current linked worktree.
  - Running containers are never stopped or replaced automatically on a mount mismatch.
- Test coverage summary:
  - Exercise `just db-start` against the stopped legacy worktree mount and verify the container is recreated with stable storage.
  - Run the minimal-feature database-backed gate, `just ci`, and `just ui-e2e`.
- Observability updates:
  - Startup reports the expected and actual mount paths when ownership differs and explains the isolated-container alternative.
  - Readiness timeout errors identify the managed container that failed to initialize.
- Status-doc validation:
  - No product status surface changes; ADR indexes are updated.
- Risk & rollback plan:
  - Path derivation failure blocks managed startup instead of selecting ambiguous storage. Rollback restores the prior worktree-local default after stopping the managed container.
- Dependency rationale:
  - No dependencies are added; the recipe uses Git, Docker, and POSIX shell tools already required by local validation.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and `.github/instructions/devops.instructions.md`.
  - Drift was found in the worktree-storage lifecycle contract and corrected in the Rust instruction file; no contradictory references were retained.
