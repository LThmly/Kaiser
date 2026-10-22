# Architecture

Kaiser is a single Rust binary. It uses a synchronous terminal event loop and background workers for document extraction, authentication, and model requests. There is no required service, database, or document-conversion process.

## Module map

| Location | Responsibility |
| --- | --- |
| `src/main.rs` | CLI, terminal lifecycle, event loop, automatic saving, external editor suspension |
| `src/app/mod.rs` | Shared application state, initialization, typed action dispatch |
| `src/app/state.rs` | Focus, tabs, modal state, actions, pending-job types |
| `src/app/commands.rs` | Named commands, palette/colon/leader registries and dispatch |
| `src/app/input.rs`, `modals.rs` | Normal/editor input, pasted text, modal input and completion |
| `src/app/notebooks.rs`, `workspace.rs` | Notebook lifecycle, working directories, navigation, notes and card interaction |
| `src/app/generation.rs`, `sources.rs` | Background chat/card generation and document imports, applied to their originating notebook |
| `src/app/accounts.rs`, `settings.rs` | Account operations, shared startup/reload settings, temporary theme preview |
| `src/ui/mod.rs` | Responsive layout composition, directory label, global transparency |
| `src/ui/home.rs`, `sidebar.rs`, `workspace.rs`, `chat.rs` | Individual views |
| `src/ui/dialogs.rs` | Pickers, command/leader menus, confirmation and editing dialogs |
| `src/ui/widgets.rs`, `status.rs` | Shared borders, text layout, empty states, neutral status surfaces |
| `src/config/` | Settings schema/defaults, loading/initialization, theme registry/inheritance |
| `src/demo/` | Disposable screenshot showcase used by demo and snapshot modes |
| `src/test_support.rs`, `tests/` | Synthetic test fixtures and real-terminal checks; never loaded by normal sessions |
| `src/model.rs` | Serializable library, notebooks, sources, conversations, cards, atomic state-file writes |
| `src/import.rs`, `files.rs` | Portable text/document/image extraction and import path completion |
| `src/retrieval.rs` | Source scoping, complete/focused context, bounded multi-stage plans |
| `src/provider.rs`, `auth.rs`, `usage.rs` | Model transport/pipelines, account credentials, request budgets/cache/accounting |
| `src/flashcards.rs` | Generation request parsing and atomic validation of cited card output |
| `src/notes.rs`, `markdown.rs`, `progress.rs`, `logo.rs` | Note editing, Markdown rendering, review activity, terminal image support |

## State and effects

Keyboard and mouse input become application actions. App methods update the in-memory library and mark it dirty. The event loop saves dirty state. Views read that state and emit clickable actions; rendering may update display-only state such as hit regions and scroll bounds. Persistence, HTTP, and document extraction do not belong in view code.

Background jobs capture their notebook and source evidence when started. Completion handlers apply the result to that notebook, even after navigation. Notebook deletion rebases or discards pending jobs. Dropping the progress receiver prevents later pipeline requests; an already-sent HTTP request may still finish. The shared usage runtime protects against overlapping inference across provider/account changes.

`Command` is a named enum. Palette order and filtered indices select values from the command registry; they do not encode behavior. `Action` also uses named command values. Colon and leader dispatch belong in the command module; input modules handle editing and selection mechanics.

## Configuration and themes

The public config schema is in `src/config/settings.rs`; `config/config.toml` is the commented user-facing reference. A regression test keeps its values equal to omitted settings and Rust defaults. Loading and reloads use the same validation, and `App::apply_settings` applies valid settings after all fallible checks.

Built-in color values live only in `config/themes/*.toml`. Rust embeds those files and maintains canonical names and compatibility aliases in `src/config/themes.rs`. Bundled themes must provide complete palettes. A user theme can select a built-in base and override individual colors. There is no separate Rust palette to update.

## Tests and compatibility

Application behavior tests are grouped under `src/app/tests/`. UI tests use Ratatui's test backend across supported sizes and themes. Configuration tests protect defaults, aliases, inheritance, strict validation, and non-destructive initialization. Smaller domain modules keep their focused unit tests inline. Python smoke tests exercise the real terminal loop with temporary data and configuration.

Keep existing JSON library fields, TOML keys, theme aliases, and shortcuts compatible unless a migration is intentional and documented. Refactoring a view or command must not change the storage format. See [CONTRIBUTING.md](../CONTRIBUTING.md) for build and validation commands.
