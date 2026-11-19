# Proposal

## Why

The inbox ships only two built-in themes. Users of tinted-theming (base16) already keep their terminal, editor, and tools on one scheme from the tinted-theming/schemes repository. Letting the inbox read the same scheme file keeps every tool on one palette without hand-copying hex colors.

## What Changes

- Add an optional `base16` configuration key: a path to a local tinted-theming scheme YAML file (spec-0.11 format, `palette` with `base00`-`base0F` hex colors)
- When set, the scheme palette replaces the built-in theme colors, including the inbox background (`base00`) and default text (`base05`); `[colors]` overrides still win on top
- Add `background` and `text` to the configurable `[colors]` keys; built-in presets leave both unset so the terminal colors show through as today
- Color layer order becomes: built-in preset < base16 scheme < `[colors]` overrides, with `HD__` env vars overriding the configuration file as today
- Enable bonfig's `yaml` feature to parse scheme files; no network access, users download scheme files themselves
- A missing, unreadable, or invalid scheme file is an error that names the `base16` key and prevents the inbox from opening

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `command-inbox`: a new requirement covers loading a base16 scheme file as the inbox theme; the theme-colors requirement gains the scheme layer between preset and overrides

## Impact

- `Cargo.toml`: enable the `yaml` feature on the existing `bonfig` dependency
- `src/config.rs`: `base16` setting, scheme parsing, palette-to-theme mapping, resolution order
- `README.md`: document the `base16` key, where to get scheme files, and the mapping
- No changes to keybindings, storage, alerts, or non-interactive commands
