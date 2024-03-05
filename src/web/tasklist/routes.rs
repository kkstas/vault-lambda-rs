use aws_sdk_dynamodb::Client;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use serde_json::{json, Value};

use super::model::TaskListEntry;
use super::TABLE_NAME;
use crate::AResult;

pub fn router(db_client: Client) -> Router {
    Router::new()
        .route("/", post(create))
        .route("/:pk/:sk", get(query))
        .route("/active", get(list_active))
        .layer(Extension(db_client))
}

async fn query(
    Extension(db_client): Extension<Client>,
    Path((pk, sk)): Path<(String, String)>,
) -> AResult<(StatusCode, Json<Value>)> {
    let response = TaskListEntry::ddb_query(db_client, TABLE_NAME.to_string(), pk, sk).await?;
    return Ok((StatusCode::OK, Json(json!(response))));
}

async fn create(
    Extension(client): Extension<Client>,
    Json(payload): Json<TaskListEntry>,
) -> AResult<StatusCode> {
    TaskListEntry::ddb_create(client, TABLE_NAME.to_string(), payload).await?;
    Ok(StatusCode::CREATED)
}

async fn list_active(
    Extension(db_client): Extension<Client>,
) -> AResult<(StatusCode, Json<Value>)> {
    let response = TaskListEntry::ddb_list_active(db_client, TABLE_NAME.to_string()).await?;
    return Ok((StatusCode::OK, Json(json!(response))));
}
