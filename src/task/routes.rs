use aws_sdk_dynamodb::Client;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Extension, Json, Router};
use serde::Serialize;
use serde_json::{json, Value};

use super::{Task, TaskFC};
use crate::taskproto::TaskProto;
use crate::utils::time::get_date_x_days_ago;
use crate::AResult;

pub fn router() -> Router {
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
    Extension(db_client): Extension<Client>,
) -> AResult<(StatusCode, Json<Value>)> {
    let result_tasks = find_last_week_tasks(db_client).await?;
    return Ok((StatusCode::OK, Json(json!(result_tasks))));
}

pub async fn find_last_week_tasks(db_client: Client) -> AResult<Vec<ProtoWithTasks>> {
    let active_task_list_entries = TaskProto::ddb_list_active(db_client.clone()).await?;

    let week_ago = get_date_x_days_ago(7);
    let mut result_tasks: Vec<ProtoWithTasks> = Vec::new();

    for task_list_entry in active_task_list_entries {
        let t: Vec<Task> = Task::ddb_query(
            db_client.clone(),
            task_list_entry.sk.clone(),
            week_ago.clone(),
        )
        .await?;
        result_tasks.push(ProtoWithTasks {
            proto: task_list_entry,
            tasks: t,
        });
    }
    Ok(result_tasks)
}

async fn query(
    Extension(db_client): Extension<Client>,
    Path((pk, sk)): Path<(String, String)>,
) -> AResult<(StatusCode, Json<Value>)> {
    let response = Task::ddb_query(db_client, pk, sk).await?;
    return Ok((StatusCode::OK, Json(json!(response))));
}

async fn create(
    Extension(client): Extension<Client>,
    Json(payload): Json<TaskFC>,
) -> AResult<StatusCode> {
    Task::ddb_create(client, payload).await?;
    Ok(StatusCode::CREATED)
}

async fn delete_task(
    Extension(client): Extension<Client>,
    Path((pk, sk)): Path<(String, String)>,
) -> AResult<StatusCode> {
    let query_res = Task::ddb_query(client.clone(), pk.clone(), sk.clone()).await?;

    if query_res.is_empty() {
        return Ok(StatusCode::NOT_FOUND);
    }

    Task::ddb_delete(client, pk, sk).await?;
    Ok(StatusCode::NO_CONTENT)
}
