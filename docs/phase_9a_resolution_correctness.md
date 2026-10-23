# Phase 9A: Reminder Engine Resolution Correctness

## Status

Implementation work is isolated on branch `codex/phase-9a-resolution-correctness`. This document records the phase boundary; it does not claim that implementation or verification is complete.

## Ownership

- `ReminderEngineService` owns session lifecycle and resolution decisions.
- `SchedulerService` owns countdown timing and accepts a resolution handoff only after persistence succeeds.
- `SessionRepository` performs database access only.
- The frontend and presentation systems do not determine whether a session outcome has committed.

## Resolution protocol

The intended lifecycle is:

```text
Idle -> SessionActive -> Resolving -> Idle
                         |             (terminal outcome)
                         +-> Snoozed
                         |
                         +-> SessionActive
                              (recoverable persistence failure)
Snoozed -> SessionActive
          (snooze timer fires)
```

A resolution request must validate the active session ID and reserve the `Resolving` state synchronously before the first asynchronous suspension. Only one request may reserve a given active session. Requests that arrive while the session is resolving must not perform persistence or scheduler handoff.

The accepted operation must not be cancelled merely because the IPC caller or timeout-waiting task is cancelled. Its persistence work therefore needs an independently owned task or equivalent cancellation-safe boundary.

## Persistence and handoff order

1. Validate request and reserve the active session.
2. Load required settings without silently replacing a database error with defaults.
3. Persist the session transition and any related statistics in one SQLite transaction.
4. Commit the transaction.
5. Update engine state to `Snoozed` or `Idle`.
6. Notify the scheduler and emit outcome events only after commit.

A failed transaction must not emit a successful outcome or hand scheduling off as though the resolution committed. The engine must leave `Resolving` through a defined recovery path. If the session remains triggered, timeout protection must be restored.

## Snooze contract

- Reject non-positive snooze durations at the service boundary.
- Preserve the same session ID.
- Increment the snooze count exactly once per accepted snooze.
- Schedule one re-trigger only after the snoozed transition commits.
- Do not reinterpret a failed snooze as a successful scheduler handoff.

## Acceptance criteria

- An unattended reminder times out, persists once, and schedules the next reminder.
- Concurrent outcomes yield one accepted transition and one scheduler handoff.
- Stale or invalid requests cannot cancel legitimate timeout protection.
- Database failures do not report success or leave the engine permanently stuck in `Resolving`.
- Snooze preserves the session, increments once, and re-triggers once after the configured delay.
- Engine correctness does not depend on presentation acknowledgements.

## Explicit exclusions

No frontend or overlay work, character/audio assets, statistics redesign, migration changes, startup recovery, security configuration, installer work, or future-phase placeholders belong in Phase 9A.

## Verification honesty

The GitHub connector can edit this branch, but this chat does not expose the local Windows filesystem or a Windows build/test runner. Local `cargo test --locked --offline --lib` must not be reported as passing unless it actually executes successfully in a suitable environment.
