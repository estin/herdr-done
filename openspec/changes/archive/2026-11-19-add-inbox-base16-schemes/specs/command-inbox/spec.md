# Spec Delta

## ADDED Requirements

### Requirement: Load a base16 scheme as the inbox theme
The system MUST accept an optional `base16` configuration key naming a local tinted-theming scheme file in the spec-0.11 YAML format and MUST map its palette to the inbox colors. Individual `[colors]` overrides MUST still win over the scheme. The system MUST NOT fetch scheme files over the network. An unreadable or invalid scheme file MUST produce an error that names the `base16` key and MUST prevent the inbox from opening.

#### Scenario: Palette mapping
- **WHEN** the system maps a scheme palette to the inbox colors
- **THEN** title uses `base0D`, unread marker and running label use `base0A`, secondary text uses `base04`, success label uses `base0B`, failure label uses `base08`, background uses `base00`, and default text uses `base05`

#### Scenario: Scheme colors are applied
- **WHEN** the configuration sets `base16` to a readable spec-0.11 scheme file
- **THEN** the inbox renders with the mapped palette colors instead of the built-in theme colors

#### Scenario: Scheme background covers the inbox
- **WHEN** the inbox renders with a base16 scheme configured
- **THEN** the whole inbox area, including borders and empty space, uses the scheme background and default text colors

#### Scenario: Color override wins over the scheme
- **WHEN** the configuration sets `base16` and an individual `[colors]` key
- **THEN** that color replaces the mapped scheme value and the other mapped colors stay unchanged

#### Scenario: Scheme file is missing
- **WHEN** `base16` names a file that does not exist or cannot be read
- **THEN** the system reports an error that names the `base16` key and does not open the inbox

#### Scenario: Scheme file is invalid
- **WHEN** the scheme file is not valid spec-0.11 YAML, lacks a required palette entry, or contains an invalid hex color
- **THEN** the system reports an error that names the `base16` key and does not open the inbox

#### Scenario: No scheme configured
- **WHEN** the configuration does not set `base16`
- **THEN** theme resolution behaves exactly as before, with the built-in preset and `[colors]` overrides

## MODIFIED Requirements

### Requirement: Configure inbox theme colors
The system MUST ship built-in `dark` (default) and `light` themes and MUST allow individual color overrides on top of the selected theme. When a base16 scheme is configured, its mapped palette MUST replace the built-in theme colors before individual overrides apply. Color values MUST accept named colors and `#RRGGBB` hex strings. The configurable colors MUST cover the inbox title, unread marker, secondary text, success, failure, and running result labels, the inbox background, and the default text color. Built-in themes MUST leave background and default text unset so the terminal colors show through. The selection highlight MUST stay readable on any terminal background.

#### Scenario: Select a built-in theme
- **WHEN** the configuration sets `theme = "light"`
- **THEN** the inbox uses the light theme colors for every element the theme defines and keeps the terminal background

#### Scenario: Manual background override
- **WHEN** the configuration sets `[colors] background` or `[colors] text` without a base16 scheme
- **THEN** the inbox uses that value on top of the built-in theme
