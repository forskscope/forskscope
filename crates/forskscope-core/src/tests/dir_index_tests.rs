//! Directory index and equality evidence tests (RFC-008 §5 comparison
//! strategy, RFC-037 §"Acceptance Criteria").
//!
//! All tests are pure data — no I/O. The index is built from hand-crafted
//! records; real scanning is tested separately in `dir_cancel_tests`.

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::dir::{
    ContentDigest, DirectoryEntryRecord, DirectoryIndex, EntryType,
    EqualityEvidence, IndexRevision, pair_entries,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn mtime(secs: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(secs)
}

fn file_record(rel: &str, size: u64, mtime_secs: u64) -> DirectoryEntryRecord {
    DirectoryEntryRecord {
        relative_path: PathBuf::from(rel),
        entry_type:    EntryType::File,
        size:          Some(size),
        modified:      Some(mtime(mtime_secs)),
        digest:        None,
        error:         None,
    }
}

fn file_with_digest(rel: &str, size: u64, digest_hex: &str) -> DirectoryEntryRecord {
    DirectoryEntryRecord {
        relative_path: PathBuf::from(rel),
        entry_type:    EntryType::File,
        size:          Some(size),
        modified:      None,
        digest:        Some(ContentDigest::fnv1a64(digest_hex)),
        error:         None,
    }
}

fn dir_record(rel: &str) -> DirectoryEntryRecord {
    DirectoryEntryRecord {
        relative_path: PathBuf::from(rel),
        entry_type:    EntryType::Directory,
        size:          None,
        modified:      None,
        digest:        None,
        error:         None,
    }
}

fn error_record(rel: &str) -> DirectoryEntryRecord {
    DirectoryEntryRecord {
        relative_path: PathBuf::from(rel),
        entry_type:    EntryType::File,
        size:          None,
        modified:      None,
        digest:        None,
        error:         Some("permission denied".into()),
    }
}

fn index(root: &str, records: Vec<DirectoryEntryRecord>) -> DirectoryIndex {
    DirectoryIndex::from_records(PathBuf::from(root), records, true)
}

fn evidence_for(left: &DirectoryIndex, right: &DirectoryIndex, rel: &str) -> EqualityEvidence {
    let set = pair_entries(left, right);
    set.entries.into_iter()
        .find(|e| e.relative_path == PathBuf::from(rel))
        .map(|e| e.evidence)
        .unwrap_or(EqualityEvidence::Unknown)
}

// ── DirectoryIndex ────────────────────────────────────────────────────────────

#[test]
fn empty_index_has_no_entries() {
    let idx = DirectoryIndex::empty(PathBuf::from("/root"));
    assert!(idx.entries.is_empty());
    assert!(!idx.is_complete);
}

#[test]
fn from_records_sets_revision_1_and_complete() {
    let idx = index("/root", vec![file_record("a.rs", 100, 1000)]);
    assert_eq!(idx.revision, IndexRevision(1));
    assert!(idx.is_complete);
}

#[test]
fn get_finds_entry_by_relative_path() {
    let idx = index("/root", vec![
        file_record("src/main.rs", 200, 1000),
        dir_record("src"),
    ]);
    assert!(idx.get("src/main.rs".as_ref()).is_some());
    assert!(idx.get("src".as_ref()).is_some());
    assert!(idx.get("missing.rs".as_ref()).is_none());
}

#[test]
fn files_iterator_excludes_directories() {
    let idx = index("/root", vec![
        file_record("a.rs", 100, 1000),
        dir_record("src"),
        file_record("b.rs", 200, 1000),
    ]);
    assert_eq!(idx.files().count(), 2);
    assert_eq!(idx.directories().count(), 1);
}

// ── ContentDigest ─────────────────────────────────────────────────────────────

#[test]
fn digest_matches_same_algorithm_and_hex() {
    let a = ContentDigest::fnv1a64("deadbeef");
    let b = ContentDigest::fnv1a64("deadbeef");
    assert!(a.matches(&b));
}

#[test]
fn digest_does_not_match_different_hex() {
    let a = ContentDigest::fnv1a64("aabbccdd");
    let b = ContentDigest::fnv1a64("11223344");
    assert!(!a.matches(&b));
}

#[test]
fn digest_does_not_match_different_algorithm() {
    let a = ContentDigest { algorithm: "fnv1a64".into(), hex: "aabb".into() };
    let b = ContentDigest { algorithm: "sha256".into(),  hex: "aabb".into() };
    assert!(!a.matches(&b), "same hex but different algorithm must not match");
}

// ── EqualityEvidence predicates ───────────────────────────────────────────────

#[test]
fn digest_equal_is_equal_and_present_on_both_sides() {
    assert!(EqualityEvidence::DigestEqual.is_equal());
    assert!(EqualityEvidence::DigestEqual.present_on_both_sides());
    assert!(!EqualityEvidence::DigestEqual.is_different());
    assert!(!EqualityEvidence::DigestEqual.is_pending());
}

#[test]
fn size_different_is_different() {
    let e = EqualityEvidence::SizeDifferent { left_size: 100, right_size: 200 };
    assert!(e.is_different());
    assert!(!e.is_equal());
    assert!(!e.is_pending());
}

#[test]
fn metadata_only_is_pending() {
    assert!(EqualityEvidence::MetadataOnly.is_pending());
    assert!(!EqualityEvidence::MetadataOnly.is_equal());
    assert!(!EqualityEvidence::MetadataOnly.is_different());
}

#[test]
fn left_only_is_different_and_not_on_both_sides() {
    assert!(EqualityEvidence::LeftOnly.is_different());
    assert!(!EqualityEvidence::LeftOnly.present_on_both_sides());
}

#[test]
fn right_only_is_different_and_not_on_both_sides() {
    assert!(EqualityEvidence::RightOnly.is_different());
    assert!(!EqualityEvidence::RightOnly.present_on_both_sides());
}

// ── pair_entries: RFC-008 §5 comparison strategy ──────────────────────────────

#[test]
fn file_present_only_on_left_is_left_only() {
    let l = index("/l", vec![file_record("only_left.rs", 100, 1000)]);
    let r = index("/r", vec![]);
    assert_eq!(evidence_for(&l, &r, "only_left.rs"), EqualityEvidence::LeftOnly);
}

#[test]
fn file_present_only_on_right_is_right_only() {
    let l = index("/l", vec![]);
    let r = index("/r", vec![file_record("only_right.rs", 100, 1000)]);
    assert_eq!(evidence_for(&l, &r, "only_right.rs"), EqualityEvidence::RightOnly);
}

#[test]
fn different_sizes_yields_size_different_without_digest() {
    // RFC-008 §5 rule 3: size differs → SizeDifferent, skip digest.
    let l = index("/l", vec![file_record("f.rs", 100, 1000)]);
    let r = index("/r", vec![file_record("f.rs", 200, 1000)]);
    assert!(matches!(
        evidence_for(&l, &r, "f.rs"),
        EqualityEvidence::SizeDifferent { left_size: 100, right_size: 200 }
    ));
}

#[test]
fn matching_digests_yield_digest_equal() {
    // RFC-008 §5 rule 4a: same size, both digests match.
    let l = index("/l", vec![file_with_digest("f.rs", 100, "aabbcc")]);
    let r = index("/r", vec![file_with_digest("f.rs", 100, "aabbcc")]);
    assert_eq!(evidence_for(&l, &r, "f.rs"), EqualityEvidence::DigestEqual);
}

#[test]
fn different_digests_yield_digest_different() {
    // RFC-008 §5 rule 4b: same size, digests differ.
    let l = index("/l", vec![file_with_digest("f.rs", 100, "aabbcc")]);
    let r = index("/r", vec![file_with_digest("f.rs", 100, "112233")]);
    assert_eq!(evidence_for(&l, &r, "f.rs"), EqualityEvidence::DigestDifferent);
}

#[test]
fn same_size_same_mtime_no_digest_is_metadata_equal() {
    // RFC-008 §5 rule 5: size equal, same mtime, no digest.
    let l = index("/l", vec![file_record("f.rs", 100, 1000)]);
    let r = index("/r", vec![file_record("f.rs", 100, 1000)]);
    assert_eq!(evidence_for(&l, &r, "f.rs"), EqualityEvidence::MetadataEqual);
}

#[test]
fn same_size_different_mtime_no_digest_is_metadata_only() {
    // RFC-008 §5 rule 6: size equal, mtime differs, no digest → pending.
    let l = index("/l", vec![file_record("f.rs", 100, 1000)]);
    let r = index("/r", vec![file_record("f.rs", 100, 2000)]);
    assert_eq!(evidence_for(&l, &r, "f.rs"), EqualityEvidence::MetadataOnly);
}

#[test]
fn type_mismatch_file_vs_directory() {
    let l = index("/l", vec![file_record("name", 100, 1000)]);
    let r = index("/r", vec![dir_record("name")]);
    assert!(matches!(
        evidence_for(&l, &r, "name"),
        EqualityEvidence::TypeMismatch { .. }
    ));
}

#[test]
fn error_on_either_side_yields_error_evidence() {
    let l = index("/l", vec![error_record("f.rs")]);
    let r = index("/r", vec![file_record("f.rs", 100, 1000)]);
    assert!(matches!(evidence_for(&l, &r, "f.rs"), EqualityEvidence::Error { .. }));
}

#[test]
fn directories_on_both_sides_are_metadata_equal() {
    let l = index("/l", vec![dir_record("src")]);
    let r = index("/r", vec![dir_record("src")]);
    assert_eq!(evidence_for(&l, &r, "src"), EqualityEvidence::MetadataEqual);
}

// ── PairedEntrySet summary counts ────────────────────────────────────────────

#[test]
fn paired_entry_set_counts_are_accurate() {
    let l = index("/l", vec![
        file_record("eq.rs",      100, 1000),   // same mtime → MetadataEqual
        file_record("left.rs",    200, 1000),   // left only
        file_with_digest("d.rs",  300, "aabb"), // digest equal
    ]);
    let r = index("/r", vec![
        file_record("eq.rs",      100, 1000),   // MetadataEqual
        file_record("right.rs",   200, 1000),   // right only
        file_with_digest("d.rs",  300, "aabb"), // DigestEqual
    ]);
    let set = pair_entries(&l, &r);
    assert_eq!(set.equal_count(),       2, "eq.rs + d.rs");
    assert_eq!(set.left_only_count(),   1, "left.rs");
    assert_eq!(set.right_only_count(),  1, "right.rs");
    assert_eq!(set.different_count(),   2, "left.rs + right.rs");
    assert_eq!(set.pending_count(),     0);
}

#[test]
fn empty_both_sides_produces_empty_set() {
    let l = index("/l", vec![]);
    let r = index("/r", vec![]);
    let set = pair_entries(&l, &r);
    assert!(set.entries.is_empty());
    assert_eq!(set.equal_count(), 0);
}

#[test]
fn index_revision_next_increments() {
    let r = IndexRevision(5);
    assert_eq!(r.next(), IndexRevision(6));
}
