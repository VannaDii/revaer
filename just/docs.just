docs-install:
    bash scripts/ensure-exact-cargo-tool.sh mdbook mdbook 0.5.0
    bash scripts/ensure-exact-cargo-tool.sh mdbook-mermaid mdbook-mermaid 0.17.0
    "${CARGO_HOME:-${HOME}/.cargo}/bin/mdbook-mermaid" install ./docs

docs-build:
    cd docs && "${CARGO_HOME:-${HOME}/.cargo}/bin/mdbook" build

docs-serve:
    cd docs && "${CARGO_HOME:-${HOME}/.cargo}/bin/mdbook" serve --open

docs-index:
    cargo run -p revaer-doc-indexer --release

docs-link-check:
    bash scripts/ensure-exact-cargo-tool.sh lychee lychee 0.24.2
    "${CARGO_HOME:-${HOME}/.cargo}/bin/lychee" --verbose --no-progress docs

docs:
    just docs-install
    just docs-build
    just docs-index
