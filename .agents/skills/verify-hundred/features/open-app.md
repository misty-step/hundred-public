# Open the app

The owner opens Hundred from the application menu or by running `hundred`. A dark window titled `Hundred` shows today's date, three exercise rows at `0 / 100`, a rep entry with presets, the last 28 days, and recent sets. Without sync configuration the status line reads `Local only · sync not configured`. Saving still works.

## Sub-features

- `open-empty` paints the first window from an empty store, with `Your first set starts here.`
- `open-offline` shows `Local only · sync not configured` when no server or token is set.
- `open-reopen` keeps every set across quit and relaunch.
- `open-narrow` keeps all controls and exact numbers readable at 760×640.

## How to get to it (user POV)

- Choose **Hundred** in the desktop application menu.
- Run `hundred` in a terminal.
- Quit with Ctrl+Q.

## Driving it with verify-hundred

Preconditions:

- `$V doctor` passes, and no session from an earlier run is live (`$V cleanup` if one is).

- **Launch empty.** Run `$V app start`. It prints `session`, `display`, `data` and `evidence`. Then run `$V app wait-text 'Your first set starts here'`.
- **Offline label.** Run `$V app wait-text 'sync not configured'`. Run `$V app status`. It shows `"totals": [0, 0, 0]` and `"pending": 0`.
- **Capture.** Run `$V app shot empty`. The screenshot shows the title, `0 / 100` on all three rows, and no art in the pigment field.
- **Reopen.** Log one set (`$V app key ctrl+3`, `$V app key ctrl+a`, `$V app type 20`, `$V app key Return`, then `$V app wait-status '.totals == [0,0,20]'`). Then run `$V app restart`. `$V app status` still shows `[0,0,20]`, and `$V app wait-text 'Squats'` matches. `$V app shot reopened` shows `20 / 100` on Squats.
- **Narrow.** Run `$V app resize 760 640`, then `$V app shot narrow`. Every exercise row, the entry field, `Log set` and the sync label stay visible and unclipped. The rest of the page scrolls. Run `$V app scroll down`, and `$V app wait-text 'Recent sets'` matches with each row's `Undo` reachable.
- **Scripted equivalent.** `$V walk native` captures `empty`, `reopened` and `narrow` with assertions in `native/`.
- **Teardown.** Run `$V app stop`.

## Gotchas

- The first paint under software Vulkan takes several seconds. `app start` waits for legible `Hundred` text. A black frame before that is not a failure.
- OCR renders `·` as `-` or drops it. Match `sync not configured`, not the whole label.
- A `--data-dir` already bound to one server or identity refuses another. Use a fresh store per identity.
- `app restart` sends Ctrl+Q. If the app is wedged it is killed after 5 seconds. Treat that as a finding, not a pass.
