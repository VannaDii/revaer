# Revaer release tooling

The Python migration uses **Python Semantic Release** as the version and release
notes engine. `rv` coordinates Revaer's build artifacts and Helm chart operations.
The post-merge/tag workflow now invokes those Python tasks. Other workflow and
media-stack parity checks remain in progress; this document does not declare the
migration complete.

## Commands and responsibilities

| Command | Responsibility |
| --- | --- |
| `rv release preview` | Ask the engine for the next version and notes; print the source commit and expected artifacts. |
| `rv release-artifacts` | Build the application and OpenAPI document; record their source commit and SHA-256 digests. |
| `rv release publish` | Calculate a release on an eligible branch, prepare optional chart assets, publish the tag/release, and verify uploaded files. |
| `rv release publish --tag v1.2.3` | Publish an existing stable tag, including from a detached CI checkout. |
| `rv release resume TAG` | Inspect a partial release and upload only missing, previously prepared artifacts. |
| `rv helm-lint` | Lint and package an unsigned chart for local validation. |
| `rv helm-package VERSION APP_VERSION` | Package the chart, with signing enabled by default. |
| `rv helm-publish VERSION APP_VERSION` | Verify and publish an already packaged chart and Artifact Hub metadata. |
| `rv helm-verify VERSION APP_VERSION` | Read an existing chart through Helm and verify its requested identity and metadata; signed mode also verifies provenance. |

Publishing is an explicit operation. The migration tests use disposable local Git
remotes and a local release API with the real command-line tools. They do not
create GitHub releases or push chart registry artifacts.

## Version policy

[`semantic-release.toml`](semantic-release.toml) preserves the existing branches:
`main` produces `dev` prereleases and `gh-pages` produces stable releases. Feature
branches are ineligible. A preview uses the same configuration as publication.
It never rewrites branch eligibility to make a feature checkout look like `main`.

The existing Conventional Commit rules remain: `feat` increments minor,
`fix`/`perf` and the configured maintenance types increment patch, and breaking
changes increment major. Tags use `vVERSION`. Real temporary Git histories test
patch, minor, breaking, successive prerelease, stable, ineligible, and unchanged
cases while checking that previews leave refs and files unchanged.

### Existing stable tags

The stable-tag workflow already receives its version from an operator-created
`vMAJOR.MINOR.PATCH` tag. `rv release publish --tag TAG` retains that behavior and
the `Revaer TAG` release title. Git verifies that the local tag, remote tag, and
checked-out commit agree; annotated and lightweight tags are supported. This
operation does not calculate a new version or generate release notes. Existing
release notes are retained.

Python Semantic Release requires an attached eligible branch, even for its
existing-tag commands. The stable-tag task therefore invokes `gh release create
--verify-tag` for the existing workflow's operation. It does not manufacture a
branch or change the engine's branch policy.

## Prepare, publish, recover

### Prepare artifacts

Run `rv release-artifacts` in the intended clean checkout. It builds through the
same `rv` tasks used locally, obtains Cargo's actual target directory, and writes:

- `dist/revaer-app`
- `dist/revaer-app.sha256`
- `dist/openapi.json`
- `dist/release-artifacts.json`, containing the source commit and file digests

The task checks the working tree and source commit again after building. It
writes the manifest last, so an interrupted preparation cannot leave a new
manifest claiming that old files were built successfully. `CARGO_TARGET_DIR`
and Cargo's target-directory configuration remain supported.

### Publish the prepared files

Run `rv release preview` to inspect the next version and notes, then invoke the
appropriate publication command. Publication requires a clean source checkout
matching the artifact manifest. When Helm release assets are enabled, normal
publication packages and verifies them before creating the release.

`rv` inspects existing GitHub assets before making publication changes. Expected
assets must match their name, size, uploaded state, and server-reported SHA-256
digest. Missing files are uploaded individually by name; conflicting files cause
an error. Upload completion is checked against the server's asset metadata.

The engine and GitHub CLI share an explicit repository destination derived from
the engine's configured origin and GitHub server. A conflicting inherited
`GH_REPO` or `GH_HOST` stops publication. Git URL parsing uses PSR's documented
helper, and the local API tests reject requests for an unexpected repository.

`release/next-release.json` records the version, tag, source commit, and completion
state. GitHub Actions receives `version`, `tag`, and `released` outputs only at
the orchestration boundary. A release is reported as complete after all expected
assets verify. Individual tool success does not mark a partial operation complete.

### GitHub Actions handoff

[`.github/workflows/ci.yml`](../.github/workflows/ci.yml) handles main pushes and
version tags. The build job records actual Git identity and publishes `dist` with
its source-bound manifest. The release job downloads that exact artifact and
invokes `rv release publish`, or `rv release publish --tag "$RELEASE_TAG"` for
an existing stable tag. Both publication paths retain full Git history.

The task writes completion outputs directly to `GITHUB_OUTPUT`. A main run with
no new release emits `released=false`; chart and image publication then skip.
Completed releases upload their signed chart as an artifact. Registry jobs
download that exact package, take the verified version/tag from the release
outputs, and run `rv helm-publish`. They receive registry credentials; signing
material stays in the packaging job. Stable image publication remains independent
of the main-only prerelease job.

Python Semantic Release still owns calculated versions and notes. Workflow
mutation tests guard artifact identity, eligibility and completion ordering.
Hosted execution remains an acceptance step of the overall migration.

### Recover a partial publication

Keep the original checkout and `dist` files, resolve the reported transport or
credential problem, then run `rv release resume TAG`. Recovery requires the
original tagged commit and checks every prepared artifact again. It never
rebuilds the application, regenerates chart signatures, overwrites an asset, or
force-pushes a tag.

For an engine-created prerelease, recovery uses Python Semantic Release's
`changelog --post-to-release-tag` command to restore its release notes if needed.
For a stable tag, recovery uses the stable-tag operation described above. Both
paths inspect existing assets first and transfer only missing files. A conflicting
tag, draft release, incomplete asset metadata, or changed artifact stops recovery
with an error requiring investigation.

## Dependency compatibility

The release engine uses upstream Python Semantic Release revision
[`fa87c95ffbdaae3b7516dcc355e57bd5f4632c1f`](https://github.com/python-semantic-release/python-semantic-release/commit/fa87c95ffbdaae3b7516dcc355e57bd5f4632c1f),
installed by uv from its official source archive. `uv.lock` retains the archive
SHA-256 and the complete dependency graph. This upstream revision repairs the
published 10.6.2 Click constraint and supports current GitPython; no local patch,
resolver override or advisory waiver is used. Keep `click>=8.3.3` and
`gitpython>=3.1.60` as security floors. Replace the archive pin with a corrected
official release after repeating the release tests and audit.

`rv tooling-audit` exports every dependency group to standard PEP 751
`artifacts/python-audit/pylock.toml`, using `uv export --locked --all-groups
--no-emit-project --format pylock.toml`. This format retains source versions and
hashes that requirements.txt cannot express together. `pip-audit --locked
--strict` consumes it without resolving another graph. The task rejects skipped,
empty, malformed or vulnerable results even if the process returns zero, and
retains successful JSON evidence alongside the exported lock. It invalidates old
success evidence before auditing. Auditing remains an advisory check on declared
package versions; uv separately verifies source provenance and archive bytes.

The isolated candidate passed 60 release tests and an audit of all 89 third-party
packages. Full migration acceptance still requires the integrated project gates.

## Helm inputs and artifacts

`REVAER_ENABLE_HELM_RELEASE_ASSETS=1` enables chart assets during application
release preparation. This preserves the existing opt-in default.
`REVAER_HELM_SIGN=0` permits unsigned local packaging; publication still requires
signed artifacts and verifies them before pushing.

Signing uses `HELM_GPG_PRIVATE` and `HELM_GPG_PUBLIC`. Keys are imported into a
temporary GnuPG home; temporary secret keyrings are private. Registry credentials
use `HELM_REGISTRY_USERNAME`/`HELM_REGISTRY_PASSWORD`, with `HELM_API_KEY_ID`/
`HELM_API_KEY_SECRET` as the existing fallback. GHCR may use the job-scoped
`GITHUB_TOKEN` and actor. Passwords travel over stdin and are redacted in logs.

The chart, provenance, public keys, and `artifacthub-repo.yml` are written beneath
`dist/helm`. Repository metadata is published separately from the chart tarball.
`HELM_REGISTRY_HOST`, `HELM_REGISTRY_NAMESPACE`, `REVAER_RELEASE_REPOSITORY`,
`REVAER_HELM_IMAGE_REPOSITORY`, and `ARTIFACTHUB_*` retain their existing roles.
See [`settings.py`](../tools/src/revaer_tooling/settings.py) for the typed boundary
and the exact override precedence.

Registry publication uses Helm and ORAS's official `--registry-config` option
with a private temporary configuration, removed when the operation ends. It does
not add release credentials to the developer's persistent credential store.
`HELM_REGISTRY_CA_FILE` supplies a CA bundle for a private TLS registry through
both clients' supported `--ca-file` option. ORAS receives `--plain-http=false`
explicitly because it otherwise chooses plaintext for localhost. Certificate
and hostname verification remain enabled.

## Tests and remaining integration work

[`tools/tests`](../tools/tests/README.md) exercise real Git, Cargo, Helm, GnuPG,
Python Semantic Release, and GitHub CLI processes. Release fixtures cover
creation and upload failures, retry with exact artifact reuse, changed source or
artifact rejection, conflicting remote assets, server digest verification,
detached stable tags, and preservation of existing notes. Chart tests cover
signed and unsigned packages, signature verification, metadata, and preservation
of the previous output when preparation fails. A pinned Distribution registry
runs locally with TLS and password authentication. The test downloads the chart,
provenance, and repository metadata and compares them byte for byte with the
packaged files; incorrect credentials and missing certificate trust must fail.

GitHub Actions integration and the media stack's additional chart/compliance
checks remain in the
[migration inventory](../tools/migration.md), along with the full foundation and
combined-stack gates required before removing the old implementations.
