use axum::{extract::State, routing::get, Json, Router};
use serde_json::{json, Value};

use crate::{
    entry::find_last_week_entries, record::find_last_week_records, task::find_last_week_tasks,
    AResult, AppState,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(common_last_week_handler))
}

async fn common_last_week_handler(State(state): State<AppState>) -> AResult<Json<Value>> {
    let tasks = find_last_week_tasks(&state).await?;
    let records = find_last_week_records(&state).await?;
    let entries = find_last_week_entries(&state).await?;

    Ok(Json(json!({
        "tasks_data": tasks,
        "records_data": records,
        "entries_data": entries,
    })))
}
