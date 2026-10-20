use hundred_core::{PAGE_LIMIT, RemoteEvent, SyncRequest, SyncResponse};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use worker::{wasm_bindgen::JsValue, *};

#[derive(Deserialize)]
struct Credential {
    sha256: String,
    owner: String,
}
#[derive(Deserialize)]
struct Row {
    seq: u64,
    payload: String,
}

#[event(fetch)]
pub async fn main(mut request: Request, env: Env, _context: Context) -> Result<Response> {
    // No request, credential or record contents are logged, including on errors.
    match route(&mut request, &env).await {
        Ok(response) => private(response),
        Err(error) => {
            let conflict = error.to_string().contains("event_conflict");
            private(Response::error(
                if conflict {
                    "Record identity conflict"
                } else {
                    "Service unavailable"
                },
                if conflict { 409 } else { 503 },
            )?)
        }
    }
}

fn private(mut response: Response) -> Result<Response> {
    let headers = response.headers_mut();
    headers.set("Cache-Control", "no-store")?;
    headers.set("X-Content-Type-Options", "nosniff")?;
    headers.set("Referrer-Policy", "no-referrer")?;
    headers.set(
        "Content-Security-Policy",
        "default-src 'none'; frame-ancestors 'none'",
    )?;
    Ok(response)
}

async fn route(request: &mut Request, env: &Env) -> Result<Response> {
    let path = request.path();
    if path == "/health" && request.method() == Method::Get {
        let db = env.d1("DB")?;
        db.prepare("SELECT seq FROM events LIMIT 1").all().await?;
        return Response::from_json(
            &serde_json::json!({"ok":true,"release":env.var("RELEASE")?.to_string()}),
        );
    }
    if path == "/" && request.method() == Method::Get {
        return Response::ok(
            "Hundred — a private native rep tracker. Your history is available only in your authenticated app.",
        );
    }
    if path != "/v1/identity" && path != "/v1/sync" {
        return Response::error("Not found", 404);
    }
    let credentials: Vec<Credential> =
        serde_json::from_str(&env.secret("CREDENTIALS")?.to_string())?;
    let token = request
        .headers()
        .get("Authorization")?
        .and_then(|header| header.strip_prefix("Bearer ").map(str::to_owned));
    let Some(token) = token.filter(|token| token.len() >= 32 && token.len() <= 256) else {
        return Response::error("Authentication required", 401);
    };
    let digest = format!("{:x}", Sha256::digest(token.as_bytes()));
    let owner = credentials
        .iter()
        .find(|credential| {
            credential.sha256.len() == 64
                && bool::from(credential.sha256.as_bytes().ct_eq(digest.as_bytes()))
                && !credential.owner.is_empty()
                && credential.owner.len() <= 64
        })
        .map(|credential| credential.owner.as_str());
    let Some(owner) = owner else {
        return Response::error("Credential rejected", 401);
    };
    if path == "/v1/identity" && request.method() == Method::Get {
        return Response::from_json(&serde_json::json!({"owner":owner}));
    }
    if path != "/v1/sync" || request.method() != Method::Post {
        return Response::error("Method not allowed", 405);
    }
    let length = request
        .headers()
        .get("Content-Length")?
        .and_then(|length| length.parse::<usize>().ok());
    if length.is_none_or(|length| length > 65_536) {
        return Response::error("Body too large or missing length", 413);
    }
    if !request
        .headers()
        .get("Content-Type")?
        .is_some_and(|content_type| content_type.starts_with("application/json"))
    {
        return Response::error("Send JSON", 415);
    }
    let text = request.text().await?;
    if text.len() > 65_536 {
        return Response::error("Body too large", 413);
    }
    let sync: SyncRequest = match serde_json::from_str(&text) {
        Ok(sync) => sync,
        Err(_) => return Response::error("Invalid sync request", 400),
    };
    if let Err(message) = sync.validate() {
        return Response::error(message, 400);
    }
    let db = env.d1("DB")?;
    let session = db.with_session_constraint(D1SessionConstraint::FirstPrimary)?;
    if !sync.events.is_empty() {
        let mut statements = Vec::with_capacity(sync.events.len());
        for event in &sync.events {
            statements.push(
                session
                    .prepare("INSERT OR IGNORE INTO events(owner,id,payload) VALUES(?1,?2,?3)")
                    .bind(&[
                        JsValue::from_str(owner),
                        JsValue::from_str(&event.id),
                        JsValue::from_str(&serde_json::to_string(event)?),
                    ])?,
            );
        }
        // D1 batch is transactional. The trigger also catches changed UUID content in races.
        let results = session.batch(statements).await?;
        if results.iter().any(|result| !result.success()) {
            return Response::error("Store unavailable", 503);
        }
    }
    let rows = session
        .prepare("SELECT seq,payload FROM events WHERE owner=?1 AND seq>?2 ORDER BY seq LIMIT ?3")
        .bind(&[
            JsValue::from_str(owner),
            JsValue::from_f64(sync.after as f64),
            JsValue::from_f64((PAGE_LIMIT + 1) as f64),
        ])?
        .all()
        .await?
        .results::<Row>()?;
    let more = rows.len() > PAGE_LIMIT;
    let records = rows
        .into_iter()
        .take(PAGE_LIMIT)
        .map(|row| {
            Ok(RemoteEvent {
                seq: row.seq,
                event: serde_json::from_str(&row.payload)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let next = records.last().map_or(sync.after, |record| record.seq);
    Response::from_json(&SyncResponse {
        acknowledged: sync.events.into_iter().map(|event| event.id).collect(),
        records,
        next,
        more,
    })
}
