#!/usr/bin/env bash
set -o nounset -o pipefail -o errexit

# You can run this script from within your IDE (see `.vscode/tasks.json` for an
# example configuration)

# Applies to all packages
cargo clippy \
    --locked \
    --all-targets \
    --all-features \
    -- \
    --deny clippy::all \
    --deny warnings \
    --deny unsafe_code \
