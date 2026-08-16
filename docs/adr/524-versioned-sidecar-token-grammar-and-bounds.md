# Versioned sidecar token grammar and bounds

- Status: Proposed
- Date: 2026-08-16
- Operator approval: Pending

## Problem

- Accepted ADR 450 selects a versioned restricted token grammar for adjacent
  subtitle discovery, but it leaves the grammar, zero-rule meaning, limits,
  aliases, pair handling, and security failures undecided.
- Current hard-coded matching and resource constants are implementation evidence,
  not an approved persisted contract. Falling back to them when snapshot rules
  are absent would violate accepted ADRs 446 and 451.
- Filename semantics influence filesystem traversal, sidecar ownership,
  aggregate identity in proposed ADR 516, and destructive replacement under ADR
  500.

## Options

1. **Ratify the current implicit matcher.** Preserve fixed forms and constants
   without a persisted grammar version. This cannot make immutable rules
   authoritative.
2. **Adopt a closed four-form token grammar with explicit budgets.** Persist
   ordered canonical templates, compile them without a regex/glob engine, and
   fail closed on unsafe or ambiguous matching state.
3. **Adopt a configurable glob or regex subset.** This is more expressive but
   expands traversal, escaping, complexity, and compatibility obligations beyond
   the first-release need rejected by ADR 450.

## Recommendation

- Adopt option 2.
- `sidecar_grammar_version = 1` accepts exactly these canonical complete-file
  templates and no other text:
  - `{stem}.{lang}.{role}.{ext}`
  - `{stem}.{lang}.{ext}`
  - `{stem}.{role}.{ext}`
  - `{stem}.{ext}`
- Braces, tokens, and dots are literal grammar bytes. There is no escaping,
  wildcard, optional segment, path separator, recursive match, character class,
  alternation, free-form literal, or regular expression. Each template may
  appear at most once and is ordered by unique precedence 0 through 3.
- API and YAML emit the canonical template strings. Snapshot rows include grammar
  version, template enum, precedence, and enabled state; runtime never reparses
  a live profile pattern.

### Token Semantics

- Matching is anchored to the complete filename of a regular entry in the
  source media file's immediate parent directory.
- `{stem}` is the source filename without its final extension. Source and
  candidate names must be valid UTF-8 and nonempty. Matching preserves Unicode
  bytes and folds ASCII letters only; it performs no locale or Unicode
  normalization.
- `{lang}` is 2 or 3 ASCII letters. It is normalized through the versioned media
  language catalog to one lowercase ISO 639-2/T code. Unsupported codes do not
  match; they are not retained as arbitrary language text.
- `{role}` accepts these case-insensitive ASCII aliases and emits the value on
  the right:
  - `forced` or `foreign` -> `forced`
  - `commentary` or `comment` -> `commentary`
  - `sdh`, `hi`, `hearing-impaired`, or `hearing_impaired` -> `sdh`
  - `signs`, `songs`, `signs-songs`, or `signs_songs` -> `signs_songs`
  - `karaoke` -> `karaoke`
- `{ext}` accepts only `srt`, `ass`, `vtt`, `sup`, `sub`, and `idx`, with ASCII
  case folding. `.idx` represents one VobSub logical sidecar and requires the
  same basename with `.sub`. A `.sub` with a same-basename `.idx` is only that
  pair's companion; otherwise it is one standalone `sub` sidecar.
- Rules are evaluated by ascending precedence. The first enabled rule that
  parses the complete filename owns it. Two physical candidates that normalize
  to the same `(language, role, format)` identity are an aggregate error, not a
  last-wins choice.

### Zero Rules And Bounds

- A snapshot with zero enabled rules means sidecar discovery is explicitly
  disabled. The worker does not enumerate the adjacent directory for sidecars
  and does not use defaults.
- A desired target or retention policy that requires an existing sidecar while
  discovery is disabled is a compile error. Zero rows and four disabled rows
  have the same behavior but different immutable audit identity.
- A snapshot contains at most four rows and at most four enabled rows. Pattern
  storage is bounded to the canonical strings above; arbitrary 256-byte pattern
  text is not retained in the v1 contract.
- One source discovery pass is bounded to:
  - 4,096 directory entries examined;
  - 64 logical sidecars retained;
  - two physical files per logical sidecar;
  - 256 MiB total retained physical sidecar bytes;
  - 256 MiB for any one logical sidecar including a VobSub pair;
  - 10 seconds elapsed through an injected monotonic clock.
- Exceeding a count, byte, or physical-member bound, or reaching the elapsed
  deadline, fails the aggregate. Exact count, byte, and physical-member limits
  pass. Discovery does not return a partial set or silently ignore later entries.

### Filesystem Security

- Enumeration starts from an open ADR 523 source-root handle and resolves the
  source parent descriptor-relatively without following symlinks. Candidates
  are opened with no-follow, create no files, and must be regular files within
  the same parent.
- A matching symlink, directory, device, socket, unreadable entry, unstable
  identity, missing VobSub companion, or non-UTF-8 directory entry fails the
  aggregate with a stable reason. Unknown well-formed names that match no rule
  are ignored after bounded enumeration.
- Record device, inode, byte length, high-resolution modification/change
  observations, and an open-handle identity for each physical member. Proposed
  ADR 516 owns any later aggregate hash and scheduling contract; this grammar
  does not approve its pending algorithm or budgets.
- Revalidate the complete selected sidecar membership and descriptor identities
  under ADR 500's cooperative aggregate ownership immediately before mutation.
  A changed directory or member blocks replacement.

### Stable Errors

- Use stable bounded codes for `media_sidecar_grammar_version_unsupported`,
  `media_sidecar_rule_invalid`, `media_sidecar_rule_duplicate`,
  `media_sidecar_rules_required`, `media_sidecar_name_non_utf8`,
  `media_sidecar_candidate_unsafe`, `media_sidecar_companion_missing`,
  `media_sidecar_identity_ambiguous`, `media_sidecar_entry_limit`,
  `media_sidecar_count_limit`, `media_sidecar_byte_limit`,
  `media_sidecar_elapsed_limit`, and `media_sidecar_identity_changed`.
- Diagnostics identify rule ordinal, format, and reason only. They do not expose
  full paths or filenames in metrics.

## Consequences

- Persisted ordered rules have one deterministic meaning across API, YAML,
  snapshots, discovery, audit, and aggregate membership.
- The closed forms cover ordinary language, role, combined, and unlabelled
  adjacent sidecars without introducing a general pattern engine.
- Non-UTF-8 or unsafe matching directories fail closed, which can require an
  operator to rename unrelated entries before destructive processing.
- Fixed bounds provide predictable filesystem work but may reject unusually
  large subtitle sets or VobSub payloads; changing them requires a new approved
  contract rather than a hidden constant edit.

## Implementation Boundary

- This proposal authorizes no implementation while its status is `Proposed`.
- Acceptance would authorize only grammar version 1, four templates, token and
  alias semantics, zero-rule behavior, exact budgets, descriptor security,
  pair handling, ambiguity behavior, and stable errors described above.
- Accepted ADRs 446, 450, 451, and 500 remain binding. ADRs 517, 518, 521, and
  523 must carry the versioned rules without changing their meaning.
- Acceptance would not authorize glob, regex, recursion, external acquisition,
  OCR, automatic watcher or schedule activation, proposed ADR 516 aggregate
  encoding, or proposed ADR 509 unmatched actions.
- No schema, API, YAML, UI, filesystem, runtime, workflow, or generated-contract
  behavior may change before explicit decision-specific approval.

## Validation

- Proposal validation is documentation-only.
- After approval, exhaustively test all four forms, every role alias, language
  normalization, extension case, precedence order, zero and disabled rules, and
  every invalid token or extra component.
- Test exact and exceeded entry, count, physical-member, per-sidecar, aggregate-
  byte, and elapsed bounds with an injected clock.
- Add descriptor race tests for symlink swaps, replacement, deletion, inode
  reuse, VobSub companion changes, unreadable and non-UTF-8 entries, and duplicate
  semantic identities.
- Add real text, PGS, standalone SUB, and paired VobSub fixtures and prove all
  fixture media is cleaned after validation.
- An accepted implementation is not complete until focused filesystem and media
  tests, `just ci`, and `just ui-e2e` pass.

## Follow-up

- Obtain explicit operator approval before implementation or status change.
- Decide ADRs 517, 518, 521, and 523 with this record so persistence, compilation,
  API, and root traversal use one grammar version.
- If ADR 516 is later approved, import this exact membership contract without
  adopting any unapproved scheduler or hashing value by implication.

## Task Record

- Motivation:
  - Complete grammar and security details deferred by accepted ADR 450.
- Design notes:
  - A small closed parser replaces both hard-coded fallback and general-purpose
    pattern languages.
- Test coverage summary:
  - Proposal only; no parser, filesystem, schema, API, or media tests were added.
- Observability updates:
  - Future metrics use bounded format, rule ordinal, and reason only. Filenames,
    paths, source ids, and language values must not be labels.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, accepted ADRs 446-451, 484, 500, and 501,
    and proposed ADRs 507-516. No pending aggregate behavior is claimed.
- Risk & rollback plan:
  - This record can be removed with its catalogue entries. A later rollback must
    reject snapshots whose grammar version it cannot interpret exactly.
- Dependency rationale:
  - No dependency is proposed; a closed token enum and existing descriptor APIs
    are sufficient.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/revaer-data.instructions.md`, and
    `.github/instructions/devops.instructions.md`; no drift or relaxation was
    found.
