# Back up and restore

The owner exports a version-1 JSON archive of every set and cancellation, including ones not yet synced, to a new private file (mode 0600). Restoring it into a fresh store reproduces the same history with the same record identities. Exporting again yields identical bytes. Export never overwrites an existing file.

## Sub-features

- `backup-export` with `hundred export <new file>` prints `Exported N records.` and writes a 0600 file.
- `backup-no-overwrite` refuses an existing path with `Choose a new archive filename`, and the file is untouched.
- `backup-restore` with `hundred --data-dir <new> restore <file>` prints `Restored N records. Sync to reconcile with the cloud.`
- `backup-roundtrip` produces a restored store whose re-export is byte-identical to the original.
- `backup-unsent` includes pending, unsynced sets and cancellations in the export.

## How to get to it (user POV)

- In a terminal: `hundred export ~/hundred-backup.json`, then `hundred --data-dir ~/hundred-restored restore ~/hundred-backup.json`.

## Driving it with verify-hundred

Preconditions:

- An offline session with a set and an undone set: `$V app start`, log `20` squats, log `10` push-ups, then `$V app key ctrl+z`.
- `<run>` below is the `evidence` directory `app start` printed.

- **Export.** Run `$V app cli export <run>/archive.json`. Stdout reads `Exported 3 records.` and `stat -c %a <run>/archive.json` prints `600`.
- **No overwrite.** Run `$V app cli export <run>/archive.json` again. It exits 1 with `Choose a new archive filename`, and `sha256sum` of the file is unchanged.
- **Restore.** Run `HUNDRED_TOKEN= target/debug/hundred --data-dir <run>/restored restore <run>/archive.json`. It reports `Restored 3 records.`
- **Round trip.** Run `target/debug/hundred --data-dir <run>/restored export <run>/roundtrip.json`, then `cmp <run>/archive.json <run>/roundtrip.json`. It exits 0. `status` on the restored store shows `[0,0,20]`.
- **Unsent included.** The archive contains the `cancel` event, and the session's `pending` was non-zero at export.
- **Scripted equivalent.** The last step of `$V walk native` does the export, restore and `cmp`.
- **Teardown.** Run `$V app stop`, then delete `<run>/restored`. Archives from synthetic sessions may stay as evidence.

## Gotchas

- Archives from a real store contain private exercise data. Never commit or attach one. Only archives from disposable sessions are evidence.
- Restoring into a store bound to another identity, then syncing, is refused. Restore into a fresh `--data-dir`.
- `restore` merges and never replaces. A conflicting record with the same UUID and different content is rejected as a whole.
