//! Save and file safety (RFC-007, RFC-023).
//!
//! Saving is conservative and explicit. Before writing, the loaded
//! fingerprint is compared against the current on-disk fingerprint; a
//! mismatch is reported as [`CoreError::Conflict`] so the UI can offer
//! reload / overwrite / save-as rather than silently clobbering external
//! edits. Writes go through a temp file in the same directory, then
//! `rename` — atomic in the sense that matters most day to day: a
//! concurrent reader sees the old file or the new one, never a partial
//! write (F9/N2). This is **not** a power-loss durability guarantee —
//! neither the temp file nor its parent directory is `fsync`ed, so a crash
//! at the wrong moment can still leave the target missing or, on some
//! filesystems/mount options, present with unexpected content. An optional
//! backup is taken before the rename.

use std::fs;
use std::path::{Path, PathBuf};

use crate::document::{ExternalFileState, FileFingerprint, check_external_state};
use crate::encoding::encode_text;
use crate::error::{CoreError, IoOperation, Result};

/// Backup behavior for a save.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackupPolicy {
    /// Never create a backup.
    None,
    /// Copy the existing target to `<name>.bak` before overwriting.
    #[default]
    SiblingBak,
}

/// A save request describing exactly what will be written.
#[derive(Debug, Clone)]
pub struct SaveRequest {
    pub target: PathBuf,
    pub content: String,
    /// Encoding label to encode with; unknown labels fall back to UTF-8.
    pub encoding_label: String,
    /// What must be true about `target` immediately before writing
    /// (RFC-077). Replaces the older `Option<FileFingerprint>`, which could
    /// not distinguish "skip the check" from "the path must be absent."
    pub precondition: TargetPrecondition,
    pub backup: BackupPolicy,
}

/// The result of a successful save.
#[derive(Debug, Clone)]
pub struct SaveOutcome {
    pub written_bytes: u64,
    pub new_fingerprint: FileFingerprint,
    pub backup_path: Option<PathBuf>,
    /// `true` when the requested encoding label was unknown and UTF-8 was
    /// substituted; the UI should warn rather than treat this as success.
    pub encoding_fallback_to_utf8: bool,
}

/// Save text to a file with conflict detection, optional backup, and an
/// atomic commit — no-clobber (`persist_noclobber`) when `precondition` is
/// [`TargetPrecondition::MustBeAbsent`], temp-then-rename
/// (`atomic_replace`) otherwise. RFC-077: a save whose target must not
/// exist never falls back to an overwriting write, even if no-clobber
/// commit fails for some other reason — the error propagates instead.
pub fn save_text(request: &SaveRequest) -> Result<SaveOutcome> {
    let target = request.target.as_path();

    check_precondition(target, &request.precondition)?;

    let (bytes, fallback) = encode_text(&request.content, &request.encoding_label);

    let backup_path = if request.backup == BackupPolicy::SiblingBak && target.exists() {
        let bak = backup_path_for(target);
        fs::copy(target, &bak).map_err(|e| CoreError::io(target, IoOperation::CreateBackup, &e))?;
        Some(bak)
    } else {
        None
    };

    match request.precondition {
        TargetPrecondition::MustBeAbsent => persist_noclobber(target, &bytes)?,
        TargetPrecondition::MustMatch(_) | TargetPrecondition::Force => {
            // Create parent directories for Save As to new nested paths.
            if let Some(parent) = target.parent()
                && !parent.exists()
            {
                fs::create_dir_all(parent)
                    .map_err(|e| CoreError::io(parent, IoOperation::Write, &e))?;
            }
            atomic_replace(target, &bytes)?;
        }
    }

    let new_fingerprint = FileFingerprint::capture(target, Some(&bytes))?;
    Ok(SaveOutcome {
        written_bytes: bytes.len() as u64,
        new_fingerprint,
        backup_path,
        encoding_fallback_to_utf8: fallback,
    })
}

/// Writes `bytes` to a same-directory temp file, then renames it onto
/// `target`, overwriting whatever is there. Atomic *visibility* on POSIX
/// (`rename` within the same volume): a concurrent reader sees the old file
/// or the new one, never a partial write. This is not a power-loss
/// durability guarantee — no `fsync`/`sync_all` is called on the temp file
/// or the parent directory (F9/N2) — only that no reader ever observes a
/// torn write while the process keeps running.
///
/// F89/RFC-082 §D5: the temp file is created via `tempfile::Builder`, the
/// same primitive [`persist_noclobber_with_hook`] already uses below — a
/// random `O_EXCL` name, not the predictable `.{filename}.fsk-tmp` this
/// function used to hand-roll. A predictable sibling path is a symlink
/// target an attacker can pre-create: this function used to `fs::write`
/// straight through a pre-existing symlink at that path, silently
/// overwriting whatever the link pointed at and leaving `target` itself
/// replaced by a symlink after the rename (CWE-59/CWE-378). The random name
/// also removes the multi-writer collision a predictable shared path
/// invited — two concurrent saves of the same `target` no longer race on
/// the same temp file.
///
/// Deliberately **not** `persist_noclobber`: `atomic_replace` overwrites an
/// existing `target` by contract — that's its whole job, and the caller
/// (`save_text`'s `MustMatch`/`Force` arm) has already run its own
/// precondition check. Deliberately **not** creating `target`'s parent
/// directory either, unlike [`persist_noclobber_with_hook`] — that call
/// belongs to `save_text` (Save As to a new nested path) and to
/// `atomic_write_envelope` (`persist::schema`'s first-write-on-fresh-install
/// case, F61/F62); folding it in here would make Save As's own
/// `create_dir_all` redundant and would change plain-save semantics
/// (writing into a missing directory would start silently succeeding).
///
/// `pub(crate)` so `persist::schema`'s repositories (RFC-076) can reuse this
/// primitive for settings/session writes instead of hand-rolling their own
/// temp-then-rename logic. This function carries no document-save-specific
/// behavior (no fingerprint check, no `.bak` backup) — those stay in
/// [`save_text`]; callers needing them apply their own policy.
pub(crate) fn atomic_replace(target: &Path, bytes: &[u8]) -> Result<()> {
    let dir = target.parent().unwrap_or_else(|| Path::new("."));
    // See persist_noclobber_with_hook's comment: 0o666 (kernel applies the
    // umask), not NamedTempFile's 0600 default — this temp file becomes the
    // permanent target file and must end up with the same permissions a
    // plain fs::write would have produced.
    let mut builder = tempfile::Builder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(fs::Permissions::from_mode(0o666));
    }
    let mut tmp = builder
        .tempfile_in(dir)
        .map_err(|e| CoreError::io(dir, IoOperation::Write, &e))?;
    std::io::Write::write_all(&mut tmp, bytes)
        .map_err(|e| CoreError::io(target, IoOperation::Write, &e))?;
    tmp.persist(target)
        .map_err(|e| CoreError::io(target, IoOperation::Rename, &e.error))?;
    Ok(())
}

// ── RFC-077: explicit target precondition and no-clobber commit ───────────────

/// What a save must find true about its target immediately before writing.
/// `SaveRequest::precondition`'s type — replaces an earlier
/// `Option<FileFingerprint>`, which could not express "the path must not
/// exist" (`None` there meant "skip the check," not "must be absent").
///
/// The save-time counterpart of [`crate::compare_prep::TargetExpectation`],
/// the load-time snapshot this is checked against — read them together, not
/// as one type: a snapshot is captured once at preparation time, a
/// precondition is checked immediately before every write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetPrecondition {
    /// The target must exist, be a plain text file, and match this
    /// fingerprint exactly.
    MustMatch(FileFingerprint),
    /// The target must not exist at all — any entry (file, directory, or
    /// otherwise) at the path is a conflict.
    MustBeAbsent,
    /// Skip the check. Constructed only after an explicit, one-time user
    /// overwrite confirmation — never stored as a tab's load-time snapshot
    /// (RFC-077).
    Force,
}

/// Checks `precondition` against `target`'s current on-disk state, without
/// writing anything. Reuses [`check_external_state`] (RFC-036) for
/// `MustMatch` rather than re-deriving fingerprint comparison — the same
/// missing/changed/replaced distinctions apply to both.
///
/// Returns [`CoreError::Conflict`] on any precondition failure, never
/// [`CoreError::Io`] for an ordinary "not what was expected" case — review
/// 038 C1 (RFC-076) established that a stale-read conflict and a genuine I/O
/// failure must stay distinguishable. `MustBeAbsent` uses
/// [`fs::symlink_metadata`] rather than [`Path::exists`] so a dangling
/// symlink (an entry the path-following `exists()` would misreport as
/// absent) is correctly treated as present, and a genuine read failure (e.g.
/// permission denied on a parent directory) propagates as `Io` instead of
/// being silently swallowed as "absent" (review 046 N1).
pub fn check_precondition(target: &Path, precondition: &TargetPrecondition) -> Result<()> {
    match precondition {
        TargetPrecondition::Force => Ok(()),
        TargetPrecondition::MustBeAbsent => match fs::symlink_metadata(target) {
            Ok(_) => Err(CoreError::Conflict {
                message: "target already exists".into(),
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(CoreError::io(target, IoOperation::Metadata, &e)),
        },
        TargetPrecondition::MustMatch(expected) => {
            match check_external_state(target, expected, false) {
                ExternalFileState::Clean => Ok(()),
                other => Err(CoreError::Conflict {
                    message: format!("target changed on disk after it was loaded ({other:?})"),
                }),
            }
        }
    }
}

/// Atomically commits `bytes` to `target`, failing rather than replacing if
/// any entry already exists there. Same-directory temp file + platform
/// no-clobber commit (`tempfile::NamedTempFile::persist_noclobber`), per
/// RFC-077: "If any filesystem entry appears after preparation, normal save
/// returns a conflict and preserves that entry." Creates missing parent
/// directories first, mirroring [`save_text`]'s Save-As-to-new-path support.
///
/// If the platform cannot provide this guarantee, the error propagates —
/// callers must not fall back to an overwriting write (RFC-077: "normal save
/// fails rather than falling back to replacement").
pub fn persist_noclobber(target: &Path, bytes: &[u8]) -> Result<()> {
    persist_noclobber_with_hook(target, bytes, || {})
}

/// The same commit, with a hook that runs after the temp file is fully
/// written but before the no-clobber commit — the deterministic race seam
/// RFC-077's test design calls for ("a test-only before-commit seam creates
/// the competing file after preparation/temp write but before
/// `persist_noclobber`"), used instead of sleeps to exercise the race
/// reliably. `pub(crate)` — only [`persist_noclobber`] and this module's own
/// tests need it.
pub(crate) fn persist_noclobber_with_hook(
    target: &Path,
    bytes: &[u8],
    before_commit: impl FnOnce(),
) -> Result<()> {
    if let Some(parent) = target.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent).map_err(|e| CoreError::io(parent, IoOperation::Write, &e))?;
    }
    let dir = target.parent().unwrap_or_else(|| Path::new("."));
    // `NamedTempFile` defaults to 0600 (deliberately narrow, since temp
    // files often hold sensitive scratch data) — but this one becomes a
    // permanent, ordinary output file, and it must end up with the
    // permissions `atomic_replace`'s plain `fs::write` would have produced:
    // whatever the process umask allows for a freshly created file, not a
    // hardcoded constant that's only correct under `umask 022`. Requesting
    // 0o666 and letting the kernel apply the umask (the same thing
    // `open(2)`'s default create mode does for `fs::write`) gets that
    // property without querying or touching the process-wide umask
    // ourselves. No Windows equivalent (no POSIX mode bits); its default
    // ACL behavior is unaffected.
    let mut builder = tempfile::Builder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(fs::Permissions::from_mode(0o666));
    }
    let mut tmp = builder
        .tempfile_in(dir)
        .map_err(|e| CoreError::io(dir, IoOperation::Write, &e))?;
    std::io::Write::write_all(&mut tmp, bytes)
        .map_err(|e| CoreError::io(target, IoOperation::Write, &e))?;

    before_commit();

    tmp.persist_noclobber(target).map_err(|e| {
        if e.error.kind() == std::io::ErrorKind::AlreadyExists {
            CoreError::Conflict {
                message: "target already exists".into(),
            }
        } else {
            CoreError::io(target, IoOperation::Rename, &e.error)
        }
    })?;
    Ok(())
}

fn file_name_string(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "forskscope".into())
}

fn backup_path_for(target: &Path) -> PathBuf {
    let name = format!("{}.bak", file_name_string(target));
    sibling(target, &name)
}

fn sibling(target: &Path, name: &str) -> PathBuf {
    match target.parent() {
        Some(parent) => parent.join(name),
        None => PathBuf::from(name),
    }
}
