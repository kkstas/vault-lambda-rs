use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, put};
use axum::{Json, Router};
use serde_json::{json, Value};

use crate::{AResult, AppState};

use super::{EntryProto, EntryProtoFC};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", put(put_entry_proto))
        .route("/:pk/:sk", get(find))
        .route("/active", get(list_active))
        .route("/inactive", get(list_inactive))
        .route("/active/:sk", put(set_as_active))
        .route("/inactive/:sk", put(set_as_inactive))
}

async fn list_active(State(state): State<AppState>) -> AResult<(StatusCode, Json<Value>)> {
    let response = EntryProto::ddb_list_active(&state).await?;
    return Ok((StatusCode::OK, Json(json!(response))));
}
async fn list_inactive(State(state): State<AppState>) -> AResult<(StatusCode, Json<Value>)> {
    let response = EntryProto::ddb_list_inactive(&state).await?;
    return Ok((StatusCode::OK, Json(json!(response))));
}

async fn set_as_active(
    State(state): State<AppState>,
    Path(sk): Path<String>,
) -> AResult<StatusCode> {
    EntryProto::set_as_active(&state, sk).await?;
    return Ok(StatusCode::CREATED);
}

async fn set_as_inactive(
    State(state): State<AppState>,
    Path(sk): Path<String>,
) -> AResult<StatusCode> {
    EntryProto::set_as_inactive(&state, sk).await?;
    return Ok(StatusCode::CREATED);
}

async fn find(
    State(state): State<AppState>,
    Path((pk, sk)): Path<(String, String)>,
) -> AResult<(StatusCode, Json<Value>)> {
    let response = EntryProto::ddb_find(&state, pk, sk).await?;
    return Ok((StatusCode::OK, Json(json!(response))));
}

async fn put_entry_proto(
    State(state): State<AppState>,
    Json(payload): Json<EntryProtoFC>,
) -> AResult<StatusCode> {
    EntryProto::ddb_put_item(&state, payload).await?;
    return Ok(StatusCode::CREATED);
}
