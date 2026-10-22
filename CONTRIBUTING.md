# Contributing to Kaiser

Kaiser is a Rust terminal application. Start with [the architecture guide](docs/architecture.md) for where a change belongs, and [configuration](docs/configuration.md) for user settings and themes.

## Build and run

Install stable Rust with Cargo. From the repository root:

```sh
cargo build --locked
cargo run --locked -- --demo
```

The debug binary is `target/debug/kaiser` (`kaiser.exe` on Windows). After editing Rust files, run `cargo build --locked` again. Use `cargo build --locked --release` for `target/release/kaiser`.

To install the checkout as the `kaiser` terminal command, run `cargo install --path . --locked`, or `sh scripts/install.sh` on Unix. Fresh libraries are empty; example notebooks are limited to explicit demo/snapshot mode and synthetic test fixtures.

`--demo` opens the fictional showcase and keeps study data ephemeral. `--demo-view chat`, `--demo-view flashcards`, and `--demo-view notes` open populated workspaces for screenshots. Demo/snapshot sessions do not activate configured API providers. To test persistence without touching a real library, use `--data-dir PATH` and `--config PATH` pointing to temporary directories. `--snapshot --size 120x40` renders a plain-text home screen without opening an interactive terminal.

## Checks

Run the same Rust checks as CI:

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked
```

On Unix, Python 3 runs the terminal smoke checks. Each accepts an optional binary path; all default to the debug binary:

```sh
python3 tests/tui_smoke.py
python3 tests/config_smoke.py
python3 tests/logo_smoke.py
python3 tests/demo_smoke.py
```

CI checks Rust on Linux, macOS, and Windows, and runs terminal smoke checks on Linux against the release binary. Add `--offline` to Cargo commands when dependencies are already cached. Model tests use local fixtures and injected responses, without real accounts or inference usage.

Shared Rust fixtures live in `src/test_support.rs` and compile only in tests. Python terminal fixtures live in `tests/smoke_support.py`; use its disambiguated Escape key when scripting navigation. Screenshot content lives in `src/demo/`.

Before publishing, stage the intended files and run `python3 scripts/check_public_files.py`. It checks tracked files for private runtime paths, agent instruction files, symlinks, and common credential patterns; Git ignore rules alone are not a substitute for inspecting the publication set. The Cargo package also has an explicit file allowlist. `tests/fixtures/chatgpt-identity.json` is a synthetic test token signed with a disposable key, not an account credential.

## Change conventions

- Keep terminal rendering in `src/ui/`, interaction and orchestration in `src/app/`, and document/model logic in the domain modules.
- Use the named `Command` and `Action` types for dispatch. Palette positions are presentation details, never action identifiers.
- Keep config additions optional with defaults, reject unknown fields, and validate before changing live state. Update `config/config.toml` and its defaults consistency test when changing defaults.
- Edit built-in colors in `config/themes/`; avoid hardcoded palettes in Rust. Keep status surfaces neutral and honor transparency through the existing shared renderer.
- Use `Kaiser` for the product name, `kaiser` for the binary/package, and `kaiser-*` for canonical built-in theme names. Keep existing aliases compatible.
- Background results must stay with their originating notebook. Preserve usage accounting, request limits, cancellation between requests, and credential handling when changing generation.
- Add regression tests for meaningful behavior changes. Keep large application tests grouped by feature under `src/app/tests/`; view and config tests live beside their modules.
- Avoid platform-specific document-conversion tools or new dependencies when the existing portable implementations suffice.

For a review, describe the changed behavior, relevant checks, and any migration or compatibility effects. Library files, credentials, and local configuration should never be committed.
