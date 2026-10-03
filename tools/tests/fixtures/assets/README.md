# Historical ASSET-1 fixture inputs

These three files are exact copies of the existing deletion inventory and ADR
588 decision documents from the media integration at `13082a69`. They support
regression tests for the Python guard. Their source hashes are recorded in the
task's `artifacts/media-tooling-inputs-13082a69/source-identity.json`.

The tests reconstruct the original 213 binary files from Git's object database,
then delete them in a disposable repository. Full repository history is required;
no replacement images or recomputed inventory pins are accepted. The fixture
verifies 21,064,113 original bytes, modes, blob identities and SHA-256 digests.

Provider responses are deliberately synthetic. These copies and test successes
do not grant consent, reopen the exception or qualify an actual PR. Production
commands read the decision/inventory from the selected checkout, require its
checked head to contain the implementation, and read fresh provider history.
