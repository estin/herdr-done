# Tasks

## 1. Capture Herdr Names

- [x] 1.1 Extend Herdr context parsing and event storage for workspace, tab, and optional pane names; test documented response parsing and backward-compatible loading of events without names
- [x] 1.2 Capture workspace and tab labels plus pane label or title from documented Herdr interfaces; test partial and unavailable name results without failing command tracking

## 2. Display Herdr Names

- [x] 2.1 Update inbox list and detail location rendering to show available names and never use IDs as names; test complete, partial, and missing-name contexts
- [x] 2.2 Verify pane jump still uses the stored pane ID after adding display names by running the focused Herdr navigation tests

## 3. Integration Verification

- [x] 3.1 Run formatting, Clippy, and the full test suite; confirm existing databases and events remain readable
