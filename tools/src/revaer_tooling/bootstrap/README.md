# Bootstrap ownership and measurement

`setup.sh` is the Bash entry point. It changes to its own checkout, installs uv
through the official versioned installer when needed, and delegates environment
creation to `uv sync --locked`. It then invokes `rv setup` through `uv run`.
Python provisioning, dependency resolution, lockfiles and launcher installation
remain owned by uv. The script has no custom platform downloads or environment
builder. See [getting started](../../../README.md#getting-started).

## Official uv options

The bootstrap forwards uv's supported installer environment options. In
particular, `UV_INSTALL_DIR` selects the binary directory and takes precedence
when it is also supplied with `UV_UNMANAGED_INSTALL`. Unmanaged installation
prevents shell-profile changes and self-updates. The script adds the selected
directory to its own PATH so setup can continue immediately.

Normal setup includes every configured dependency group. Container build stages
can select the core CLI with uv's `--no-default-groups` option; they use the same
lockfile and uv-managed interpreter. This avoids installing browser and release
engines where those commands are not used. Group ownership and audit behavior
are described in the [tooling guide](../../../README.md#dependency-groups).

An activated environment or project override from another checkout must not
redirect initialization. The script clears those selectors before the locked
sync and run. After initialization, the installed launcher selects the current
checkout; see the [launcher guide](../../../launcher/README.md).

## Architecture

| Component | Responsibility |
| --- | --- |
| `tasks/setup.py` | Static setup task: verify uv, provision the selected profile, and install the launcher through uv. |
| `external/bash.py` | Typed bootstrap arguments, explicit Bash/kcov executables, exact kcov version and failing diagnostics. |
| `external/python.py` | Invoke the native child through uv's supported `UV_RUN_RLIMIT_NOFILE` option. |
| `coverage.py` | Validate source identity, expected outcomes and complete native records across the required setup cases. |
| `tasks/bootstrap.py` | Run the measured bootstrap tests under an operation lock and publish the generic coverage input after validation. |
| `install.py` / `external/cmake.py` | Verify the existing source pin, build/install with CMake, and check cached native files. |

uv owns the child process limit through its documented
[`UV_RUN_RLIMIT_NOFILE`](https://docs.astral.sh/uv/reference/environment/#uv_run_rlimit_nofile)
option. Kcov derives its trace descriptor from the soft limit, so the adapter
requests at most 4096 descriptors, preserves a lower inherited limit, and rejects
values below 16. uv changes the soft limit immediately before launching kcov;
the parent and hard limits remain unchanged. Explicit directory, project and
interpreter options prevent foreign uv selectors from redirecting this native
command. `--no-project --no-config` avoids another dependency synchronization.
Kcov uses its documented `--bash-method=DEBUG` interface, which clears
the helper's `BASH_ENV` before descendant tools run. No helper script is patched.

## Native prerequisites

Measurement requires kcov **43** and Bash **4.2 or newer**. `REVAER_KCOV_COMMAND`
and `REVAER_KCOV_BASH` may select explicit executable paths; version requirements
still apply. Native tools remain separate from uv's Python environment.

Linux CI can install the existing media stack's exact source pin with:

```console
rv setup --profile python --no-launcher --kcov
```

This requires CMake, a C++ compiler, pkg-config, and the curl/OpenSSL, ELF/DWARF,
binutils/libiberty and zlib development packages from the native package manager.
The workflow's coverage package profile supplies them. Its migration is still
in progress.

The commit and archive SHA-256 are in `tools/versions.toml`. Installation defaults
to `~/.local/revaer/kcov/COMMIT/installed`; `REVAER_KCOV_INSTALL_ROOT` changes the
parent directory. `REVAER_KCOV_ARCHIVES_URL` can select an HTTPS archive directory;
the source hash remains mandatory. CMake runs configure, build and install in a
fresh private staging directory. A valid version probe precedes publication.
The receipt records every installed file's permission bits and SHA-256. Matching
installs are reused; changed files are rebuilt. Failed builds preserve the
previous install and retain configure/build/install logs beside the receipt.

The source installer targets Linux CI. Local macOS coverage uses native kcov 43,
including Homebrew's packaged release. The pinned source's macOS CMake discovery
does not locate the currently installed DWARF headers; that failed probe is
recorded in the task ADR. No upstream source or diagnostic suppression is added.

## Selecting native setup requirements

`rv setup` installs the complete configured Cargo and browser set by default.
For a focused environment, use `--cargo-tools sqlx-cli,trunk --browsers chromium`.
An explicitly empty string selects none; omitted options retain the full default.
Tool versions and features always come from `tools/versions.toml`. Unknown names,
duplicate names and incompatible Python-only selections fail before installation.

On Debian/Ubuntu, `--apt-profile base`, `db`, `coverage`, or `media` installs the
corresponding native packages through apt. `--apt-packages` accepts an explicit
whitespace-separated override. Both remain opt-in locally. The typed adapter
validates package tokens, retains signature verification, and propagates a failed
repository refresh before attempting installation. Package state belongs to apt.
The [workflow action guide](../../../../.github/actions/setup-revaer/README.md)
describes CI selections and supported caches.

## Tests and coverage evidence

### Measured root script

`tools/tests/test_bootstrap.py` executes the unchanged script in disposable
checkouts, both directly and through kcov. It checks:

- An existing uv installs the launcher into fixture-owned tool directories.
- A missing uv uses the official unmanaged installer.
- Explicit installation directories preserve uv's documented precedence.
- A stale lockfile fails without rewriting the lock or claiming setup succeeded.
- Paths containing spaces work, and foreign environment/project selectors leave
  an unrelated checkout untouched.

`rv script-coverage` runs the measured cases and retains their private logs,
source hashes, expected exit codes and raw kcov reports in `coverage/scripts/cases`.
The merger requires the same complete executable-line inventory in every case.
It changes only temporary source paths back to `setup.sh` after verifying the
source bytes. A line becomes covered only if a native report observed it running.
Missing, changed, malformed or out-of-tree evidence fails. The final configured
Sonar input is `coverage/script-coverage.xml`.

The collected installer case currently measures all **9/9** executable lines.
The operator-approved rule permits at most one uncovered executable line in
root `setup.sh`, and also accepts full coverage. Empty or wholly unexecuted
reports fail. The producer retains every real zero-hit record and does not
alter measured counts. See
[ADR 592](../../../../docs/adr/592-python-tooling.md#bootstrap-coverage-allowance).

## Upstream interfaces

- [uv installer options](https://docs.astral.sh/uv/reference/installer/)
- [uv projects and locked sync](https://docs.astral.sh/uv/concepts/projects/sync/)
- [uv dependency groups](https://docs.astral.sh/uv/concepts/projects/dependencies/#dependency-groups)
- [kcov usage](https://github.com/SimonKagstrom/kcov/blob/master/README.md)
- [Pinned kcov build instructions](https://github.com/SimonKagstrom/kcov/blob/a39874f938ce13f7a65f253120d1ec946b349ffe/INSTALL.md)
- [CMake command-line interface](https://cmake.org/cmake/help/latest/manual/cmake.1.html)
- [Python tar extraction filters](https://docs.python.org/3/library/tarfile.html#extraction-filters)
