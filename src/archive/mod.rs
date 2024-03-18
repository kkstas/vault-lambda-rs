mod model;
mod routes;

pub use model::ArchiveEntry;
pub use model::ArchiveEntryFC;
pub use routes::router;

pub const TABLE_NAME: &str = "vault_tasks";
pub const ARCHIVE_SK: &str = "Archive::Entry";
