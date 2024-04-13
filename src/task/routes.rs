use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Serialize;
use serde_json::{json, Value};

use super::{Task, TaskFC};
use crate::taskproto::TaskProto;
use crate::utils::time::get_date_x_days_ago;
use crate::{AResult, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create))
        .route("/last-week", get(find_last_week_handler))
        .route("/:pk/:sk", delete(delete_task))
        .route("/:pk/:sk", get(query))
}

#[derive(Serialize)]
pub struct ProtoWithTasks {
    pub proto: TaskProto,
    pub tasks: Vec<Task>,
}

async fn find_last_week_handler(
    State(state): State<AppState>,
) -> AResult<(StatusCode, Json<Value>)> {
    let result_tasks = find_last_week_tasks(&state).await?;
    return Ok((StatusCode::OK, Json(json!(result_tasks))));
}

pub async fn find_last_week_tasks(state: &AppState) -> AResult<Vec<ProtoWithTasks>> {
    let active_task_list_entries = TaskProto::ddb_list_active(&state).await?;

    let week_ago = get_date_x_days_ago(7);
    let mut result_tasks: Vec<ProtoWithTasks> = Vec::new();

    for task_list_entry in active_task_list_entries {
        let t: Vec<Task> = Task::ddb_query(&state, &task_list_entry.sk, &week_ago).await?;
        result_tasks.push(ProtoWithTasks {
            proto: task_list_entry,
            tasks: t,
        });
    }
    Ok(result_tasks)
}

async fn query(
    State(state): State<AppState>,
    Path((pk, sk)): Path<(String, String)>,
) -> AResult<(StatusCode, Json<Value>)> {
    let response = Task::ddb_query(&state, pk, sk).await?;
    return Ok((StatusCode::OK, Json(json!(response))));
}

async fn create(State(state): State<AppState>, Json(payload): Json<TaskFC>) -> AResult<StatusCode> {
    Task::ddb_create(&state, payload).await?;
    Ok(StatusCode::CREATED)
}

async fn delete_task(
    State(state): State<AppState>,
    Path((pk, sk)): Path<(String, String)>,
) -> AResult<StatusCode> {
    let query_res = Task::ddb_query(&state, &pk, &sk).await?;

    if query_res.is_empty() {
        return Ok(StatusCode::NOT_FOUND);
    }

    Task::ddb_delete(&state, &pk, &sk).await?;
    Ok(StatusCode::NO_CONTENT)
}
