# Kaiser

A Rust terminal workspace for learning from your sources. Import PDFs, documents, and images, ask questions with source citations, review flashcards, and write Markdown notes.

## Install

Requires stable Rust and Cargo.

```sh
cargo install --git https://github.com/LThmly/Kaiser.git --locked
kaiser
```

From a checkout, use `cargo install --path . --locked` or `sh scripts/install.sh`. Make sure Cargo's `bin` directory is on your PATH.

## Get started

1. Run `kaiser` and use `:new` to create a notebook.
2. Import a source with `:import`, or `:import ~/Downloads/chapter.pdf`.
3. Use `:login` and `:model` for AI chat. Press `i` to type a question and Enter to send it.
4. Run `:generate-cards` to create flashcard drafts, or `:notes` to write notes.

New installs start empty. `kaiser --demo` opens a disposable showcase with six notebooks and 84 flashcards. For a screenshot-ready workspace, run `kaiser --demo-view flashcards` (also `chat`, `notes`, or `home`). Demo sessions make no AI requests. Without a model, normal chat shows matching source excerpts.

## Navigation

Press **Space**, release it, then a command key. Press Space alone to see the menu; Esc returns to navigation.

| Keys / command | Action |
| --- | --- |
| `Space n` / `Space f` / `Space h` | New notebook / select notebook / home |
| `Space c` / `Space F` / `Space o` | Chat / flashcards / notes |
| `Space p` | Toggle Markdown editor and preview |
| `:generate-cards 5 chapter 1` | Generate cards for a specific scope |
| `:drafts` | Review generated drafts |
| `:config-open` / `:config-reload` | Edit and reload settings |
| `:theme` | Choose and preview a theme |
| `:q` | Save and quit |

Press `:` for command suggestions and Tab to cycle choices. In flashcards, Enter flips the card and arrow keys move between cards.

## Documentation

- [User guide](docs/user-guide.md) — sources, chat, flashcards, and shortcuts.
- [Configuration](docs/configuration.md) — settings, themes, and AI usage controls.
- [Contributing](CONTRIBUTING.md) and [architecture](docs/architecture.md) — building and working on the code.

Kaiser code is [MIT licensed](LICENSE). Bundled OCR models use [CC BY-SA 4.0](assets/models/README.md).
