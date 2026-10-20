# Log reps

The owner picks Push-ups, Crunches or Squats, enters 1–1000 reps, and saves with Enter, `Log set ↵`, or a preset. The app confirms only after the set is in local SQLite: `Saved 20 squats · 80 left`. The row total, remaining reps, recent sets and the 28-day trace update. Undo cancels the latest active set durably. A day with all three exercises at 100 is complete and starts a streak.

## Sub-features

- `log-keyboard` selects an exercise with Ctrl+1/2/3 or Tab, types reps, and saves with Enter.
- `log-pointer` saves using the exercise row, a preset (10, 20, 25), `–`/`+`, and `Log set ↵`.
- `log-cli` saves with `hundred add <exercise> <reps>`, with optional `--at RFC3339` for backfill.
- `log-invalid` refuses 0 or more than 1000 reps and keeps the entered number.
- `log-undo` cancels with Ctrl+Z, a row's `Undo`, or `hundred undo`.
- `log-complete` shows `Today's hundred, done.` and a streak of 1 when all three exercises reach 100.
- `log-history` opens `Your days` with Ctrl+H or `History`, and Escape or `Today` returns.

## How to get to it (user POV)

- In the window: Ctrl+1/2/3 or Tab, type a number, then Enter. Or click a row, a preset, then `Log set ↵`.
- In a terminal: `hundred add squats 20`, `hundred undo`, `hundred status`.
- History: Ctrl+H or the `History` button.

## Driving it with verify-hundred

Preconditions:

- A fresh offline session: `$V app start`, then `$V app wait-text 'sync not configured'`.

- **Keyboard save.** Run `$V app key ctrl+3`, `$V app key ctrl+a`, `$V app type 20`, `$V app key Return`. Then `$V app wait-text 'Saved 20 squats'` and `$V app wait-status '.totals == [0,0,20] and .pending == 1'`.
- **Invalid count.** Run `$V app key ctrl+a`, `$V app type 0`, `$V app key Return`. `$V app wait-text 'Enter between 1 and 1000 reps'` matches, and `$V app status` totals are unchanged. Repeat with `1001`.
- **Undo.** Run `$V app key ctrl+z`. `$V app wait-text 'Set undone'` matches and `$V app wait-status '.totals == [0,0,0]'` passes. `$V app cli history` still lists the set plus a `cancel` event, because undo does not delete.
- **Pointer save.** Click the Push-ups row, the `10` preset, then `Log set ↵`: `$V app click 150 200`, `$V app click 720 477`, `$V app click 910 477`. `$V app wait-status '.totals == [10,0,0]'` passes. A row's `Undo` button cancels it the same way (`scripts/walk-native` uses `920 745`).
- **CLI save.** Run `$V app cli add crunches 15`. Stdout reads `Saved 15 crunches · <uuid>`. The open window picks it up within 3 seconds: `$V app wait-text '15 / 100'`.
- **Complete day.** For each of `ctrl+1`, `ctrl+2` and `ctrl+3`, send the key, then `ctrl+a`, `$V app type 100`, `Return`. `$V app wait-status '.current_streak == 1'` passes and `$V app wait-text "hundred, done"` matches. Run `$V app shot complete`.
- **History.** Run `$V app key ctrl+h`, then `$V app wait-text 'Your days'`, then `$V app shot history`. Today's row lists the exact totals. `$V app key Escape` returns to `Log`.
- **Scripted equivalent.** `$V walk native` covers keyboard, pointer, undo, complete and history with assertions.

## Gotchas

- Ctrl+A selects the entry before typing. Without it, typed digits append to the current value (default `20`).
- `Saving here…` shows before the store replies. Wait for `Saved …` or the `status` totals.
- Pointer coordinates assume the default 980×850 window. After `app resize`, use the keyboard.
- `status` reports today in the machine's local timezone. Backfilled sets (`add --at`) count on the day and offset they carry.
- A CLI save made while the window is open syncs on the window's next sync, not immediately.
