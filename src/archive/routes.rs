use aws_sdk_dynamodb::Client;
use axum::{
    extract::Path,
    http::StatusCode,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use serde_json::{json, Value};

use crate::AResult;

use super::{ArchiveEntry, ArchiveEntryFC};

pub fn router() -> Router {
    Router::new()
        .route("/all", get(find_all))
        .route("/", post(create_handler))
        .route("/:sk", delete(delete_handler))
        .route("/increment/:sk", put(increment_read_times_handler))
}

async fn find_all(Extension(db_client): Extension<Client>) -> AResult<(StatusCode, Json<Value>)> {
    let response = ArchiveEntry::ddb_find_all(db_client).await?;
    return Ok((StatusCode::OK, Json(json!(response))));
}

async fn create_handler(
    Extension(client): Extension<Client>,
    Json(payload): Json<ArchiveEntryFC>,
) -> AResult<StatusCode> {
    ArchiveEntry::ddb_create(client, payload).await?;
    return Ok(StatusCode::CREATED);
}

async fn delete_handler(
    Extension(client): Extension<Client>,
    Path(sk): Path<String>,
) -> AResult<StatusCode> {
    ArchiveEntry::ddb_delete(client, sk).await?;
    return Ok(StatusCode::NO_CONTENT);
}

async fn increment_read_times_handler(
    Extension(client): Extension<Client>,
    Path(sk): Path<String>,
) -> AResult<StatusCode> {
    ArchiveEntry::ddb_increment_read_times(client, sk).await?;
    return Ok(StatusCode::OK);
}
