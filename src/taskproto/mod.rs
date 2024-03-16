mod model;
mod routes;

pub use model::TaskProto;
pub use model::TaskProtoFC;
pub use routes::router;

pub const TABLE_NAME: &str = "vault_tasks";
