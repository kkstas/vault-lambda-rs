use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde_json::{json, Value};

use super::{TaskProto, TaskProtoFC};
use crate::{AResult, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create))
        .route("/:pk/:sk", get(find))
        .route("/active", get(list_active))
        .route("/inactive", get(list_inactive))
        .route("/active/:sk", put(set_as_active))
        .route("/inactive/:sk", put(set_as_inactive))
        .route("/", put(update))
}

async fn set_as_active(
    State(state): State<AppState>,
    Path(sk): Path<String>,
) -> AResult<StatusCode> {
    TaskProto::set_as_active(&state, sk).await?;
    return Ok(StatusCode::CREATED);
}

async fn set_as_inactive(
    State(state): State<AppState>,
    Path(sk): Path<String>,
) -> AResult<StatusCode> {
    TaskProto::set_as_inactive(&state, sk).await?;
    return Ok(StatusCode::CREATED);
}

async fn find(
    State(state): State<AppState>,
    Path((pk, sk)): Path<(String, String)>,
) -> AResult<(StatusCode, Json<Value>)> {
    let response = TaskProto::ddb_find(&state, pk, sk).await?;
    return Ok((StatusCode::OK, Json(json!(response))));
}

async fn create(
    State(state): State<AppState>,
    Json(payload): Json<TaskProtoFC>,
) -> AResult<StatusCode> {
    TaskProto::create(&state, payload).await?;
    return Ok(StatusCode::CREATED);
}

async fn update(
    State(state): State<AppState>,
    Json(payload): Json<TaskProtoFC>,
) -> AResult<StatusCode> {
    TaskProto::update(&state, payload).await?;

    return Ok(StatusCode::CREATED);
}

async fn list_active(State(state): State<AppState>) -> AResult<(StatusCode, Json<Value>)> {
    let response = TaskProto::ddb_list_active(&state).await?;
    return Ok((StatusCode::OK, Json(json!(response))));
}
async fn list_inactive(State(state): State<AppState>) -> AResult<(StatusCode, Json<Value>)> {
    let response = TaskProto::ddb_list_inactive(&state).await?;
    return Ok((StatusCode::OK, Json(json!(response))));
}
