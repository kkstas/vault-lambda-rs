use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;

pub type AResult<T> = core::result::Result<T, AError>;

#[derive(Debug)]
pub struct AError(pub anyhow::Error);

impl IntoResponse for AError {
    fn into_response(self) -> axum::response::Response {
        println!("  ->> ERROR {:?}", self.0);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "statusCode": StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                "message":format!("Something went wrong: {:?}", self.0)
            })),
        )
            .into_response()
    }
}

impl<E> From<E> for AError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
