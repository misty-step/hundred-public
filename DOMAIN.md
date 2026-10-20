# Domain and ownership

- Three exercises, each with a fixed daily goal of 100. Totals are not capped; remaining reps saturate at zero.
- Immutable UUID-addressed events: a positive set or a cancellation naming a set UUID. Repeated cancellation has the same effect as one. A cancellation received before its set still cancels it.
- An event's RFC3339 timestamp includes its original local UTC offset. Its calendar day is derived from that timestamp, never from the viewer's current zone or the server's clock.
- Complete means all three goals were met. Current streak ends today if complete, otherwise yesterday. Best streak spans all completed dates.
- D1 retains the canonical append-only log scoped to an authenticated owner. Global ingestion sequence supplies a pagination cursor; timestamps never act as cursors. UUID collisions with changed content fail, not overwrite.
- SQLite commits new records before saying Saved. FULL synchronous mode, WAL, busy timeout, private directories. Cache rows and acknowledgements commit together. Only acknowledged UUIDs leave the outbox. Response loss means retry with the same UUID.
- Pushes are bounded to 100 events; pulls to 500 events. The server rejects oversized requests, invalid records and unknown credentials. Sync never follows HTTP redirects carrying credentials.
- The desktop uses a background owner thread for database/network work. An unavailable server cannot stop local saving. Sync retries every 30 seconds and on request.
- Exports carry schema version 1 and immutable event data, not credentials. Restores merge idempotently; changed data under an existing UUID is rejected in a transaction. No history is physically deleted by product operations.
- No telemetry or AI. Only synthetic acceptance records may be written during verification.
