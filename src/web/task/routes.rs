use aws_sdk_dynamodb::Client;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Extension, Json, Router};
use serde_json::{json, Value};

use super::model::{Task, TaskFC};
use super::TABLE_NAME;
use crate::AResult;

pub fn router(db_client: Client) -> Router {
    Router::new()
        .route("/", post(create))
        .route("/:pk/:sk", delete(delete_task))
        .route("/:pk/:sk", get(query))
        .layer(Extension(db_client))
}

async fn query(
    Extension(db_client): Extension<Client>,
    Path((pk, sk)): Path<(String, String)>,
) -> AResult<(StatusCode, Json<Value>)> {
    let response = Task::ddb_query(db_client, TABLE_NAME.to_string(), pk, sk).await?;
    return Ok((StatusCode::OK, Json(json!(response))));
}

async fn create(
    Extension(client): Extension<Client>,
    Json(payload): Json<TaskFC>,
) -> AResult<StatusCode> {
    Task::ddb_create(client, TABLE_NAME.to_string(), payload).await?;
    Ok(StatusCode::CREATED)
}

async fn delete_task(
    Extension(client): Extension<Client>,
    Path((pk, sk)): Path<(String, String)>,
) -> AResult<StatusCode> {
    let query_res = Task::ddb_query(
        client.clone(),
        TABLE_NAME.to_string(),
        pk.clone(),
        sk.clone(),
    )
    .await?;

    if query_res.is_empty() {
        return Ok(StatusCode::NOT_FOUND);
    }

    Task::ddb_delete(client, TABLE_NAME.to_string(), pk, sk).await?;
    Ok(StatusCode::NO_CONTENT)
}