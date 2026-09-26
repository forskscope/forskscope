//! Explorer workspace: two-pane directory browser (RFC-054).
//!
//! This file owns signal setup, digest computation, and top-level layout.
//! Sub-components live in the `explorer/` subdirectory:
//!
//! - `tree.rs`    — aligned two-pane tree with keyboard navigation
//! - `compact.rs` — compact (unaligned) tree view (RFC-068)
//! - `filter.rs`  — filter bar UI and filter predicate (RFC-067)
//! - `footer.rs`  — targets label and Compare button (RFC-069)

pub mod compact;
pub mod filter;
pub mod footer;
pub mod tree;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dioxus::prelude::*;
use dioxus_swdir_tree::{DirectoryTree, use_scan_driver};

use forskscope_core::IgnoreRules;
use forskscope_core::dir::{
    DigestOutcome, EntryType, EqualityEvidence, file_digest_equal_with_cancel,
    list_recursive_for_display_with_rules,
};
use forskscope_core::error::Result as CoreResult;

use crate::i18n::t;
use crate::state::Store;
use crate::ui::view::digest_epoch::{DigestEpoch, EpochStamp};
use crate::ui::view::dir_pane::{
    FilteringExecutor, NavHistory, PathBar, SharedIgnoreRules, home_dir, short_name,
};
use forskscope_ui_logic::{
    DirVerdict, Tier1Action, Tier1Trigger, compute_aligned_rows, dir_verdict,
};

use compact::CompactTree;
use filter::{FilterBar, apply_filter};
use footer::ExplorerFooter;
use tree::AlignedTree;

// ── Shared types ──────────────────────────────────────────────────────────────

/// Default directory for an explorer pane when no directory has been persisted
/// (e.g. first boot with no saved settings).
///
/// Preference order:
/// 1. the user's home directory (the most useful starting point), then
/// 2. the process working directory, then
/// 3. the filesystem root as a last resort.
///
/// Home is preferred over the working directory because at first launch the
/// working directory is wherever the app was started from — often `/` for a
/// desktop launcher — which is not a useful place to begin browsing.
///
/// Uses [`dir_pane::home_dir`] (HOME / USERPROFILE), already used elsewhere in
/// the explorer, falling back to the working directory only if home cannot be
/// resolved.
fn default_explorer_dir() -> PathBuf {
    let home = home_dir();
    if home.as_os_str().is_empty() || home == std::path::Path::new("/") {
        std::env::current_dir().unwrap_or(home)
    } else {
        home
    }
}

/// Typed key for the digest map (RFC-059 §M2).
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum DigestKey {
    Common(PathBuf),
    RightOnly(PathBuf),
}

/// F74 review 072: what a left-side entry's classification will be, before
/// any async work starts. `Final` is inserted immediately; `NeedsDigest`
/// means the caller inserts a pending placeholder and starts the real
/// digest comparison - a file present on both sides is the one case this
/// function cannot resolve synchronously.
#[derive(Debug, PartialEq)]
enum EntryClassification {
    Final(EqualityEvidence),
    NeedsDigest {
        left_abs: PathBuf,
        right_abs: PathBuf,
    },
}

/// F74 review 072 / handoff 007 §7b: the per-entry classification,
/// extracted from the `use_effect` below so a test can drive it against
/// real directories - this is the exact call site `9f355c6` got wrong
/// (`if cp.is_dir() { DigestState::Equal }`), reachable without a
/// `VirtualDom`. Takes only plain values - no signal reads here, per the
/// extraction risk `dir_pane.rs`'s `apply_navigation` split already
/// established (keep signal reads in the closure, pass plain values in).
///
/// Emits `EqualityEvidence` (core's vocabulary), not a display state - F76's
/// two instances fall out of getting this right:
/// - a directory whose counterpart is a directory → `Unknown` (the
///   Explorer never examines directory contents; that verdict is Deep
///   Compare's job, design decision handoff 002 §2 - `RowStatusKind`
///   renders `Unknown` as `NotCompared`, never a claimed `Equal`);
/// - a directory whose counterpart is a **file**, or the mirror (a file
///   whose counterpart is a directory) → `TypeMismatch` - both sides have
///   *something* at this path, so `LeftOnly`/`RightOnly` would be false;
/// - neither a directory nor a file counterpart exists → `LeftOnly`,
///   genuinely one-sided.
fn classify_entry(rel: &Path, is_dir: bool, l_root: &Path, r_root: &Path) -> EntryClassification {
    let cp = r_root.join(rel);
    if is_dir {
        if cp.is_dir() {
            EntryClassification::Final(EqualityEvidence::Unknown)
        } else if cp.is_file() {
            EntryClassification::Final(EqualityEvidence::TypeMismatch {
                left: EntryType::Directory,
                right: EntryType::File,
            })
        } else {
            EntryClassification::Final(EqualityEvidence::LeftOnly)
        }
    } else if cp.is_dir() {
        EntryClassification::Final(EqualityEvidence::TypeMismatch {
            left: EntryType::File,
            right: EntryType::Directory,
        })
    } else if cp.is_file() {
        EntryClassification::NeedsDigest {
            left_abs: l_root.join(rel),
            right_abs: cp,
        }
    } else {
        EntryClassification::Final(EqualityEvidence::LeftOnly)
    }
}

/// F76's second instance: `file_digest_equal_with_cancel`'s outcome, mapped
/// to the evidence it actually establishes. A read failure (`Err`) becomes
/// `Error`, not a fabricated `DigestDifferent` - the pre-existing defect
/// this handoff closes was asserting a verdict nothing measured. `None`
/// means "establish nothing, apply nothing" (`Cancelled` - the root change
/// that cancelled this already cleared the map).
fn classify_digest_outcome(outcome: CoreResult<DigestOutcome>) -> Option<EqualityEvidence> {
    match outcome {
        Ok(DigestOutcome::Equal) => Some(EqualityEvidence::DigestEqual),
        Ok(DigestOutcome::Different) => Some(EqualityEvidence::DigestDifferent),
        Ok(DigestOutcome::Cancelled) => None,
        Err(e) => Some(EqualityEvidence::Error {
            message: e.to_string(),
        }),
    }
}

/// F78: applies a completed async digest result to `digest_map` only if
/// `stamp` is still current for `epoch` - a comparison spawned under one
/// pair of roots that finishes after the roots changed must not land on
/// whatever entry now occupies that key. This is required even with the
/// epoch's own cancellation token: cancellation is inherently racy (a
/// comparison can finish in the window between the root change and the
/// token being observed), so the token stops wasted work while this guard
/// stops a wrong result from reaching a live key - neither substitutes for
/// the other. Takes the map and epoch directly (not `Signal`s) so a test
/// can drive it without a Dioxus runtime - the same reason F77's
/// `apply_digest_result` took the map by value, now backed by
/// `DigestEpoch::is_current` instead of a raw generation-number compare.
fn apply_epoch_result(
    digest_map: &mut HashMap<DigestKey, EqualityEvidence>,
    key: DigestKey,
    state: EqualityEvidence,
    stamp: EpochStamp,
    epoch: &DigestEpoch,
) {
    if epoch.is_current(stamp) {
        digest_map.insert(key, state);
    }
}

/// Key of a tier-1 directory verdict (RFC-080 §5): the pair of roots it was
/// measured under and the directory's path relative to both.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Tier1Key {
    pub left_root: PathBuf,
    pub right_root: PathBuf,
    pub rel: PathBuf,
}

/// Tier-1 results. **Not** `digest_map`: every tree expand clears that map
/// wholesale, so a verdict stored there would be destroyed by an unrelated
/// toggle. This has its own invalidation — cleared when the roots change and
/// when the ignore rules do — and is never persisted.
pub type Tier1Map = HashMap<Tier1Key, EqualityEvidence>;

/// The evidence a row renders. A same-named directory pair sits at `Unknown`
/// ("directory contents not compared") in `digest_map`; if a tier-1 walk has
/// produced a result for it, that result is shown instead. Nothing else is
/// overridden — a file row, a one-sided row or a type mismatch keeps its own.
pub fn row_evidence(
    base: Option<EqualityEvidence>,
    tier1: &Tier1Map,
    l_root: &Path,
    r_root: &Path,
    rel: &Path,
) -> Option<EqualityEvidence> {
    match base {
        Some(EqualityEvidence::Unknown) => {
            let key = Tier1Key {
                left_root: l_root.to_path_buf(),
                right_root: r_root.to_path_buf(),
                rel: rel.to_path_buf(),
            };
            tier1.get(&key).cloned().or(base)
        }
        other => other,
    }
}

/// The directory row a tier-1 walk would be started for: the first selected
/// directory (left pane's, then right's) whose pair `digest_map` classified as
/// an unexamined same-named pair, and which has no tier-1 result yet (a cached
/// verdict, including "no verdict", is never walked again).
fn tier1_candidate(
    selected_dirs: &[PathBuf],
    digest_map: &HashMap<DigestKey, EqualityEvidence>,
    tier1: &Tier1Map,
    l_root: &Path,
    r_root: &Path,
) -> Option<PathBuf> {
    selected_dirs.iter().find_map(|rel| {
        let unexamined = matches!(
            digest_map.get(&DigestKey::Common(rel.clone())),
            Some(EqualityEvidence::Unknown)
        );
        let key = Tier1Key {
            left_root: l_root.to_path_buf(),
            right_root: r_root.to_path_buf(),
            rel: rel.clone(),
        };
        // The in-flight row's own "Comparing…" marker is not a result: it must stay
        // the candidate, or the effect would cancel the very walk it started.
        let has_result = tier1
            .get(&key)
            .is_some_and(|e| *e != EqualityEvidence::MetadataOnly);
        (unexamined && !has_result).then(|| rel.clone())
    })
}

/// Relative paths of the selected directory rows in one pane's tree.
fn selected_dirs(tree: &DirectoryTree, root: &Path) -> Vec<PathBuf> {
    tree.visible_rows()
        .into_iter()
        .filter(|(n, _)| n.is_dir && n.is_selected)
        .filter_map(|(n, _)| n.path.strip_prefix(root).ok().map(Path::to_path_buf))
        .filter(|rel| !rel.as_os_str().is_empty())
        .collect()
}

/// What tier 1 concluded, as the evidence a row renders. `Unknown` is stored
/// too — it renders as "not compared" and, being cached, stops the selection
/// effect from walking the same row again forever.
fn verdict_evidence(verdict: DirVerdict) -> EqualityEvidence {
    match verdict {
        DirVerdict::Different => EqualityEvidence::TreeDifferent,
        DirVerdict::MetadataMatch => EqualityEvidence::MetadataMatch,
        DirVerdict::Unknown => EqualityEvidence::Unknown,
    }
}

/// What a finished tier-1 walk may apply. `None` when the walk was cancelled or
/// its epoch is no longer current: a cancelled walk returns a *partial* scan,
/// and a partial scan can read as "names and sizes match" — applying it would be
/// exactly the false result cancellation exists to prevent. Checked, not assumed
/// (the F61 pattern).
fn tier1_outcome(
    scan: &forskscope_core::dir::RecursiveScan,
    cancelled: bool,
    epoch_current: bool,
) -> Option<EqualityEvidence> {
    if cancelled || !epoch_current {
        return None;
    }
    Some(verdict_evidence(dir_verdict(scan)))
}

/// Start the tier-1 walk for `rel` (RFC-080 §3, §5). The row shows "Comparing…"
/// while it runs. It goes through a `DigestEpoch` of its own (concurrency 1: at
/// most one walk) — the same mechanism every other comparison uses, so this adds
/// no second way to cancel. A result is applied only if the epoch is still
/// current *and* the token was not cancelled: the selection moving or the roots
/// changing both `restart()` it.
#[allow(clippy::too_many_arguments)]
fn start_tier1_walk(
    rel: PathBuf,
    l_root: PathBuf,
    r_root: PathBuf,
    rules: IgnoreRules,
    lang: crate::state::Lang,
    mut tier1_map: Signal<Tier1Map>,
    tier1_epoch: Signal<DigestEpoch>,
    mut trigger: Signal<Tier1Trigger<PathBuf>>,
    mut announcement: Signal<String>,
) {
    let key = Tier1Key {
        left_root: l_root.clone(),
        right_root: r_root.clone(),
        rel: rel.clone(),
    };
    tier1_map
        .write()
        .insert(key.clone(), EqualityEvidence::MetadataOnly);
    let (stamp, token, sem) = tier1_epoch.read().begin_task();
    spawn(async move {
        let _permit = sem.acquire_owned().await;
        let (left, right) = (l_root.join(&rel), r_root.join(&rel));
        let walk_token = token.clone();
        let scan = tokio::task::spawn_blocking(move || {
            list_recursive_for_display_with_rules(&left, &right, &walk_token, &rules)
        })
        .await;
        trigger.write().finished(&rel);
        // Cancelled, superseded, or the walk itself panicked: nothing was
        // established, so nothing is applied. Drop the "Comparing…" marker so a
        // later rest can walk this row again.
        let Ok(scan) = scan else {
            tier1_map.write().remove(&key);
            return;
        };
        let Some(evidence) = tier1_outcome(
            &scan,
            token.is_cancelled(),
            tier1_epoch.read().is_current(stamp),
        ) else {
            return;
        };
        let kind = forskscope_ui_logic::RowStatusKind::from_evidence(&evidence);
        tier1_map.write().insert(key, evidence);
        // Completion is announced, not silently swapped in (§6): a polite live
        // region, `role="status"`, present in the Explorer at all times.
        let label = crate::ui::view::dir_pane::status_kind_label(kind, lang, true);
        announcement.set(format!("{}: {}", short_name(&rel), label));
    });
}

/// Which pane currently receives keyboard events (RFC-061).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    Left,
    Right,
}

impl FocusedPane {
    pub fn toggle(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
    pub fn is_left(self) -> bool {
        self == Self::Left
    }
    pub fn is_right(self) -> bool {
        self == Self::Right
    }
}

/// A user's pending pick in one pane.
#[derive(Clone, PartialEq, Eq)]
pub enum PickKind {
    File(PathBuf),
    Dir(PathBuf),
}

impl PickKind {
    pub fn path(&self) -> &PathBuf {
        match self {
            Self::File(p) | Self::Dir(p) => p,
        }
    }
    pub fn is_file(&self) -> bool {
        matches!(self, Self::File(_))
    }
}

/// Derived action from the current left + right picks.
#[derive(Clone, PartialEq, Eq)]
pub enum CompareAction {
    Files(PathBuf, PathBuf),
    Dirs(PathBuf, PathBuf),
    None,
}

pub fn compare_action(lp: &Option<PickKind>, rp: &Option<PickKind>) -> CompareAction {
    match (lp, rp) {
        (Some(PickKind::File(l)), Some(PickKind::File(r))) => {
            CompareAction::Files(l.clone(), r.clone())
        }
        (Some(PickKind::Dir(l)), Some(PickKind::Dir(r))) => {
            CompareAction::Dirs(l.clone(), r.clone())
        }
        _ => CompareAction::None,
    }
}

// ── Explorer root component ───────────────────────────────────────────────────

#[component]
pub fn Explorer() -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();

    // F112: the rules the scans use are a shared handle the executors read at each
    // scan request, and a memo that changes only when the rules do. The two tree
    // effects below read the memo, so a settings change re-runs them — rebuilding
    // both trees under the new rules — without a restart or a navigation.
    let settings = store.settings;
    let ignore_rules = use_memo(move || settings.read().ignore_rules());
    let shared_rules = use_hook(|| SharedIgnoreRules::new(settings.peek().ignore_rules()));
    let binary_enabled = store.settings.read().enable_binary_comparison;
    let compact_mode = store.settings.read().explorer_compact;

    // Binary sniff cache — cleared on directory change (RFC-066).
    let mut binary_cache: Signal<HashMap<PathBuf, bool>> = use_signal(Default::default);

    // ── Left pane ─────────────────────────────────────────────────────────────
    let remember = store.settings.read().remember_explorer_dirs;
    let init_l = if remember {
        store
            .settings
            .read()
            .last_left_dir
            .clone()
            .unwrap_or_else(default_explorer_dir)
    } else {
        default_explorer_dir()
    };
    let left_dir: Signal<PathBuf> = use_signal(|| init_l.clone());
    let mut left_hist: Signal<NavHistory> = use_signal(NavHistory::default);
    use_hook(|| left_hist.write().push(init_l.clone()));

    let exec_l = Arc::new(FilteringExecutor {
        rules: shared_rules.clone(),
    });
    let mut tree_l: Signal<DirectoryTree> = use_signal(|| DirectoryTree::new(init_l.clone()));
    let scans_l = use_scan_driver(tree_l, exec_l);

    let rules_l = shared_rules.clone();
    use_effect(move || {
        // Subscribes to the rules: a change re-runs this and rebuilds the tree.
        rules_l.set(ignore_rules());
        let root = left_dir.read().cloned();
        let mut nt = DirectoryTree::new(root.clone());
        binary_cache.write().clear();
        if let Some(req) = nt.on_toggled(&root) {
            tree_l.set(nt);
            scans_l.send(req);
        } else {
            tree_l.set(nt);
        }
    });

    // ── Right pane ────────────────────────────────────────────────────────────
    let init_r = if remember {
        store
            .settings
            .read()
            .last_right_dir
            .clone()
            .unwrap_or_else(default_explorer_dir)
    } else {
        default_explorer_dir()
    };
    let right_dir: Signal<PathBuf> = use_signal(|| init_r.clone());
    let mut right_hist: Signal<NavHistory> = use_signal(NavHistory::default);
    use_hook(|| right_hist.write().push(init_r.clone()));

    let exec_r = Arc::new(FilteringExecutor {
        rules: shared_rules.clone(),
    });
    let mut tree_r: Signal<DirectoryTree> = use_signal(|| DirectoryTree::new(init_r.clone()));
    let scans_r = use_scan_driver(tree_r, exec_r);

    let rules_r = shared_rules.clone();
    use_effect(move || {
        rules_r.set(ignore_rules());
        let root = right_dir.read().cloned();
        let mut nt = DirectoryTree::new(root.clone());
        binary_cache.write().clear();
        if let Some(req) = nt.on_toggled(&root) {
            tree_r.set(nt);
            scans_r.send(req);
        } else {
            tree_r.set(nt);
        }
    });

    // ── Digest map ────────────────────────────────────────────────────────────
    let mut digest_map: Signal<HashMap<DigestKey, EqualityEvidence>> = use_signal(HashMap::new);
    let mut digest_roots: Signal<(PathBuf, PathBuf)> =
        use_signal(|| (PathBuf::new(), PathBuf::new()));
    // F78: `digest_epoch` owns the generation guard, the cancellation
    // token, and (new for the Explorer) the concurrency bound that
    // `deep_compare.rs` already had and this view didn't - one
    // `spawn_blocking` per common file, unbounded, before this change.
    // `restart()` is called at the same place `digest_map` is cleared -
    // see the `changed` block below.
    let mut digest_epoch: Signal<DigestEpoch> =
        use_signal(|| DigestEpoch::new(forskscope_core::DIGEST_CONCURRENCY_LIMIT));

    // RFC-080 tier 1. Its own results map (see `Tier1Map`), and its own
    // `DigestEpoch` — concurrency 1, so at most one walk in flight — restarted
    // when the selection moves as well as when the roots change. The debounce
    // is the pure state machine in `ui-logic`; this view supplies the clock and
    // the spawning.
    let mut tier1_map: Signal<Tier1Map> = use_signal(HashMap::new);
    let mut tier1_epoch: Signal<DigestEpoch> = use_signal(|| DigestEpoch::new(1));
    let mut tier1_trigger: Signal<Tier1Trigger<PathBuf>> =
        use_signal(Tier1Trigger::<PathBuf>::default);
    let tier1_announcement: Signal<String> = use_signal(String::new);
    let clock = use_hook(std::time::Instant::now);

    use_effect(move || {
        let l_root = left_dir.read().cloned();
        let r_root = right_dir.read().cloned();
        if l_root.as_os_str().is_empty() || r_root.as_os_str().is_empty() {
            return;
        }

        {
            let roots = digest_roots.read();
            let changed = roots.0 != l_root || roots.1 != r_root;
            drop(roots);
            if changed {
                // Cancel outstanding comparisons under the old roots
                // before anything else observes the new ones - stops
                // wasted work for whatever hasn't finished yet.
                digest_epoch.write().restart();
                digest_map.write().clear();
                // Navigation drops tier-1 results too (§5) and stops the walk.
                tier1_epoch.write().restart();
                tier1_map.write().clear();
                tier1_trigger.write().selected(clock.elapsed(), None);
                digest_roots.set((l_root.clone(), r_root.clone()));
            }
        }

        let left_entries: Vec<(PathBuf, bool)> = tree_l
            .read()
            .visible_rows()
            .into_iter()
            .filter_map(|(n, _)| {
                let rel = n.path.strip_prefix(&l_root).ok()?.to_path_buf();
                if rel.as_os_str().is_empty() {
                    return None;
                }
                Some((rel, n.is_dir))
            })
            .collect();

        for (rel, is_dir) in left_entries {
            if digest_map
                .read()
                .contains_key(&DigestKey::Common(rel.clone()))
            {
                continue;
            }
            match classify_entry(&rel, is_dir, &l_root, &r_root) {
                EntryClassification::Final(state) => {
                    digest_map.write().insert(DigestKey::Common(rel), state);
                }
                EntryClassification::NeedsDigest {
                    left_abs,
                    right_abs,
                } => {
                    let key = rel.clone();
                    let mut dmap = digest_map;
                    dmap.write().insert(
                        DigestKey::Common(key.clone()),
                        EqualityEvidence::MetadataOnly,
                    );
                    // F78: captured now, at spawn time - the stamp this
                    // comparison belongs to, the token that can stop it if
                    // the roots change before it finishes, and the
                    // semaphore permit that bounds how many of these run
                    // at once (new for the Explorer - see digest_epoch.rs).
                    let (stamp, token, sem) = digest_epoch.read().begin_task();
                    let epoch_signal = digest_epoch;
                    spawn(async move {
                        // Acquired here, inside the task, after the
                        // `begin_task()` read guard above has already been
                        // dropped - never held across an await.
                        let _permit = sem.acquire_owned().await;
                        let joined = tokio::task::spawn_blocking(move || {
                            file_digest_equal_with_cancel(&left_abs, &right_abs, &token)
                        })
                        .await;
                        let evidence = match joined {
                            Ok(outcome) => classify_digest_outcome(outcome),
                            // A join error (the blocking task panicked) has
                            // no real verdict either - F76's second
                            // instance applies here too: establish nothing,
                            // don't fabricate one.
                            Err(_) => Some(EqualityEvidence::Error {
                                message: "digest comparison task panicked".to_string(),
                            }),
                        };
                        let Some(evidence) = evidence else {
                            // Cancelled - established nothing, there is no
                            // verdict to (conditionally or not) apply. The
                            // root change that cancelled this also cleared
                            // digest_map already.
                            return;
                        };
                        apply_epoch_result(
                            &mut dmap.write(),
                            DigestKey::Common(key),
                            evidence,
                            stamp,
                            &epoch_signal.read(),
                        );
                    });
                }
            }
        }

        let r_root2 = right_dir.read().cloned();
        let l_root2 = left_dir.read().cloned();
        let right_entries: Vec<PathBuf> = tree_r
            .read()
            .visible_rows()
            .into_iter()
            .filter_map(|(n, _)| {
                let rel = n.path.strip_prefix(&r_root2).ok()?.to_path_buf();
                if rel.as_os_str().is_empty() {
                    return None;
                }
                Some(rel)
            })
            .collect();
        for rel in right_entries {
            let key = DigestKey::RightOnly(rel.clone());
            if digest_map.read().contains_key(&key) {
                continue;
            }
            if !l_root2.join(&rel).exists() {
                digest_map.write().insert(key, EqualityEvidence::RightOnly);
            }
        }
    });

    // The ignore rules changing invalidates tier-1 results (a verdict under old
    // rules is a different answer) and stops any walk.
    use_effect(move || {
        let _ = ignore_rules();
        tier1_epoch.write().restart();
        tier1_map.write().clear();
        tier1_trigger.write().selected(clock.elapsed(), None);
    });

    // ── Tier-1 trigger (RFC-080 §5, criterion 7) ──────────────────────────────
    // Runs whenever the selection, the rows or the classifications change. A row
    // starts a walk only after being rested on for `TIER1_DEBOUNCE`; passing over
    // rows starts none. Reads of `tier1_map` and the trigger are `peek`s: this
    // effect writes both, and must not subscribe to its own writes.
    let rules_for_walk = shared_rules.clone();
    use_effect(move || {
        let l_root = left_dir.read().cloned();
        let r_root = right_dir.read().cloned();
        let mut selected = selected_dirs(&tree_l.read(), &l_root);
        selected.extend(selected_dirs(&tree_r.read(), &r_root));
        let candidate = tier1_candidate(
            &selected,
            &digest_map.read(),
            &tier1_map.peek(),
            &l_root,
            &r_root,
        );
        let now = clock.elapsed();
        let action = tier1_trigger.write().selected(now, candidate);
        if let Some(Tier1Action::CancelWalk) = action {
            // The selection moved off the row being walked: stop it, and drop
            // its "Comparing…" marker so the row returns to "not compared".
            tier1_epoch.write().restart();
            tier1_map
                .write()
                .retain(|_, v| *v != EqualityEvidence::MetadataOnly);
        }
        let Some(due) = tier1_trigger.peek().due_at() else {
            return;
        };
        let delay = due.saturating_sub(now);
        let rules = rules_for_walk.clone();
        let (l, r) = (l_root, r_root);
        spawn(async move {
            tokio::time::sleep(delay + std::time::Duration::from_millis(5)).await;
            let started = tier1_trigger.write().tick(clock.elapsed());
            if let Some(rel) = started {
                start_tier1_walk(
                    rel,
                    l,
                    r,
                    rules.get(),
                    lang,
                    tier1_map,
                    tier1_epoch,
                    tier1_trigger,
                    tier1_announcement,
                );
            }
        });
    });

    // ── Filter state ──────────────────────────────────────────────────────────
    let filter_open: Signal<bool> = use_signal(|| false);
    let filter_query: Signal<String> = use_signal(String::new);
    let filter_hide_bin: Signal<bool> = use_signal(|| false);
    let filter_hide_eq: Signal<bool> = use_signal(|| false);

    // ── Picks ─────────────────────────────────────────────────────────────────
    let left_pick: Signal<Option<PickKind>> = use_signal(|| None);
    let right_pick: Signal<Option<PickKind>> = use_signal(|| None);

    let mut focused_pane: Signal<FocusedPane> = use_signal(|| FocusedPane::Left);

    use_effect(move || {
        let lp = left_pick.read();
        store.left_pick.set(
            lp.as_ref()
                .filter(|p| p.is_file())
                .map(|p| p.path().clone()),
        );
    });
    use_effect(move || {
        let rp = right_pick.read();
        store.right_pick.set(
            rp.as_ref()
                .filter(|p| p.is_file())
                .map(|p| p.path().clone()),
        );
    });

    // ── Compute rows ──────────────────────────────────────────────────────────
    let l_root_snap = left_dir.read().cloned();
    let r_root_snap = right_dir.read().cloned();

    let left_flat: Vec<(PathBuf, bool, bool, bool, u32)> = tree_l
        .read()
        .visible_rows()
        .into_iter()
        .filter(|(n, _)| n.path != l_root_snap)
        .map(|(n, d)| (n.path.clone(), n.is_dir, n.is_expanded, n.is_selected, d))
        .collect();
    let right_flat: Vec<(PathBuf, bool, bool, bool, u32)> = tree_r
        .read()
        .visible_rows()
        .into_iter()
        .filter(|(n, _)| n.path != r_root_snap)
        .map(|(n, d)| (n.path.clone(), n.is_dir, n.is_expanded, n.is_selected, d))
        .collect();

    let aligned = compute_aligned_rows(&left_flat, &right_flat, &l_root_snap, &r_root_snap);
    let aligned = apply_filter(
        aligned,
        &filter_query.read().to_lowercase(),
        *filter_hide_bin.read(),
        *filter_hide_eq.read(),
        binary_enabled,
        &digest_map.read(),
        &mut binary_cache,
    );

    rsx! {
        div { class: "explorer",
            div { class: "explorer-browse",

                // ── Path bars ─────────────────────────────────────────────
                div { class: "explorer-path-bars",
                    PathBar {
                        path: left_dir.read().cloned(),
                        can_back:    left_hist.read().can_back(),
                        can_forward: left_hist.read().can_forward(),
                        on_back:    move |_| { let p = left_hist.write().back();    if let Some(p) = p { crate::ui::view::dir_pane::navigate_to_from_history(p, true,  store, left_dir); } },
                        on_forward: move |_| { let p = left_hist.write().forward(); if let Some(p) = p { crate::ui::view::dir_pane::navigate_to_from_history(p, true,  store, left_dir); } },
                        on_navigate: move |p| crate::ui::view::dir_pane::navigate_to(p, true, store, left_hist, left_dir),
                        lang,
                    }
                    PathBar {
                        path: right_dir.read().cloned(),
                        can_back:    right_hist.read().can_back(),
                        can_forward: right_hist.read().can_forward(),
                        on_back:    move |_| { let p = right_hist.write().back();    if let Some(p) = p { crate::ui::view::dir_pane::navigate_to_from_history(p, false, store, right_dir); } },
                        on_forward: move |_| { let p = right_hist.write().forward(); if let Some(p) = p { crate::ui::view::dir_pane::navigate_to_from_history(p, false, store, right_dir); } },
                        on_navigate: move |p| crate::ui::view::dir_pane::navigate_to(p, false, store, right_hist, right_dir),
                        lang,
                    }
                }

                // ── Filter bar ────────────────────────────────────────────
                FilterBar { lang, filter_open, filter_query, filter_hide_bin, filter_hide_eq }

                // ── Pane-root labels ──────────────────────────────────────
                div { class: "pane-root-bar",
                    div {
                        class: if focused_pane.read().is_left() { "pane-root-cell pane-focused" } else { "pane-root-cell" },
                        role: "heading",
                        aria_label: format!("{} — {}", t(lang, "Left pane"), short_name(&l_root_snap)),
                        onclick: move |_| focused_pane.set(FocusedPane::Left),
                        span { class: "root-label", "📁 " }
                        span { class: "root-name", title: "{l_root_snap.display()}", {short_name(&l_root_snap)} }
                    }
                    div {
                        class: if focused_pane.read().is_right() { "pane-root-cell pane-focused" } else { "pane-root-cell" },
                        role: "heading",
                        aria_label: format!("{} — {}", t(lang, "Right pane"), short_name(&r_root_snap)),
                        onclick: move |_| focused_pane.set(FocusedPane::Right),
                        span { class: "root-label", "📁 " }
                        span { class: "root-name", title: "{r_root_snap.display()}", {short_name(&r_root_snap)} }
                    }
                }

                // ── Tree ──────────────────────────────────────────────────
                if !compact_mode {
                    AlignedTree {
                        lang, aligned,
                        tree_l, tree_r, scans_l, scans_r,
                        left_dir, right_dir, left_hist, right_hist,
                        left_pick, right_pick, focused_pane,
                        digest_map, tier1_map, binary_cache, binary_enabled,
                    }
                } else {
                    CompactTree {
                        lang,
                        left_flat, right_flat,
                        l_root: l_root_snap.clone(), r_root: r_root_snap.clone(),
                        tree_l, tree_r, scans_l, scans_r,
                        left_dir, right_dir, left_hist, right_hist,
                        left_pick, right_pick,
                        digest_map, tier1_map, binary_cache, binary_enabled,
                        filter_query,
                    }
                }

                // ── Footer ────────────────────────────────────────────────
                ExplorerFooter { lang, left_pick, right_pick }

                // A tier-1 result is announced, not silently swapped in (RFC-080
                // §6). The app's only other polite region is the toast, which is
                // rendered only while a toast exists — a live region has to be
                // in the page before its text changes — so this minimal one is
                // added, always present and visually hidden.
                div {
                    class: "sr-only",
                    role: "status",
                    aria_live: "polite",
                    "{tier1_announcement}"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("fsk-ui-explorer-f74-{tag}-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    // F74 review 072: drives `classify_entry` - the real call site - against
    // two real directories that share a name and genuinely differ.
    // Confirmed to fail with the original `9f355c6` defect restored (see
    // the commit message/review request for the observed failure output).
    #[test]
    fn real_directories_with_the_same_name_and_differing_contents_are_not_compared() {
        let base = temp_dir("real-path-not-equal");
        let l_root = base.join("left");
        let r_root = base.join("right");
        std::fs::create_dir_all(l_root.join("sub")).unwrap();
        std::fs::create_dir_all(r_root.join("sub")).unwrap();
        std::fs::write(l_root.join("sub/a.txt"), "left content").unwrap();
        std::fs::write(
            r_root.join("sub/a.txt"),
            "right content, genuinely different",
        )
        .unwrap();

        let rel = std::path::PathBuf::from("sub");
        let result = classify_entry(&rel, true, &l_root, &r_root);

        assert_eq!(
            result,
            EntryClassification::Final(EqualityEvidence::Unknown),
            "a same-named directory whose contents differ must never be \
             classified Equal - see 9f355c6's original defect"
        );
    }

    // Handoff 007 §8.4: F74's behaviour must survive the refactor -
    // end-to-end, not just at `classify_entry`'s own boundary. Drives the
    // real directory-pair classification THROUGH `RowStatusKind::from_evidence`
    // and confirms the row a user would see is `NotCompared`, never a
    // claimed verdict. Falsify by temporarily reverting status.rs's
    // `Unknown => NotCompared` back to `Unknown => Computing` - the
    // assertion below then fails (confirmed; restored - see the review
    // request for the observed output, shared with §8.3's falsification
    // in status.rs since both exercise the same mapping).
    #[test]
    fn a_same_named_directory_pair_still_shows_not_compared_end_to_end() {
        let base = temp_dir("dir-pair-not-compared");
        let l_root = base.join("left");
        let r_root = base.join("right");
        std::fs::create_dir_all(l_root.join("sub")).unwrap();
        std::fs::create_dir_all(r_root.join("sub")).unwrap();

        let rel = std::path::PathBuf::from("sub");
        let evidence = match classify_entry(&rel, true, &l_root, &r_root) {
            EntryClassification::Final(e) => e,
            EntryClassification::NeedsDigest { .. } => {
                panic!("a directory pair must never need a digest")
            }
        };

        assert_eq!(
            forskscope_ui_logic::RowStatusKind::from_evidence(&evidence),
            forskscope_ui_logic::RowStatusKind::NotCompared,
            "F74: a same-named directory pair must render as not-compared, \
             never a claimed verdict"
        );
    }

    // Handoff 007 §8.1 / F76's first instance: a directory on one side and
    // a same-named FILE on the other is not "only on this side" - it exists
    // on both sides, just as different types. Falsify by restoring the old
    // `Unique`/one-sided classification (temporarily collapse the
    // `cp.is_file()` branch into the final `LeftOnly` arm) - confirmed to
    // fail; restored.
    #[test]
    fn a_directory_with_a_same_named_file_counterpart_is_a_type_mismatch_not_one_sided() {
        let base = temp_dir("type-mismatch-dir-file");
        let l_root = base.join("left");
        let r_root = base.join("right");
        std::fs::create_dir_all(l_root.join("x")).unwrap();
        std::fs::create_dir_all(&r_root).unwrap();
        std::fs::write(r_root.join("x"), "a file, not a directory").unwrap();

        let rel = std::path::PathBuf::from("x");
        let result = classify_entry(&rel, true, &l_root, &r_root);

        assert_eq!(
            result,
            EntryClassification::Final(EqualityEvidence::TypeMismatch {
                left: EntryType::Directory,
                right: EntryType::File,
            }),
            "a directory whose counterpart is a same-named file must be a \
             type mismatch, not falsely reported as present on one side only"
        );
    }

    // The mirror of the case above: a file whose counterpart is a
    // same-named directory. Same defect family, opposite sides.
    #[test]
    fn a_file_with_a_same_named_directory_counterpart_is_a_type_mismatch() {
        let base = temp_dir("type-mismatch-file-dir");
        let l_root = base.join("left");
        let r_root = base.join("right");
        std::fs::create_dir_all(&l_root).unwrap();
        std::fs::write(l_root.join("x"), "a file").unwrap();
        std::fs::create_dir_all(r_root.join("x")).unwrap();

        let rel = std::path::PathBuf::from("x");
        let result = classify_entry(&rel, false, &l_root, &r_root);

        assert_eq!(
            result,
            EntryClassification::Final(EqualityEvidence::TypeMismatch {
                left: EntryType::File,
                right: EntryType::Directory,
            })
        );
    }

    // A directory with no counterpart at all (nothing at that path on the
    // right) is genuinely one-sided - unchanged behaviour, renamed from the
    // pre-F76 `Unique` vocabulary to `LeftOnly`.
    #[test]
    fn a_directory_with_no_counterpart_at_all_is_left_only() {
        let base = temp_dir("dir-no-counterpart");
        let l_root = base.join("left");
        let r_root = base.join("right");
        std::fs::create_dir_all(l_root.join("only-here")).unwrap();
        std::fs::create_dir_all(&r_root).unwrap();

        let rel = std::path::PathBuf::from("only-here");
        let result = classify_entry(&rel, true, &l_root, &r_root);

        assert_eq!(
            result,
            EntryClassification::Final(EqualityEvidence::LeftOnly)
        );
    }

    // Handoff 007 §8.2 / F76's second instance: a failed digest comparison
    // establishes nothing and must not be reported as `Different`. Drives
    // `classify_digest_outcome` with a REAL `Err` produced by
    // `file_digest_equal_with_cancel` against a path that does not exist
    // (portable, no permission tricks needed - `fs::metadata` fails the
    // same way on every platform) rather than a hand-constructed error
    // value, so this is the real call site's output, not a synthetic
    // stand-in. Falsify by temporarily changing the `Err(e) => ...` arm to
    // `Err(_) => Some(EqualityEvidence::DigestDifferent)` - confirmed to
    // fail; restored.
    #[test]
    fn a_failed_comparison_is_reported_as_error_not_different() {
        let missing_a = std::path::Path::new("/nonexistent/fsk-explorer-f76/a");
        let missing_b = std::path::Path::new("/nonexistent/fsk-explorer-f76/b");
        let outcome = forskscope_core::dir::file_digest_equal_with_cancel(
            missing_a,
            missing_b,
            &forskscope_core::CancellationToken::new(),
        );
        assert!(outcome.is_err(), "a nonexistent path must fail to compare");

        let evidence = classify_digest_outcome(outcome);

        assert!(
            matches!(evidence, Some(EqualityEvidence::Error { .. })),
            "a failed comparison must be reported as Error, not a fabricated \
             verdict: {evidence:?}"
        );
        assert!(
            !matches!(evidence, Some(EqualityEvidence::DigestDifferent)),
            "must never collapse a read failure into Different"
        );
    }

    // F78: drives `apply_epoch_result` directly against a real `HashMap`
    // and a real `DigestEpoch` - no Dioxus runtime needed, since the
    // function takes both by value/reference rather than as `Signal`s.
    // Per handoff 004 §14 (still true here): the race that makes this
    // defect *reachable* is timing-dependent, but the guard itself is
    // not - it is a plain stamp/generation comparison, tested
    // deterministically by constructing the stale case directly rather
    // than trying to race a real root change against a real spawn.
    //
    // This replaces F77's `a_stale_generation_result_does_not_mutate_the_map`
    // / `a_current_generation_result_is_applied`, which drove the removed
    // `apply_digest_result` - review 074's falsification was re-run against
    // this converted call site by temporarily short-circuiting the
    // `if epoch.is_current(stamp)` check above to always insert, and
    // confirming the first assertion below then fails; restored.
    #[test]
    fn a_stale_stamp_result_does_not_mutate_the_map() {
        let mut epoch = DigestEpoch::new(4);
        let (stale_stamp, _, _) = epoch.begin_task();
        // The roots changed after `stale_stamp` was taken.
        epoch.restart();

        let mut map: HashMap<DigestKey, EqualityEvidence> = HashMap::new();
        let key = DigestKey::Common(PathBuf::from("a.txt"));

        apply_epoch_result(
            &mut map,
            key.clone(),
            EqualityEvidence::DigestEqual,
            stale_stamp,
            &epoch,
        );

        assert!(
            !map.contains_key(&key),
            "a stale-stamp result must not mutate the map"
        );
    }

    #[test]
    fn a_current_stamp_result_is_applied() {
        let epoch = DigestEpoch::new(4);
        let (stamp, _, _) = epoch.begin_task();

        let mut map: HashMap<DigestKey, EqualityEvidence> = HashMap::new();
        let key = DigestKey::Common(PathBuf::from("a.txt"));

        apply_epoch_result(
            &mut map,
            key.clone(),
            EqualityEvidence::DigestEqual,
            stamp,
            &epoch,
        );

        assert_eq!(map.get(&key), Some(&EqualityEvidence::DigestEqual));
    }

    // ── RFC-080 tier 1 ───────────────────────────────────────────────────────
    //
    // Real temporary trees, the real fast listing (`list_recursive_for_display_*`)
    // and the real fold (`dir_verdict`) — nothing here is a helper this change
    // introduces standing in for the thing it tests. Permission-dependent tests
    // are unix-only and skip, loudly, when `chmod` has no effect (running as root).

    use forskscope_core::CancellationToken;
    use forskscope_core::dir::RecursiveScan;

    fn tier1_dir(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("fsk-tier1-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("l")).unwrap();
        std::fs::create_dir_all(d.join("r")).unwrap();
        d
    }

    fn write(base: &Path, rel: &str, content: &str) {
        let p = base.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, content).unwrap();
    }

    fn walk(l: &Path, r: &Path) -> RecursiveScan {
        list_recursive_for_display_with_rules(
            l,
            r,
            &CancellationToken::new(),
            &IgnoreRules::default(),
        )
    }

    fn verdict(l: &Path, r: &Path) -> DirVerdict {
        dir_verdict(&walk(l, r))
    }

    /// Criterion 1: differing only in a file's **size** is `Different`.
    #[test]
    fn a_pair_differing_only_in_a_files_size_is_different() {
        let b = tier1_dir("size");
        write(&b.join("l"), "d/a.txt", "one");
        write(&b.join("r"), "d/a.txt", "three");
        write(&b.join("l"), "same.txt", "s");
        write(&b.join("r"), "same.txt", "s");
        assert_eq!(verdict(&b.join("l"), &b.join("r")), DirVerdict::Different);
        let _ = std::fs::remove_dir_all(&b);
    }

    /// Criteria 2 and 3 (tier-1 halves), and the proof that **no contents are
    /// read**: the trees hold a file whose contents differ at identical size, and
    /// the verdict does not move. Were contents read, this would be `Different`.
    #[test]
    fn contents_that_differ_at_identical_size_are_a_metadata_match_never_different() {
        let b = tier1_dir("samesize");
        write(&b.join("l"), "a.txt", "abcdef");
        write(&b.join("r"), "a.txt", "abcdeX"); // one character, same size
        write(&b.join("l"), "sub/b.txt", "same");
        write(&b.join("r"), "sub/b.txt", "same");
        let v = verdict(&b.join("l"), &b.join("r"));
        assert_eq!(v, DirVerdict::MetadataMatch);
        // ... and it is not equality, on any of the ways a row could read it.
        let evidence = verdict_evidence(v);
        assert!(!evidence.is_equal());
        let kind = forskscope_ui_logic::RowStatusKind::from_evidence(&evidence);
        assert_eq!(kind, forskscope_ui_logic::RowStatusKind::MetadataMatch);
        assert_ne!(kind, forskscope_ui_logic::RowStatusKind::Equal);
        let _ = std::fs::remove_dir_all(&b);
    }

    /// Criterion 3: an identical pair is a tier-1 match, **never `Identical`** from
    /// tier 1 alone.
    #[test]
    fn an_identical_pair_is_a_metadata_match_never_identical() {
        let b = tier1_dir("identical");
        for side in ["l", "r"] {
            write(&b.join(side), "a.txt", "same");
            write(&b.join(side), "d/b.txt", "same too");
        }
        let v = verdict(&b.join("l"), &b.join("r"));
        assert_eq!(v, DirVerdict::MetadataMatch);
        assert!(!verdict_evidence(v).is_equal());
        let _ = std::fs::remove_dir_all(&b);
    }

    /// The "no reads" proof, stronger: a file with **no read permission** and the
    /// same size on both sides. Reading its contents would fail; the metadata-only
    /// listing does not, so the verdict is still a match.
    #[cfg(unix)]
    #[test]
    fn an_unreadable_file_is_never_opened_by_tier_1() {
        use std::os::unix::fs::PermissionsExt;
        let b = tier1_dir("noread");
        write(&b.join("l"), "secret.bin", "0123456789");
        write(&b.join("r"), "secret.bin", "abcdefghij");
        let f = b.join("l/secret.bin");
        let _ = std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o000));
        if std::fs::read(&f).is_ok() {
            eprintln!("skipping an_unreadable_file_is_never_opened: chmod had no effect (root?)");
            return;
        }
        let v = verdict(&b.join("l"), &b.join("r"));
        let _ = std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o644));
        let _ = std::fs::remove_dir_all(&b);
        assert_eq!(v, DirVerdict::MetadataMatch, "a read would have failed");
    }

    /// Criterion 4: an unreadable **subdirectory** and an unreadable **root** are
    /// `Unknown` — neither a verdict nor a tier-1 match.
    #[cfg(unix)]
    #[test]
    fn an_unreadable_subdirectory_and_an_unreadable_root_are_unknown() {
        use std::os::unix::fs::PermissionsExt;
        let b = tier1_dir("unreadable");
        for side in ["l", "r"] {
            write(&b.join(side), "ok.txt", "same");
            write(&b.join(side), "locked/x.txt", "same");
        }
        let locked = b.join("l/locked");
        let _ = std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000));
        if std::fs::read_dir(&locked).is_ok() {
            eprintln!("skipping the unreadable test: chmod had no effect (root?)");
            return;
        }
        let sub = verdict(&b.join("l"), &b.join("r"));
        let _ = std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755));

        let root = b.join("r");
        let _ = std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o000));
        let root_verdict = if std::fs::read_dir(&root).is_err() {
            Some(verdict(&b.join("l"), &root))
        } else {
            None
        };
        let _ = std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755));
        let _ = std::fs::remove_dir_all(&b);

        assert_eq!(sub, DirVerdict::Unknown, "an unreadable subdirectory");
        assert_eq!(
            root_verdict,
            Some(DirVerdict::Unknown),
            "an unreadable root"
        );
    }

    /// Criterion 8: a symlink on either side is `Unknown`, not a verdict.
    #[cfg(unix)]
    #[test]
    fn a_symlink_on_either_side_is_unknown() {
        let b = tier1_dir("symlink");
        write(&b.join("l"), "a.txt", "same");
        write(&b.join("r"), "a.txt", "same");
        write(&b.join("l"), "target.txt", "t");
        write(&b.join("r"), "target.txt", "t");
        std::os::unix::fs::symlink(b.join("l/target.txt"), b.join("l/link")).unwrap();
        write(&b.join("r"), "link", "a regular file of the same name");
        assert_eq!(verdict(&b.join("l"), &b.join("r")), DirVerdict::Unknown);
        assert_eq!(verdict(&b.join("r"), &b.join("l")), DirVerdict::Unknown);
        let _ = std::fs::remove_dir_all(&b);
    }

    /// Criterion 5, the applying half: a walk whose token was cancelled returns a
    /// **partial** scan, and applying it would show a false result. The outcome is
    /// `None` when cancelled or superseded, however the scan reads. Falsify by
    /// dropping the `cancelled` test in `tier1_outcome`: the first assertion fails.
    #[test]
    fn a_cancelled_or_superseded_walk_applies_nothing() {
        let b = tier1_dir("cancel");
        write(&b.join("l"), "a.txt", "same");
        write(&b.join("r"), "a.txt", "same");
        // A walk cancelled up front returns whatever it had — here, an empty scan,
        // which would fold to a match.
        let token = CancellationToken::new();
        token.cancel();
        let partial = list_recursive_for_display_with_rules(
            &b.join("l"),
            &b.join("r"),
            &token,
            &IgnoreRules::default(),
        );
        assert_eq!(
            dir_verdict(&partial),
            DirVerdict::MetadataMatch,
            "the premise"
        );
        assert_eq!(tier1_outcome(&partial, token.is_cancelled(), true), None);
        assert_eq!(tier1_outcome(&partial, false, false), None, "superseded");
        assert!(tier1_outcome(&partial, false, true).is_some());
        let _ = std::fs::remove_dir_all(&b);
    }

    /// Criterion 5, the cancelling half — the token is *observed*, not assumed:
    /// the protocol the trigger effect follows (a selection that moves off the
    /// walked row returns `CancelWalk`, and the view `restart()`s the epoch) really
    /// cancels the token the walk was handed and outdates its stamp.
    #[test]
    fn the_selection_moving_cancels_the_token_the_walk_was_given() {
        let mut trigger: Tier1Trigger<PathBuf> = Tier1Trigger::default();
        let mut epoch = DigestEpoch::new(1);
        let t0 = std::time::Duration::ZERO;
        trigger.selected(t0, Some(PathBuf::from("a")));
        assert_eq!(
            trigger.tick(forskscope_ui_logic::TIER1_DEBOUNCE),
            Some(PathBuf::from("a"))
        );
        let (stamp, token, _) = epoch.begin_task();
        assert!(!token.is_cancelled());

        if let Some(Tier1Action::CancelWalk) = trigger.selected(t0, Some(PathBuf::from("b"))) {
            epoch.restart();
        }
        assert!(
            token.is_cancelled(),
            "the walk's token must observe the cancellation"
        );
        assert!(!epoch.is_current(stamp), "and its result must be outdated");
    }

    // The candidate: only an unexamined same-named directory pair that has no
    // result yet, and the in-flight row keeps being one (or the effect would
    // cancel the walk it started); a cached "no verdict" is never walked again.
    #[test]
    fn only_an_unexamined_directory_pair_without_a_result_is_a_candidate() {
        let (l, r) = (PathBuf::from("/l"), PathBuf::from("/r"));
        let dir = PathBuf::from("d");
        let key = || Tier1Key {
            left_root: l.clone(),
            right_root: r.clone(),
            rel: dir.clone(),
        };
        let digest = |e: EqualityEvidence| {
            let mut m = HashMap::new();
            m.insert(DigestKey::Common(dir.clone()), e);
            m
        };
        let unexamined = digest(EqualityEvidence::Unknown);
        let sel = [dir.clone()];

        assert_eq!(
            tier1_candidate(&sel, &unexamined, &HashMap::new(), &l, &r),
            Some(dir.clone())
        );
        // Not an unexamined directory pair: a file digest, a type mismatch, nothing.
        for other in [
            EqualityEvidence::DigestEqual,
            EqualityEvidence::MetadataOnly,
            EqualityEvidence::LeftOnly,
        ] {
            assert_eq!(
                tier1_candidate(&sel, &digest(other), &HashMap::new(), &l, &r),
                None
            );
        }
        assert_eq!(
            tier1_candidate(&sel, &HashMap::new(), &HashMap::new(), &l, &r),
            None
        );
        // The in-flight marker keeps it a candidate; any result, including "no
        // verdict", ends it.
        let mut cache = HashMap::new();
        cache.insert(key(), EqualityEvidence::MetadataOnly);
        assert_eq!(
            tier1_candidate(&sel, &unexamined, &cache, &l, &r),
            Some(dir.clone())
        );
        for done in [
            EqualityEvidence::TreeDifferent,
            EqualityEvidence::MetadataMatch,
            EqualityEvidence::Unknown,
        ] {
            cache.insert(key(), done);
            assert_eq!(tier1_candidate(&sel, &unexamined, &cache, &l, &r), None);
        }
    }

    /// A result overrides only the unexamined-directory state, and only under the
    /// roots it was measured for.
    #[test]
    fn a_tier_1_result_overrides_only_the_unexamined_directory_state_under_its_roots() {
        let (l, r) = (PathBuf::from("/l"), PathBuf::from("/r"));
        let rel = PathBuf::from("d");
        let mut cache = HashMap::new();
        cache.insert(
            Tier1Key {
                left_root: l.clone(),
                right_root: r.clone(),
                rel: rel.clone(),
            },
            EqualityEvidence::MetadataMatch,
        );
        let unknown = Some(EqualityEvidence::Unknown);
        assert_eq!(
            row_evidence(unknown.clone(), &cache, &l, &r, &rel),
            Some(EqualityEvidence::MetadataMatch)
        );
        // Other roots (navigation): not shown.
        assert_eq!(
            row_evidence(unknown, &cache, &PathBuf::from("/elsewhere"), &r, &rel),
            Some(EqualityEvidence::Unknown)
        );
        // A file's digest verdict, or a one-sided row, is never overridden.
        for base in [
            Some(EqualityEvidence::DigestEqual),
            Some(EqualityEvidence::LeftOnly),
            None,
        ] {
            assert_eq!(row_evidence(base.clone(), &cache, &l, &r, &rel), base);
        }
    }
}
