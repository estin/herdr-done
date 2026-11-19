# Design

## Context

`hd run` supervises one child process, stores its result, and returns that result. The CLI already uses Herdr's documented command interface when available. See proposal.md and the `command-inbox` spec delta for the user-facing contract.

## Goals / Non-Goals

**Goals:**

- Keep alerts disabled unless requested for a specific run
- Deliver alerts after the child result is known and event persistence has been attempted
- Keep alert failures from changing the child's exit result

**Non-Goals:**

- Add persistent alert preferences or configuration
- Implement sound playback directly or read Herdr's private configuration
- Add alert actions or include command output in alerts

## Decisions

### Use Herdr's CLI for the configured completion sound

When `--sound` is set and Herdr is available, invoke `herdr notification show` with a completion title, result summary, and `--sound done`. This lets Herdr apply its configured sound and keeps `hd` independent of sound files and audio backends. If Herdr cannot be invoked, report the alert failure to stderr without changing the command result. Do not substitute a terminal bell or another sound.

### Use notify-rust for system notifications

When `--notify` is set, send a desktop notification with the command and outcome through `notify-rust`. Notification creation or delivery errors are best-effort and must not change `hd run`'s exit result. Do not add a Herdr CLI fallback because the option explicitly requests a system notification.

### Keep alert selection per invocation

Add independent boolean flags to `hd run`; neither is enabled by default. Launch failures do not count as completed commands and do not trigger alerts. After a child exits, attempt each requested alert independently so failure of one does not block the other.

## Risks / Trade-offs

- [Herdr may be unavailable or lack notification sound support] → Keep alert errors non-fatal and report them to stderr
- [Desktop notification services may be missing or reject a notification] → Keep notification errors non-fatal and return the child result
- [Invoking Herdr for sound adds a process after command completion] → Do so only when `--sound` is requested

## Migration Plan

No stored data changes are needed. Existing `hd run` invocations remain quiet and keep their current behavior. New flags are opt-in.
