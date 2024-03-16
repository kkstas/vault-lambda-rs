use aws_sdk_dynamodb::Client;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Extension, Json, Router};
use serde_json::{json, Value};

use super::TABLE_NAME;
use super::{Record, RecordFC};
use crate::utils::time::get_date_x_days_ago;
use crate::AResult;

pub fn router(db_client: Client) -> Router {
    Router::new()
        .route("/", post(create))
        .route("/:sk", delete(delete_task))
        .route("/last-week", get(find_last_week_handler))
        .route("/:sk", get(query))
        .layer(Extension(db_client))
}

async fn query(
    Extension(db_client): Extension<Client>,
    Path(sk): Path<String>,
) -> AResult<(StatusCode, Json<Value>)> {
    let response = Record::ddb_query(db_client, TABLE_NAME.to_string(), sk).await?;
    return Ok((StatusCode::OK, Json(json!(response))));
}

async fn create(
    Extension(client): Extension<Client>,
    Json(payload): Json<RecordFC>,
) -> AResult<StatusCode> {
    Record::ddb_create(client, TABLE_NAME.to_string(), payload).await?;
    Ok(StatusCode::CREATED)
}

async fn delete_task(
    Extension(client): Extension<Client>,
    Path(sk): Path<String>,
) -> AResult<StatusCode> {
    let query_res = Record::ddb_query(client.clone(), TABLE_NAME.to_string(), sk.clone()).await?;

    if query_res.is_empty() {
        return Ok(StatusCode::NOT_FOUND);
    }

    Record::ddb_delete(client, TABLE_NAME.to_string(), sk).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn find_last_week_handler(
    Extension(db_client): Extension<Client>,
) -> AResult<(StatusCode, Json<Value>)> {
    return Ok((
        StatusCode::OK,
        Json(json!(find_last_week_records(db_client).await?)),
    ));
}

pub async fn find_last_week_records(db_client: Client) -> AResult<Vec<Record>> {
    let response =
        Record::ddb_query(db_client, TABLE_NAME.to_string(), get_date_x_days_ago(7)).await?;
    Ok(response)
}
