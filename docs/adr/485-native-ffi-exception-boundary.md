# Native FFI exception boundary

- Status: Accepted
- Date: 2026-08-15
- Operator approval: Recommendation approved wholesale by the operator on
  2026-08-15: "I approve the ADRs as they are now and I'm resuming your goal."
- Context:
  - Sonar reports twelve generic `std::exception` catches in the authored
    libtorrent shim. Simply narrowing or deleting those catches can allow a C++
    exception to cross the CXX boundary or terminate the process.
  - Repository policy requires deterministic failure translation, prohibits
    hiding the findings, and permits broad native catches only when the ABI truly
    requires them and the contract is documented and tested.
  - Libtorrent provides error-code overloads for many operations, while authored
    validation and conversion can return explicit errors instead of throwing.
- Decision:
  - Recommended option: make the shim error-code-first and exception-specific.
    Use nonthrowing libtorrent overloads wherever available; make authored
    parsing, validation, and conversion return explicit native result values; and
    catch only the concrete exception families remaining at the smallest call
    site that can translate them.
  - CXX-exposed methods return an explicit success/error result contract. They
    become `noexcept` only after every operation in that method is nonthrowing or
    has a tested, typed translation. Allocation failure is translated separately
    as resource exhaustion rather than folded into an opaque runtime error.
  - Add test-only native throw adapters for every retained typed exception path
    and prove the Rust caller receives the stable error without unwind, process
    abort, or partially applied session state.
  - Treat any remaining third-party call without a nonthrowing or documented
    typed-exception contract as unresolved. Stop at that call rather than adding
    a generic catch or accepting a Sonar disposition.
  - Alternative considered: retain generic catches and mark the Sonar findings
    safe or accepted. Rejected because operator policy forbids using issue
    disposition or criteria relaxation to manufacture a pass.
  - Alternative considered: remove the catches and rely on `noexcept` termination.
    Rejected because process termination is not recoverable failure translation.
  - Alternative considered: isolate libtorrent in a supervised sidecar process.
    This provides a process fault boundary but is rejected for now as a much
    larger deployment, IPC, and lifecycle architecture change.
- Consequences:
  - Native failures become stable, inspectable domain errors and all twelve
    generic-catch findings can be fixed without a suppression.
  - Some libtorrent calls may need API-specific wrappers, increasing native test
    surface and compatibility work across supported libtorrent versions.
  - The implementation cannot complete if a required library operation exposes
    only an undocumented exception contract; that case returns for operator
    review with exact evidence.
- Follow-up:
  - Inventory each generic catch by protected operation and supported libtorrent
    version before editing behavior.
  - Convert error-code-capable operations first, then authored throws, then the
    remaining typed library exceptions.
  - Run native tests for every supported ABI branch, FFI failure translation,
    full CI, media-independent torrent tests, and strict Sonar analysis.

## Task Record

- Motivation:
  - Resolve native Sonar findings without weakening either ABI safety or scanner
    policy.
- Design notes:
  - The recommendation changes failure plumbing, not torrent behavior or the
    public Rust domain model.
- Test coverage summary:
  - Proposal only. Native throw-injection and cross-version integration tests are
    mandatory before implementation handoff.
- Observability updates:
  - Emit stable failure codes at the Rust boundary and log the originating native
    error once without credentials or raw exception type leakage.
- Status-doc validation:
  - Updated the ADR index and documentation summary. Runtime claims are unchanged.
- Risk & rollback plan:
  - Migrate one protected operation at a time. Revert that operation if its typed
    contract is incomplete; do not restore a generic catch as a passing shortcut.
- Dependency rationale:
  - No dependency is proposed.
- Stale-policy check:
  - Reviewed `AGENTS.md`, Rust/FFI instructions, current native catches, and the
    unresolved Sonar rules.
  - No criteria relaxation is proposed; the missing decision is the native
    failure-translation contract itself.
