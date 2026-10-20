# Owner sign-in

Hundred has no login page. The owner signs in by giving the desktop app a bearer token. The deployed Worker hashes the token and maps it to an owner identity, and only that identity's history is ever returned. The real owner is the maintainer, phaedrus, through the token his installed app uses. When sign-in works, his app shows `Synced` and pulls his full history into a fresh store. A missing token leaves the app local-only. A wrong token shows `Sync credential rejected. Local sets are safe.` The production server origin, the token's pass entry name and the owner label are private by this repository's rule (see README, "Run a local Worker"). They live only in the operator's private configuration, never in this repository.

## Sub-features

- `sign-in-config` resolves the server and token pass entry from the operator's launcher config, and the expected owner from `verify.env`.
- `sign-in-identity` returns `GET /v1/identity` 200 for the owner token, with an owner equal to the expected owner.
- `sign-in-app` shows `Synced` in the real app with the owner token and a fresh store, with the owner's history pulled and `pending` at 0.
- `sign-in-signed-out` answers `GET /v1/identity` with no token with 401 `Authentication required`.
- `sign-in-wrong-credential` answers a random 64-hex token with 401 `Credential rejected`, and the app shows `Sync credential rejected`, never `Synced`.
- `sign-in-launcher` makes the installed `hundred` launcher read the same config and fetch the token through `pass-env`. It falls back to offline if pass is locked.

## How to get to it (user POV)

- Launch **Hundred** from the menu. The launcher reads `~/.config/hundred/launcher.env` (`HUNDRED_SERVER`, `HUNDRED_TOKEN_PASS_ENTRY`), takes the token from pass, and starts the app with it in the environment only.
- The status line under the title shows the sign-in result.

## Driving it with verify-hundred

Preconditions:

- The operator's private files exist, with directory mode 0700 and file mode 0600:
  - `~/.config/hundred/launcher.env` holds `HUNDRED_SERVER=<https origin>` and `HUNDRED_TOKEN_PASS_ENTRY=<pass entry>`. `HUNDRED_CONFIG` selects another file.
  - `~/.config/hundred/verify.env` holds `HUNDRED_EXPECTED_OWNER=<owner label>`.
  - Environment variables of the same names override both files. The private repository `misty-step/hundred` records the maintainer's values in its own copy of this skill.
- The pass store is unlocked: `pass-env list` shows the entry, and `pass-env run -e HUNDRED_TOKEN=<entry> -- true` exits 0.
- `$V doctor --prod` prints no `FAIL`.

- **Full journey.** Run `$V sign-in`. It runs every step below in order, stops at the first failure, and prints `PASS owner sign-in: …` at the end.
- **Health.** Check `GET /health`. It returns `{"ok":true,"release":"<deployed commit>"}`. The release is a commit in the private deployment repository.
- **Signed out.** Send `GET /v1/identity` with no token. It returns HTTP 401.
- **Wrong credential.** Send a fresh random 64-hex token. It returns HTTP 401 `Credential rejected`.
- **Owner identity.** Send the owner token, read from pass inside a child shell and passed to curl through `-K -` on stdin. It returns HTTP 200, and `owner` equals `HUNDRED_EXPECTED_OWNER`. The transcript records only `matches expected: yes`.
- **Owner app.** Run `$V app start --prod` on a fresh store. `$V app wait-text 'Synced' 60` matches, `status` shows `pending` 0, and `cli history` is non-empty. The owner's real history renders only for the owner. The full screenshot stays in `private/`. `owner-synced-header.png` is a crop showing only the title, date and `Synced`.
- **Wrong credential in the app.** Start the app with a random token file. `$V app wait-text 'credential rejected' 45` matches. `wrong-credential.png` shows an empty store and the rejection.
- **Installed launcher.** Run `$V doctor`. It prints `installed app: hundred <version> (<commit>)`. Compare that commit with what you expect to have shipped. `scripts/test-config` proves the launcher's config parsing and pass fallback with doubles.
- **Teardown.** `sign-in` stops both sessions and deletes the owner store itself.

## Gotchas

- This journey is read-only. The store is empty, so its sync uploads nothing. Never log, undo or restore in a `--prod` session. That writes to the owner's production history.
- If pass is locked (gpg-agent has no cached passphrase), `sign-in` stops with `owner sign-in UNPROVEN: pass entry unreadable`. Ask the operator to unlock it. Do not substitute a synthetic token, because that proves nothing about the owner.
- `owner matches expected: NO` means the deployed `CREDENTIALS` mapping or `verify.env` is wrong. The owner's existing history is stored under the old label and would become invisible. Treat it as a production incident, not a skill fix.
- Committed transcripts redact the server and owner (`VERIFY_HUNDRED_REDACT=1`, the default). Never commit `private/`.
- The installed launcher before the public extraction read a fixed pass entry and had the server compiled in. It ignores `launcher.env`. `doctor` shows the installed commit, so you can tell which launcher is live.
