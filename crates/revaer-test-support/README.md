# PostgreSQL test fixtures

## Empty databases

`start_postgres` and `start_postgres_at` allocate a uniquely named database on an
explicit test PostgreSQL service. They keep the database empty for tests of
uninitialized state and other bootstrap rejection paths. Existing databases are
never reset. Supply an administrative test endpoint through
`REVAER_TEST_DATABASE_URL` or `DATABASE_URL`.

## Initialized runtime databases

Call `TestDatabase::initialize_runtime` explicitly with the complete canonical
initializer when the test needs a running application. The helper checks the
actual database identity before setup, creates isolated owner/runtime roles,
applies the supplied SQL and seals its exact UTF-8 SHA-256 digest in one
transaction, and disables the owner login. It verifies the runtime database and
role before exposing that connection string. No caller-provided seal or
production baseline bypass is used.

The helper does not bundle a second initializer. Media callers include the
selected checkout's `crates/revaer-data/init.sql`; raw-fixture tests stay raw.
Credentials are omitted from the handle's Debug representation.

## Cleanup and evidence

Use `close` after collecting the test result so cleanup errors can fail the test.
Drop also attempts cleanup after early returns or initialization failure and
reports errors. Only the owned database and its confirmed fixture roles are
removed. Empty/repeated initialization is rejected. Integration checks cover
failed initialization and both explicit and Drop cleanup; the media integration
additionally checks the full initializer, restricted permissions and disabled
owner, then runs the real app launch/compliance/bootstrap regressions.

These fixture checks do not replace whole-application CI, API/browser acceptance,
coverage, Sonar, or package qualification.
