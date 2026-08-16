# Configuration RNG cleanup

- Status: Accepted
- Date: 2026-08-12
- Context:
  - Configuration token and password-salt generation used both `rand` and a direct `rand_core` dependency.
  - The workspace `rand` API already provides the required operating-system-backed random generation.
- Decision:
  - Generate alphanumeric tokens through `SampleString` and fill fixed-size salt bytes through the workspace `rand` facade.
  - Remove the redundant `rand_core` dependency and Argon2 `rand` feature while retaining Argon2 password hashing.
- Consequences:
  - Configuration randomness uses one workspace dependency surface with equivalent token and salt behavior.
  - Salt encoding is now an explicit fallible operation mapped to the existing configuration error type.
- Follow-up:
  - Keep future random generation on the workspace `rand` facade unless a narrower dependency is justified.

## Task Record

- Motivation:
  - Separate unrelated dependency cleanup from the media API deliverable so it can be reviewed and reverted independently.
- Design notes:
  - Salt length remains 16 random bytes and token length remains caller-controlled.
  - Encoding failure is propagated as `ConfigError::SecretHashFailed`; no error is suppressed.
- Test coverage summary:
  - Re-run configuration crate tests and workspace dependency checks for this child.
- Observability updates:
  - No logging, metrics, health, or event behavior changed.
- Status-doc validation:
  - Reviewed repository status documentation; no operator-facing status changed.
- Risk & rollback plan:
  - A random-generation regression would affect newly generated setup tokens or secret hashes; configuration tests cover both paths.
  - Revert this child commit without affecting either media API child.
- Dependency rationale:
  - Removes the direct `rand_core` dependency and uses the existing workspace `rand` dependency.
- Stale-policy check:
  - Reviewed `AGENTS.md` and `.github/instructions/rust.instructions.md`.
  - No policy drift or stale references were found.
