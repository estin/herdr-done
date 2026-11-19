# Design

## Context

`HerdrContext` stores workspace, tab, and pane IDs, plus a workspace name field that is not populated. The inbox uses these IDs as location text. Herdr's documented schema exposes workspace and tab `label` values, and optional pane `label` and `title` values. Existing database rows may not have names.

## Goals / Non-Goals

**Goals:**

- Capture available Herdr labels with each command event.
- Show names in the inbox while keeping IDs available for pane focus.
- Render old or partial context without using IDs as names.

**Non-Goals:**

- Resolve names for old events by querying Herdr later.
- Change pane jump behavior or introduce new Herdr dependencies.

## Decisions

- Extend saved Herdr context with tab and pane display names, and populate workspace name from Herdr's workspace label. Read pane metadata from `pane.current` where available and query the associated workspace and tab labels through documented Herdr commands.
- Keep IDs as separate context fields. They remain the stable targets for focus operations; names are display-only snapshots of the labels at command start.
- Use the pane label when present, then its title when present. Do not substitute IDs when names are missing. Keep schema-compatible optional fields so older database rows load with absent names.
- Update the shared location formatter for both inbox list and details. Render only available names, with an unavailable marker when no location name exists.

## Risks / Trade-offs

- [Risk] Herdr may be unavailable or a label query may fail -> Store any names that were retrieved and keep command tracking functional with absent names.
- [Risk] Herdr labels can change after event capture -> Treat saved labels as historical display text and retain IDs for navigation.
- [Risk] Additional Herdr CLI calls add startup latency -> Query only the current workspace and tab needed for the active pane; do not resolve labels during inbox refresh.
