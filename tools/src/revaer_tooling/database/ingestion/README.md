# Ingestion proof evidence

This package is part of the final database proof port. It validates observations
from disposable PostgreSQL sessions; it does not implement application ingestion
or approve the complete D3 qualification. The existing proof explicitly reports
that complete scope as unproven. Porting individual cases must preserve that
status until the actual qualification is established.

## Verified routine inventory

`IngestionInventory.collect` reads both native catalogs as the proof administrator,
retaining server version, all twelve reviewed helper bodies/signatures/settings,
ownership/security metadata and triggers on the eighteen ingestion tables.
Reference and final databases must be distinct. Previous inventory files are
invalidated before collection and replacements have private permissions.

The verifier requires PostgreSQL 18.6 and exactly the reviewed helper names.
Bodies and signatures must match except for the existing approved ingestion
corrections and local variable-conflict directive. The reference must retain
its original function setting; the final function must have only the directive.
It reuses the reviewed byte transformations rather than duplicating them.

A successful `VerifiedInventory` contains both retained snapshots and the
`IngestionEvidence` parser bound to the verified reference body/signature. The
collector emits its positive check only after both snapshots pass. This stage
captures ownership, security and triggers for downstream checks; it does not
claim those additional qualifications from body equality alone.

## Evidence parser

`IngestionEvidence` receives the independently verified reference routine body,
signature and runtime role. `parse` requires:

- A direct connection as the expected session/current role. Runtime superuser,
  role-creation and row-security-bypass capabilities must all be false.
- Recognized records only, one result per successful SQLSTATE, and one matching
  diagnostic per failed SQLSTATE. Duplicate JSON keys are rejected.
- Unchanged caller variable-conflict settings and a clock for each transaction.
- Exact helper-first known answers, observed before the first ingestion result.

Diagnostics must match the native error/detail/hint/context/location structure.
The function signature must match the frozen routine. Any embedded SQL statement
must occur in that body, and source line numbers must be within it. Final-runtime
line numbers account for exactly the independently checked one-line local
directive. Warnings, unexpected notices and unrelated errors fail parsing.

## Comparison normalization

`comparable` requires two complete committed-table inventories and checks every
generated public identifier as a unique UUIDv4. Each returned public identifier
must identify a committed row. It recursively replaces only those proven IDs and
the explicitly observed transaction clocks, without mutating the input. Other
values remain part of the comparison. Missing inventory, uncommitted results or
invalid identities fail rather than being normalized away.

## Session producer and transaction controls

`sessions.py` preserves the 24 named SQL fixture arguments and thirteen original
cold/helper-first/warm cases. Unknown argument names fail. Helper-first sessions
receive the fixture SQL from their owning composition; the producer does not
read files or discover configuration. An empty helper fixture fails explicitly.

Single-call sessions roll back a failed call to its savepoint. Warm sessions
commit the first transaction and observe a second transaction with its own clock
and savepoint. `SessionControls` exercises that exact producer, replacing its two
calls with writes to a temporary counter table. Three native controls require
`1,2`, `1,3`, and `1` after an injected division error. Expected errors still
require exact SQLSTATE/diagnostic framing; arbitrary failures are not accepted.

The connection and check collector are injected. Generated SQL, stdout and stderr
are retained privately under `session-control-*`. Old diagnostics are invalidated
before launch, so transport failure cannot leave prior output attached to new SQL.
These controls prove harness behavior, not application ingestion semantics.

## Isolated case runner

`IsolatedIngestion` receives the owning proof connection, validated principals,
seed SQL and verified evidence parser. The reference variant clones the reference
database and connects as postgres. The final variant clones the final database
and connects directly as the runtime role. Every clone receives explicit database
access grants and the injected seed fixture before its case runs.

The runner snapshots all eighteen ingestion tables before and after execution,
retaining empty tables and native ordering by each table's first column. Generated
SQL, stdout, stderr and table snapshots are private per-case evidence. Prior
artifacts are invalidated before a rerun. Only parsed, normalized evidence is
returned, and only after the created clone has been dropped.

Seed, transport and parser failures still trigger cleanup. A failed CREATE does
not grant ownership of an existing database with that name; the runner never
drops that collision. The outer proof continues to own container/storage cleanup.
Cleanup errors retain the earlier failure in their diagnostic. Safe case names
prevent artifact-path traversal, and the runner cannot clone onto its source.

Native tests prove equivalent reference/final normalized observations, unchanged
source data, complete snapshots, retained private artifacts and cleanup after
seed, SQL, warning and evidence failures. They also prove collision preservation.
These fixtures validate isolation; actual ingestion correctness remains a separate
application qualification.

## Approved correction comparisons

`ApprovedCorrections` recognizes only the exact ADR 588 D4 warm-commit and D5
existing-external-ID corrections. It binds the reference rejection to the frozen
statement hash, routine signature, source line, native location and complete
error metadata. It then compares the full expected final observation, including
all table effects and result flags. D5 also requires the existing-case semantic
checks. An unrecognized case or unrelated difference receives no approval.

Only the documented refresh clocks, external-ID rows, observation updates and
result changes are permitted. Missing table inventory is an error. Comparison
preserves the original numeric equality while distinguishing booleans from
numbers, avoiding Python's `True == 1` behavior. Inputs are not mutated.

The [recorded fixtures](../../../../tests/fixtures/ingestion-corrections.md) come
from the original Ruby transformations, with statement hashes verified against
the frozen migration. They are synthetic comparison inputs, not application
acceptance. Regression mutations ensure the port cannot broaden those approvals.

## Cold, helper-first and warm case composition

`IngestionMatrix` runs all thirteen original cases through the injected isolated
runner, in reference/final pairs. It uses the existing session producer, expected
SQLSTATEs and exact approved-correction matcher. Each case requires equivalent
observations or the precise approved correction, plus the required final outcome.
A failure shared by both variants is still a failure; equality alone is not success.

The first failed case stops the matrix. `ingestion-matrix.json` retains evaluated
cases and the reason, including interruption or transport failure, replacing stale
receipts. Its `matrix_completed` and `matrix_passed` fields describe only this
phase. Top-level `complete` and `passed` remain false because the original proof's
complete D3 qualification is still unproven. The full proof must preserve that
boundary even when this matrix and other individual qualifications pass.

Composition tests inject phase outcomes to verify ordering, helper selection,
shared failures, unapproved differences, transport errors, interruption and exact
D4 acceptance. The separately tested native runner owns database isolation and
parsing. Actual application matrix acceptance remains an integration requirement.

## Validation and remaining work

Native inventory tests also reject helper/version/body/signature/settings drift
and prove a live body change prevents a positive check. Tests cover role escalation/substitution, missing records, changed settings,
helper answers/order, duplicate keys and invalid committed identities. A pinned
PostgreSQL fixture produces real success and error output for reference/final
bodies and verifies equal normalized diagnostic lines. Mutated signatures,
statements, locations and out-of-range lines are rejected.

Application-specific case producers,
concurrency/native instrumentation and complete final-proof composition remain
under migration. This parser's passing tests are evidence about framing and
normalization, not application ingestion acceptance.
