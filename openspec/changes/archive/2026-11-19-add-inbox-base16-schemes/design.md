# Design

## Context

See proposal.md - Why. The themes change is already implemented and archived: `src/config.rs` holds `Settings { theme, colors, keys }` with `#[bonfig::settings]` and `deny_unknown_fields`, a `resolve()` that picks the dark/light preset then applies `override_color` per key, and a shared `parse_color` for named colors and `#RRGGBB`. bonfig 0.1 is already a dependency; its `yaml` feature (figment/yaml) is available but not enabled. Verified spec-0.11 scheme format from tinted-theming/schemes: top-level `system`, `name`, `author`, `variant`, and `palette` mapping `base00`-`base0F` to `"#RRGGBB"` strings.

## Goals / Non-Goals

**Goals:**
- One new optional key, reusing the existing color parser and override machinery
- Scheme errors surface before the inbox opens, naming `base16`

**Non-Goals:**
- No network fetching, no scheme browser or installer command
- No base24 or tinted8 support (different palettes; base16 only)
- No background changes for the built-in presets - they keep the terminal background

## Decisions

### D1: Parse with bonfig's yaml feature, not a new crate
Enable `features = ["yaml"]` on the existing bonfig dependency. Define a plain serde struct in `src/config.rs`:

```rust
#[derive(Deserialize)]
struct Scheme { palette: BTreeMap<String, String> }
```

Read the file with `std::fs::read_to_string` and parse with `bonfig::FromSource::from_yaml` (implemented for every Default + Serialize + Deserialize type; give `Scheme` a Default or parse via a `#[bonfig::settings]` struct - implementer picks whichever compiles smallest). Alternative: add `serde_yaml` directly - rejected, it is deprecated upstream and bonfig already carries a maintained YAML path through figment. Alternative: runtime HTTP fetch from GitHub - rejected, adds a dependency and failure modes for no gain; users download scheme files themselves.

### D2: Resolution order in resolve()
`Settings` gains `base16: Option<String>` (None default); `Colors` gains `background` and `text` (both `Option<String>`, None in both presets). `Theme` gains `background: Option<Color>` and `text: Option<Color>`. In `resolve()`: pick the preset by `theme` as today; if `base16` is `Some(path)`, read and parse the scheme and set the theme colors from the mapping (title=base0D, unread=base0A, running=base0A, secondary=base04, success=base0B, failure=base08, background=base00, text=base05) using the existing `parse_color`; then apply the `[colors]` overrides, including the two new keys, unchanged in mechanism. The scheme replaces the preset entirely, so `theme` is ignored when `base16` is set; `variant` in the file is informational only. Secondary text moves from base03 to base04: base03 is a comments color and is too dim against base00; base04 is the spec's dark-foreground tone and stays readable on the scheme background. Missing palette entries or bad hex values are errors prefixed `base16:` including the palette key, e.g. `base16: palette.base0D missing`. File read/parse errors also use the `base16:` prefix. Alternative: merge scheme over preset per-key - rejected, a partial palette should fail loudly, not silently mix palettes.

### D3: Rendering the background
When `Theme.background` is set, `render()` first paints the full frame area with a `Block` styled `bg(background).fg(text)` (ratatui `Block::default().style(...)` over `frame.area()`), then renders the existing widgets on top. Widget styles only set foregrounds today, so the painted background shows through everywhere, including borders and empty rows; the reversed selection highlight flips the scheme fg/bg and stays readable. When background is None (built-in presets, no override), rendering is byte-identical to today. Alternative: set `.style()` on every widget and block - rejected, more touch points and easy to miss empty cells.

### D4: Path handling
Use the path as given; expand a leading `~/` to `$HOME` (one small helper or `shellexpand`-free manual strip - no new dependency). Relative paths resolve against the process working directory, documented in README. Env override works for free: `HD__BASE16=/path/to/scheme.yaml`.

## Risks / Trade-offs

- [Spec-0.11 files may carry extra top-level fields] → the `Scheme` struct only reads `palette` and ignores unknown fields (no deny_unknown_fields on it)
- [A painted background fights terminals with transparency or a different palette] → background only applies when a scheme or explicit override sets it; `[colors] background = "reset"` restores the terminal background
- [Users point base16 at a base24 file] → required base00-range keys we map are present in base24 too, so it degrades gracefully; document base16 as the supported format
