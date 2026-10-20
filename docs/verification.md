# Verification and release receipts

Commission: 2026-10-03. Product scope is US-001 through US-006, not a phone client or a general fitness platform.

## Real proof routes

- `cargo test -p hundred-core -p hundred-client`: six behavior tests exercise calendar offsets, all-three-goals streaks, out-of-order cancellations, exact over-goal counts, disk close/reopen, lost acknowledgements, conflicting restores, forged acknowledgements and identity binding.
- `scripts/test-worker`: the actual compiled Rust Wasm Worker runs in workerd with local D1. The acceptance client uses two newly generated identities and 510 immutable synthetic records. It exercises response loss after server commit, independent desktops, cross-owner refusal, cancellation isolation, atomic collision rejection, 100-record pushes, 500-record pages and full restore/resync. This path also runs in CI; lint and packaging alone are not server proof.
- `scripts/walk-native`: the actual GPUI executable on a private Xvfb display. It drives keys and pointer, checks resulting SQLite progress through the actual CLI, captures empty/partial/complete/reopened/history/narrow states and compares an exported/restored archive byte for byte. It waits for visible text rather than capturing an unpainted software-Vulkan surface.
- `HUNDRED_WALK_SYNC=1 scripts/test-worker`: the actual GPUI background owner runs behind a disposable loopback TCP relay. The relay stops, a set is saved, the app process restarts offline, the relay returns, and the automatic retry must drain the outbox without pressing Sync. A second independent store must read exactly one set.
- Live provider acceptance repeats the HTTP/D1 walk with temporary synthetic credentials on the deployed Cloudflare Worker. Those temporary credentials are removed afterward.
- Installed-launcher verification checks the native binary revision and exercises private sync with no workout writes. Missing or locked pass access falls back to offline operation.

Release receipts, provider versions, installed binary hashes, and private evidence
paths belong in external operational records. This document describes verification
routes; it is not an exact-head release receipt. No private PR or resource identity
is needed to run the synthetic checks.

## Deliberate limits

Linux x86-64 is verified. GPUI 0.2.2 is pre-1.0; screen-reader acceptance, other desktop platforms and physical-phone use are not claimed. The character field is static and interpretive; only the numeric totals and history trace are measurements. No physiological benefit, exercise prescription or medical advice is implied.

Exports include private exercise records. A verified local round trip is not an off-device backup until the operator copies that archive to their approved encrypted storage. Cloudflare D1 Time Travel is provider recovery, not an independent offsite copy.

## Configuration checks

`scripts/test-config` runs the launcher with a temporary HOME and a minimal PATH.
The fake pass-env and fake binary verify environment precedence, literal config
parsing, explicit and XDG paths, missing/locked pass fallback, exact token bytes,
and token isolation from arguments and output. It never opens a real pass entry,
installed launcher, local exercise store, or network endpoint.

The client behavior tests prove that missing server or token configuration keeps
pending local sets intact. Full native/Worker acceptance remains a separate check;
launcher doubles do not prove GUI or provider behavior.
