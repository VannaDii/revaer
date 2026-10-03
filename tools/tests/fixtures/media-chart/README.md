# Media compliance chart fixture

This is the unchanged `charts/revaer` tree from Revaer revision
`f4b80bf76043c03091a1860c5c4b3de33ed256fe`. Its source worktree had no chart
modifications when copied on 2026-09-21. It establishes the media chart contract
while the tooling foundation still carries the earlier application chart.

`test_chart_compliance.py` preserves the Ruby suite's ten accepted and ninety
rejected cases, exercising real Helm lint, schema-only rendering and template-only
rendering. Two additional cases verify duplicate-key and YAML-alias rejection.
Hashes in renderer inputs are synthetic; no storage or release is qualified.

The regular tooling suite tests this recorded fixture. `rv compliance-chart-test`
always selects the current checkout's `charts/revaer` through the test boundary's
`REVAER_TEST_CHART` input. `rv helm-lint` invokes that task when the selected chart
has the media compliance contract. Both validation paths are needed: a reference
fixture cannot substitute for testing the actual integrated chart.

Update the fixture only from a reviewed chart revision, record its identity here,
and preserve failure cases. All files remain authored scanner inputs. Tests
write override values and schema-only copies under their owned temporary paths;
they never change the selected source chart or publish artifacts.
