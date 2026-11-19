# Tasks

## 1. Configuration foundation

- [x] 1.1 Add `bonfig = "0.1"` to Cargo.toml and refresh Cargo.lock
- [x] 1.2 Create `src/config.rs` with `#[bonfig::settings]` structs `Settings`, `Colors`, `Keys` and the defaults from design D2, register the module in `src/lib.rs`
- [x] 1.3 Add the loader (`env_prefix("HD")`, optional `bonfig::xdg_config_file("herdr-done")`) and a `load()` that maps errors to `String` naming the offending key
- [x] 1.4 Implement color parsing (named ratatui colors, `#RRGGBB`) and the dark and light `Theme` presets with per-color overrides (design D3)
- [x] 1.5 Implement key binding parsing into `KeyEvent` lists, rejecting unknown names and `Ctrl-C` with an error naming the key path (design D4)
- [x] 1.6 Unit tests: defaults load with no file, partial TOML merges key by key, env var overrides file (use `FromSource`/temp files), invalid color/key/theme errors name the key, `Ctrl-C` binding is rejected

## 2. Inbox integration

- [x] 2.1 Pass resolved `Theme` and `Bindings` from `main.rs` through `inbox::run` into `App` and the render functions; load errors print and exit before the terminal opens
- [x] 2.2 Replace literal key matches in `App::handle_key` with binding lookups, keeping view-specific behavior (inspect only in list, back returns from detail, quit notice in list) and fixed double `Ctrl-C`
- [x] 2.3 Apply theme styles in `render`, `render_list`, `render_detail` (title, unread marker, secondary text, result labels; selection stays reversed)
- [x] 2.4 Generate the footer help from the active bindings
- [x] 2.5 Update existing inbox tests to construct `App` with default bindings; add tests for a custom binding driving an action, a replaced default key doing nothing, and footer text reflecting bindings

## 3. Documentation and verification

- [x] 3.1 Document the config file path, layering, `HD__` env syntax with quoting, themes, color keys, and keybinding names in README.md
- [x] 3.2 Run `just ci` and fix any findings
- [x] 3.3 Manually smoke-test `hd inbox` with no config, a light-theme config, and one custom binding

## Workflow follow-up

- Verify with the openspec-verify-change workflow, then archive when the user asks
