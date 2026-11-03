# verify-hundred proof run, 2026-10-20

Produced by `.agents/skills/verify-hundred/bin/verify-hundred` on Linux x86-64 (Hyprland workstation, private Xvfb displays from :100). `sign-in/`, `sync/`, `native/`, `native-sync/` and `acceptance.txt` were run at 131d4fc. `local/` and `doctor-prod.txt` were re-run at bd98f3d, whose only helper change redacts the installed build's revision in `doctor`. Every store was disposable and synthetic, except in the read-only owner sign-in. That run's full screenshot shows real history, so it stayed in the operator's private directory outside Git. The server origin, owner label and private-repository revisions are redacted.

| Path | Command | Shows |
|---|---|---|
| `sign-in/` | `verify-hundred sign-in` | Owner sign-in **proven** on production. A signed-out request and a random credential are refused. The owner credential resolves to the expected owner. The real app shows `Synced`, lists the owner's recent sets, pulls the history and has nothing pending. The random credential shows `Sync credential rejected` in the app. |
| `doctor-prod.txt` | `verify-hundred doctor --prod` | Deployed health, signed-out refusal, and the owner credential resolving to the expected owner |
| `local/` | open-app, log-reps and backup-restore recipes, every command as written | Transcript with each command and its output, plus screenshots: empty, invalid counts, CLI save picked up, completed day, history, reopen, 760×640, and scrolled to `Recent sets` |
| `sync/` | sync recipes, every command as written | Auto sync, backend down, offline restart, recovery without a keypress, second desktop, wrong identity, bound store, and signed-out 401 |
| `native/` | `verify-hundred walk native` | `scripts/walk-native`: keyboard and pointer logging, undo, completed day, reopen, history, 760×640, and archive round trip |
| `acceptance.txt`, `native-sync/` | `verify-hundred walk sync` | workerd and local D1 acceptance with two synthetic identities, then the GUI's offline save, restart, automatic retry and second-desktop convergence |
