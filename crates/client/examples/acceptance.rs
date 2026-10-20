//! Real HTTP/D1 acceptance. Only run against newly provisioned synthetic identities.
use anyhow::{Result, ensure};
use chrono::NaiveDate;
use hundred_client::{Store, sync};
use hundred_core::{Archive, Change, Exercise, SyncRequest, SyncResponse, progress};
use reqwest::blocking::Client;
use std::time::Duration;

fn main() -> Result<()> {
    let server = std::env::var("HUNDRED_SERVER")?;
    let token = std::env::var("HUNDRED_QA_TOKEN")?;
    let other = std::env::var("HUNDRED_QA_OTHER_TOKEN")?;
    let http = Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    ensure!(
        http.get(format!("{server}/v1/identity"))
            .send()?
            .status()
            .as_u16()
            == 401
    );
    ensure!(
        http.get(format!("{server}/v1/identity"))
            .bearer_auth("x".repeat(64))
            .send()?
            .status()
            .as_u16()
            == 401
    );
    let a_dir = tempfile::tempdir()?;
    let b_dir = tempfile::tempdir()?;
    let c_dir = tempfile::tempdir()?;
    let mut a = Store::open(a_dir.path())?;
    sync(&mut a, &server, &token)?;
    ensure!(
        a.events()?.is_empty(),
        "Acceptance identity must start empty"
    );
    let set = a.add(
        Exercise::Squats,
        20,
        Some("2026-10-03T23:59:00-05:00".into()),
    )?;
    drop(a);
    let mut a = Store::open(a_dir.path())?;
    ensure!(a.pending()? == 1 && a.events()? == vec![set.clone()]);
    let pending = a.request()?;
    // Server commits; deliberately do not apply its response, simulating response loss.
    let response = http
        .post(format!("{server}/v1/sync"))
        .bearer_auth(&token)
        .json(&pending)
        .send()?;
    ensure!(response.status().is_success());
    drop(response);
    sync(&mut a, &server, &token)?;
    ensure!(a.pending()? == 0 && a.events()? == vec![set.clone()]);
    let mut b = Store::open(b_dir.path())?;
    sync(&mut b, &server, &token)?;
    ensure!(b.events()? == a.events()?);
    b.add(
        Exercise::PushUps,
        30,
        Some("2026-10-03T12:00:00+02:00".into()),
    )?;
    a.add(
        Exercise::Crunches,
        20,
        Some("2026-10-03T12:00:00-05:00".into()),
    )?;
    sync(&mut b, &server, &token)?;
    sync(&mut a, &server, &token)?;
    sync(&mut b, &server, &token)?;
    ensure!(a.events()? == b.events()?);
    let today = NaiveDate::from_ymd_opt(2026, 10, 3).unwrap();
    ensure!(progress(&a.events()?, today).days[&today] == [30, 20, 20]);
    b.undo(Some(&set.id))?;
    sync(&mut b, &server, &token)?;
    sync(&mut a, &server, &token)?;
    ensure!(progress(&a.events()?, today).days[&today] == [30, 20, 0]);
    println!(
        "PASS offline restart, committed-response loss, retry, two desktops, undo, original local date"
    );

    let mut invalid = set.clone();
    invalid.change = Change::Set {
        exercise: Exercise::Squats,
        reps: 0,
    };
    let response = http
        .post(format!("{server}/v1/sync"))
        .bearer_auth(&token)
        .json(&SyncRequest {
            after: 0,
            events: vec![invalid],
        })
        .send()?;
    ensure!(response.status().as_u16() == 400);
    let mut collision = set.clone();
    collision.change = Change::Set {
        exercise: Exercise::Squats,
        reps: 99,
    };
    let before = a.events()?;
    let new = hundred_core::Event {
        id: uuid::Uuid::new_v4().to_string(),
        ..set.clone()
    };
    let response = http
        .post(format!("{server}/v1/sync"))
        .bearer_auth(&token)
        .json(&SyncRequest {
            after: 0,
            events: vec![new, collision],
        })
        .send()?;
    ensure!(
        response.status().as_u16() == 409,
        "Changed UUID must conflict atomically"
    );
    sync(&mut a, &server, &token)?;
    ensure!(
        a.events()? == before,
        "Conflicting batch wrote a partial new record"
    );

    let mut c = Store::open(c_dir.path())?;
    sync(&mut c, &server, &other)?;
    ensure!(c.events()?.is_empty(), "Second identity must start empty");
    let malicious_cancel = hundred_core::Event {
        id: uuid::Uuid::new_v4().to_string(),
        at: set.at.clone(),
        change: Change::Cancel {
            target: set.id.clone(),
        },
    };
    c.append(&malicious_cancel)?;
    sync(&mut c, &server, &other)?;
    sync(&mut a, &server, &token)?;
    ensure!(
        a.events()? == before,
        "Another identity changed the first identity's log"
    );
    a.add(Exercise::Squats, 10, Some(set.at.clone()))?;
    ensure!(
        sync(&mut a, &server, &other).is_err(),
        "Switching identity must fail before uploading pending sets"
    );
    sync(&mut c, &server, &other)?;
    ensure!(
        c.events()?.len() == 1,
        "Wrong identity received pending private sets"
    );
    sync(&mut a, &server, &token)?;
    println!(
        "PASS unauthorized access, invalid input, atomic conflict, owner isolation, credential switching"
    );

    for _ in 0..505 {
        a.add(
            Exercise::PushUps,
            1,
            Some("2026-10-04T10:00:00-05:00".into()),
        )?;
    }
    sync(&mut a, &server, &token)?;
    let first: SyncResponse = http
        .post(format!("{server}/v1/sync"))
        .bearer_auth(&token)
        .json(&SyncRequest {
            after: 0,
            events: vec![],
        })
        .send()?
        .error_for_status()?
        .json()?;
    ensure!(
        first.records.len() == 500 && first.more,
        "Real D1 pagination was not exercised"
    );
    let fresh_dir = tempfile::tempdir()?;
    let mut fresh = Store::open(fresh_dir.path())?;
    sync(&mut fresh, &server, &token)?;
    ensure!(fresh.events()? == a.events()?);
    let archive: Archive = serde_json::from_slice(&serde_json::to_vec(&a.archive()?)?)?;
    let restored_dir = tempfile::tempdir()?;
    let mut restored = Store::open(restored_dir.path())?;
    restored.restore(&archive)?;
    ensure!(restored.events()? == archive.events);
    sync(&mut restored, &server, &token)?;
    sync(&mut a, &server, &token)?;
    ensure!(restored.events()? == archive.events && a.events()? == archive.events);
    println!(
        "PASS 100-record pushes, 500-record pages, fresh desktop reconstruction, full archive restore and resync"
    );
    println!(
        "Acceptance passed: {} immutable synthetic records; no production identity was used.",
        archive.events.len()
    );
    Ok(())
}
