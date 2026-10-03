# Installed rv launcher

This package supplies the user-level `rv` command. It has no runtime dependencies
and does no project setup itself.

## Install and remove

Normally `./setup.sh` calls `rv setup`, which installs this package using:

```console
uv tool install --python 3.13.12 --reinstall ./tools/launcher
```

The Python version comes from the checkout's `.python-version`. Reinstallation
refreshes local source changes without binding the installed tool to a worktree.
uv owns its virtual environment, executable, and installation record. No
`--force` is used: a conflicting executable is an error that must be resolved
explicitly.

```console
uv tool dir --bin
uv tool update-shell
uv tool uninstall revaer-launcher
```

## Dispatch contract

1. Start in the caller's current directory and find the nearest `.git` boundary.
2. Require a Revaer `rv` implementation in that checkout. Never search past a
   nested repository or use the install-source checkout as a fallback.
3. Replace the launcher process with `uv run --locked --directory <checkout> --
   python -m revaer_tooling.cli`, forwarding arguments without shell parsing.

Using the module name prevents recursion into the global `rv` executable.
Replacing the process preserves signals and the command's exit status. uv
synchronizes the selected project's dependencies before running its code.

## Why a separate package?

`uv tool install` creates an isolated tool environment; it does not use a
project's `uv.lock`. Installing the full implementation as a user tool would
create a second dependency graph and couple every checkout to one implementation.
This small package performs only the worktree selection that is specific to
Revaer. The implementation and its dependencies stay in the selected uv project.

## Tests

The root tooling suite covers worktree selection, argument forwarding, nested
repository refusal, environment isolation, and installation behavior. Run it with
`uv run --locked -- rv tooling-check` from the repository root.
