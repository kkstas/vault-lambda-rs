use aws_sdk_dynamodb::Client;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Extension, Json, Router};
use serde_json::{json, Value};

use super::model::{TaskListEntry, TaskListEntryFC};
use super::TABLE_NAME;
use crate::AResult;

pub fn router(db_client: Client) -> Router {
    Router::new()
        .route("/", post(create))
        .route("/:pk/:sk", get(query))
        .route("/active", get(list_active))
        .route("/inactive", get(list_inactive))
        .route("/", put(update))
        .layer(Extension(db_client))
}

async fn query(
    Extension(db_client): Extension<Client>,
    Path((pk, sk)): Path<(String, String)>,
) -> AResult<(StatusCode, Json<Value>)> {
    let response = TaskListEntry::find(db_client, TABLE_NAME.to_string(), pk, sk).await?;
    return Ok((StatusCode::OK, Json(json!(response))));
}

async fn create(
    Extension(client): Extension<Client>,
    Json(payload): Json<TaskListEntryFC>,
) -> AResult<StatusCode> {
    TaskListEntry::create(client, TABLE_NAME.to_string(), payload.clone()).await?;

    return Ok(StatusCode::CREATED);
}

async fn update(
    Extension(client): Extension<Client>,
    Json(payload): Json<TaskListEntryFC>,
) -> AResult<StatusCode> {
    TaskListEntry::update(client, TABLE_NAME.to_string(), payload.clone()).await?;

    return Ok(StatusCode::CREATED);
}

async fn list_active(
    Extension(db_client): Extension<Client>,
) -> AResult<(StatusCode, Json<Value>)> {
    let response = TaskListEntry::list_active(db_client, TABLE_NAME.to_string()).await?;
    return Ok((StatusCode::OK, Json(json!(response))));
}
async fn list_inactive(
    Extension(db_client): Extension<Client>,
) -> AResult<(StatusCode, Json<Value>)> {
    let response = TaskListEntry::list_inactive(db_client, TABLE_NAME.to_string()).await?;
    return Ok((StatusCode::OK, Json(json!(response))));
}
