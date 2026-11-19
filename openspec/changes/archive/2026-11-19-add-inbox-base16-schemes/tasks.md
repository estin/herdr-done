# Tasks

## 1. Scheme loading

- [x] 1.1 Enable the `yaml` feature on the bonfig dependency in Cargo.toml and refresh Cargo.lock
- [x] 1.2 Add `base16: Option<String>` to `Settings` and a `Scheme` struct with `palette: BTreeMap<String, String>` parsed via `FromSource::from_yaml`
- [x] 1.3 In `resolve()`, load the scheme when `base16` is set (with `~/` expansion), map the accent colors per design D2 through the existing `parse_color`, and prefix all scheme errors with `base16:` naming the file or palette key
- [x] 1.4 Keep `[colors]` overrides applied after the scheme so they win

## 2. Background and text (new requirement)

- [x] 2.1 Add `background`/`text` to `Colors` and `Theme` (None in both presets); map background=base00, text=base05 in `apply_scheme`; move secondary from base03 to base04
- [x] 2.2 Apply `[colors] background`/`text` overrides after the scheme; `reset` restores the terminal background
- [x] 2.3 In `render()`, paint the full frame with the theme background and text when set (design D3); leave rendering unchanged when unset
- [x] 2.4 Tests: fixture maps all eight colors; empty buffer cells carry the scheme background; without a scheme cells keep the default background; `background = "reset"` overrides the scheme

## 3. Scheme tests (done)

- [x] 3.1 Test: a spec-0.11 fixture file maps the accent colors and ignores extra top-level fields
- [x] 3.2 Test: a `[colors]` key overrides the mapped scheme color while other mapped colors stay
- [x] 3.3 Test: missing file, invalid YAML, missing palette entry, and bad hex each error with the `base16` prefix
- [x] 3.4 Test: without `base16`, resolution is unchanged (existing tests keep passing)

## 4. Documentation and verification

- [x] 4.1 Document the `base16` key in README.md: where to download spec-0.11 scheme files, the palette mapping table, path handling, and that `[colors]` still wins
- [x] 4.2 Update README.md for background/text: mapping table rows, the two new `[colors]` keys, and `reset` behavior
- [x] 4.3 Run `just ci` again after the background work and fix any findings
- [x] 4.4 Smoke-test `hd inbox` with a real scheme: confirm the background covers the whole inbox and one `[colors]` override still wins

## Workflow follow-up

- Verify with the openspec-verify-change workflow, then archive when the user asks
