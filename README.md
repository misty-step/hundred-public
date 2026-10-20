# Hundred

A quiet, native daily rep tracker. Log push-ups, crunches and squats toward a hundred of each. Exact counts, daily history, streaks and a static ASCII pigment field. Rust + GPUI + Cloudflare, with no AI or telemetry.

Linux x86-64 is the verified release target. Exercise history and deployment configuration stay private.

## Use

Launch **Hundred** from the desktop application menu or run `hundred`. Tab changes exercise; type reps and press Enter. Ctrl+1/2/3 selects an exercise. Up/Down adjusts reps, Ctrl+Z undoes the latest active set, Ctrl+R syncs, Ctrl+H opens history, Escape returns, Ctrl+Q quits. Buttons and presets offer the same primary actions.

Saving commits to local SQLite before confirmation. Sync runs in the background and retries every 30 seconds. An internet outage never prevents saving. Every set retains the calendar day and UTC offset where it was recorded. A day is complete when every exercise reaches 100; today's incomplete day does not break yesterday's streak. Undo is a durable cancellation, not deletion.

```sh
hundred add squats 20
hundred status
hundred undo                         # latest active set
hundred sync
hundred export ~/hundred-backup.json # new filename, private mode 0600
hundred --data-dir ~/hundred-restored restore ~/hundred-backup.json
```

`status` is JSON; `history` prints a version-1 event archive. `add --at RFC3339` supports deliberate backfill. CLI saves are local until `sync`, or until a running app syncs them. Export includes unsent sets and cancellations. Archive files contain private exercise data: keep them in private backup storage, not Git or a public issue.

## Build and install

Rust 1.98.1 is pinned. The lockfile retains libc 0.2.186 because GPUI 0.2.2's tar dependency uses the Linux ENOATTR alias removed in later libc releases; use `--locked`. On Ubuntu 24.04:

```sh
sudo apt-get install build-essential pkg-config libfontconfig1-dev libfreetype6-dev \
  libxkbcommon-dev libxkbcommon-x11-dev libxcb1-dev libwayland-dev libssl-dev clang cmake
cargo build --locked --release -p hundred
scripts/install
```

In a source archive without Git metadata, set the same nonempty
`HUNDRED_BUILD_SHA` when building and installing. The installer requires an exact
match with the binary's reported revision. In a Git checkout, the checked commit
is authoritative.

Runtime needs a graphical Wayland/X11 session, fontconfig, and a working Vulkan driver. The installer owns only the Hundred executable, launcher, icon and desktop entry. It does not configure autostart or edit Omarchy/Hyprland. Remove those four owned paths to uninstall; local history remains.

Data defaults to `$XDG_DATA_HOME/hundred` or `~/.local/share/hundred`. `--data-dir` selects an independent store. The directory is 0700, SQLite is 0600. The canonical cloud log is reconstructed locally on a fresh sync. Local stores bind to their server and authenticated identity before uploading: use a separate directory for a different account.

## Configure sync

Hundred starts offline with no default server or credential. Local saving, history,
export, and restore work without either. Configure both `HUNDRED_SERVER` and
`HUNDRED_TOKEN` to sync. The CLI `--server` option overrides `HUNDRED_SERVER`.
Only HTTPS or literal loopback HTTP is accepted; redirects never carry credentials.
Use a separate `--data-dir` for each server and identity.

The installed launcher reads `$XDG_CONFIG_HOME/hundred/launcher.env`, or
`~/.config/hundred/launcher.env` when XDG_CONFIG_HOME is unset. `HUNDRED_CONFIG`
selects an explicit file; a missing explicit file is an error. The binary itself
reads environment variables only, so direct binary callers set them explicitly.
Create the private file outside the checkout with directory mode 0700 and file
mode 0600. [config/launcher.env.example](config/launcher.env.example) shows the two
accepted keys using synthetic values:

```text
HUNDRED_SERVER=https://sync.example.invalid
HUNDRED_TOKEN_PASS_ENTRY=example/hundred/token
```

Values are literal, unquoted `KEY=value` lines. Blank lines and whole-line `#`
comments are accepted; unknown or duplicate keys fail without echoing values.
The file never contains a raw token. Exported environment settings override file
settings, including empty values. An explicitly empty `HUNDRED_TOKEN` disables
pass lookup. Otherwise the launcher looks up only `HUNDRED_TOKEN_PASS_ENTRY`
through `pass-env`, if configured. Missing pass-env or a locked/missing entry
opens the app offline. Token bytes travel only in the app's environment, never
its arguments. No pass entry is assumed or created.

## Run a local Worker

Rust 1.98.1, GPUI 0.2.2, worker-build 0.8.5, Wrangler, and both lockfiles pin the
build inputs. With the existing build prerequisites and worker-build available:

```sh
npm ci --ignore-scripts
scripts/test-worker
```

This uses `wrangler.local.toml`, disposable local D1, and new synthetic identities.
It does not require private configuration, production credentials, or a remote
repository. `npm run dev` also selects this local config. Never bind acceptance
checks to production data. `/health` checks D1 schema availability and reports
the release, not completion of a user story.

The API hashes bearer tokens and scopes every query to its authenticated owner.
The Worker secret `CREDENTIALS` is a JSON list of `{ "sha256": "64-character digest",
"owner": "stable private identity" }`. A token rotation retains its owner; a
different owner cannot read or cancel another history. Records and credentials
are not logged. Keep real account IDs, Worker names/routes, D1 names/IDs, endpoint,
credential mappings, and owner identities in external private configuration.
[config/pass.env.example](config/pass.env.example) is a synthetic mapping example.

The private deployment pipeline is retained until a coordinated cutover.
[docs/public-export.md](docs/public-export.md) defines the exact export boundary,
external configuration contract, and proposed hosted checks. Never expose this
repository's existing Git history or private deployment files.

## Recovery

Sync, then export; unsent records are also included. Copy the private archive off-device using your normal encrypted backup. Restore into a new `--data-dir`, export again and compare event contents before treating it as a proven backup. Syncing a restore reuses UUIDs, so existing cloud records are not duplicated. Never delete the original until the restored history is verified. D1 Time Travel provides provider-side recovery, but is not an independent offsite backup or proof of a restore. Rollback code with Wrangler versions; it does not roll back connected D1 data.

## Verify

`scripts/check` runs isolated launcher checks, formatting, domain/client behavior, desktop lint/build checks, Worker lint for Wasm and runtime-dependency audit. `cargo test -p hundred-core -p hundred-client` proves calendar, streak, cancellation, disk-restart, acknowledgement, conflict and restore behavior. `scripts/acceptance` exercises the real HTTP/D1 Worker using isolated synthetic identities. `scripts/walk-native` drives the real GPUI app on a private Xvfb display; it never opens a window on the operator's desktop. With `HUNDRED_WALK_SYNC=1`, `scripts/test-worker` also exercises the real app's automatic retry across an outage and process restart (requires Xvfb, xdotool, ImageMagick, tesseract, jq and socat).

GPUI 0.2.2 is pinned rather than following upstream's moving API. Keyboard and pointer interaction are verified; screen-reader support and physical-device acceptance on other desktop platforms are not claimed.

Product contract: [USER_STORIES.md](USER_STORIES.md), [DOMAIN.md](DOMAIN.md), [DESIGN.md](DESIGN.md). Observed release evidence: [docs/verification.md](docs/verification.md).
