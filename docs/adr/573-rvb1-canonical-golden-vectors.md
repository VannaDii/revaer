# RVB1 canonical golden vectors

- Status: Recorded
- Date: 2026-09-10
- Operator approval: Not applicable: nonarchitectural task record
- Context:
  - The parent owns the pure bounded RVB1 codec and its consuming tests. This
    independent deliverable fixes literal expected frames directly from
    [ADR 558 Annex A](558-rvb1-native-process-broker-wire-contract.md#annex-a-exact-codec-draft-for-review),
    without reading, importing, or executing the parent codec.
  - The parent agent assigned this worktree, branch, fixture path, and ADR 573
    under the operator's authorization to parallelize approved work.
    ADR 558's resolution records approval of B1-B3
    and ADR 559 G1; this record neither grants nor expands that approval.
    E1 remains held. No environment values or package policy are selected.
- Decision:
  - Record 25 canonical positive frames and structured expected values in
    `crates/revaer-media-runtime/tests/fixtures/rvb1-vectors.json`.
    Use independent Ruby standard-library packing only to calculate the
    initial literals; author the checked-in file with `apply_patch`.
  - Do not derive goldens from production encoding or regenerate expectations
    during a codec test. The parent must compare both encode and decode
    behavior against these literal bytes.
- Consequences:
  - A matching encoder and decoder cannot hide the same layout error by merely
    round-tripping each other.
  - Small positive vectors do not prove allocation limits, rejection behavior,
    deadline enforcement, process containment, or executable authority.
- Follow-up:
  - The parent owns consuming tests, malformed/boundary coverage, canonical
    validation recipes, full `just ci`, `just ui-e2e`, and repository Sonar
    acceptance. This fixture subtask does not claim those gates passed.
  - ADRs 571 and 572 remain reserved to their respective parent/package tasks.

## Fixture Schema And Cases

The root object has `version: 1` and `vectors`. Each vector has `name`, `kind`,
`lane`, `frame_hex`, and `fields`. Kind and lane are the Annex A names, not Rust
discriminants. Every `_hex` byte field uses lowercase hexadecimal; `args_hex`
and `evidence_items_hex` are ordered arrays of byte strings. Empty byte strings
are `""`, while no arguments or evidence items are `[]`. All `u64` fields use
decimal JSON strings, including small values, so consumers never lose precision.
Narrow integer fields are JSON numbers. `protocol_version` is always 1.
Response `flags` is the literal byte; bit 0 denotes truncated secondary evidence.

- Both lanes have hello and ack with nonce bytes `00` through `0f`. Ack PID and
  PGID are 1234, executable length is `"123456"`, and executable SHA-256 is
  the synthetic 32-byte pattern `00` through `1f`.
- Each lane has a request for program bytes `/bin/tool` with ID `"1"`, deadline
  `"1"` millisecond, no arguments, and both stream limits zero.
- Each lane also has ID `"18446744073709551615"`, deadline `"30000"` for control
  or `"86400000"` for job, and both stream limits 16,777,216. Its arguments
  are one empty byte string and `61fffe62`, which is non-UTF-8 and contains no
  NUL. These are the existing lane ceilings, not new timeout policy.
- Every request carries the distinct synthetic native-tool identity SHA-256
  pattern `20` through `3f`. Neither digest pattern certifies a real binary or
  the ADR 519 execution closure. `/bin/tool` is never opened or executed.
- Both lanes have all eight status values from success through
  supervision_failed, with ID `"1"` and flag zero. Success carries complete
  binary stdout `00017f80ff0a` and stderr `ff0041fe0a`, no evidence items, and
  the canonical four-byte empty evidence field.
- Each failure has empty stdout, an ASCII detail equal to its status name with
  underscores replaced by spaces, and three unique UTF-8 evidence items in
  unsigned byte order: `cleanup: reaped`, `read: \u00e9chec`, and
  `recovery: stopped`. The escape here denotes U+00E9; the fixture stores its
  actual UTF-8 bytes as hex, not a backslash escape on the wire.
- One additional job supervision_failed frame uses the maximum ID, flag 1,
  bounded detail, and only `cleanup: reaped` as retained evidence. It represents
  the canonical flag encoding after upstream omission, not proof that a real
  evidence producer reached its budget. Omitted evidence is not transmitted.

These are independent frames, not a broker-lifetime transcript. Maximum IDs do
not authorize skipping identifiers, and a response is only valid in a matching
active-request context with sufficient stream limits and unexpired authority.

## Independent Layout Math

All offsets below are payload-relative; add 12 for frame-relative offsets.

| Kind | Independent derivation | Payload bytes | Frame bytes |
| --- | --- | ---: | ---: |
| hello | `2 + 1 + 1 + 16` | 20 | 32 |
| hello_ack | `20 + 4 + 4 + 8 + 32` | 68 | 80 |
| request, no args | `60 + 4 + 9 + 4` | 77 | 89 |
| request, two args | `77 + (4 + 0) + (4 + 4)` | 89 | 101 |
| success | `28 + 6 + 5 + 4` | 43 | 55 |
| ordinary failure | `28 + 0 + detail_bytes + 60` | `88 + detail_bytes` | `100 + detail_bytes` |
| truncated failure | `28 + 0 + 18 + 23` | 69 | 81 |

The ordinary evidence field is `4 + (4 + 15) + (4 + 12) + (4 + 17) = 60`
bytes. Its count is 3, below `floor((60 - 4) / 5) = 11`. The truncated
field is `4 + 4 + 15 = 23`, with count 1 below the derived maximum 3.

The header is `RVB1`, one kind byte, three zero bytes, then a `u32` payload
length at offset 8. Request offsets are version/lane/reserved at 0/2/3,
ID at 4, deadline at 12, stream limits at 20/24, identity at 28, program
length at 60, and program at 64. Ack PID/PGID/length/digest are at
20/24/28/36. Response status is at 3, ID at 4, flags at 12, three reserved
zeros at 13, and stdout/detail/evidence lengths at 16/20/24. Streams and
evidence start at 28 with no padding.

The original calculation command was `just --command ruby -rjson -` with
a quoted `RUBY` stdin heredoc. Its independent packing directives were:

```ruby
header = "RVB1".b + [kind, 0, 0, 0, payload.bytesize].pack("C4N")
hello = [1, lane, 0].pack("nCC") + nonce
ack = hello + [1234, 1234, 123456].pack("NNQ>") + executable_digest
request_prefix = [1, lane, 0, id, ms, limit, limit].pack("nCCQ>Q>NN")
response_prefix = [1, lane, status, id, flags, 0, 0, 0,
                   stdout.bytesize, detail.bytesize, evidence.bytesize]
                  .pack("nCCQ>C4N3")
```

Program/argument/item lengths and argument/item counts use `pack("N")`.
The request digest immediately follows its fixed prefix. Literal hex comes
from `unpack1("H*")`; the final root object uses `JSON.pretty_generate`.
No repository recipe, production codec, or additional dependency is used by
this one-off calculation. The full command and independent verification
transcript are retained outside the worktree for the parent handoff.

## Task Record

- Motivation:
  - Supply independent interoperability expectations for the parent's codec.
- Design notes:
  - Preserve Annex A assignments, exact bounds, byte semantics, sorted
    evidence, status consistency, and the separate E1 hold. No architecture,
    production source, dependency, recipe, or remote state changes.
- Test coverage summary:
  - All 25 frames passed the independent fixed-offset reader and metadata/case
    coverage checks below. Full CI/UI and parent-code integration are not run.
- Observability updates:
  - None. Fixture bytes are synthetic test data, not logs or runtime telemetry.
- Status-doc validation:
  - Reviewed `README.md`, `MEDIA_TRANSCODING.md`, and ADR 559 approval/readiness
    boundaries. No product readiness claim changes. Update the ADR index,
    mdBook summary, and generated document catalogs only.
- Risk & rollback plan:
  - The primary risk is a shared mistaken reading of the annex; retain
    independent offset/length checks and require the parent to test literal
    bytes. Revert this fixture and its ADR/catalog additions together if
    defective. No native process, database, deployment, or package rollback.
- Dependency rationale:
  - None added. Ruby packing and JSON are standard-library tools used only
    for independent fixture arithmetic; Rust remains untouched.
- Stale-policy check:
  - Reviewed `AGENTS.md`, Rust and Sonar scoped instructions, the ADR template,
    and ADRs 501, 554, 558, and 559. No contradiction or stale policy requiring
    edits was found in this bounded fixture scope. Annex A's historical draft
    wording remains subordinate to the existing approval resolution. No
    instruction, Sonar criteria, workflow, or quality gate is relaxed.

## Validation Record

- `just --command ruby -rjson -rdigest -` with the independent validation
  heredoc: passed all 25 frames. It reads unsigned integers by byte-folding
  at Annex A offsets, not by the generation packing directives. Checks cover
  exact magic/kind/lane/version, header and payload lengths, reserved bytes,
  all metadata, request limits/IDs/deadlines/argument cases, complete binary
  success streams, failure consistency, UTF-8/unique/sorted evidence,
  count/item derived bounds, and both-lane status coverage.
- Fixture SHA-256:
  `2b86ddd73a62391dc1739274888ce76eea02bd4d0c9aa25c5fee92926cbcc9b0`.
- `just --command sonar verify --file crates/revaer-media-runtime/tests/fixtures/rvb1-vectors.json --project VannaDii_Revaer`:
  blocked with HTTP 403 because Agentic/Vortex Analysis is not available for
  this organization. No quality-analysis pass is claimed.
- `just --command sonar analyze secrets` against the complete fixture,
  ADR 573, ADR index, mdBook summary, and both generated JSON catalogs:
  completed successfully with no findings reported. This is secrets analysis,
  not JSON quality analysis or the repository Sonar gate; JSON was not routed
  to another language.
- `just docs-index`: passed and generated 512 catalog entries. The only
  catalog changes are ADR 573's entry and the manifest generation timestamp.
- `just instruction-drift`: passed without output beyond its recipe command.
- `just clean-test-fixtures`: passed. No native tool or media conversion was
  run and no test media was acquired.
- `git diff --check`: passed after catalog generation.
- Full `just ci` and `just ui-e2e` are not run by this subtask. The operator
  delegated subtask leaves those gates with the parent. This record is not repository
  completion or production-readiness evidence.
