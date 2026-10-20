# Sync

With a server and token configured, the app syncs in the background after every save, every 30 seconds, and on Ctrl+R or `Sync`. The status line moves through `Not synced yet`, `Syncing` and `Synced`. Saving never waits on the network. If the server is unreachable, the label says so and the set waits with a count. It drains automatically when the server returns, including after a restart. Two desktops on the same identity converge. A different identity never sees or cancels another's sets.

## Sub-features

- `sync-auto` syncs after a save with no keypress, and `pending` returns to 0.
- `sync-manual` syncs on Ctrl+R or the `Sync` button.
- `sync-backend-down` shows `Server unreachable. Local sets are safe. · N waiting` and keeps the set.
- `sync-recover` drains the outbox without a keypress once the server is back, including across an app restart.
- `sync-second-desktop` gives a second store on the same identity the same totals, with no duplicates after repeated syncs.
- `sync-wrong-identity` keeps another identity's history invisible: a second identity reads zero sets and cannot cancel them.
- `sync-signed-out` without a token shows `Local only · sync not configured`, and the API answers 401.
- `sync-bound-store` refuses to sync a store bound to one identity with another identity's credential.

## How to get to it (user POV)

- Configure `HUNDRED_SERVER` and the token pass entry in the launcher config, then open the app.
- Ctrl+R or the `Sync` button. In a terminal: `hundred sync`.

## Driving it with verify-hundred

Preconditions:

- `$V worker start` printed `server http://127.0.0.1:8798` and two `token-file` paths. Call them `$A` (`synthetic-a`) and `$B` (`synthetic-b`).
- `$V app start --server http://127.0.0.1:8798 --token-file $A`, then `$V app wait-text 'Synced' 60`.

- **Auto sync.** Run `$V app key ctrl+1`, `$V app key ctrl+a`, `$V app type 30`, `$V app key Return`. `$V app wait-status '.pending == 0' 45` passes and `$V app wait-text 'Synced'` matches.
- **Backend down.** Run `$V worker stop`, log `15` crunches the same way, then `$V app key ctrl+r`. `$V app wait-text 'unreachable'` matches and `$V app wait-status '.pending == 1'` passes. Run `$V app shot backend-down`. The label reads `Server unreachable. Local sets are safe. · 1 waiting`.
- **Restart offline.** Run `$V app restart`. `$V app status` still shows the set with `pending` at 1.
- **Recover.** Run `$V worker start`. Without pressing anything, `$V app wait-status '.pending == 0' 60` passes within one 30-second retry. Then `$V app wait-text 'Synced'` and `$V app shot recovered`.
- **Second desktop.** Run `HUNDRED_TOKEN=$(cat $A) target/debug/hundred --data-dir <new dir> --server http://127.0.0.1:8798 sync`, then `status` on that dir. Its totals equal `$V app status` totals. Run `sync` again and the totals do not change.
- **Wrong identity.** Repeat with `$B` and another new dir. It prints `Synced.` and `status` shows `[0,0,0]`.
- **Bound store.** Run `HUNDRED_TOKEN=$(cat $B) target/debug/hundred --data-dir <the $A dir> --server http://127.0.0.1:8798 sync`. It exits 1 with `This local store belongs to another server or identity.`
- **Signed out.** Run `curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:8798/v1/identity`. It prints `401`. `$V app start` with no server shows `sync not configured`.
- **Scripted equivalent.** `$V walk sync` runs the HTTP/D1 acceptance (510 synthetic records, lost responses, cross-owner refusal, paging), then the outage, restart, auto-retry and second-desktop walk. It asserts on each step.
- **Teardown.** Run `$V app stop`, then `$V worker purge`.

## Gotchas

- The first requests to a fresh local Worker compile the Wasm and can take 15 seconds each. `worker start` waits for `/health`.
- Known bug ([#4](https://github.com/misty-step/hundred-public/issues/4)): a set saved while a sync is already in flight makes that sync end with `Server made no sync progress. Local sets are safe. · 1 waiting`. The set is safe and drains on the next sync, within 30 seconds. Wait on `.pending == 0` rather than on the first `Synced`.
- `app status`, `app cli` and the CLI examples above use an empty `HUNDRED_TOKEN` or your explicit one. A token exported in your shell would otherwise make `sync` act as that identity.
- Token files are synthetic and live under `~/.cache/tmp/verify-hundred/worker/`. `worker purge` removes them.
- Slow-network behavior is not scripted. The client times out after 12 seconds and reports `Server unreachable`.
