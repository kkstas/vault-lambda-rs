use std::collections::HashMap;

use aws_sdk_dynamodb::types::{AttributeValue, KeysAndAttributes};
use aws_sdk_dynamodb::Client;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::routing::{delete, get, put};
use axum::{Extension, Json, Router};
use serde::Serialize;
use serde_dynamo::from_items;
use serde_json::{json, Value};

use super::TABLE_NAME;
use super::{Entry, EntryFC};
use crate::entryproto::EntryProto;
use crate::utils::time::get_date_x_days_ago;
use crate::AResult;

pub fn router() -> Router {
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
    Extension(db_client): Extension<Client>,
) -> AResult<(StatusCode, Json<Value>)> {
    let result_entries = find_last_week_entries(db_client).await?;

    return Ok((StatusCode::OK, Json(json!(result_entries))));
}

pub async fn find_last_week_entries(db_client: Client) -> AResult<Vec<ProtoWithEntries>> {
    let active_ep = EntryProto::ddb_list_active(db_client.clone()).await?;
    let week_ago = get_date_x_days_ago(7);
    let mut result_entries: Vec<ProtoWithEntries> = Vec::new();

    for entry_proto in active_ep {
        let t: Vec<Entry> =
            Entry::ddb_query(db_client.clone(), entry_proto.sk.clone(), week_ago.clone()).await?;
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
    Extension(db_client): Extension<Client>,
    Path(date): Path<String>,
) -> AResult<(StatusCode, Json<Value>)> {
    let active_ep = EntryProto::ddb_list_active(db_client.clone()).await?;
    let v: Vec<HashMap<String, AttributeValue>> = active_ep
        .iter()
        .map(|ep| {
            HashMap::from([
                ("pk".to_string(), AttributeValue::S(ep.sk.clone())),
                ("sk".to_string(), AttributeValue::S(date.clone())),
            ])
        })
        .collect();

    let ddb_res = db_client
        .batch_get_item()
        .request_items(
            TABLE_NAME,
            KeysAndAttributes::builder().set_keys(Some(v)).build()?,
        )
        .send()
        .await?;

    let entries: Vec<Entry> = from_items(ddb_res.responses.unwrap().remove(TABLE_NAME).unwrap())?;

    Ok((
        StatusCode::OK,
        Json(json!({ "entries": entries, "proto": active_ep })),
    ))
}

async fn query(
    Extension(db_client): Extension<Client>,
    Path((pk, sk)): Path<(String, String)>,
) -> AResult<(StatusCode, Json<Value>)> {
    let response = Entry::ddb_query(db_client, pk, sk).await?;
    return Ok((StatusCode::OK, Json(json!(response))));
}

async fn put_item(
    Extension(client): Extension<Client>,
    Json(payload): Json<EntryFC>,
) -> AResult<StatusCode> {
    Entry::ddb_put_item(client, payload).await?;
    Ok(StatusCode::CREATED)
}

async fn delete_entry(
    Extension(client): Extension<Client>,
    Path((pk, sk)): Path<(String, String)>,
) -> AResult<StatusCode> {
    let query_res = Entry::ddb_query(client.clone(), pk.clone(), sk.clone()).await?;

    if query_res.is_empty() {
        return Ok(StatusCode::NOT_FOUND);
    }

    Entry::ddb_delete(client, pk, sk).await?;
    Ok(StatusCode::NO_CONTENT)
}
