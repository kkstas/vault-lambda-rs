mod model;
mod routes;

pub use model::Entry;
pub use model::EntryFC;
pub use routes::find_last_week_entries;
pub use routes::router;

pub const TABLE_NAME: &str = "vault_tasks";
