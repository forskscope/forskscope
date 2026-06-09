//! Directory listing, digest comparison, file copy, and recursive diff (RFC-005, RFC-008, RFC-037).

mod copy;
mod digest;
mod listing;
mod recursive;

pub use copy::{CopyOutcome, copy_file};
pub use digest::{dir_digest_equal, file_digest_equal};
pub use listing::{DirectoryListing, FileEntry, list_dir};
pub use recursive::{RecEntry, RecStatus, recursive_diff};
