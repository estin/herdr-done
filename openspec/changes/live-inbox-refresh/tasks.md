# Tasks

## 1. CLI and Inbox Behavior

- [x] 1.1 Add `--oneshot` to the inbox CLI and pass the selected mode through dispatch; test default and explicit mode parsing
- [x] 1.2 Refresh inbox events at the designed interval while preserving selection by event ID; test event refresh and selection behavior
- [x] 1.3 Keep live inbox open after pane jumps and show focus errors in the inbox; test successful and failed live jumps
- [x] 1.4 Preserve close-after-jump behavior for `hd inbox --oneshot`; test the one-shot flow

## 2. User Documentation

- [x] 2.1 Update README inbox examples, key behavior, and CLI usage for live default and `--oneshot`; verify the docs usage test passes

## 3. Integration Verification

- [x] 3.1 Run formatting, Clippy, and the full test suite; confirm all checks pass
