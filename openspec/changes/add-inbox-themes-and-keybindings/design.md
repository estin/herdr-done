# Design

## Context

See proposal.md - Why. Today `src/inbox.rs` hard-codes every key in `App::handle_key` and every style in `render`, `render_list`, and `render_detail`. There is no configuration file; the only environment knob is `HD_DATABASE` in `src/store.rs`. The project lints deny `unwrap_used`, `expect_used`, and `panic`, so configuration errors must travel as `Result<_, String>` like the rest of the code. `just ci` runs fmt, clippy, and nextest.

The `bonfig` crate (0.1.0) provides layered loading on figment: struct defaults from `#[bonfig::settings]`, optional files via `bonfig::xdg_config_file("herdr-done")` (exactly the XDG path the spec names), and `HD__`-prefixed env vars with `__` nesting. Its default features include TOML. Verified against the crate README and source.

## Goals / Non-Goals

**Goals:**
- One settings struct tree, loaded once before the inbox opens
- Defaults that reproduce today's keys and look byte for byte
- Theme presets plus per-color overrides, custom bindings per action

**Non-Goals:**
- No config for other commands (`hd run`, `hd event`, alerts, storage)
- No `--config` flag, no runtime reload, no config-writing command
- No remapping of double `Ctrl-C` quit
- No per-event or per-workspace themes

## Decisions

### D1: bonfig with default features, one loader
Add `bonfig = "0.1"` with default features (toml + tracing). Define one `Loader` in `src/config.rs`: `env_prefix("HD")`, optional file `bonfig::xdg_config_file("herdr-done")`. Alternative: hand-rolled TOML loading with `toml` + `serde` - rejected because the user asked for bonfig and it already implements the exact layering and XDG path. Alternative: disable the `tracing` default feature - keep it, it is small and the crate logs loads through it; hd installs no subscriber so nothing prints.

### D2: Settings shape
`#[bonfig::settings]` structs in `src/config.rs`:

```toml
theme = "dark"            # "dark" | "light"

[colors]                  # every key optional, overrides the theme
title = "#ffffff"
unread = "yellow"
secondary = "gray"
success = "green"
failure = "red"
running = "yellow"

[keys]                    # every key optional, list of key names
down = ["down", "j"]
up = ["up", "k"]
inspect = ["enter"]
back = ["q", "esc"]
next_unread = ["n"]
mark_read = ["r"]
mark_unread = ["u"]
jump = ["J"]
clear_read = ["c"]
```

`Settings { theme: String, colors: Colors, keys: Keys }`. `Colors` fields are `Option<String>` (None = take the theme value); `Keys` fields are `Vec<String>` with the current keys as `#[builder(default = ...)]`. `#[bonfig::settings]` adds `#[serde(default)]`, so partial files merge key by key. Alternative: `BTreeMap<String, String>` for keys - rejected, typed fields catch typos in action names at deserialization time.

### D3: Theme resolution and selection highlight
Two preset `Theme` values built from `ratatui::style::Color` and `Style`. Resolution: pick the preset by `theme`, then apply each `Some` color override. Colors parse from named ratatui colors (case-insensitive, e.g. `gray`, `light-blue`) or `#RRGGBB`. The selection highlight stays `Modifier::REVERSED` in both presets and is not configurable - a reversed highlight is readable on any terminal background, and a color-valued highlight would need foreground/background pairs. There is no `selection` config key; both presets carry the reversed highlight. Alternative: configurable highlight fg/bg pair - deferred until someone asks.

Dark preset matches today's rendering: default terminal colors, bold title, gray secondary, reversed selection, green success, red failure, yellow running, yellow unread marker. Light preset: same structure with colors chosen for light backgrounds (e.g. dark-gray secondary, blue title accent). Exact values live in code, not the spec.

### D4: Key parsing
Parse each binding string in `src/config.rs` into `crossterm::event::KeyEvent`: single characters map to `KeyCode::Char` (case-sensitive, so `J` means shift-j as today), named keys `enter`, `esc`, `up`, `down`, `left`, `right`, `tab`, `space`, `home`, `end`, `pageup`, `pagedown`, `backspace`, `delete`, `f1`-`f12`, and `ctrl-<char>` add `KeyModifiers::CONTROL`. Unknown names are an error naming the key path (e.g. `keys.jump`). Reject any binding equal to `Ctrl-C` so quit stays fixed. Parse once at load into a `Bindings` struct of `Vec<KeyEvent>` per action; `App::handle_key` matches against it instead of literal `KeyCode`s. Alternative: parse on every keypress - rejected, errors must surface before the inbox opens.

### D5: Wiring
`main.rs` loads `Settings` before `inbox::run` for `hd inbox` and `hd next`; a load error prints and returns failure without touching the terminal. `inbox::run` takes the resolved `Theme` and `Bindings` (or the `Settings` and resolves internally - implementer picks the smaller diff; resolution itself lives in `config.rs`). `App` stores bindings; render functions take the theme. The footer help string is generated from the active bindings. Existing tests construct `App` with defaults via a `Default` impl or a test helper so most tests stay unchanged.

## Risks / Trade-offs

- [bonfig 0.1.0 is young, 103 downloads] → the API surface used is small (Loader, settings macro, xdg_config_file); figment underneath is mature; pin with `--locked` CI
- [Env var syntax `HD__KEYS__DOWN` needs TOML values, e.g. `'["down"]'`] → document the quoting in README; file config is the primary path
- [Users can bind the same key to two actions] → first matching action in a fixed order wins; document that bindings should be unique; not an error
- [Light theme colors are guesses without user testing] → per-color overrides let users fix any single color without waiting for a release
