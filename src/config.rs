use bonfig::FromSource as _;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::{Color, Modifier, Style};
use std::collections::BTreeMap;

#[bonfig::settings]
#[derive(Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    #[builder(default = "dark")]
    pub theme: String,
    pub base16: Option<String>,
    pub colors: Colors,
    pub keys: Keys,
}

#[bonfig::settings]
#[derive(Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Colors {
    pub background: Option<String>,
    pub text: Option<String>,
    pub title: Option<String>,
    pub unread: Option<String>,
    pub secondary: Option<String>,
    pub success: Option<String>,
    pub failure: Option<String>,
    pub running: Option<String>,
}

#[bonfig::settings]
#[derive(Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Keys {
    #[builder(default = vec!["down".to_owned(), "j".to_owned()])]
    pub down: Vec<String>,
    #[builder(default = vec!["up".to_owned(), "k".to_owned()])]
    pub up: Vec<String>,
    #[builder(default = vec!["enter".to_owned()])]
    pub inspect: Vec<String>,
    #[builder(default = vec!["q".to_owned(), "esc".to_owned()])]
    pub back: Vec<String>,
    #[builder(default = vec!["n".to_owned()])]
    pub next_unread: Vec<String>,
    #[builder(default = vec!["r".to_owned()])]
    pub mark_read: Vec<String>,
    #[builder(default = vec!["u".to_owned()])]
    pub mark_unread: Vec<String>,
    #[builder(default = vec!["J".to_owned()])]
    pub jump: Vec<String>,
    #[builder(default = vec!["c".to_owned()])]
    pub clear_read: Vec<String>,
}

#[bonfig::settings]
#[derive(Debug, Clone, PartialEq)]
struct Scheme {
    palette: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Config {
    pub theme: Theme,
    pub bindings: Bindings,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub background: Option<Color>,
    pub text: Option<Color>,
    pub title: Style,
    pub unread: Style,
    pub secondary: Style,
    pub success: Style,
    pub failure: Style,
    pub running: Style,
    pub selection: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    fn dark() -> Self {
        Self {
            background: None,
            text: None,
            title: Style::default().add_modifier(Modifier::BOLD),
            unread: Style::default().fg(Color::Yellow),
            secondary: Style::default().fg(Color::Gray),
            success: Style::default().fg(Color::Green),
            failure: Style::default().fg(Color::Red),
            running: Style::default().fg(Color::Yellow),
            selection: Style::default().add_modifier(Modifier::REVERSED),
        }
    }

    fn light() -> Self {
        Self {
            background: None,
            text: None,
            title: Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            unread: Style::default().fg(Color::Red),
            secondary: Style::default().fg(Color::DarkGray),
            success: Style::default().fg(Color::Green),
            failure: Style::default().fg(Color::Red),
            running: Style::default().fg(Color::Blue),
            selection: Style::default().add_modifier(Modifier::REVERSED),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bindings {
    pub down: Vec<KeyEvent>,
    pub up: Vec<KeyEvent>,
    pub inspect: Vec<KeyEvent>,
    pub back: Vec<KeyEvent>,
    pub next_unread: Vec<KeyEvent>,
    pub mark_read: Vec<KeyEvent>,
    pub mark_unread: Vec<KeyEvent>,
    pub jump: Vec<KeyEvent>,
    pub clear_read: Vec<KeyEvent>,
}

impl Default for Bindings {
    fn default() -> Self {
        Self {
            down: vec![plain(KeyCode::Down), char_key('j')],
            up: vec![plain(KeyCode::Up), char_key('k')],
            inspect: vec![plain(KeyCode::Enter)],
            back: vec![char_key('q'), plain(KeyCode::Esc)],
            next_unread: vec![char_key('n')],
            mark_read: vec![char_key('r')],
            mark_unread: vec![char_key('u')],
            jump: vec![char_key('J')],
            clear_read: vec![char_key('c')],
        }
    }
}

impl Bindings {
    pub fn help(&self) -> String {
        [
            action_help(&self.down, "next"),
            action_help(&self.up, "prev"),
            action_help(&self.inspect, "view"),
            action_help(&self.back, "back"),
            action_help(&self.next_unread, "unread"),
            action_help(&self.mark_read, "read"),
            action_help(&self.mark_unread, "unread"),
            action_help(&self.jump, "jump"),
            action_help(&self.clear_read, "clear read"),
            "Ctrl-C twice".to_owned(),
        ]
        .join(" - ")
    }
}

pub fn load() -> Result<Config, String> {
    let settings: Settings = loader().load().map_err(|error| config_error(&error))?;
    resolve(&settings)
}

fn loader() -> bonfig::Loader {
    bonfig::Loader::builder()
        .env_prefix("HD")
        .files(bonfig::xdg_config_file("herdr-done"))
        .build()
}

fn config_error(error: &bonfig::Error) -> String {
    let detail = error.to_string();
    if error.path.is_empty() {
        format!("configuration error: {detail}")
    } else {
        format!("{}: {detail}", error.path.join("."))
    }
}

fn resolve(settings: &Settings) -> Result<Config, String> {
    let mut theme = if settings.base16.is_some() {
        Theme::dark()
    } else {
        match settings.theme.as_str() {
            "dark" => Theme::dark(),
            "light" => Theme::light(),
            other => return Err(format!("theme: unknown theme {other:?}")),
        }
    };
    if let Some(path) = settings.base16.as_deref() {
        theme = apply_scheme(theme, path)?;
    }
    theme.background = override_optional_color(
        theme.background,
        settings.colors.background.as_deref(),
        "colors.background",
    )?;
    theme.text =
        override_optional_color(theme.text, settings.colors.text.as_deref(), "colors.text")?;
    theme.title = override_color(
        theme.title,
        settings.colors.title.as_deref(),
        "colors.title",
    )?;
    theme.unread = override_color(
        theme.unread,
        settings.colors.unread.as_deref(),
        "colors.unread",
    )?;
    theme.secondary = override_color(
        theme.secondary,
        settings.colors.secondary.as_deref(),
        "colors.secondary",
    )?;
    theme.success = override_color(
        theme.success,
        settings.colors.success.as_deref(),
        "colors.success",
    )?;
    theme.failure = override_color(
        theme.failure,
        settings.colors.failure.as_deref(),
        "colors.failure",
    )?;
    theme.running = override_color(
        theme.running,
        settings.colors.running.as_deref(),
        "colors.running",
    )?;
    Ok(Config {
        theme,
        bindings: parse_bindings(&settings.keys)?,
    })
}

fn apply_scheme(mut theme: Theme, path: &str) -> Result<Theme, String> {
    let path = expand_path(path)?;
    let contents = std::fs::read_to_string(&path)
        .map_err(|error| format!("base16: cannot read {path:?}: {error}"))?;
    let scheme = Scheme::from_yaml(&contents)
        .map_err(|error| format!("base16: cannot parse {path:?}: {error}"))?;
    theme.title = theme.title.fg(scheme_color(&scheme, "base0D")?);
    theme.unread = theme.unread.fg(scheme_color(&scheme, "base0A")?);
    theme.secondary = theme.secondary.fg(scheme_color(&scheme, "base04")?);
    theme.background = Some(scheme_color(&scheme, "base00")?);
    theme.text = Some(scheme_color(&scheme, "base05")?);
    theme.success = theme.success.fg(scheme_color(&scheme, "base0B")?);
    theme.failure = theme.failure.fg(scheme_color(&scheme, "base08")?);
    theme.running = theme.running.fg(scheme_color(&scheme, "base0A")?);
    Ok(theme)
}

fn scheme_color(scheme: &Scheme, key: &str) -> Result<Color, String> {
    let value = scheme
        .palette
        .get(key)
        .ok_or_else(|| format!("base16: palette.{key} missing"))?;
    parse_color(value, &format!("base16: palette.{key}"))
}

fn expand_path(path: &str) -> Result<String, String> {
    let Some(rest) = path.strip_prefix("~/") else {
        return Ok(path.to_owned());
    };
    let home = std::env::var_os("HOME")
        .ok_or_else(|| format!("base16: cannot expand {path:?}, HOME is unset"))?;
    let home = home.to_string_lossy();
    Ok(format!("{}/{}", home.trim_end_matches('/'), rest))
}

fn override_optional_color(
    current: Option<Color>,
    value: Option<&str>,
    key: &str,
) -> Result<Option<Color>, String> {
    match value {
        None => Ok(current),
        Some(value) => match parse_color(value, key)? {
            Color::Reset => Ok(None),
            color => Ok(Some(color)),
        },
    }
}

fn override_color(style: Style, value: Option<&str>, key: &str) -> Result<Style, String> {
    match value {
        None => Ok(style),
        Some(value) => Ok(style.fg(parse_color(value, key)?)),
    }
}

fn parse_color(value: &str, key: &str) -> Result<Color, String> {
    let normalized = value.trim().to_ascii_lowercase();
    let color = match normalized.as_str() {
        "reset" => Color::Reset,
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" => Color::Gray,
        "darkgray" | "dark-gray" | "darkgrey" | "dark-grey" => Color::DarkGray,
        "lightred" | "light-red" => Color::LightRed,
        "lightgreen" | "light-green" => Color::LightGreen,
        "lightyellow" | "light-yellow" => Color::LightYellow,
        "lightblue" | "light-blue" => Color::LightBlue,
        "lightmagenta" | "light-magenta" => Color::LightMagenta,
        "lightcyan" | "light-cyan" => Color::LightCyan,
        "white" => Color::White,
        _ => {
            let Some(digits) = normalized.strip_prefix('#') else {
                return Err(format!("{key}: unknown color {value:?}"));
            };
            if digits.len() != 6 || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(format!("{key}: invalid color {value:?}, use #RRGGBB"));
            }
            let hex = |range: std::ops::Range<usize>| {
                u8::from_str_radix(&digits[range], 16)
                    .map_err(|_| format!("{key}: invalid color {value:?}, use #RRGGBB"))
            };
            Color::Rgb(hex(0..2)?, hex(2..4)?, hex(4..6)?)
        }
    };
    Ok(color)
}

fn parse_bindings(keys: &Keys) -> Result<Bindings, String> {
    Ok(Bindings {
        down: parse_keys(&keys.down, "keys.down")?,
        up: parse_keys(&keys.up, "keys.up")?,
        inspect: parse_keys(&keys.inspect, "keys.inspect")?,
        back: parse_keys(&keys.back, "keys.back")?,
        next_unread: parse_keys(&keys.next_unread, "keys.next_unread")?,
        mark_read: parse_keys(&keys.mark_read, "keys.mark_read")?,
        mark_unread: parse_keys(&keys.mark_unread, "keys.mark_unread")?,
        jump: parse_keys(&keys.jump, "keys.jump")?,
        clear_read: parse_keys(&keys.clear_read, "keys.clear_read")?,
    })
}

fn parse_keys(values: &[String], key: &str) -> Result<Vec<KeyEvent>, String> {
    values.iter().map(|value| parse_key(value, key)).collect()
}

fn parse_key(value: &str, key: &str) -> Result<KeyEvent, String> {
    let trimmed = value.trim();
    let lowered = trimmed.to_ascii_lowercase();
    let event = if let Some(rest) = lowered.strip_prefix("ctrl-") {
        let mut chars = rest.chars();
        let Some(ch) = chars.next() else {
            return Err(format!("{key}: unknown key {value:?}"));
        };
        if chars.next().is_some() || !ch.is_ascii() {
            return Err(format!("{key}: unknown key {value:?}"));
        }
        KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL)
    } else {
        match lowered.as_str() {
            "enter" => plain(KeyCode::Enter),
            "esc" | "escape" => plain(KeyCode::Esc),
            "up" => plain(KeyCode::Up),
            "down" => plain(KeyCode::Down),
            "left" => plain(KeyCode::Left),
            "right" => plain(KeyCode::Right),
            "tab" => plain(KeyCode::Tab),
            "space" => char_key(' '),
            "home" => plain(KeyCode::Home),
            "end" => plain(KeyCode::End),
            "pageup" | "page-up" => plain(KeyCode::PageUp),
            "pagedown" | "page-down" => plain(KeyCode::PageDown),
            "backspace" => plain(KeyCode::Backspace),
            "delete" => plain(KeyCode::Delete),
            "f1" => plain(KeyCode::F(1)),
            "f2" => plain(KeyCode::F(2)),
            "f3" => plain(KeyCode::F(3)),
            "f4" => plain(KeyCode::F(4)),
            "f5" => plain(KeyCode::F(5)),
            "f6" => plain(KeyCode::F(6)),
            "f7" => plain(KeyCode::F(7)),
            "f8" => plain(KeyCode::F(8)),
            "f9" => plain(KeyCode::F(9)),
            "f10" => plain(KeyCode::F(10)),
            "f11" => plain(KeyCode::F(11)),
            "f12" => plain(KeyCode::F(12)),
            _ => {
                let mut chars = trimmed.chars();
                let Some(ch) = chars.next() else {
                    return Err(format!("{key}: unknown key {value:?}"));
                };
                if chars.next().is_some() {
                    return Err(format!("{key}: unknown key {value:?}"));
                }
                char_key(ch)
            }
        }
    };
    if event.code == KeyCode::Char('c') && event.modifiers.contains(KeyModifiers::CONTROL) {
        return Err(format!("{key}: ctrl-c cannot be remapped"));
    }
    Ok(event)
}

fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn char_key(ch: char) -> KeyEvent {
    plain(KeyCode::Char(ch))
}

fn action_help(keys: &[KeyEvent], label: &str) -> String {
    let labels: Vec<String> = keys.iter().copied().map(key_label).collect();
    if labels.is_empty() {
        label.to_owned()
    } else {
        format!("{} {label}", labels.join("/"))
    }
}

fn key_label(key: KeyEvent) -> String {
    let name = match key.code {
        KeyCode::Char(' ') => "Space".to_owned(),
        KeyCode::Char(ch) => ch.to_string(),
        code => format!("{code:?}"),
    };
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        format!("Ctrl-{name}")
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::result_large_err)]

    use super::{
        Bindings, Color, KeyCode, KeyEvent, KeyModifiers, Modifier, Settings, config_error, load,
        parse_color, plain, resolve,
    };
    use bonfig::FromSource as _;
    use figment::Jail;
    use serde_json::json;
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;

    fn char_key(ch: char) -> KeyEvent {
        plain(KeyCode::Char(ch))
    }

    fn write_config(jail: &mut Jail, contents: &str) -> figment::Result<()> {
        let _ = jail.create_dir("herdr-done")?;
        let _ = jail.create_file("herdr-done/config.toml", contents)?;
        Ok(())
    }

    const SPEC_011_SCHEME: &str = r##"system: "base16"
name: "Test Scheme"
author: "Test"
variant: "dark"
palette:
  base00: "#000000"
  base01: "#111111"
  base02: "#222222"
  base03: "#333333"
  base04: "#444444"
  base05: "#555555"
  base06: "#666666"
  base07: "#777777"
  base08: "#888888"
  base09: "#999999"
  base0A: "#aaaaaa"
  base0B: "#bbbbbb"
  base0C: "#cccccc"
  base0D: "#dddddd"
  base0E: "#eeeeee"
  base0F: "#ffffff"
"##;

    fn write_scheme(dir: &Path, name: &str, contents: &str) -> Result<String, String> {
        let path = dir.join(name);
        fs::write(&path, contents).map_err(|error| error.to_string())?;
        Ok(path.to_string_lossy().into_owned())
    }

    fn settings_with_base16(path: &str) -> Result<Settings, String> {
        Settings::from_serialize(json!({ "base16": path })).map_err(|error| config_error(&error))
    }

    fn resolve_error_owned(settings: &Settings) -> String {
        match resolve(settings) {
            Ok(_) => "expected a configuration error".to_owned(),
            Err(error) => error,
        }
    }

    fn resolve_error(contents: &str) -> String {
        let settings = match Settings::from_toml(contents) {
            Ok(settings) => settings,
            Err(error) => return config_error(&error),
        };
        match resolve(&settings) {
            Ok(_) => "expected a configuration error".to_owned(),
            Err(error) => error,
        }
    }

    #[test]
    fn defaults_load_with_no_file() {
        Jail::expect_with(|jail| {
            jail.set_env("XDG_CONFIG_HOME", ".");
            let config = load().map_err(figment::Error::from)?;
            assert_eq!(config.theme.title.fg, None);
            assert!(config.theme.title.add_modifier.contains(Modifier::BOLD));
            assert_eq!(
                config.bindings.down,
                vec![plain(KeyCode::Down), char_key('j')]
            );
            assert_eq!(config.bindings.jump, vec![char_key('J')]);
            Ok(())
        });
    }

    #[test]
    fn partial_toml_merges_key_by_key() {
        Jail::expect_with(|jail| {
            jail.set_env("XDG_CONFIG_HOME", ".");
            write_config(jail, "theme = \"light\"\n\n[keys]\ndown = [\"down\"]\n")?;
            let config = load().map_err(figment::Error::from)?;
            assert_eq!(config.theme.title.fg, Some(Color::Blue));
            assert_eq!(config.bindings.down, vec![plain(KeyCode::Down)]);
            assert_eq!(config.bindings.up, vec![plain(KeyCode::Up), char_key('k')]);
            Ok(())
        });
    }

    #[test]
    fn env_var_overrides_file() {
        Jail::expect_with(|jail| {
            jail.set_env("XDG_CONFIG_HOME", ".");
            write_config(jail, "theme = \"dark\"\n\n[keys]\nnext_unread = [\"n\"]\n")?;
            jail.set_env("HD__THEME", r#""light""#);
            jail.set_env("HD__KEYS__NEXT_UNREAD", r#"["N"]"#);
            let config = load().map_err(figment::Error::from)?;
            assert_eq!(config.theme.title.fg, Some(Color::Blue));
            assert_eq!(config.bindings.next_unread, vec![char_key('N')]);
            Ok(())
        });
    }

    #[test]
    fn color_override_replaces_one_theme_color() -> Result<(), String> {
        let settings = Settings::from_toml("theme = \"light\"\n\n[colors]\ntitle = \"#123456\"\n")
            .map_err(|error| config_error(&error))?;
        let config = resolve(&settings)?;
        assert_eq!(config.theme.title.fg, Some(Color::Rgb(0x12, 0x34, 0x56)));
        assert_eq!(config.theme.secondary.fg, Some(Color::DarkGray));
        Ok(())
    }

    #[test]
    fn named_colors_are_accepted() -> Result<(), String> {
        assert_eq!(parse_color("Light-Blue", "colors.title")?, Color::LightBlue);
        assert_eq!(parse_color("gray", "colors.title")?, Color::Gray);
        Ok(())
    }

    #[test]
    fn unknown_key_error_names_the_key() {
        let settings = Settings::from_toml("unknown = 1");
        let error = match settings {
            Ok(_) => "expected a configuration error".to_owned(),
            Err(error) => config_error(&error),
        };
        assert!(error.contains("unknown"), "{error}");
    }

    #[test]
    fn unknown_theme_error_names_the_key() {
        let error = resolve_error("theme = \"sepia\"");
        assert!(error.contains("theme"), "{error}");
    }

    #[test]
    fn invalid_color_error_names_the_key() {
        let error = resolve_error("[colors]\ntitle = \"sunset\"");
        assert!(error.contains("colors.title"), "{error}");
    }

    #[test]
    fn invalid_key_error_names_the_key() {
        let error = resolve_error("[keys]\ndown = [\"upup\"]");
        assert!(error.contains("keys.down"), "{error}");
    }

    #[test]
    fn ctrl_c_binding_is_rejected() {
        let error = resolve_error("[keys]\ndown = [\"ctrl-c\"]");
        assert!(error.contains("keys.down"), "{error}");
        assert!(error.contains("ctrl-c"), "{error}");
    }

    #[test]
    fn base16_scheme_maps_all_eight_colors() -> Result<(), String> {
        let dir = tempdir().map_err(|error| error.to_string())?;
        let path = write_scheme(dir.path(), "scheme.yaml", SPEC_011_SCHEME)?;
        let settings = settings_with_base16(&path)?;
        let config = resolve(&settings)?;
        assert_eq!(config.theme.title.fg, Some(Color::Rgb(0xdd, 0xdd, 0xdd)));
        assert_eq!(config.theme.unread.fg, Some(Color::Rgb(0xaa, 0xaa, 0xaa)));
        assert_eq!(
            config.theme.secondary.fg,
            Some(Color::Rgb(0x44, 0x44, 0x44))
        );
        assert_eq!(config.theme.success.fg, Some(Color::Rgb(0xbb, 0xbb, 0xbb)));
        assert_eq!(config.theme.failure.fg, Some(Color::Rgb(0x88, 0x88, 0x88)));
        assert_eq!(config.theme.running.fg, Some(Color::Rgb(0xaa, 0xaa, 0xaa)));
        assert_eq!(config.theme.background, Some(Color::Rgb(0x00, 0x00, 0x00)));
        assert_eq!(config.theme.text, Some(Color::Rgb(0x55, 0x55, 0x55)));
        Ok(())
    }

    #[test]
    fn colors_override_the_scheme() -> Result<(), String> {
        let dir = tempdir().map_err(|error| error.to_string())?;
        let path = write_scheme(dir.path(), "scheme.yaml", SPEC_011_SCHEME)?;
        let settings = Settings::from_serialize(json!({
            "base16": path,
            "colors": {
                "title": "#123456",
                "background": "#010203",
                "text": "#fefefe",
            },
        }))
        .map_err(|error| config_error(&error))?;
        let config = resolve(&settings)?;
        assert_eq!(config.theme.title.fg, Some(Color::Rgb(0x12, 0x34, 0x56)));
        assert_eq!(config.theme.unread.fg, Some(Color::Rgb(0xaa, 0xaa, 0xaa)));
        assert_eq!(
            config.theme.secondary.fg,
            Some(Color::Rgb(0x44, 0x44, 0x44))
        );
        assert_eq!(config.theme.background, Some(Color::Rgb(0x01, 0x02, 0x03)));
        assert_eq!(config.theme.text, Some(Color::Rgb(0xfe, 0xfe, 0xfe)));
        Ok(())
    }

    #[test]
    fn background_reset_overrides_the_scheme() -> Result<(), String> {
        let dir = tempdir().map_err(|error| error.to_string())?;
        let path = write_scheme(dir.path(), "scheme.yaml", SPEC_011_SCHEME)?;
        let settings = Settings::from_serialize(json!({
            "base16": path,
            "colors": { "background": "reset" },
        }))
        .map_err(|error| config_error(&error))?;
        let config = resolve(&settings)?;
        assert_eq!(config.theme.background, None);
        assert_eq!(config.theme.text, Some(Color::Rgb(0x55, 0x55, 0x55)));
        Ok(())
    }

    #[test]
    fn base16_errors_name_the_file_or_palette_key() -> Result<(), String> {
        let dir = tempdir().map_err(|error| error.to_string())?;
        let missing = dir
            .path()
            .join("missing.yaml")
            .to_string_lossy()
            .into_owned();
        let invalid = write_scheme(
            dir.path(),
            "invalid.yaml",
            "palette: [not, a, map]
",
        )?;
        let incomplete = write_scheme(
            dir.path(),
            "incomplete.yaml",
            &SPEC_011_SCHEME.replace("  base0D: \"#dddddd\"\n", ""),
        )?;
        let bad_hex = write_scheme(
            dir.path(),
            "bad-hex.yaml",
            &SPEC_011_SCHEME.replace("#dddddd", "nope"),
        )?;
        let cases = [
            (missing.as_str(), "missing.yaml"),
            (invalid.as_str(), "invalid.yaml"),
            (incomplete.as_str(), "base0D"),
            (bad_hex.as_str(), "base0D"),
        ];
        for (path, expected) in cases {
            let settings = settings_with_base16(path)?;
            let error = resolve_error_owned(&settings);
            assert!(error.starts_with("base16:"), "{error}");
            assert!(error.contains(expected), "{error}");
        }
        Ok(())
    }

    #[test]
    fn without_base16_resolution_is_unchanged() -> Result<(), String> {
        let settings =
            Settings::from_toml("theme = \"light\"").map_err(|error| config_error(&error))?;
        let config = resolve(&settings)?;
        assert_eq!(config.theme.title.fg, Some(Color::Blue));
        assert_eq!(config.theme.unread.fg, Some(Color::Red));
        assert_eq!(config.theme.secondary.fg, Some(Color::DarkGray));
        Ok(())
    }

    #[test]
    fn help_includes_all_default_actions() {
        let help = Bindings::default().help();
        assert!(help.contains("j next"), "{help}");
        assert!(help.contains("Ctrl-C twice"), "{help}");
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(!Bindings::default().down.contains(&ctrl_c));
    }
}
