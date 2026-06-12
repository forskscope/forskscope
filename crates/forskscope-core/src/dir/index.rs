//! Directory index model (RFC-037 §"Directory Index", RFC-008 §5).
//!
//! A [`DirectoryIndex`] is a snapshot of one directory tree: one
//! [`DirectoryEntryRecord`] per file, carrying metadata and an optional
//! content digest. Two indices can be *paired* with [`pair_entries`] to
//! produce a [`PairedEntrySet`] whose [`EqualityEvidence`] values drive
//! the explorer status icons and the scalable compare report.
//!
//! ## Design
//!
//! This module is pure data — no I/O, no threading. Building an index
//! (scanning the filesystem) is the job of the background job pipeline
//! (RFC-008 §4); this module owns only the accumulated result and the
//! pairing logic applied to two completed indices.
//!
//! [`EqualityEvidence`] follows the RFC-008 §5 comparison strategy:
//! size mismatch → `SizeDifferent` (skip digest); size equal + no digest
//! available → `MetadataOnly`; size equal + both digests → `DigestEqual`
//! or `DigestDifferent`. One-sided and type-mismatch cases are handled
//! before any content comparison.

use std::path::PathBuf;
use std::time::SystemTime;

// ── Entry type ────────────────────────────────────────────────────────────────

/// Whether a filesystem entry is a file or a directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryType {
    File,
    Directory,
    /// Symlinks, devices, sockets, etc. — not compared as content.
    Other,
}

// ── Content digest ────────────────────────────────────────────────────────────

/// A content digest for one file, used to detect equality without relying
/// solely on mtime (which is unreliable on some filesystems).
///
/// Stored as a fixed-length hex string so it is format-agnostic: callers
/// choose the algorithm (FNV-1a, SHA-256, etc.) and encode the output.
/// ForskScope currently uses FNV-1a 64-bit for speed; the field carries the
/// algorithm tag so future migrations are safe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentDigest {
    /// Algorithm identifier, e.g. `"fnv1a64"` or `"sha256"`.
    pub algorithm: String,
    /// Lower-case hex string of the raw digest bytes.
    pub hex: String,
}

impl ContentDigest {
    pub fn fnv1a64(hex: impl Into<String>) -> Self {
        Self { algorithm: "fnv1a64".into(), hex: hex.into() }
    }

    /// `true` when both digests were produced by the same algorithm and the
    /// hex strings match. Digests from different algorithms are always
    /// considered incomparable (returns `false`).
    pub fn matches(&self, other: &Self) -> bool {
        self.algorithm == other.algorithm && self.hex == other.hex
    }
}

// ── Per-file record ───────────────────────────────────────────────────────────

/// One entry in a [`DirectoryIndex`] (RFC-037 §"DirectoryEntryRecord").
#[derive(Debug, Clone)]
pub struct DirectoryEntryRecord {
    /// Path relative to the index root, using forward slashes.
    pub relative_path: PathBuf,
    pub entry_type:    EntryType,
    /// File size in bytes. `None` for directories or on read error.
    pub size:          Option<u64>,
    /// Last-modified timestamp. `None` when unavailable.
    pub modified:      Option<SystemTime>,
    /// Content digest, if computed. `None` when not yet hashed or on error.
    pub digest:        Option<ContentDigest>,
    /// Non-fatal per-entry error (permission denied, IO, etc.).
    pub error:         Option<String>,
}

impl DirectoryEntryRecord {
    /// `true` when this entry had a read or metadata error.
    pub fn has_error(&self) -> bool { self.error.is_some() }

    /// `true` when a content digest is available.
    pub fn has_digest(&self) -> bool { self.digest.is_some() }
}

// ── Index revision ────────────────────────────────────────────────────────────

/// Monotonically increasing revision counter for a [`DirectoryIndex`].
/// Incremented each time the index is refreshed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct IndexRevision(pub u64);

impl IndexRevision {
    pub fn next(self) -> Self { Self(self.0 + 1) }
}

// ── Directory index ───────────────────────────────────────────────────────────

/// A snapshot of one directory tree (RFC-037 §"DirectoryIndex").
///
/// Built by a background scan job; consumed by [`pair_entries`] and the
/// compare report renderer. Holds no file handles.
#[derive(Debug, Clone)]
pub struct DirectoryIndex {
    pub root:     PathBuf,
    pub revision: IndexRevision,
    pub entries:  Vec<DirectoryEntryRecord>,
    /// Number of entries skipped by the ignore policy.
    pub ignored_count: usize,
    /// `true` when the scan completed without being cancelled.
    pub is_complete: bool,
}

impl DirectoryIndex {
    /// Create an empty index (not yet scanned).
    pub fn empty(root: PathBuf) -> Self {
        Self {
            root,
            revision:      IndexRevision::default(),
            entries:       vec![],
            ignored_count: 0,
            is_complete:   false,
        }
    }

    /// Build an index directly from a list of records (used in tests and
    /// for incremental merges).
    pub fn from_records(
        root:        PathBuf,
        entries:     Vec<DirectoryEntryRecord>,
        is_complete: bool,
    ) -> Self {
        Self {
            root,
            revision:      IndexRevision(1),
            entries,
            ignored_count: 0,
            is_complete,
        }
    }

    /// Look up an entry by relative path.
    pub fn get(&self, rel: &std::path::Path) -> Option<&DirectoryEntryRecord> {
        self.entries.iter().find(|e| e.relative_path == rel)
    }

    /// All file entries (not directories).
    pub fn files(&self) -> impl Iterator<Item = &DirectoryEntryRecord> {
        self.entries.iter().filter(|e| e.entry_type == EntryType::File)
    }

    /// All directory entries.
    pub fn directories(&self) -> impl Iterator<Item = &DirectoryEntryRecord> {
        self.entries.iter().filter(|e| e.entry_type == EntryType::Directory)
    }
}

// ── Equality evidence ─────────────────────────────────────────────────────────

/// The evidence behind an equality determination for one path pair
/// (RFC-008 §5 "Digest Comparison Policy").
///
/// The ordering is from "most certain equal" to "most certain different"
/// with uncertainty states in between. `PartialOrd` is not derived because
/// the ordering is not total for all variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EqualityEvidence {
    /// Both sides have identical digests computed with the same algorithm.
    DigestEqual,
    /// Both sides have the same size and mtime; no digest was needed.
    MetadataEqual,
    /// Sizes match but digest has not been computed yet (still pending).
    MetadataOnly,
    /// The entry exists only on the left side.
    LeftOnly,
    /// The entry exists only on the right side.
    RightOnly,
    /// Both sides have the same path but different entry types
    /// (e.g. file vs directory).
    TypeMismatch { left: EntryType, right: EntryType },
    /// Sizes differ — content is definitely different, no digest needed.
    SizeDifferent { left_size: u64, right_size: u64 },
    /// Both sides have digests that do not match.
    DigestDifferent,
    /// One or both sides had an error; comparison result is unreliable.
    Error { message: String },
    /// Comparison has not been attempted yet.
    Unknown,
}

impl EqualityEvidence {
    /// `true` when the evidence conclusively shows equality.
    pub fn is_equal(&self) -> bool {
        matches!(self, Self::DigestEqual | Self::MetadataEqual)
    }

    /// `true` when the evidence conclusively shows a difference.
    pub fn is_different(&self) -> bool {
        matches!(self,
            Self::SizeDifferent { .. } | Self::DigestDifferent
            | Self::TypeMismatch { .. } | Self::LeftOnly | Self::RightOnly)
    }

    /// `true` when no conclusion can be drawn yet.
    pub fn is_pending(&self) -> bool {
        matches!(self, Self::MetadataOnly | Self::Unknown)
    }

    /// `true` when the entry is present on both sides (regardless of equality).
    pub fn present_on_both_sides(&self) -> bool {
        !matches!(self, Self::LeftOnly | Self::RightOnly | Self::Unknown)
    }
}

// ── Paired entry ──────────────────────────────────────────────────────────────

/// One path's comparison result across a left/right index pair.
#[derive(Debug, Clone)]
pub struct PairedEntry {
    pub relative_path: PathBuf,
    pub evidence:      EqualityEvidence,
    pub left:          Option<DirectoryEntryRecord>,
    pub right:         Option<DirectoryEntryRecord>,
}

impl PairedEntry {
    pub fn left_size(&self)  -> Option<u64> { self.left.as_ref().and_then(|e| e.size) }
    pub fn right_size(&self) -> Option<u64> { self.right.as_ref().and_then(|e| e.size) }
}

/// The complete paired view of two directory indices.
#[derive(Debug, Clone, Default)]
pub struct PairedEntrySet {
    pub entries: Vec<PairedEntry>,
}

impl PairedEntrySet {
    pub fn equal_count(&self)     -> usize { self.entries.iter().filter(|e| e.evidence.is_equal()).count() }
    pub fn different_count(&self) -> usize { self.entries.iter().filter(|e| e.evidence.is_different()).count() }
    pub fn pending_count(&self)   -> usize { self.entries.iter().filter(|e| e.evidence.is_pending()).count() }
    pub fn left_only_count(&self) -> usize {
        self.entries.iter().filter(|e| matches!(e.evidence, EqualityEvidence::LeftOnly)).count()
    }
    pub fn right_only_count(&self) -> usize {
        self.entries.iter().filter(|e| matches!(e.evidence, EqualityEvidence::RightOnly)).count()
    }
}

// ── Pairing function ──────────────────────────────────────────────────────────

/// Pair two directory indices and compute [`EqualityEvidence`] for each path.
///
/// Follows the RFC-008 §5 strategy:
/// 1. Missing on one side → `LeftOnly` / `RightOnly`.
/// 2. Type mismatch → `TypeMismatch`.
/// 3. Size differs → `SizeDifferent` (skip digest).
/// 4. Both have digests → `DigestEqual` / `DigestDifferent`.
/// 5. Size equal, same mtime → `MetadataEqual`.
/// 6. Size equal, no digest → `MetadataOnly` (pending).
/// 7. Error on either side → `Error`.
pub fn pair_entries(left: &DirectoryIndex, right: &DirectoryIndex) -> PairedEntrySet {
    use std::collections::BTreeMap;

    // Index both sides by relative path for O(log n) lookup.
    let left_map:  BTreeMap<&PathBuf, &DirectoryEntryRecord> =
        left.entries.iter().map(|e| (&e.relative_path, e)).collect();
    let right_map: BTreeMap<&PathBuf, &DirectoryEntryRecord> =
        right.entries.iter().map(|e| (&e.relative_path, e)).collect();

    // Union of all relative paths.
    let mut all_paths: std::collections::BTreeSet<&PathBuf> = Default::default();
    all_paths.extend(left_map.keys().copied());
    all_paths.extend(right_map.keys().copied());

    let mut entries = Vec::with_capacity(all_paths.len());

    for path in all_paths {
        let l = left_map.get(path).copied();
        let r = right_map.get(path).copied();

        let evidence = compute_evidence(l, r);

        entries.push(PairedEntry {
            relative_path: (*path).clone(),
            evidence,
            left:  l.cloned(),
            right: r.cloned(),
        });
    }

    PairedEntrySet { entries }
}

fn compute_evidence(
    l: Option<&DirectoryEntryRecord>,
    r: Option<&DirectoryEntryRecord>,
) -> EqualityEvidence {
    match (l, r) {
        (None, None) => EqualityEvidence::Unknown,
        (Some(_), None) => EqualityEvidence::LeftOnly,
        (None, Some(_)) => EqualityEvidence::RightOnly,
        (Some(l), Some(r)) => {
            // Error on either side takes priority.
            if l.has_error() || r.has_error() {
                let msg = l.error.as_deref()
                    .or(r.error.as_deref())
                    .unwrap_or("read error")
                    .to_string();
                return EqualityEvidence::Error { message: msg };
            }

            // Type mismatch.
            if l.entry_type != r.entry_type {
                return EqualityEvidence::TypeMismatch {
                    left:  l.entry_type,
                    right: r.entry_type,
                };
            }

            // Directories: equal if both exist (no content to compare here).
            if l.entry_type == EntryType::Directory {
                return EqualityEvidence::MetadataEqual;
            }

            // Size comparison.
            match (l.size, r.size) {
                (Some(ls), Some(rs)) if ls != rs =>
                    return EqualityEvidence::SizeDifferent {
                        left_size: ls, right_size: rs,
                    },
                _ => {}
            }

            // Digest comparison (sizes match or are unknown).
            match (&l.digest, &r.digest) {
                (Some(ld), Some(rd)) => {
                    if ld.matches(rd) {
                        EqualityEvidence::DigestEqual
                    } else {
                        EqualityEvidence::DigestDifferent
                    }
                }
                _ => {
                    // Same size, same mtime → likely equal.
                    match (l.modified, r.modified) {
                        (Some(lm), Some(rm)) if lm == rm =>
                            EqualityEvidence::MetadataEqual,
                        _ =>
                            EqualityEvidence::MetadataOnly,
                    }
                }
            }
        }
    }
}
