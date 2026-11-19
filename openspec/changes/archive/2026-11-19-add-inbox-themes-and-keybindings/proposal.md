# Proposal

## Why

The `hd inbox` look and key bindings are hard-coded in `src/inbox.rs`. Users cannot adapt the colors to their terminal theme or the keys to their habits without a rebuild. A small configuration file with sane defaults solves both.

## What Changes

- Load layered configuration with the `bonfig` crate: built-in defaults, then an optional TOML file at `$XDG_CONFIG_HOME/herdr-done/config.toml` (fallback `$HOME/.config/herdr-done/config.toml`), then `HD__`-prefixed environment variables
- Add inbox themes: configurable colors for the inbox title, unread marker, secondary text, and success, failure, and running result labels; the selection highlight stays a fixed readable style
- Ship two built-in themes, `dark` (default) and `light`, selected with the `theme` key; individual `[colors]` keys override the selected theme
- Add custom keybindings: each inbox action (down, up, inspect, back, next unread, mark read, mark unread, jump, clear read) accepts a list of key names; defaults reproduce today's keys exactly
- Keep double `Ctrl-C` as the fixed quit behavior; it is not remappable
- Render the inbox footer help from the active bindings instead of a fixed string
- Report invalid configuration (unknown keys, bad colors, unparsable keys) as an error before the inbox opens; a missing config file is not an error

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `command-inbox`: the interactive inbox requirement gains theme and keybinding configuration; a new requirement covers configuration loading, defaults, and validation

## Impact

- `Cargo.toml`: add `bonfig = "0.1"` with default features (TOML support)
- New `src/config.rs` for the settings structs, theme presets, and key parsing
- `src/inbox.rs`: `App` and render functions take resolved theme and bindings; footer help is generated
- `src/main.rs`: load configuration before opening the inbox (`hd inbox` and `hd next`)
- `README.md`: document the config file, theme keys, and keybinding syntax
- No changes to storage, event creation, alerts, or non-interactive commands
