# Hundred verification map

This directory is the maintained source for verifying Hundred's user-facing behavior. Read this index before driving the app, then use the matching feature file as the recipe. `$V` is `.agents/skills/verify-hundred/bin/verify-hundred`, run from the repository root.

## Baseline preconditions

- `$V build` was run at the current `HEAD`, and `$V doctor` prints no `FAIL`.
- Every session starts from an empty disposable store (`$V app start`) unless a recipe passes `--data-dir`.
- Sync recipes use the local Worker (`$V worker start`) and its synthetic identities. Production is used only for the read-only [sign-in](sign-in.md) recipe.
- Never drive the operator's own store (`~/.local/share/hundred`) or the installed app's window.

## Driving conventions

- Prefer the keyboard. Pointer coordinates are only valid at the default window size.
- After an action, wait on state with `$V app wait-text` or `$V app wait-status`, not a fixed sleep. The UI updates when the store replies, which can take up to 3 seconds.
- Confirm every save or sync in a second view: `$V app status`, `$V app cli history`, or a second store that syncs the same identity.
- Teardown is `$V app stop` per session, then `$V cleanup` at the end of the run.

## Proof and skip reporting

- UI proof is a screenshot plus the matching OCR or `status` output, captured after the action.
- CLI proof is the command, stdout, stderr, and exit code.
- Record which entry point (key, pointer, CLI) each artifact used. Do not report an entry point as verified because a different one passed.
- If a recipe cannot run, report the command you attempted and the missing prerequisite. Never weaken the recipe to make it pass.

## Feature entry contract

Each feature file has an H1, a one-paragraph description of what the user sees, then exactly these H2s in order: `Sub-features`, `How to get to it (user POV)`, `Driving it with verify-hundred`, `Gotchas`.

## Features

- [Open the app](open-app.md): launch from an empty store, readiness, the offline state, the 760×640 layout, and quit/reopen persistence.
- [Log reps](log-reps.md): keyboard, pointer, presets and CLI logging, invalid counts, undo, a completed day and streak, and history.
- [Sync](sync.md): automatic sync, the backend down and recovered, restart while offline, a second desktop, the wrong identity, signed out, and a store bound to another identity.
- [Owner sign-in](sign-in.md): the real owner credential against the deployed server, read-only, plus signed-out and wrong-credential refusal.
- [Back up and restore](backup-restore.md): export, restore into a fresh store, the byte-for-byte round trip, and refusal to overwrite.
