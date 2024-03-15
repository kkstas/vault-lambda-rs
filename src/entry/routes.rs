use aws_sdk_dynamodb::Client;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::routing::{delete, get, put};
use axum::{Extension, Json, Router};
use chrono::{Duration, FixedOffset, Utc};
use serde::Serialize;
use serde_json::{json, Value};

use super::model::{Entry, EntryFC};
use super::TABLE_NAME;
use crate::entryproto::model::EntryProto;
use crate::entryproto::TABLE_NAME as ENTRYPROTO_TABLE_NAME;
use crate::AResult;

pub fn router(db_client: Client) -> Router {
    Router::new()
        .route("/", put(put_item))
        .route("/last-week", get(find_last_week))
        .route("/:pk/:sk", delete(delete_entry))
        .route("/:pk/:sk", get(query))
        .layer(Extension(db_client))
}

#[derive(Serialize)]
struct ProtoWithEntries {
    proto: EntryProto,
    entries: Vec<Entry>,
}

async fn find_last_week(
    Extension(db_client): Extension<Client>,
) -> AResult<(StatusCode, Json<Value>)> {
    let active_entry_proto_list =
        EntryProto::ddb_list_active(db_client.clone(), ENTRYPROTO_TABLE_NAME.to_string()).await?;

    let tz_offset = FixedOffset::east_opt(1 * 3600).unwrap();
    let week_ago = (Utc::now().with_timezone(&tz_offset) + Duration::days(-7))
        .format("%Y-%m-%d")
        .to_string();

    let mut result_entries: Vec<ProtoWithEntries> = Vec::new();

    for entry_proto in active_entry_proto_list {
        let t: Vec<Entry> = Entry::ddb_query(
            db_client.clone(),
            TABLE_NAME.to_string(),
            entry_proto.sk.clone(),
            week_ago.clone(),
        )
        .await?;
        result_entries.push(ProtoWithEntries {
            proto: entry_proto,
            entries: t,
        });
    }

    return Ok((StatusCode::OK, Json(json!(result_entries))));
}

async fn query(
    Extension(db_client): Extension<Client>,
    Path((pk, sk)): Path<(String, String)>,
) -> AResult<(StatusCode, Json<Value>)> {
    let response = Entry::ddb_query(db_client, TABLE_NAME.to_string(), pk, sk).await?;
    return Ok((StatusCode::OK, Json(json!(response))));
}

async fn put_item(
    Extension(client): Extension<Client>,
    Json(payload): Json<EntryFC>,
) -> AResult<StatusCode> {
    Entry::ddb_put_item(client, TABLE_NAME.to_string(), payload).await?;
    Ok(StatusCode::CREATED)
}

async fn delete_entry(
    Extension(client): Extension<Client>,
    Path((pk, sk)): Path<(String, String)>,
) -> AResult<StatusCode> {
    let query_res = Entry::ddb_query(
        client.clone(),
        TABLE_NAME.to_string(),
        pk.clone(),
        sk.clone(),
    )
    .await?;

    if query_res.is_empty() {
        return Ok(StatusCode::NOT_FOUND);
    }

    Entry::ddb_delete(client, TABLE_NAME.to_string(), pk, sk).await?;
    Ok(StatusCode::NO_CONTENT)
}
