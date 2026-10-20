---
name: verify-hundred
description: Drive Hundred, the native GPUI daily rep tracker, the way its owner does — the real desktop app on a private Xvfb display, the real CLI, a disposable local Cloudflare Worker, and read-only owner sign-in on the deployed sync server. Use to prove a change to logging, history, undo, sync, backup or credentials works in the real app, or when anything about Hundred looks wrong.
---

# Verify Hundred

Hundred has three surfaces: the GPUI desktop app (primary), the `hundred` CLI on the same local SQLite store, and a Cloudflare Worker that syncs each owner's event log through D1. Every drive here runs from a shell. `bin/verify-hundred` wraps the repository's own harnesses (`scripts/walk-native`, `scripts/walk-sync`, `scripts/test-worker`) and adds an interactive session for anything they do not script.

```sh
V=.agents/skills/verify-hundred/bin/verify-hundred   # from the repository root
$V help
```

## Launch

1. Prerequisites: Rust 1.98.1 (pinned), `npm ci --ignore-scripts`, `worker-build` 0.8.5+, and `Xvfb xdotool import tesseract jq socat curl openssl`. On Ubuntu, install the packages listed in `.github/workflows/check.yml`.
2. `$V build` builds `target/debug/hundred`. The binary embeds the commit, so rebuild after every commit or doctor fails.
3. `$V app start` opens the real app on its own Xvfb display with an empty disposable store, offline. Ready means OCR can read `Hundred` in the window, not only that a process exists. Variants:
   - `$V worker start` starts workerd with local D1 on `127.0.0.1:8798` with two new synthetic identities. It prints `token-file` paths for `synthetic-a` and `synthetic-b`. `VERIFY_HUNDRED_WORKER_PORT` changes the port. Recipes say 8798, so use the `server` line it prints. Then `$V app start --server http://127.0.0.1:8798 --token-file <token-a>`.
   - `$V app start --prod` uses the owner credential against the deployed server. The helper keeps that session read-only. It refuses `type`, `click` and `cli` other than `status`/`history`, and it allows only the `ctrl+h`, `Escape` and `ctrl+r` keys. See [sign-in](features/sign-in.md).
   - `--data-dir DIR` reuses a store, for example to reopen one. `--evidence DIR` sets where screenshots go.
4. The app never opens on the operator's desktop. `WAYLAND_DISPLAY` is unset and `DISPLAY` points at the private Xvfb.

The helper and `scripts/walk-native`/`scripts/walk-sync` start Xvfb on the first display from `:100` that has no lock file and no socket. Never start Xvfb with `-displayfd` on a workstation. On Hyprland it claimed `:0`, because Xwayland there has no abstract socket. It then unlinked the desktop's `/tmp/.X11-unix/X0`, and every new X11 app on the desktop broke. If `DISPLAY=:0 xdotool getdisplaygeometry` fails after a run while `/tmp/.X11-unix/X0_` still exists, `ln /tmp/.X11-unix/X0_ /tmp/.X11-unix/X0` restores it.

Instances are isolated. Each session has its own Xvfb, store and runtime dir, so several can run side by side. The helper Worker uses port 8798. `scripts/test-worker` uses 8797 and 8877, so the two do not collide. The helper refuses a port it did not start.

## Doctor

Run `$V doctor` before the first drive and after anything surprising. It checks the tools, `node_modules`, and that the binary was built from `HEAD`. It also lists live sessions, the local Worker and the installed app's revision. `$V doctor --prod` adds the deployed server's `/health` and release, the refusal of a signed-out request, and whether the owner credential resolves to the expected owner. A `FAIL` line means stop and fix that first. Exit status is non-zero on any `FAIL`.

## Drive

Keyboard is the stable handle. Coordinates depend on window size.

| Action | Command |
|---|---|
| Select exercise | `$V app key ctrl+1` (Push-ups), `ctrl+2` (Crunches), `ctrl+3` (Squats), or `Tab` |
| Enter reps and log | `$V app key ctrl+a`, then `$V app type 20`, then `$V app key Return` |
| Adjust reps | `$V app key Up` or `$V app key Down` |
| Undo latest set | `$V app key ctrl+z` |
| Sync now | `$V app key ctrl+r` |
| History and back | `$V app key ctrl+h`, then `$V app key Escape` |
| Quit and reopen | `$V app restart` (Ctrl+Q, wait for exit, relaunch on the same store) |
| Narrow layout | `$V app resize 760 640`, then `$V app scroll down` to reach `Recent sets` |
| Pointer | `$V app click X Y` (window coordinates; see `scripts/walk-native` for the known button positions) |

Observe state through the app and through the store:

- `$V app wait-text REGEX [SECS]` matches case-sensitive extended regex against OCR. `Synced` does not match `Not synced yet`. OCR can join neighbouring labels onto one line, so do not anchor to `^` or `$`.
- `$V app ocr` prints the window text. `$V app shot NAME` saves a screenshot.
- `$V app status` prints the CLI's JSON for the session store: `totals` as `[push-ups, crunches, squats]`, streaks, and `pending` (sets not yet acknowledged by the server). `$V app wait-status '.pending == 0' [SECS]` waits for a condition. Prefer it to `sleep`.
- `$V app cli history` prints the version-1 event archive. `$V app cli export FILE` and `$V app cli restore FILE` run against the same store.

Visible sync labels, each followed by ` · N waiting` while sets are pending: `Local only · sync not configured`, `Not synced yet`, `Syncing`, `Synced`, `Saved here · N waiting to sync`, `Server unreachable. Local sets are safe.`, `Sync credential rejected. Local sets are safe.`, `Server made no sync progress. Local sets are safe.`, and `This local store belongs to another server or identity.`

Scripted journeys:

- `$V walk native [DIR]` runs `scripts/walk-native`: keyboard and pointer logging, undo, a completed day, quit/reopen, history, the 760×640 layout, and an export/restore byte-for-byte round trip.
- `$V walk sync [DIR]` runs `scripts/test-worker` with `HUNDRED_WALK_SYNC=1`. It covers HTTP/D1 acceptance with two synthetic identities, including cross-owner refusal, lost responses and paging. It then runs `scripts/walk-sync`: outage, restart, automatic retry without pressing Sync, and second-desktop convergence.
- `$V sign-in [DIR]` proves owner sign-in on production, read-only. See [features/sign-in.md](features/sign-in.md).

The feature map in [features/README.md](features/README.md) is the recipe source. A proof is incomplete if it skips an entry point the map lists.

## Evidence

- Every run writes under `~/.cache/tmp/verify-hundred/runs/<UTC>/` (mode 0700), or under the `DIR` you pass. The helper prints `evidence <path>` at the end.
- Proof standard: drive the real user path (keys, pointer, CLI), then confirm the stored state with a second view (`app status`, `cli history`, a second store's `sync`). A screenshot alone is not proof, and neither is "the request returned 200".
- Exercise data is private. Only synthetic runs (offline, local Worker) may be committed. Production screenshots contain the owner's real history. `sign-in` writes them only to `~/.cache/tmp/verify-hundred/private/<UTC>/`, outside any checkout, whatever `DIR` you pass. It writes a commit-safe `sign-in.txt` and header crop to `DIR`. Stopping a session against a non-loopback server deletes its scratch frames. Never commit a production frame, a store, an archive, or a token file.
- Commit evidence under `docs/evidence/verify-hundred/<YYYY-MM-DD>/` only after checking each file for real data.

## Cleanup

- `$V app stop` quits the app, kills that session's Xvfb, and deletes its disposable store. A `--data-dir` store you supplied is left in place. If the app ignores Ctrl+Q for 5 seconds, it is killed, a `warn` line is printed, and `stop` and `restart` exit 1. Treat that as a finding.
- `$V worker stop` stops the local Worker but keeps its D1 and identities, which is how to simulate "backend down". `$V worker purge` removes them.
- `$V cleanup` stops every session and Worker this helper started and purges the Worker. It kills recorded process groups only, and only when the PID's kernel start time still matches, never by name. `sign-in` stops its own sessions however it ends.
- Evidence under `runs/` survives every cleanup. After `cleanup`, confirm with `ls ~/.cache/tmp/verify-hundred/runs/<run>`.

## Helpers

- `bin/verify-hundred` is the only helper. `$V help` lists every subcommand, and every invocation above is literal.
- Environment: `HUNDRED_BIN` (binary, default `target/debug/hundred`), `VERIFY_HUNDRED_HOME` (scratch and evidence root), `VERIFY_HUNDRED_SESSION` (drive a session other than the newest), `VERIFY_HUNDRED_WORKER_PORT` (default 8798), and `VERIFY_HUNDRED_REDACT=0` to print the private server and owner in transcripts (never for committed evidence).

To keep this skill honest as the app changes, run a maintenance pass. Cover every feature file from source, drive every feature live, and ship proven corrections in one PR. On the maintainer's workstation that pass is the user-level `/maintain-verification-skill`.
