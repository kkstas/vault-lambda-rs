use aws_sdk_dynamodb::Client;
use axum::{routing::get, Extension, Json, Router};
use serde_json::{json, Value};

use crate::{
    entry::find_last_week_entries, record::find_last_week_records, task::find_last_week_tasks,
    AResult,
};

pub fn router(db_client: Client) -> Router {
    Router::new()
        .route("/", get(common_last_week_handler))
        .layer(Extension(db_client))
}

async fn common_last_week_handler(Extension(db_client): Extension<Client>) -> AResult<Json<Value>> {
    let tasks = find_last_week_tasks(db_client.clone()).await?;
    let records = find_last_week_records(db_client.clone()).await?;
    let entries = find_last_week_entries(db_client).await?;

    Ok(Json(json!({
        "tasks_data": tasks,
        "records_data": records,
        "entries_data": entries,
    })))
}
