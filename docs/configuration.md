# Configuration

Kaiser uses TOML. The complete commented reference is [config/config.toml](../config/config.toml); settings you omit use its defaults. Existing files are preserved during initialization.

## Find, edit, and validate

The default is `$XDG_CONFIG_HOME/kaiser/config.toml`, or `~/.config/kaiser/config.toml` when XDG_CONFIG_HOME is unset. `--config PATH` selects another file and treats a missing file as an error.

```sh
kaiser --init-config
kaiser --check-config
kaiser --check-config --config ./config/config.toml
```

`--check-config` validates settings, the selected theme, and effective API-provider configuration, then exits without opening the TUI or creating study data. It requires the config file to exist. Errors name the file or invalid setting; the exit status is nonzero.

Inside Kaiser, `:config-open` opens the config in your editor and `:config-reload` applies it. A failed reload keeps the previous settings. `:theme-open` and `:theme-reload` do the same for theme colors. Editor changes become active only after reloading.

## Minimal config

```toml
theme = "kaiser-main"

[keys]
leader = "space"

[ui]
transparent_background = true
```

The leader is a sequential prefix; press Space, release it, then the command key. Super is the keyboard modifier used by `<super> + f/n/h`; it is separate from the configurable leader. Unknown fields and malformed colors are rejected instead of silently ignored.

| Section | Purpose |
| --- | --- |
| `theme` | Canonical built-in name or a custom theme filename without `.toml` |
| `[keys]` | `leader`: `space` or one printable character |
| `[ui]` | `sidebar`, `transparent_background` |
| `[editor]` | `command`: literal argument array; empty uses VISUAL, then EDITOR, then `helix` |
| `[model]` | Optional API base URL, model, and credential environment-variable name |
| `[model.usage]` | Serialized input/history/source byte limits, output target, daily/request limits, cache bounds |

Explicit editor arrays preserve paths with spaces, for example `command = ["code", "--wait"]`. VISUAL/EDITOR fallback values are split on whitespace; use an explicit array for complex arguments. No editor command is run through a shell.

## Themes

Create `themes/my-theme.toml` beside your config:

```toml
base = "kaiser-bagels"
accent = "#B9A6FF"
border = "#665A80"
home_border = "#B9A6FF"
```

Then set `theme = "my-theme"`, reload, or run `:theme my-theme`. All color fields use `#RRGGBB`. A custom theme may override `background`, `panel`, `border`, `home_border`, `foreground`, `muted`, `accent`, `success`, `warning`, and `selection`. Bases are built-in themes, not other custom files. Omitting a base uses Tallgeese.

Tab cycles and previews `:theme` options; Enter keeps the selected theme for this session and Esc restores the prior theme. Set `theme` in the config for persistence. Status surfaces stay neutral across themes. `transparent_background = true` uses the terminal background on every surface, including selections and popups.

## Models and usage

For a ChatGPT account, use `:login` and `:model`; credentials stay outside the config. For an API provider, configure both `api_url` and `model` or supply `KAISER_API_URL` and `KAISER_MODEL`. Those environment variables override their file values. `api_key_env` chooses the credential environment variable, defaulting to `KAISER_API_KEY`; never put credentials directly in TOML.

Byte budgets count serialized UTF-8 context, not exact tokens. `daily_request_limit = 0` disables the local daily cap; setting either cache bound to zero disables caching. Multi-stage summaries and card generation remain bounded by `max_summary_requests`, and larger plans require review before sending. Card generation uses a count-based output target up to 1200 words. See the [AI usage controls](user-guide.md#ai-usage-controls) for accounting, cooldowns, cancellation, and cache behavior.

Config, study data, and authentication have separate storage. `--data-dir PATH` changes study/account storage; it does not change the config path. Theme selection does not rewrite notebooks, sources, or cards.
