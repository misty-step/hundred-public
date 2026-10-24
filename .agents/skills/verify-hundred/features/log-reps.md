# Log reps

The owner picks Push-ups, Crunches or Squats, enters 1–1000 reps, and saves with Enter, `Log set ↵`, or a preset. The app confirms only after the set is in local SQLite: `Saved 20 squats · 80 left`. The row total, remaining reps, recent sets and the 28-day trace update. Undo cancels the latest active set durably. A day with all three exercises at 100 is complete and starts a streak.

## Sub-features

- `log-keyboard` selects an exercise with Ctrl+1/2/3 or Tab, types reps, and saves with Enter.
- `log-pointer` saves using the exercise row, a preset (10, 20, 25), `–`/`+`, and `Log set ↵`.
- `log-cli` saves with `hundred add <exercise> <reps>`, with optional `--at RFC3339` for backfill.
- `log-invalid` refuses 0 or more than 1000 reps, shows `Enter between 1 and 1000 reps.`, and keeps the entered number.
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
- **Invalid count.** Run `$V app key ctrl+a`, `$V app type 0`, `$V app key Return`, then immediately `$V app wait-text 'Enter between' 3`. `$V app status` totals are unchanged. Run `$V app shot invalid-0`. The screenshot shows `0` still in the entry, which OCR usually drops. Repeat with `1001`. There, `$V app ocr | grep 1001` finds the kept number. The pass criterion is the unchanged `status` and the kept number. Because of the known bug below, the notice can already be gone, so a missed `wait-text` alone is not a failure.
- **Undo.** Run `$V app key ctrl+z`. `$V app wait-text 'Set undone'` matches and `$V app wait-status '.totals == [0,0,0]'` passes. `$V app cli history | jq -c '[.events[].change.kind] | sort'` prints `["cancel","set"]`, because undo does not delete.
- **Pointer save.** Click the Push-ups row, the `10` preset, then `Log set ↵`: `$V app click 150 200`, `$V app click 720 477`, `$V app click 910 477`. `$V app wait-status '.totals == [10,0,0]'` passes. A row's `Undo` button cancels it the same way (`scripts/walk-native` uses `920 745`).
- **CLI save.** Run `$V app cli add crunches 15`. Stdout reads `Saved 15 crunches · <uuid>`. The open window picks it up within 3 seconds: `$V app wait-text '15 ?/ ?100'`.
- **Complete day.** For each of `ctrl+1`, `ctrl+2` and `ctrl+3`, send the key, then `ctrl+a`, `$V app type 100`, `Return`, then wait for that exercise's notice (`$V app wait-text 'Saved 100 push-ups'`, then `crunches`, then `squats`) before the next one. `$V app wait-status '.current_streak == 1'` passes, and `$V app wait-text '1 ?day streak'` matches. Each row reads `Done` or `N extra`. Run `$V app shot complete`. The screenshot shows `Today's hundred, done.` under the art. That caption is too small for reliable OCR.
- **History.** Run `$V app key ctrl+h`, then `$V app wait-text 'Your days'`, then `$V app shot history`. Today's row lists the exact totals. `$V app key Escape` returns to today, and `$V app wait-text 'History'` matches the header button again.
- **Scripted equivalent.** `$V walk native` covers keyboard, pointer, undo, complete and history with assertions.

## Gotchas

- Known bug ([#5](https://github.com/misty-step/hundred-public/issues/5)): the `Enter between 1 and 1000 reps.` notice is replaced within 3 seconds by the previous notice, such as `Saved 20 squats · 80 left`, when the window's periodic refresh arrives. Capture it right after Return. Prove the refusal through the unchanged `status`, not the lingering text.
- OCR often reads `1 and` as `1and`, and `15 / 100` as `15 /100`. Match short phrases, and use ` ?` around slashes.
- Ctrl+A selects the entry before typing. Without it, typed digits append to the current value (default `20`).
- `Saving here…` shows before the store replies. Enter is ignored until the window shows the save, even when `status` already has it, so a quick second save is silently dropped. Wait for the exercise-specific `Saved N <exercise>` notice before the next save.
- Pointer coordinates assume the default 980×850 window. After `app resize`, use the keyboard.
- `status` reports today in the machine's local timezone. Backfilled sets (`add --at`) count on the day and offset they carry.
- A CLI save made while the window is open syncs on the window's next sync, not immediately.
