#!/usr/bin/env bash
set -o nounset -o pipefail -o errexit

cargo fmt -- --check

RUSTDOCFLAGS='--deny warnings' cargo doc --locked --no-deps --document-private-items

scripts/cargo-clippy.sh
scripts/cargo-audit.sh
