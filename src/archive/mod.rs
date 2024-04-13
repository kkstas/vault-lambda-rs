mod model;
mod routes;

pub use model::ArchiveEntry;
pub use model::ArchiveEntryFC;
pub use routes::router;

pub const ARCHIVE_SK: &str = "Archive::Entry";
