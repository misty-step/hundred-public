# Hundred

A small Rust + GPUI + Cloudflare product. A keyboard-first desktop rep tracker for the daily hundred-rep routine; not medical advice.

## US-001 Log a set without interrupting my day
Select Push-ups, Crunches or Squats; enter 1–1000 reps; save with Enter or a labelled button. The app confirms only after durable local storage. Today's exact total, remaining reps toward 100, and over-goal reps update. Repeated sets are separate records. An error keeps the entered number.

## US-002 Never lose or double-count an offline set
Save without internet, close, reopen, and retain every set. Pending sets sync automatically when the server returns. A retry after a lost response never duplicates a set. Two desktops using the same identity converge without replacing each other's sets. Pending/syncing/authentication/error states are visible; no false Synced message.

## US-003 See progress and correct mistakes
See all three totals, recent sets, the last 28 days, completed days, best and current streaks. A day is complete only when each exercise reaches 100. Yesterday's streak remains current until today ends. Each set belongs to the local calendar date when it was recorded, even after travel. Undo cancels a selected set without deleting history and synchronizes to other desktops.

## US-004 Keep my exercise history private
The API requires a private bearer credential. Different identities cannot read or undo each other's history. Credentials and rep records never enter logs, analytics, Git or public pages. No model is required. Missing server authentication configuration fails closed.

## US-005 Get my history back
Export a versioned JSON archive including unsent records. Restore into a fresh local store, preserving record identities and cancellations. Resync never duplicates existing cloud records. Verify a restore against the original archive. D1 is the cloud authority; local SQLite is a durable outbox and reconstructable cache.

## US-006 Use a real native desktop app
Install a GPUI Linux executable and launcher without changing existing apps, global shortcuts or compositor configuration. Every primary action works by keyboard and pointer. The layout fits at 760×640 or larger. ASCII pigment art is deterministic, static and driven by real progress; exact numbers remain readable without colour. No phone client, cloud coaching or social features in this release.
