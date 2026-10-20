//! # forskscope-report
//!
//! Turns a ForskScope comparison into text (RFC-093 §1): unified diff
//! (`patch`), and Markdown/JSON/HTML reports (`report`), with SHA-256
//! checksums of the bytes a report describes.
//!
//! **This crate never writes a file and never spawns a process** — it
//! returns text and bytes, and the caller writes them. Enforced by `cargo
//! xtask crate-boundaries` (RFC-093 §2), not just documented here.

pub mod checksum;
pub mod patch;
pub mod report;

pub use patch::{
    LineOrigin, PatchDocument, PatchFileChange, PatchFormat, PatchHunk, PatchLine, PatchOptions,
    PatchSummary, patch_from_binary_notice, patch_from_directories,
    patch_from_directories_with_rules, patch_from_file_diff, to_unified,
};

#[cfg(test)]
mod tests;
