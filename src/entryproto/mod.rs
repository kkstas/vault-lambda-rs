mod model;
mod routes;

pub use model::EntryProto;
pub use model::EntryProtoFC;
pub use routes::router;

pub const TABLE_NAME: &str = "vault_tasks";
