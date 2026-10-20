CREATE TABLE events (
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  owner TEXT NOT NULL,
  id TEXT NOT NULL,
  payload TEXT NOT NULL CHECK (json_valid(payload)),
  UNIQUE(owner, id)
) STRICT;
CREATE INDEX events_owner_seq ON events(owner, seq);
-- Reject UUID collisions atomically, including races between clients.
CREATE TRIGGER immutable_event BEFORE INSERT ON events
WHEN EXISTS (SELECT 1 FROM events WHERE owner = NEW.owner AND id = NEW.id AND payload != NEW.payload)
BEGIN
  SELECT RAISE(ABORT, 'event_conflict');
END;
CREATE TRIGGER no_event_updates BEFORE UPDATE ON events
BEGIN
  SELECT RAISE(ABORT, 'immutable_event');
END;
