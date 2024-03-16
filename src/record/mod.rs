mod model;
mod routes;

pub use model::Record;
pub use model::RecordFC;
pub use routes::find_last_week_records;
pub use routes::router;

pub const TABLE_NAME: &str = "vault_tasks";
