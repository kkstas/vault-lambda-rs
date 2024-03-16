use std::collections::HashMap;

use aws_sdk_dynamodb::types::{AttributeValue, KeysAndAttributes};
use aws_sdk_dynamodb::Client;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::routing::{delete, get, put};
use axum::{Extension, Json, Router};
use serde_dynamo::from_items;
use serde_json::{json, Value};

use super::model::{Entry, EntryFC};
use super::TABLE_NAME;
use crate::entryproto::model::EntryProto;
use crate::entryproto::TABLE_NAME as ENTRYPROTO_TABLE_NAME;
use crate::AResult;

pub fn router(db_client: Client) -> Router {
    Router::new()
        .route("/", put(put_item))
        .route("/:pk/:sk", delete(delete_entry))
        .route("/:pk/:sk", get(query))
        .route("/:date", get(get_all_from_date))
        .layer(Extension(db_client))
}

async fn get_all_from_date(
    Extension(db_client): Extension<Client>,
    Path(date): Path<String>,
) -> AResult<(StatusCode, Json<Value>)> {
    find_x_days_ago(db_client, date).await
}

async fn find_x_days_ago(db_client: Client, date: String) -> AResult<(StatusCode, Json<Value>)> {
    let active_entry_proto_list =
        EntryProto::ddb_list_active(db_client.clone(), ENTRYPROTO_TABLE_NAME.to_string()).await?;

    let v: Vec<HashMap<String, AttributeValue>> = active_entry_proto_list
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
    let response = Json(json!({"entries":entries, "proto": active_entry_proto_list}));

    Ok((StatusCode::OK, response))
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
