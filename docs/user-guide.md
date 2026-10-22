# User guide

Detailed instructions for using Kaiser. See the [README](../README.md) for a quick start.

## Install

With a current stable Rust toolchain and Cargo:

```sh
cargo install --git https://github.com/LThmly/Kaiser.git --branch main --locked kaiser
kaiser
```

From a source checkout, run `cargo install --path . --locked`. On Linux and macOS, `sh scripts/install.sh` runs that command and also finds Cargo in its usual location if it is not yet on PATH. The executable is installed in Cargo's `bin` directory: normally `~/.cargo/bin` on Unix or `%USERPROFILE%\.cargo\bin` on Windows. Add that directory to PATH if needed; on Unix, `. "$HOME/.cargo/env"` initializes the usual Rust environment. Installation builds the self-contained binary and does not copy your notebooks, settings, or accounts.

To update, rerun the same install command. To uninstall a Cargo installation, run `cargo uninstall kaiser`; your local study data is preserved. Rust is required to build/install from source. A Rust toolchain is not needed to run the resulting executable.

## Screenshot demo

```sh
kaiser --demo
kaiser --demo-view flashcards
kaiser --demo-view chat
kaiser --demo-view notes
```

The demo contains six fictional notebooks, 84 cited flashcards, sample source text, Markdown notes, and a prepared conversation. `--demo` opens Home; `--demo-view` opens the selected workspace directly and implies `--demo`. The Flashcards view begins with an answer revealed. Use the normal notebook picker, tabs, theme previews, and editor to arrange your screenshot.

Your configured theme is used. A terminal around 140 columns by 40 rows gives the panels room; `:theme kaiser-bagels` or `:theme kaiser-main` can change the look for the session. Press Space f to browse the other notebooks or Space h to return to Home.

Demo source filenames and content are illustrative; no PDFs or personal documents are loaded. Notebook changes are discarded on exit. Saved accounts, study data, and AI usage storage are not opened, and configured API providers are disabled. Chat questions use local source lookup; the initial conversation is a scripted example. Explicit config/theme editing commands still operate on the selected config files.

For a plain-text preview rather than an interactive session, combine a demo view with `--snapshot --size 140x40`.

## A first study session

1. Launch `kaiser` and press `<super> + n` or type `:new` to create your first notebook. `<super> + f` opens the notebook picker after you have notebooks.
2. Press `Space a` to import a PDF, document, image, or text file. `Space s` focuses Sources; toggle included sources with `x` and read the highlighted source with `Enter`.
3. For AI answers, use `:login` and `:model`, or configure an API provider. Press `Space c`, then `i`, ask a question about your sources, and press `Enter`.
4. Offline, Kaiser returns matching source excerpts. Click a citation beneath an answer to read its source. `Space e` opens the notebook's flashcard list.
5. Use `:generate-cards` to generate cited drafts, or `Space m` to make a card from the exchange. Edit the question and answer; `Tab` switches fields, `Ctrl+U` clears the current field. `Ctrl+S` saves a draft; `Ctrl+A` finishes the card.
6. Press `Space F` to open flashcards. Select a question or term from the list; `Enter` flips it, `←`/`→` moves between cards, and `e` edits it.
7. Quit with `Space Q` (uppercase Q). Your notebooks, source selection, conversations, notes, and cards are saved locally. Existing review history is preserved when loading older libraries.

Press `Space a` or `:import PATH` to import a source. Paths beginning with `~/` are expanded, and paths with spaces are supported without shell quoting. Imports run locally in a background worker, so you can keep navigating or switch notebooks. The result is always added to the notebook that started the import.

The import dialog lists files and directories from the notebook's working directory. Type a path such as `~/Downloads/`, then part of a filename to search that directory (case-insensitive). Tab/Shift+Tab or arrows select a match; Right completes its path, and Enter opens a directory or imports a file. Hidden entries appear when the filename query starts with a dot. Search stays within the displayed directory.

| Sources | Extraction |
| --- | --- |
| PDF | Embedded text, with OCR for pages without text; page labels retained |
| Word `.docx` | Paragraphs, tables, footnotes, and endnotes |
| PowerPoint `.pptx` | Slide text in presentation order, plus speaker notes |
| OpenDocument `.odt`, `.odp` | Document and slide text |
| PNG, JPEG, WebP, TIFF, BMP, GIF, PNM | Bundled OCR for printed text; first image/frame |
| Markdown, text, RST, CSV, TSV, JSON, YAML, TOML, logs, TeX | UTF-8 text |

The extraction engine is compiled into Kaiser using Rust libraries. It requires no LibreOffice, Tesseract, Poppler, Python, GPU, API, or runtime model download. OCR model weights are embedded in the binary. Imports are capped at 100 MiB per file, 10 MiB extracted text, and 500 PDF pages or PowerPoint slides. OCR is best for clear printed Latin text; handwriting, diagrams, complex layouts, and embedded Office images are not interpreted. Password-protected documents must be exported without a password. Legacy `.doc`/`.ppt` files should be saved as `.docx`/`.pptx` or PDF first.

Extracted text and the source filename are stored in the library, not the original binary file. Citations refer to lines in that extracted text; PDF pages and PowerPoint slides have visible section labels. Later edits to the original document are not automatically imported. Extraction failures leave the notebook unchanged. Press `<super> + n` to create a notebook.

## Interface

The notebook workspace has Chat, Flashcards, and Notes tabs. Source documents open in a separate reader popup from Sources (`Enter`), search, `F6`, or `:read`; `j`/`k`, arrows, PageUp/PageDown, Home/End scroll it and `Esc` returns to your workspace. Each notebook has its working directory in the top-right border. Each notebook remembers its own directory; existing notebooks initially use the launch directory, and new notebooks inherit the current notebook’s directory. Use `:cd PATH` to change it (`:cd` alone selects your home directory), or `:pwd` to show the full path. Relative source-import paths resolve from that notebook’s directory. Switching notebooks restores its saved directory without changing the process-wide directory.

Chat fills the workspace with a conversation and a rounded composer. Questions appear on the right; Kaiser replies render Markdown on the left, with clickable source citations underneath. Click the composer or press `i` to write, Enter or **Send** submits, and Shift+Enter inserts a newline on terminals that report the key combination. Pasted paragraphs and spaces are preserved. The composer grows to four visible lines and keeps the cursor in view. Esc returns to leader navigation. The source count and selected model appear below the composer; browsing or opening Chat does not generate an AI request.

The Flashcards panel below the Notes and Flashcards workspaces lists all cards in the selected notebook, including drafts. Press `Space e` to focus it, use arrows or `j`/`k` to choose, and press `Enter` to open it. Click a row to show that card in the workspace. `Enter` flips between question and definition, `←`/`→` moves to the previous or next card, and `e` edits it. `Space m` makes a card. In Chat or on smaller terminals, focusing the panel expands it into the workspace area; `Space w` returns to the workspace. The list shows all questions and terms; the workspace displays one selected flashcard at a time. There are no review sessions, ratings, or queues.

A single status bar spans the bottom edge: current input mode on the left, status messages in the middle, and flashcard counts on the right after opening a notebook. It uses neutral grey colors across all themes and keeps long messages on one line. Panel and menu footer bars use the same neutral treatment.

Home has two panels: Kaiser with the logo, introduction, and startup shortcuts; and Notebooks as a Bagels-inspired tree table with a uniform panel background, uniform muted notebook dots and tree branches, and indented source counts, flashcard counts, and directories. Click anywhere on a notebook entry to open it. Use `<super> + n` to create a notebook and `<super> + f` to browse the full list. Wide terminals show the panels side by side; smaller terminals stack them and use compact notebook rows. Thin rounded borders and a neutral footer follow the Bagels-inspired style. Notebook and Sources panels appear after selecting a notebook. Every launch starts with the original Kaiser PNG logo on terminals supporting Kitty graphics, iTerm2 images, or Sixel, a few app details, and shortcuts to select or create a notebook. No notebook opens automatically, even if one was used last time. The top-left control shows only the selected notebook; click it, press `<super> + f`, or focus it and press `Enter` to open a searchable picker. `<super> + f` also opens the picker from any notebook workspace or menu, including text and card editing. Tab/Shift+Tab cycles through notebook, command palette, and search choices with wraparound. Moving through the picker does not change notebooks until you confirm. Press `Ctrl+d` to delete the highlighted notebook. A confirmation shows its contents; `y` deletes, `n`/`Esc` keeps it, or use Tab and Enter (Keep is selected initially). Deletion removes its saved sources, conversations, notes, and cards from Kaiser; original source files remain untouched. Deleting the final notebook leaves an empty Home page, where `<super> + n` creates a new one. `Esc` cancels. <super> + f requires a terminal with the Kitty keyboard protocol and must reach the app; compositor shortcuts can intercept it. `:notebook` and clicking the selector also open the picker.

```text
┌ Notebook ──────────┐ ┌ Workspace ───────────────────────────┐
│ Distributed sys  ▾ │ │ Chat   Flashcards       Notes       │
└────────────────────┘ │                                     │
┌ Sources ───────────┐ │ Conversation, reader, or study card  │
│ ☑ replication.md   │ │                                     │
│ ☑ consistency.md   │ │ › Ask a question…                    │
│                    │ └─────────────────────────────────────┘
│                    │ ┌ Flashcards ──────────────────────────┐
│                    │ │ ready  Why can replicas serve old data?             │
│                    │ │ draft  How can we avoid a stale read?   │
└────────────────────┘ └─────────────────────────────────────┘
```

Only the empty Sources sidebar uses a diagonal hatch with a centered message. Full-cell `╱` strokes meet across adjacent rows to form continuous diagonal lines, like Bagels. Other empty views show plain messages.

Set `[ui] transparent_background = true` in `:config-open`, then run `:config-reload` to use the terminal's background across all themes, panels, popups, selection highlights, and status bars. Theme text and border colors stay active. Set it to `false` to restore theme backgrounds. The terminal controls any actual opacity or wallpaper transparency.

Ten additional palettes are available: `kaiser-ember` (orange), `kaiser-forest` (green), `kaiser-ocean` (blue), `kaiser-violet` (lavender), `kaiser-glacier` (cyan), `kaiser-orchid` (pink), `kaiser-slate` (steel), `kaiser-copper` (warm brown), `kaiser-paper` (light ivory), and `kaiser-sandstone` (light parchment). Use `:theme NAME` to try one and `:theme-open` to customize it. The two light themes are designed for a light terminal background when transparency is enabled. Neutral status bars and the transparency setting apply to every theme.

Run `:theme kaiser-bagels` for a Bagels-inspired charcoal and grey palette with muted purple focus. To keep it, set `theme = "kaiser-bagels"` in `:config-open`, then run `:config-reload`. Its editable palette is in `config/themes/kaiser-bagels.toml` (installed under `~/.config/kaiser/themes/`).

Run `:theme kaiser-main` for the home-page palette throughout Kaiser: black surfaces, white text and focus, muted rose-grey borders, and soft grey details. It works with your existing transparency setting. To keep it across launches, set `theme = "kaiser-main"` in `:config-open` and run `:config-reload`. Use `:theme-open` to edit its palette.

Tallgeese uses pure black and white, coral for focus, blush for source selection and approved flashcards, and light coral for drafts. Its colors match the Helix Tallgeese palette. The notebook selector, notebook picker, startup actions, tabs, source rows, and flashcard rows support mouse clicks. Mouse scrolling operates on the focused panel. Navigation shortcuts stay inactive while you type.

Press and release the leader, then press a command key. The default leader is **Space**. Pressing it opens a guide; `Esc` cancels. Sequences wait for your next key without a timeout. Leader keys work only in navigation mode, so spaces and other leader characters remain ordinary text in inputs and editors.

| Sequence | Action |
| --- | --- |
| `Space c` / `Space F` | Chat / Flashcards |
| `Space o` / `Space d` | Notes / Drafts |
| `<super> + f` / `Space f` | Select notebook |
| `Space s` | Focus Sources |
| `Space w` / `Space e` | Focus Workspace / Flashcards panel |
| `Space a` | Import source |
| `<super> + n` / `Space n` | Make new notebooks |
| `<super> + h` / `Space h` | Return to the home screen |
| `<super> + ?` | Keybindings |
| `Space m` | Make a card from the conversation |
| `Space g` | Generate flashcard drafts from selected sources |
| `Space /` / `Space P` | Search / Command palette |
| `Space p` | Toggle Notes preview / editor |
| `Space z` / `Space B` | Maximize workspace / Toggle sidebar |
| `Space ?` / `Space Q` | Help / Save and quit |

Uppercase suffixes (`F`, `P`, `B`, `Q`) require Shift. Bare navigation letters no longer switch screens or quit. Local controls stay direct:

| Key | Action |
| --- | --- |
| `Tab` / `Shift+Tab` | Next / previous panel |
| `↑ ↓` or `j k` | Navigate lists or scroll |
| `i` / `Esc` | Enter text / return to navigation |
| `x` / `Enter` in Sources | Toggle inclusion / Read source |
| `Enter` / `e` in Flashcards | Flip / edit selected card |
| `Ctrl+S` / `Ctrl+A` in card editor | Save draft / finish card |
| `←` / `→` in Flashcards | Previous / next card |
| `↑ ↓` / `j k`, then `Enter` in Flashcards | Choose and open a card |
| `↑ ↓`, then `Enter` in palette/search | Choose and open a result |
| `PgUp` / `PgDn` / `Home` / `End` | Scroll workspace |
| `Ctrl+C` | Save and quit |

Function keys (`F1`–`F6`, `F10`) and `Ctrl+K` remain available as alternate shortcuts.

Notes store Markdown and open in rendered preview, including headings, emphasis, lists, tasks, quotes, links, tables, and code blocks. `<leader> + p` (Space then p by default) or `:notes-preview` toggles preview and the raw editor. Press `Esc` to leave insert mode before using a leader shortcut. In the editor, core Helix bindings are supported: `h/j/k/l`, `w/b/e`, `gg/ge/gh/gl`, `i/a/I/A`, `o/O`, `v` to extend a selection, `x` to select a line, `%` to select all, `d/c/y` to delete/change/yank a selection, `p/P` to paste, `r` to replace, `u/U` to undo/redo, and `Esc` to return to normal mode. Insert mode supports arrows, Home/End, Backspace/Delete, newlines, tabs, and `Ctrl+w/u/k` deletion. Changes save automatically; the raw Markdown stays intact when switching previews. This is a single-selection editor; Helix's multiple cursors, LSP, and full command set are not included. Card editing still appends to each field; clear with `Ctrl+U` to replace it.

## Colon commands

In navigation mode, press `:` to open the command line and a Helix-style suggestion menu immediately above it. Commands appear with descriptions and aliases; typing filters the list. Use `↑/↓` to select an option, `Tab` to complete and cycle forward, `Shift+Tab` to cycle backward, and `Enter` to run the selected or typed command. Completion wraps around and keeps the original filter and full matching list until you edit the text. No command is selected automatically. `Esc` cancels; `Backspace` on an empty line closes it. Use `Ctrl+U` to clear the line and `Alt+↑/↓` for command history. Commands with arguments keep their usage hint visible as you type. Colons stay literal in chat, notes, and other text fields.

| Command | Action |
| --- | --- |
| `:q` / `:quit` | Save the library and quit |
| `:w` / `:write` | Save now |
| `:wq` / `:x` | Save and quit |
| `:notebook` | Open the notebook picker |
| `:notebook Biology` | Open a notebook by its full name |
| `:new` / `:new Algorithms` | Create a notebook via dialog / by name |
| `:chat` / `:cards` / `:notes` | Switch views |
| `:clear` | Clear the current notebook’s chat |
| `:notes-preview` | Toggle rendered Markdown / notes editor |
| `:sources` / `:read` | Focus Sources / read the selected source |
| `:drafts` / `:card` | Browse drafts / make a card |
| `:generate-cards` / `:generate-cards 5 chapter 1` | Generate up to 10 cards / choose a count and scope |
| `:import` / `:import ~/notes/topic.md` | Import dialog / import a source path |
| `:cd ~/Projects/course` / `:pwd` | Set / show the selected notebook’s working directory |
| `:help` | Show the keybinding guide |
| `:config-open` / `:config-reload` | Edit the active config / apply its changes |
| `:theme` / `:theme kaiser-tallgeese` | List themes / apply a theme for this session |
| `:theme-open` / `:theme-reload` | Edit the current theme / reload its colors |

Notebook names and import paths may contain spaces; no shell quoting or expansion is performed, except the supported `~/` path prefix. Commands that need a notebook prompt you to select one first. Existing automatic saving still applies, and demo sessions stay ephemeral.

To generate flashcards, select sources with `x`, then use `:generate-cards` or `Space g`. A connected model is required. The optional count is 1–20; scopes support the same chapter/page selection as summaries, for example `:generate-cards 8 pages 3–8`. Small scopes use complete extracted text in one request. Larger scopes use the existing multi-stage source planner and show a request-count review before sending; daily limits, caching, and summary request limits still apply. Generation uses source text without sending your chat history.

Generated cards save automatically as drafts with their supporting source passage attached. Review them with `:drafts`, flip with Enter, and press `e` to edit; `Ctrl+A` in the editor finishes a card. Exact question duplicates (ignoring case and whitespace) are skipped. Invalid output or invalid citation IDs add no cards; there are no automatic retries. Check the answers against their sources before finishing them. Repeating an identical request can reuse its cached cards; change the scope or use `:ai-cache-clear` when you want a fresh generation.

The original logo in `assets/kaiser.png` is embedded in the binary and rendered directly through the terminal image protocol. No external image program or system graphics library is needed. Terminal capabilities are detected at launch; text-only terminals show the Kaiser title and shortcuts. The image keeps its proportions as the window changes size.

## Configuration and themes

Kaiser reads `$XDG_CONFIG_HOME/kaiser/config.toml`, defaulting to `~/.config/kaiser/config.toml`. The first normal launch creates missing defaults. Existing config and theme files are never overwritten. `--demo` and `--snapshot` read configuration but do not create it or modify your study data.

Generate the files explicitly, or use a separate config:

```sh
./target/release/kaiser --init-config
./target/release/kaiser --check-config
./target/release/kaiser --config /path/to/config.toml
```

`--check-config` validates settings and the selected theme without launching the TUI or changing study data. See the [configuration guide](configuration.md) for precedence, validation, custom themes, and editor configuration.

Default `config.toml`:

```toml
theme = "kaiser-tallgeese"

[keys]
leader = "space"

[ui]
sidebar = true

[editor]
# Empty means $VISUAL, then $EDITOR, then helix.
command = []
# Or: command = ["helix"]
# Or: command = ["code", "--wait"]

[model]
# api_url = "http://localhost:1234/v1"
# model = "your-loaded-model"
api_key_env = "KAISER_API_KEY"
```

Run `:config-open` to edit the **active config path**, including a file selected with `--config`. Kaiser suspends the TUI, runs your editor, then restores the screen and keyboard mode when the editor exits. It creates missing default config files when you explicitly open them. No editor is launched until you run the command.

Choose an editor with `[editor] command = ["helix"]`, or an argument array such as `["code", "--wait"]`. Array elements are passed literally, including paths containing spaces; there is no shell evaluation. When the array is empty, Kaiser uses the first nonempty `$VISUAL` or `$EDITOR`, falling back to `helix`. Environment editor strings are split on whitespace; use the array for paths or arguments that contain spaces.

After saving and closing your editor, run `:config-reload`. It validates the entire config, theme, leader, editor settings, and model connection settings before applying any changes. If validation fails, your previous working settings stay active and the error appears in the status bar. Notebook contents, selection, and conversations stay in place. Existing in-flight model requests finish with the settings they started with; new requests use the reloaded settings. Environment overrides still take precedence.

Run `:theme` to see theme suggestions, or type `:theme ` to filter names directly. Built-ins (`kaiser-tallgeese`, `kaiser-bagels`, `kaiser-lavender`, `kaiser-main`, `kaiser-copper`, `kaiser-violet`, `kaiser-ember`, `kaiser-forest`, `kaiser-glacier`, `kaiser-ocean`, `kaiser-orchid`, `kaiser-paper`, `kaiser-sandstone`, `kaiser-slate`) and `.toml` files in the active config directory's `themes/` folder are listed together. Press Tab repeatedly to cycle through matching themes and preview each one immediately; Shift+Tab cycles backward. Arrow selection also previews. Enter keeps the selected theme for the session; Esc restores the exact palette from before the preview. Editing the command or leaving the menu also cancels the preview. You can start with `:theme` and press Tab directly, or type `:theme ` followed by a name prefix. Editing the text starts a new completion cycle. `:theme NAME` applies immediately for this session without modifying your config file. To keep it, set `theme = "NAME"` in `:config-open`, then run `:config-reload`. Reloading config returns to its configured theme.

`:theme-open` edits the current theme's file, creating an editable copy if it is a built-in. `:theme-reload` rereads that file without resetting the rest of your settings. Invalid colors keep the previous palette in place.

Set `[keys] leader = ","` to use a comma instead of Space. Choose `"space"` or a single printable character; an unused punctuation key avoids conflicts with local controls. Run `:config-reload` after changing config.

Use `:theme-open` to edit `~/.config/kaiser/themes/kaiser-tallgeese.toml`, then `:theme-reload` to apply its colors:

```toml
base = "kaiser-tallgeese"
background = "#000000"
panel = "#000000"
border = "#49383C"
foreground = "#FFFFFF"
muted = "#92878A"
accent = "#F7475E"
success = "#F2BEC5"
warning = "#FF7889"
selection = "#48232D"
```

All built-in IDs use the `kaiser-` prefix. Previous names remain supported aliases (`kaiser-home` → `kaiser-main`, `kaiser` → `kaiser-lavender`, `dusk` → `kaiser-violet`, and the other names gain `kaiser-`). Suggestions list only canonical IDs. Existing theme files are carried into the canonical filenames when missing, preserving user edits and the original files.

Theme colors use `#RRGGBB`. Home-page borders use `accent` by default; set the optional `home_border = "#RRGGBB"` to override them independently. `kaiser-main` keeps its original `#49383C` home borders. Workspace borders still use `border`. The `success` role colors selected sources, citations, and approved flashcards; `warning` colors drafts and card prompts. You can copy the file to `themes/personal.toml` and set `theme = "personal"`. Theme paths are relative to the config file's directory. Omitted colors inherit the `base`, which can be any built-in theme. All 14 presets work without a theme file and can serve as a custom theme’s `base`. Unknown themes, invalid colors, and misspelled config fields produce errors before entering terminal mode. Kaiser honors `NO_COLOR`; unset that variable if you want theme colors to appear.

## Optional conversational model

Use `:login` to sign in with a ChatGPT account in your browser. Approve eligible ChatGPT plan usage to enable the study tutor; Kaiser checks the granted permission before making AI requests. `Esc` cancels a pending sign-in. `:login` also reconnects the current account when its session expires or access is revoked.

Use `:account` to switch saved accounts or choose **Continue with ChatGPT** to add an account or workspace. `:model` loads the signed-in account's available model choices; Tab/Shift+Tab selects and Enter confirms. Choose a model before the first question; Kaiser saves that choice per account and never assumes catalog ordering indicates economy. If no model is saved, sending a question opens the chooser and preserves the typed question. `:usage` opens [ChatGPT usage settings](https://chatgpt.com/settings/usage). `:logout` clears this account's local tokens and attempts remote revocation; other saved registrations remain available. If remote revocation cannot be confirmed, Kaiser reports that explicitly. Signing out restores the provider configured in TOML, or offline lookup if none is configured.

ChatGPT connections use the [documented open-source sign-in flow](https://developers.openai.com/siwc/token-sharing-open-source/sign-in) and public Responses API with `store=false`. Plan eligibility and usage limits are controlled by OpenAI. Kaiser receives account identity and the permissions you approve, without access to existing ChatGPT conversations. Questions, recent notebook messages, and matching source excerpts are sent when you ask the tutor. This connection takes precedence over a configured API provider while active.

Credentials and account registrations are saved in `auth/accounts.json` under the selected data directory, separately from `state.json`. Unix permissions are owner-only (directory `0700`, file `0600`); other systems use the user profile's filesystem permissions. Files are written atomically and token renewal uses an OS file lock. Never share the `auth` directory with notebook backups. No API key, client secret, or locally installed Codex is required. Browser sign-in and AI requests require an internet connection; document import and offline study remain local. Sign-in is disabled in demo and snapshot sessions.

Without a connected ChatGPT account or configured API provider, Kaiser performs **local keyword retrieval**, showing literal source excerpts. It does not pretend to generate explanations. Set `api_url` and `model` in the config file for an OpenAI-compatible chat-completions server. You can also override these settings through the environment:

```sh
export KAISER_API_URL="http://localhost:1234/v1"
export KAISER_MODEL="your-loaded-model"
# Only if your provider requires authentication:
# export KAISER_API_KEY="..."
cargo run --release
```

`KAISER_API_URL` is the base URL, not the full `/chat/completions` endpoint. Both the URL and model must be set, through config or environment. `KAISER_API_URL` and `KAISER_MODEL` override the corresponding config values. `api_key_env` chooses which environment variable supplies credentials; the default is `KAISER_API_KEY`. Requests run on a background thread with a 120-second timeout so navigation remains responsive. Errors appear in the status line; credentials and provider response bodies aren't printed or saved. Inference requests are never automatically retried.

### AI usage controls

Defaults apply to ChatGPT and configured API providers. Add or edit this section in your config, then run `:config-reload`:

```toml
[model.usage]
max_input_bytes = 24000
history_bytes = 8000
source_bytes = 12000
max_summary_requests = 16
answer_words = 150
daily_request_limit = 50
cache_entries = 64
cache_ttl_hours = 24
```

The input cap bounds serialized study context, including escaping and message metadata. History is trimmed first; planned evidence is never silently truncated. The full current question and saved conversation remain intact. Oversized questions are rejected before sending. Byte budgets are deterministic UTF-8 size limits, not exact token counts. `answer_words` is a prompt target; the ChatGPT route does not support a hard output-token cap. The API-key route also uses an output-token cap.

Kaiser sends complete extracted source text when it fits the input budget. For larger documents, specific questions retrieve relevant passages within `source_bytes`. Summary requests instead cover the entire selected scope: source parts are summarized, their notes are combined, and a final answer is generated. Before a multi-request summary, a review dialog shows the scope, text size, and maximum request count, with Cancel selected initially. `max_summary_requests` bounds the complete pipeline; the daily allowance must also cover it. Cached stages can reduce actual requests. Progress shows each stage, and a failure stops further requests without automatic retries.

Use scopes such as “summarize chapter 1” or “summarize pages 3–8.” These use numeric chapter headings, imported page/slide markers, or chapter filenames; chapter ranges are not supported. Missing requested scopes produce an error instead of quietly summarizing a different section. Summaries cover all extracted text in the scope, but intermediate notes can lose nuance, and extraction may omit scanned or unsupported content. Inspect the grouped file citations and original sources when detail matters. Existing conversation messages are retained unchanged.

Every inference attempt, including a failed attempt, reserves one of Kaiser's daily requests before the HTTP request starts. Counts reset at midnight UTC; this local reset does not establish any ChatGPT plan reset time. Set `daily_request_limit = 0` to disable the local cap. Restarting, switching accounts, signing in again, or reloading config does not clear the counters. Usage storage errors block new requests rather than bypassing the cap.

Completed answers may be reused only when provider/account, model, bounded conversation context, instructions, and selected-source contents match. The cache stays in memory, is limited to 64 entries by default, and expires entries after 24 hours or on restart. New conversation context generally makes a repeated question a new request. Set either cache setting to zero to disable reuse; `:ai-cache-clear` clears it immediately. Local cache hits do not consume the daily request allowance.

`:ai-usage` shows today's local request attempts, completed answers, cache hits, and token counts actually reported by provider responses. Cached-input and reasoning counts are subsets, not additional totals. Interrupted or failed requests without reported usage remain marked unreported; they may have consumed usage. No remaining ChatGPT quota or dollar estimate is inferred. Usage counters and pauses persist in `ai/usage.json` under the data directory; this file contains no prompts, answers, or credentials, and keeps 90 days of daily records.

A confirmed `subscription_sharing_usage_limit_exceeded` pauses requests for that account until you review `:usage` and explicitly run `:ai-resume`. The pause survives restart and model changes. A generic rate limit or temporarily unavailable service instead causes a bounded cooldown; Kaiser does not infer a plan reset time or repeatedly resend requests. Resuming does not reset the local daily counter. Cached answers and offline document search, notes, and flashcard review remain available while requests are paused. Timeouts and stopping the app cannot guarantee that the provider stopped processing or that usage was avoided.

**Data sent:** bounded recent conversation context and complete text or relevant passages from selected sources. Multi-stage summaries also send intermediate notes between requests. Only configure a server you want to receive that material. API credentials stay in environment variables. Model instructions ask for citations and acknowledge missing evidence, but generated answers still need your judgment. Displayed citations open their source locally.

## Storage

Kaiser saves readable JSON to `$XDG_DATA_HOME/kaiser/state.json`, or `~/.local/share/kaiser/state.json` when XDG_DATA_HOME is unset. Use `--data-dir PATH` for a separate library. Back up or copy this file to preserve everything, including imported source text. It is not encrypted.

Writes use a temporary file and rename. Invalid existing data causes an error instead of being replaced. Run one instance per data directory; simultaneous instances aren't supported. `--demo` is entirely ephemeral. `--snapshot` renders the built-in example without reading or writing your library.

## Prototype scope

- Portable PDF, Office, OpenDocument, image OCR, and text imports; notebook-scoped source selection, local retrieval, and optional model chat.
- Source citations, editable flashcards, and optional card drafts.
- A simple list of all flashcards and terms, with one selected card displayed at a time. Flip to read its definition, move freely between cards, or edit it.
- Definitions stay hidden until you flip the flashcard.
- Model replies always return to the notebook that initiated the request, even if you switch notebooks.

Planned: URL ingestion, embeddings and more sophisticated retrieval, quiz/application prompts, richer note editing, Anki interchange, and sync.

