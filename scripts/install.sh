#!/bin/sh
# Build and install this checkout. Cargo's bin directory must be on PATH.
set -eu

project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
if command -v cargo >/dev/null 2>&1; then
    cargo_bin=cargo
elif [ -x "${HOME:-}/.cargo/bin/cargo" ]; then
    cargo_bin="${HOME}/.cargo/bin/cargo"
else
    printf '%s\n' 'Install a stable Rust toolchain with Cargo, then run this installer again.' >&2
    exit 1
fi

exec "$cargo_bin" install --path "$project_dir" --locked "$@"
