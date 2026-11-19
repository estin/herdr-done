# Tasks

## 1. Add store and inbox cleanup

- [x] 1.1 Add an event-store operation to delete read events and test that unread events remain
- [x] 1.2 Add a `c` inbox action that clears read events and refreshes the list; test the action and selection behavior
- [x] 1.3 Update inbox help text to show the clear-read action and verify the rendered shortcut text

## 2. Add next-unread CLI behavior

- [x] 2.1 Add `hd next` CLI parsing and dispatch; test CLI parsing and behavior when no unread event exists
- [x] 2.2 Open the inbox with the next unread event selected for `hd next`; test unread selection and verify the event remains unread

## 3. Add read-event cleanup command

- [x] 3.1 Add `hd clear` to remove read events and report the count; test empty, read-only, and mixed event stores
- [x] 3.2 Update CLI documentation and verify help and documented command behavior
- [x] 3.3 Run `just ci` to verify formatting, Clippy, and the full test suite
