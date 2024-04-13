use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde_json::{json, Value};

use crate::{AResult, AppState};

use super::{ArchiveEntry, ArchiveEntryFC};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/all", get(find_all))
        .route("/", post(create_handler))
        .route("/:sk", delete(delete_handler))
        .route("/increment/:sk", put(increment_read_times_handler))
}

async fn find_all(State(state): State<AppState>) -> AResult<(StatusCode, Json<Value>)> {
    let response = ArchiveEntry::ddb_find_all(state).await?;
    return Ok((StatusCode::OK, Json(json!(response))));
}

async fn create_handler(
    State(state): State<AppState>,
    Json(payload): Json<ArchiveEntryFC>,
) -> AResult<StatusCode> {
    ArchiveEntry::ddb_create(&state, payload).await?;
    return Ok(StatusCode::CREATED);
}

async fn delete_handler(
    State(state): State<AppState>,
    Path(sk): Path<String>,
) -> AResult<StatusCode> {
    ArchiveEntry::ddb_delete(&state, sk).await?;
    return Ok(StatusCode::NO_CONTENT);
}

async fn increment_read_times_handler(
    State(state): State<AppState>,
    Path(sk): Path<String>,
) -> AResult<StatusCode> {
    ArchiveEntry::ddb_increment_read_times(&state, sk).await?;
    return Ok(StatusCode::OK);
}
