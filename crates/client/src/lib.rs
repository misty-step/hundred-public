use anyhow::{Context, Result, bail, ensure};
use chrono::Local;
use hundred_core::{
    Archive, Change, Event, Exercise, PAGE_LIMIT, PUSH_LIMIT, SyncRequest, SyncResponse,
    active_sets,
};
use reqwest::blocking::Client;
use rusqlite::{Connection, OptionalExtension, params};
use std::{
    collections::HashSet,
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::{Path, PathBuf},
    time::Duration,
};
use uuid::Uuid;

pub const DEFAULT_SERVER: &str = "";

pub struct Store {
    connection: Connection,
}

impl Store {
    pub fn open(directory: &Path) -> Result<Self> {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(directory)?;
        ensure!(
            !directory.symlink_metadata()?.file_type().is_symlink(),
            "Data directory must not be a symlink"
        );
        fs::set_permissions(directory, fs::Permissions::from_mode(0o700))?;
        let path = directory.join("hundred.sqlite");
        if let Ok(metadata) = path.symlink_metadata() {
            ensure!(
                !metadata.file_type().is_symlink(),
                "Database must not be a symlink"
            );
        }
        let connection = Connection::open(&path)?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
            CREATE TABLE IF NOT EXISTS events(id TEXT PRIMARY KEY, payload TEXT NOT NULL, pending INTEGER NOT NULL CHECK(pending IN (0,1)));
            CREATE TABLE IF NOT EXISTS metadata(key TEXT PRIMARY KEY, value TEXT NOT NULL);
            INSERT OR IGNORE INTO metadata VALUES('cursor','0');")?;
        Ok(Self { connection })
    }

    pub fn events(&self) -> Result<Vec<Event>> {
        let mut statement = self
            .connection
            .prepare("SELECT payload FROM events ORDER BY id")?;
        statement
            .query_map([], |row| row.get::<_, String>(0))?
            .map(|payload| {
                let event: Event = serde_json::from_str(&payload?)?;
                event.validate().map_err(anyhow::Error::msg)?;
                Ok(event)
            })
            .collect()
    }

    pub fn pending(&self) -> Result<usize> {
        Ok(self
            .connection
            .query_row("SELECT COUNT(*) FROM events WHERE pending=1", [], |row| {
                row.get(0)
            })?)
    }

    pub fn append(&mut self, event: &Event) -> Result<()> {
        event.validate().map_err(anyhow::Error::msg)?;
        let transaction = self.connection.transaction()?;
        insert_event(&transaction, event, true)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn add(&mut self, exercise: Exercise, reps: u16, at: Option<String>) -> Result<Event> {
        let event = Event {
            id: Uuid::new_v4().to_string(),
            at: at.unwrap_or_else(|| Local::now().to_rfc3339()),
            change: Change::Set { exercise, reps },
        };
        self.append(&event)?;
        Ok(event)
    }

    pub fn undo(&mut self, target: Option<&str>) -> Result<Event> {
        let events = self.events()?;
        let active = active_sets(&events);
        let target = match target {
            Some(id) => active.into_iter().find(|event| event.id == id),
            None => active.last().copied(),
        }
        .context("No active set to undo")?;
        let event = Event {
            id: Uuid::new_v4().to_string(),
            at: Local::now().to_rfc3339(),
            change: Change::Cancel {
                target: target.id.clone(),
            },
        };
        self.append(&event)?;
        Ok(event)
    }

    pub fn request(&self) -> Result<SyncRequest> {
        let after = self
            .connection
            .query_row("SELECT value FROM metadata WHERE key='cursor'", [], |row| {
                row.get::<_, String>(0)
            })?
            .parse()?;
        let mut statement = self
            .connection
            .prepare("SELECT payload FROM events WHERE pending=1 ORDER BY id LIMIT ?1")?;
        let events = statement
            .query_map([PUSH_LIMIT], |row| row.get::<_, String>(0))?
            .map(|payload| Ok(serde_json::from_str(&payload?)?))
            .collect::<Result<Vec<Event>>>()?;
        Ok(SyncRequest { after, events })
    }

    /// Validate the transport before allowing it to acknowledge durable records.
    pub fn apply(&mut self, request: &SyncRequest, response: &SyncResponse) -> Result<()> {
        let submitted: HashSet<_> = request
            .events
            .iter()
            .map(|event| event.id.as_str())
            .collect();
        let mut acknowledgements = HashSet::new();
        for id in &response.acknowledged {
            ensure!(
                submitted.contains(id.as_str()) && acknowledgements.insert(id),
                "Invalid sync acknowledgement"
            );
        }
        ensure!(
            response.records.len() <= PAGE_LIMIT,
            "Oversized sync response"
        );
        let mut previous = request.after;
        for record in &response.records {
            ensure!(
                record.seq > previous && record.seq <= hundred_core::MAX_SEQUENCE,
                "Invalid sync cursor"
            );
            record.event.validate().map_err(anyhow::Error::msg)?;
            previous = record.seq;
        }
        ensure!(
            response.next == previous && (!response.more || !response.records.is_empty()),
            "Invalid sync page"
        );
        let transaction = self.connection.transaction()?;
        for record in &response.records {
            insert_event(&transaction, &record.event, false)?;
        }
        for id in &response.acknowledged {
            transaction.execute("UPDATE events SET pending=0 WHERE id=?1", [id])?;
        }
        transaction.execute("UPDATE metadata SET value=CAST(MAX(CAST(value AS INTEGER), ?1) AS TEXT) WHERE key='cursor'", [response.next])?;
        transaction.commit()?;
        Ok(())
    }

    pub fn archive(&self) -> Result<Archive> {
        Ok(Archive {
            version: 1,
            events: self.events()?,
        })
    }

    pub fn restore(&mut self, archive: &Archive) -> Result<()> {
        ensure!(archive.version == 1, "Unsupported archive version");
        for event in &archive.events {
            event.validate().map_err(anyhow::Error::msg)?;
        }
        let transaction = self.connection.transaction()?;
        for event in &archive.events {
            insert_event(&transaction, event, true)?;
        }
        transaction.commit()?;
        Ok(())
    }

    fn bind_identity(&mut self, server: &str, owner: &str) -> Result<()> {
        let identity = serde_json::to_string(&(server, owner))?;
        let transaction = self.connection.transaction()?;
        let existing: Option<String> = transaction
            .query_row(
                "SELECT value FROM metadata WHERE key='identity'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        ensure!(
            existing
                .as_ref()
                .is_none_or(|existing| existing == &identity),
            "This local store belongs to another server or identity. Choose a separate --data-dir."
        );
        transaction.execute(
            "INSERT OR IGNORE INTO metadata VALUES('identity',?1)",
            [identity],
        )?;
        transaction.commit()?;
        Ok(())
    }
}

fn insert_event(connection: &Connection, event: &Event, pending: bool) -> Result<()> {
    let payload = serde_json::to_string(event)?;
    let existing: Option<String> = connection
        .query_row(
            "SELECT payload FROM events WHERE id=?1",
            [&event.id],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(existing) = existing {
        ensure!(
            existing == payload,
            "A record identity has conflicting content; nothing was replaced"
        );
    } else {
        connection.execute(
            "INSERT INTO events VALUES(?1,?2,?3)",
            params![event.id, payload, pending],
        )?;
    }
    Ok(())
}

pub fn sync(store: &mut Store, server: &str, token: &str) -> Result<()> {
    ensure!(
        !server.trim().is_empty() && !token.trim().is_empty(),
        "Sync is not configured. Local sets are safe."
    );
    let url = reqwest::Url::parse(server).context("Invalid server URL")?;
    ensure!(
        url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none(),
        "Use a plain server origin"
    );
    let loopback = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    ensure!(
        url.scheme() == "https" || (url.scheme() == "http" && loopback),
        "Sync requires HTTPS except on loopback"
    );
    ensure!(url.path() == "/", "Use the server origin without a path");
    let server = url.as_str().trim_end_matches('/');
    let client = Client::builder()
        .timeout(Duration::from_secs(12))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let identity = client
        .get(format!("{server}/v1/identity"))
        .bearer_auth(token)
        .send()
        .context("Server unreachable. Local sets are safe.")?;
    check_status(identity.status())?;
    let identity: Identity = identity.json()?;
    ensure!(
        !identity.owner.is_empty() && identity.owner.len() <= 64,
        "Invalid server identity"
    );
    store.bind_identity(server, &identity.owner)?;
    loop {
        let request = store.request()?;
        let response = client
            .post(format!("{server}/v1/sync"))
            .bearer_auth(token)
            .json(&request)
            .send()
            .context("Server unreachable. Local sets are safe.")?;
        check_status(response.status())?;
        ensure!(
            response.content_length().is_none_or(|len| len <= 2_000_000),
            "Oversized sync response"
        );
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut std::io::Read::take(response, 2_000_001), &mut bytes)?;
        ensure!(bytes.len() <= 2_000_000, "Oversized sync response");
        let response: SyncResponse = serde_json::from_slice(&bytes)?;
        store.apply(&request, &response)?;
        if !response.more && store.pending()? == 0 {
            break;
        }
        ensure!(
            made_progress(&request, &response),
            "Server made no sync progress. Local sets are safe."
        );
    }
    Ok(())
}

/// A set saved after this round's request was built is still pending, but the
/// round sent nothing, so an empty reply is not a stall: the next round sends it.
fn made_progress(request: &SyncRequest, response: &SyncResponse) -> bool {
    request.events.is_empty() || !response.acknowledged.is_empty() || !response.records.is_empty()
}

#[derive(serde::Deserialize)]
struct Identity {
    owner: String,
}

fn check_status(status: reqwest::StatusCode) -> Result<()> {
    match status.as_u16() {
        200 => Ok(()),
        401 | 403 => bail!("Sync credential rejected. Local sets are safe."),
        409 => bail!("Record conflict. Local sets are safe; export your history before repair."),
        _ => bail!(
            "Sync returned HTTP {}. Local sets are safe.",
            status.as_u16()
        ),
    }
}

pub fn default_data_dir() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share")
        })
        .join("hundred")
}

#[cfg(test)]
mod tests {
    use super::*;
    use hundred_core::{RemoteEvent, progress};
    #[test]
    fn offline_restart_lost_response_retry_and_restore_preserve_one_set() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let mut store = Store::open(directory.path())?;
        let event = store.add(
            Exercise::Squats,
            20,
            Some("2026-10-03T23:59:00-05:00".into()),
        )?;
        drop(store);
        let mut store = Store::open(directory.path())?;
        assert_eq!(store.pending()?, 1);
        let request = store.request()?;
        let response = SyncResponse {
            acknowledged: vec![event.id.clone()],
            records: vec![RemoteEvent {
                seq: 7,
                event: event.clone(),
            }],
            next: 7,
            more: false,
        };
        // A response lost before apply leaves the exact UUID in the outbox.
        assert_eq!(store.request()?.events, request.events);
        store.apply(&request, &response)?;
        store.apply(&request, &response)?;
        assert_eq!(store.pending()?, 0);
        assert_eq!(store.events()?, vec![event.clone()]);
        assert_eq!(store.request()?.after, 7);
        store.undo(None)?;
        let archive = store.archive()?;
        let restored = tempfile::tempdir()?;
        let mut restored = Store::open(restored.path())?;
        restored.restore(&archive)?;
        restored.restore(&archive)?;
        assert_eq!(restored.events()?, archive.events);
        assert!(progress(&restored.events()?, event.day()).days.is_empty());
        Ok(())
    }
    #[test]
    fn conflicting_restore_and_forged_acknowledgement_cannot_erase_pending_data() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let mut store = Store::open(directory.path())?;
        let event = store.add(Exercise::PushUps, 20, None)?;
        let mut changed = event.clone();
        changed.change = Change::Set {
            exercise: Exercise::PushUps,
            reps: 99,
        };
        let new_event = Event {
            id: Uuid::new_v4().to_string(),
            ..event.clone()
        };
        assert!(
            store
                .restore(&Archive {
                    version: 1,
                    events: vec![new_event, changed]
                })
                .is_err()
        );
        assert_eq!(store.events()?, vec![event]);
        let request = store.request()?;
        assert!(
            store
                .apply(
                    &request,
                    &SyncResponse {
                        acknowledged: vec![Uuid::new_v4().to_string()],
                        records: vec![],
                        next: 0,
                        more: false
                    }
                )
                .is_err()
        );
        assert_eq!(store.pending()?, 1);
        Ok(())
    }
    #[test]
    fn unconfigured_sync_preserves_pending_local_sets() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let mut store = Store::open(directory.path())?;
        let event = store.add(Exercise::Squats, 20, None)?;
        for (server, token) in [
            (DEFAULT_SERVER, "synthetic-token"),
            ("https://sync.example.invalid", ""),
            (" ", "synthetic-token"),
        ] {
            assert_eq!(
                sync(&mut store, server, token).unwrap_err().to_string(),
                "Sync is not configured. Local sets are safe."
            );
            assert_eq!(store.pending()?, 1);
            assert_eq!(store.events()?, vec![event.clone()]);
        }
        Ok(())
    }
    #[test]
    fn set_saved_during_a_round_is_sent_next_round_not_reported_as_a_stall() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let mut store = Store::open(directory.path())?;
        // The round's request is built before the owner saves a set.
        let request = store.request()?;
        let event = store.add(Exercise::Squats, 5, None)?;
        let empty = SyncResponse {
            acknowledged: vec![],
            records: vec![],
            next: 0,
            more: false,
        };
        store.apply(&request, &empty)?;
        assert_eq!(store.pending()?, 1);
        assert!(made_progress(&request, &empty));
        // The next round carries the set; a server that then acknowledges nothing is a stall.
        let next = store.request()?;
        assert_eq!(next.events, vec![event]);
        assert!(!made_progress(&next, &empty));
        Ok(())
    }
    #[test]
    fn identity_change_is_refused_before_upload() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let mut store = Store::open(directory.path())?;
        store.bind_identity("https://example.com", "a")?;
        assert!(store.bind_identity("https://example.com", "b").is_err());
        assert!(
            store
                .bind_identity("https://other.example.com", "a")
                .is_err()
        );
        store.bind_identity("https://example.com", "a")?;
        Ok(())
    }
}
