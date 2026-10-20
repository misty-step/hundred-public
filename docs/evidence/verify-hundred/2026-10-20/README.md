# verify-hundred proof run, 2026-10-20

Produced by `.agents/skills/verify-hundred/bin/verify-hundred` on Linux x86-64 (Hyprland workstation, private Xvfb displays). The skill was at 3af8581 for `native/`, after the walk began polling instead of sleeping, and at 2dfd8a5 for `native-sync/`, `acceptance.txt`, `local/` and `sync/`, and at 3969955 for `doctor-prod.txt` and `sign-in/`. Only documentation changed between them. Every store was disposable and synthetic, except the read-only owner sign-in. Its full screenshot holds real history, so it was kept outside Git. The server origin and owner label are redacted.

| Path | Command | Shows |
|---|---|---|
| `native/` | `verify-hundred walk native` | `scripts/walk-native`: keyboard and pointer logging, undo, a completed day, reopen, history, 760×640, and an archive round trip (`result.txt`) |
| `acceptance.txt`, `native-sync/` | `verify-hundred walk sync` | Local workerd and D1 acceptance with two synthetic identities, then the GUI's offline save, restart, automatic retry and second-desktop convergence |
| `local/` | open-app, log-reps and backup-restore recipes as written | Transcript of each command and its output, plus screenshots |
| `sync/` | sync recipes as written | Auto sync, backend down, offline restart, recovery without a keypress, second desktop, wrong identity, bound store, and signed-out 401 |
| `doctor-prod.txt` | `verify-hundred doctor --prod` | Deployed health and release, signed-out refusal, and the owner credential resolving to the expected owner |
| `sign-in/` | `verify-hundred sign-in` | Owner sign-in proven: the real app with the owner credential shows `Synced` and pulls the owner's history, and a random credential is rejected in the API and the app |
