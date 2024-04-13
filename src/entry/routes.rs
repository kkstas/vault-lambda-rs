use std::collections::HashMap;

use aws_sdk_dynamodb::types::{AttributeValue, KeysAndAttributes};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, put};
use axum::{Json, Router};
use serde::Serialize;
use serde_dynamo::from_items;
use serde_json::{json, Value};

use super::{Entry, EntryFC};
use crate::entryproto::EntryProto;
use crate::utils::time::get_date_x_days_ago;
use crate::{AResult, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", put(put_item))
        .route("/:pk/:sk", delete(delete_entry))
        .route("/:pk/:sk", get(query))
        .route("/:date", get(find_by_date))
        .route("/last-week", get(find_last_week_handler))
}

#[derive(Serialize)]
pub struct ProtoWithEntries {
    pub proto: EntryProto,
    pub entries: Vec<Entry>,
}

async fn find_last_week_handler(
    State(state): State<AppState>,
) -> AResult<(StatusCode, Json<Value>)> {
    let result_entries = find_last_week_entries(&state).await?;

    return Ok((StatusCode::OK, Json(json!(result_entries))));
}

pub async fn find_last_week_entries(state: &AppState) -> AResult<Vec<ProtoWithEntries>> {
    let active_ep = EntryProto::ddb_list_active(state).await?;
    let week_ago = get_date_x_days_ago(7);
    let mut result_entries: Vec<ProtoWithEntries> = Vec::new();

    for entry_proto in active_ep {
        let t: Vec<Entry> = Entry::ddb_query(&state, &entry_proto.sk, &week_ago).await?;
        result_entries.push(ProtoWithEntries {
            proto: entry_proto,
            entries: t,
        });
    }
    Ok(result_entries)
}

#[derive(Serialize)]
struct FindEntriesResult {
    entries: Vec<Entry>,
    proto: Vec<EntryProto>,
}

async fn find_by_date(
    State(state): State<AppState>,
    Path(date): Path<String>,
) -> AResult<(StatusCode, Json<Value>)> {
    let active_ep = EntryProto::ddb_list_active(&state).await?;
    let v: Vec<HashMap<String, AttributeValue>> = active_ep
        .iter()
        .map(|ep| {
            HashMap::from([
                ("pk".to_string(), AttributeValue::S(ep.sk.clone())),
                ("sk".to_string(), AttributeValue::S(date.clone())),
            ])
        })
        .collect();

    let ddb_res = state
        .dynamodb_client
        .batch_get_item()
        .request_items(
            &state.table_name,
            KeysAndAttributes::builder().set_keys(Some(v)).build()?,
        )
        .send()
        .await?;

    let entries: Vec<Entry> = from_items(
        ddb_res
            .responses
            .unwrap()
            .remove(&state.table_name)
            .unwrap(),
    )?;

    Ok((
        StatusCode::OK,
        Json(json!({ "entries": entries, "proto": active_ep })),
    ))
}

async fn query(
    State(state): State<AppState>,
    Path((pk, sk)): Path<(String, String)>,
) -> AResult<(StatusCode, Json<Value>)> {
    let response = Entry::ddb_query(&state, pk, sk).await?;
    return Ok((StatusCode::OK, Json(json!(response))));
}

async fn put_item(
    State(state): State<AppState>,
    Json(payload): Json<EntryFC>,
) -> AResult<StatusCode> {
    Entry::ddb_put_item(&state, payload).await?;
    Ok(StatusCode::CREATED)
}

async fn delete_entry(
    State(state): State<AppState>,
    Path((pk, sk)): Path<(String, String)>,
) -> AResult<StatusCode> {
    let query_res = Entry::ddb_query(&state, &pk, &sk).await?;

    if query_res.is_empty() {
        return Ok(StatusCode::NOT_FOUND);
    }

    Entry::ddb_delete(&state, pk, sk).await?;
    Ok(StatusCode::NO_CONTENT)
}
