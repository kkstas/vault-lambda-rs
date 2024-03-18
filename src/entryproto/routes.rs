use aws_sdk_dynamodb::Client;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::routing::{get, put};
use axum::{Extension, Json, Router};
use serde_json::{json, Value};

use crate::AResult;

use super::{EntryProto, EntryProtoFC};

pub fn router() -> Router {
    Router::new()
        .route("/", put(put_entry_proto))
        .route("/:pk/:sk", get(find))
        .route("/active", get(list_active))
        .route("/inactive", get(list_inactive))
        .route("/active/:sk", put(set_as_active))
        .route("/inactive/:sk", put(set_as_inactive))
}

async fn list_active(
    Extension(db_client): Extension<Client>,
) -> AResult<(StatusCode, Json<Value>)> {
    let response = EntryProto::ddb_list_active(db_client).await?;
    return Ok((StatusCode::OK, Json(json!(response))));
}
async fn list_inactive(
    Extension(db_client): Extension<Client>,
) -> AResult<(StatusCode, Json<Value>)> {
    let response = EntryProto::ddb_list_inactive(db_client).await?;
    return Ok((StatusCode::OK, Json(json!(response))));
}

async fn set_as_active(
    Extension(client): Extension<Client>,
    Path(sk): Path<String>,
) -> AResult<StatusCode> {
    EntryProto::set_as_active(client, sk).await?;
    return Ok(StatusCode::CREATED);
}

async fn set_as_inactive(
    Extension(client): Extension<Client>,
    Path(sk): Path<String>,
) -> AResult<StatusCode> {
    EntryProto::set_as_inactive(client, sk).await?;
    return Ok(StatusCode::CREATED);
}

async fn find(
    Extension(db_client): Extension<Client>,
    Path((pk, sk)): Path<(String, String)>,
) -> AResult<(StatusCode, Json<Value>)> {
    let response = EntryProto::ddb_find(db_client, pk, sk).await?;
    return Ok((StatusCode::OK, Json(json!(response))));
}

async fn put_entry_proto(
    Extension(client): Extension<Client>,
    Json(payload): Json<EntryProtoFC>,
) -> AResult<StatusCode> {
    EntryProto::ddb_put_item(client, payload.clone()).await?;
    return Ok(StatusCode::CREATED);
}
