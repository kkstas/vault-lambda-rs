mod model;
mod routes;

pub use model::Task;
pub use model::TaskFC;
pub use routes::find_last_week_tasks;
pub use routes::router;

pub const TABLE_NAME: &str = "vault_tasks";
