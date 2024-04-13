use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use super::{Record, RecordFC};
use crate::utils::time::get_date_x_days_ago;
use crate::{AResult, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create))
        .route("/:sk", delete(delete_task))
        .route("/last-week", get(find_last_week_handler))
        .route("/", get(query))
}

#[derive(Deserialize)]
struct QueryParams {
    from: String,
    to: String,
}

async fn query(
    State(state): State<AppState>,
    Query(query): Query<QueryParams>,
) -> AResult<(StatusCode, Json<Value>)> {
    let response = Record::ddb_query_from_to(&state, &query.from, &query.to).await?;
    return Ok((
        StatusCode::OK,
        Json(json!({
            "records": response,
            "from": query.from,
            "to": query.to
        })),
    ));
}

async fn create(
    State(state): State<AppState>,
    Json(payload): Json<RecordFC>,
) -> AResult<StatusCode> {
    Record::ddb_create(&state, payload).await?;
    Ok(StatusCode::CREATED)
}

async fn delete_task(State(state): State<AppState>, Path(sk): Path<String>) -> AResult<StatusCode> {
    let query_res = Record::ddb_query(&state, &sk).await?;

    if query_res.is_empty() {
        return Ok(StatusCode::NOT_FOUND);
    }

    Record::ddb_delete(&state, sk).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn find_last_week_handler(
    State(state): State<AppState>,
) -> AResult<(StatusCode, Json<Value>)> {
    return Ok((
        StatusCode::OK,
        Json(json!(find_last_week_records(&state).await?)),
    ));
}

pub async fn find_last_week_records(state: &AppState) -> AResult<Vec<Record>> {
    let response = Record::ddb_query(&state, get_date_x_days_ago(7)).await?;
    Ok(response)
}
