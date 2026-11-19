# Design

## Context

`src/inbox.rs` stores the active view in the inbox app and handles key input in `App::handle_key`. The current `q` and `Esc` branch shows the quit notice for both views.

## Goals / Non-Goals

**Goals:**

- Return from detail to list when the user presses `q` or `Esc`
- Keep the list-view quit notice and double `Ctrl-C` exit behavior

**Non-Goals:**

- Change other key bindings or the inbox exit rule

## Decisions

- Handle `q` and `Esc` based on the active view. In detail view, set the view to list and do not set the quit notice. In list view, keep the existing notice behavior.
- Keep the view transition inside the existing key handler. No new action or state is needed because the app already tracks the active view.
- Add unit tests for each key in detail view and retain coverage for list view behavior.

## Risks / Trade-offs

- `q` no longer shows the quit notice in detail view. This is intentional because the key returns to the list there.

## Migration Plan

No data migration is needed. Update the inbox behavior and key help text in the same release.
