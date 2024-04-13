use aws_config::BehaviorVersion;
use aws_sdk_dynamodb::Client;
use axum::http::StatusCode;
use axum::{routing::get, Router};
use lambda_http::{run, tracing, Error};
use std::env::set_var;

pub mod archive;
pub mod common;
pub mod entry;
pub mod entryproto;
pub mod error;
pub mod record;
pub mod task;
pub mod taskproto;
pub mod utils;

pub use error::{AError, AResult};

#[derive(Clone)]
pub struct AppState {
    pub table_name: String,
    pub dynamodb_client: aws_sdk_dynamodb::Client,
}

#[tokio::main]
async fn main() -> std::result::Result<(), Error> {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;

    let state = AppState {
        table_name: "vault_tasks".to_string(),
        dynamodb_client: Client::new(&config),
    };

    // If you use API Gateway stages, the Rust Runtime will include the stage name
    // as part of the path that your application receives.
    // Setting the following environment variable, you can remove the stage from the path.
    // This variable only applies to API Gateway stages,
    // you can remove it if you don't use them.
    // i.e with: `GET /test-stage/todo/id/123` without: `GET /todo/id/123`
    set_var("AWS_LAMBDA_HTTP_IGNORE_STAGE_IN_PATH", "true");

    // required to enable CloudWatch error logging by the runtime
    tracing::init_default_subscriber();

    let app = Router::new()
        .route("/health", get(health_check))
        .nest("/api/v1/task", task::router())
        .nest("/api/v1/taskproto", taskproto::router())
        .nest("/api/v1/entry", entry::router())
        .nest("/api/v1/entryproto", entryproto::router())
        .nest("/api/v1/record", record::router())
        .nest("/api/v1/archive", archive::router())
        .nest("/api/v1/common", common::router())
        .with_state(state);

    run(app).await
}

async fn health_check() -> StatusCode {
    StatusCode::OK
}
