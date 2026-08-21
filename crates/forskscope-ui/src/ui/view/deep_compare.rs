//! Recursive directory comparison with incremental digest progress (RFC-037, RFC-038, RFC-040).
//!
//! Phase 1 (fast): `list_recursive_for_display_with_cancel` walks both trees;
//! common files get `RecStatus::Computing`.  Phase 2: per-file digest tasks
//! update entries in-place so the table refreshes as results arrive. Both
//! phases share one `DigestEpoch` (F78) — a root change cancels and
//! supersedes whichever phase is in flight.

use std::path::{Path, PathBuf};

use dioxus::prelude::*;

use forskscope_core::dir::{
    DigestOutcome, RecEntry, RecStatus, file_digest_equal_with_cancel,
    list_recursive_for_display_with_cancel,
};

use crate::i18n::t;
use crate::state::{DirOp, Lang, Modal, Store, open_compare};
use crate::ui::view::digest_epoch::{DigestEpoch, EpochStamp};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum DeepFilter {
    #[default]
    Different,
    All,
    Equal,
}

#[component]
pub fn DeepCompareView(left_root: PathBuf, right_root: PathBuf, lang: Lang) -> Element {
    // Clone once outside all closures so the props aren't moved piecemeal.
    let lr = left_root.clone();
    let rr = right_root.clone();

    let entries: Signal<Vec<RecEntry>> = use_signal(Vec::new);
    #[allow(unused_mut)]
    let mut scanning: Signal<bool> = use_signal(|| true);
    #[allow(unused_mut)]
    let mut computed: Signal<usize> = use_signal(|| 0);
    #[allow(unused_mut)]
    let mut total_common: Signal<usize> = use_signal(|| 0);
    let mut filter: Signal<DeepFilter> = use_signal(DeepFilter::default);
    // F78: tracks the roots this view last (re)started a scan for, so a
    // `use_effect` re-run that isn't an actual root change (this component
    // has no reactive signal dependencies of its own) doesn't spawn a
    // second, redundant scan on top of one already in flight.
    let mut scan_roots: Signal<Option<(PathBuf, PathBuf)>> = use_signal(|| None);
    // F78: owns the generation guard, the cancellation token shared by
    // both phases, and the concurrency bound this view already had -
    // replacing the ad hoc per-effect-run `Semaphore` below with one that
    // survives `restart()`, matching `explorer.rs`.
    let mut digest_epoch: Signal<DigestEpoch> =
        use_signal(|| DigestEpoch::new(forskscope_core::DIGEST_CONCURRENCY_LIMIT));

    use_effect(move || {
        let lr1 = lr.clone();
        let rr1 = rr.clone();
        let lr2 = lr.clone(); // for phase-2 absolute-path construction
        let rr2 = rr.clone();
        let mut ent = entries;
        let mut scan = scanning;
        let mut tc = total_common;
        let mut comp = computed;

        let roots_changed = *scan_roots.read() != Some((lr1.clone(), rr1.clone()));
        if !roots_changed {
            return;
        }
        scan_roots.set(Some((lr1.clone(), rr1.clone())));
        // Cancel outstanding work under the old roots (both phases share
        // this token) before anything else observes the new ones.
        digest_epoch.write().restart();
        scan.set(true);
        comp.set(0);
        tc.set(0);

        // Phase 1: fast listing (no I/O-heavy digests), cancellable so a
        // root change on a large tree doesn't have to wait it out.
        let (_, phase1_token, _) = digest_epoch.read().begin_task();

        spawn(async move {
            let list_token = phase1_token.clone();
            let initial = tokio::task::spawn_blocking(move || {
                list_recursive_for_display_with_cancel(&lr1, &rr1, &list_token)
            })
            .await
            .unwrap_or_default();

            if phase1_token.is_cancelled() {
                // Superseded before phase 1 finished - the run that
                // superseded us already reset `scanning`/`entries` for its
                // own roots; leave them alone rather than showing this
                // run's partial listing.
                return;
            }

            // Build the list of (rel, abs_left, abs_right) for common pairs.
            let pairs: Vec<(PathBuf, PathBuf, PathBuf)> = initial
                .iter()
                .filter(|e| e.status == RecStatus::Computing)
                .map(|e| {
                    (
                        e.rel_path.clone(),
                        lr2.join(&e.rel_path),
                        rr2.join(&e.rel_path),
                    )
                })
                .collect();

            tc.set(pairs.len());
            ent.set(initial);
            scan.set(false);

            // Phase 2: digest tasks, bounded by the epoch's semaphore
            // (DIGEST_CONCURRENCY_LIMIT concurrent blocking operations) to
            // avoid overwhelming the thread pool on large directory trees.
            for (rel, lp, rp) in pairs {
                let mut e2 = ent;
                let mut cp2 = comp;
                let (stamp, token, sem) = digest_epoch.read().begin_task();
                let epoch_signal = digest_epoch;
                spawn(async move {
                    // Acquired here, inside the task, after the
                    // `begin_task()` read guard above has already been
                    // dropped - never held across an await.
                    let _permit = sem.acquire_owned().await;
                    let outcome = tokio::task::spawn_blocking(move || {
                        file_digest_equal_with_cancel(&lp, &rp, &token)
                    })
                    .await
                    // A join error (the blocking task panicked) has no
                    // real verdict either - same fallback the old code
                    // used for any failure.
                    .unwrap_or(Ok(DigestOutcome::Different));
                    let status = match outcome {
                        Ok(DigestOutcome::Equal) => RecStatus::Equal,
                        Ok(DigestOutcome::Different) => RecStatus::Changed,
                        Err(_) => RecStatus::Changed,
                        Ok(DigestOutcome::Cancelled) => {
                            // Established nothing - the root change that
                            // cancelled this already superseded this run.
                            return;
                        }
                    };
                    apply_epoch_result(&mut e2.write(), &rel, status, stamp, &epoch_signal.read());
                    let next = *cp2.read() + 1;
                    cp2.set(next);
                });
            }
        });
    });

    let f = *filter.read();
    let snap = entries.read();
    let changed = snap
        .iter()
        .filter(|e| e.status == RecStatus::Changed)
        .count();
    let equal = snap.iter().filter(|e| e.status == RecStatus::Equal).count();
    let left_only = snap
        .iter()
        .filter(|e| e.status == RecStatus::LeftOnly)
        .count();
    let right_only = snap
        .iter()
        .filter(|e| e.status == RecStatus::RightOnly)
        .count();
    let computing = snap
        .iter()
        .filter(|e| e.status == RecStatus::Computing)
        .count();
    let done = *computed.read();
    let tc = *total_common.read();
    let is_scan = *scanning.read();
    let in_flight = !is_scan && tc > 0 && done < tc;
    let visible: Vec<RecEntry> = snap
        .iter()
        .filter(|e| match f {
            DeepFilter::Different => e.status != RecStatus::Equal,
            DeepFilter::All => true,
            DeepFilter::Equal => e.status == RecStatus::Equal,
        })
        .cloned()
        .collect();
    drop(snap);

    rsx! {
        div { class: "deep-compare",
            div { class: "deep-roots",
                span { class: "deep-root-label", {t(lang, "Left")}": " }
                span { class: "deep-root-path", {left_root.display().to_string()} }
                span { class: "deep-root-sep", " ↔ " }
                span { class: "deep-root-label", {t(lang, "Right")}": " }
                span { class: "deep-root-path", {right_root.display().to_string()} }
            }
            div { class: "deep-compare-toolbar",
                span { class: "deep-label", {t(lang, "Deep compare")} }
                button { class: if f==DeepFilter::Different {"filter-btn active"} else {"filter-btn"},
                    onclick: move |_| filter.set(DeepFilter::Different), {t(lang, "Different")} }
                button { class: if f==DeepFilter::All {"filter-btn active"} else {"filter-btn"},
                    onclick: move |_| filter.set(DeepFilter::All), {t(lang, "All")} }
                button { class: if f==DeepFilter::Equal {"filter-btn active"} else {"filter-btn"},
                    onclick: move |_| filter.set(DeepFilter::Equal), {t(lang, "Equal only")} }
                span { class: "spacer" }
                BatchCopyButtons { entries, left_root: left_root.clone(), right_root: right_root.clone() }
            }
            if is_scan {
                div { class: "deep-scanning", {t(lang, "Scanning…")} }
            } else {
                div { class: "deep-stats",
                    {format!("{} {} · {} {} · {} {} · {} {}",
                        changed,    t(lang, "different"),
                        equal,      t(lang, "equal"),
                        left_only,  t(lang, "left only"),
                        right_only, t(lang, "right only"))}
                    if computing > 0 || in_flight {
                        span { class: "deep-progress",
                            {format!(" · {} {}/{}…", t(lang, "checking"), done, tc)}
                        }
                    }
                }
                div { class: "deep-table",
                    for entry in visible {
                        DeepRow { entry, lang, left_root: left_root.clone(), right_root: right_root.clone() }
                    }
                }
            }
        }
    }
}

// F73/F68 (review 068 §5, handoff §2): this component used to substitute
// `store.settings.read().last_left_dir`/`last_right_dir` (Explorer's
// remembered pane directory) for the actual compare roots, because it was
// never given them - unlike `BatchCopyButtons`, which receives `left_root`/
// `right_root` as props one call site up and was always correct. That
// substitution both hid the per-row buttons when `remember_explorer_dirs`
// was off (F68) and, when the substitute happened to differ from the real
// roots, sent a copy or Compare silently to the wrong location (F73). Taking
// `left_root`/`right_root` as props here removes the substitution and the
// gate it required - the roots are now always present, so there is nothing
// left to gate on.
#[component]
fn DeepRow(entry: RecEntry, lang: Lang, left_root: PathBuf, right_root: PathBuf) -> Element {
    let mut store = use_context::<Store>();
    let (icon, cls) = match entry.status {
        RecStatus::Changed => ("⚠", "status-changed"),
        RecStatus::LeftOnly => ("←", "status-only"),
        RecStatus::RightOnly => ("→", "status-only"),
        RecStatus::Equal => ("✓", "status-equal"),
        RecStatus::Computing => ("⊙", "status-cmp"),
        RecStatus::Symlink => ("↗", "status-symlink"),
    };
    let path_str = entry.rel_path.display().to_string();
    let can_cmp = !matches!(
        entry.status,
        RecStatus::Equal | RecStatus::Computing | RecStatus::Symlink
    );
    // Copy direction: LeftOnly/Changed → copy left→right; RightOnly → copy right→left.
    // Changed entries show both directions (RFC-062 B3). F68: this is now
    // the *entire* visibility condition for the per-row copy buttons - no
    // `remember_explorer_dirs`/settings gate remains for it to interact
    // with, unlike the old `has_left_root && has_right_root && ...` form.
    let copy_left_to_right: bool = can_copy_left_to_right(entry.status);
    let copy_right_to_left: bool = can_copy_right_to_left(entry.status);
    let e2 = entry.clone();
    let lr_cmp = left_root.clone();
    let rr_cmp = right_root.clone();
    rsx! {
        div { class: "deep-row",
            span { class: "dir-status {cls}", "{icon}" }
            span { class: "deep-path", "{path_str}" }
            span { class: "dir-size", {size_label(&entry)} }
            if can_cmp {
                button { class: "deep-compare-btn",
                    onclick: move |_| {
                        let (lp, rp) = left_then_right(&e2, &lr_cmp, &rr_cmp);
                        open_compare(&mut store, lp, rp);
                    },
                    {t(lang, "Compare")}
                }
            }
            if copy_left_to_right {
                {
                    let entry3 = entry.clone();
                    let l = left_root.clone();
                    let r = right_root.clone();
                    rsx! {
                        button {
                            class: "deep-compare-btn",
                            title: t(lang, "Copy to right"),
                            onclick: move |_| {
                                let (src, dst) = left_then_right(&entry3, &l, &r);
                                store.modal.set(Modal::ConfirmDirOp(DirOp { src, dst, label: String::new() }));
                            },
                            {t(lang, "Copy to right")}
                        }
                    }
                }
            }
            if copy_right_to_left {
                {
                    let entry4 = entry.clone();
                    let l = left_root.clone();
                    let r = right_root.clone();
                    rsx! {
                        button {
                            class: "deep-compare-btn",
                            title: t(lang, "Copy to left"),
                            onclick: move |_| {
                                let (src, dst) = right_then_left(&entry4, &l, &r);
                                store.modal.set(Modal::ConfirmDirOp(DirOp { src, dst, label: String::new() }));
                            },
                            {t(lang, "Copy to left")}
                        }
                    }
                }
            }
        }
    }
}

/// F78: applies a completed phase-2 digest result to `entries` only if
/// `stamp` is still current for `epoch`. Locating the entry by `rel_path`
/// happens after `ent.set(initial)` may already have replaced the whole
/// list with a later run's listing - without this guard, a stale result
/// would either land on an unrelated entry that now occupies the same
/// `rel_path`, or (once the map is keyed correctly) silently resurrect a
/// verdict from superseded roots. `can_copy_left_to_right`/
/// `BatchCopyButtons`/`can_cmp` all key off `entry.status`, so a stale
/// `Equal` here is a real data-safety defect (F78 §2), not a display glitch.
/// Takes `entries` and `epoch` directly (not `Signal`s) so a test can drive
/// it without a Dioxus runtime, mirroring `explorer.rs`'s
/// `apply_epoch_result`.
fn apply_epoch_result(
    entries: &mut [RecEntry],
    rel: &Path,
    status: RecStatus,
    stamp: EpochStamp,
    epoch: &DigestEpoch,
) {
    if !epoch.is_current(stamp) {
        return;
    }
    if let Some(entry) = entries.iter_mut().find(|e| e.rel_path == rel) {
        entry.status = status;
    }
}

/// `(entry`'s path under `left_root`, under `right_root)` - used for both
/// the Compare button and "Copy to right" (whose destination is the second
/// element). Takes the compare roots as plain parameters, never reads
/// `store.settings` - the F73 defect was exactly that the old code read
/// `store.settings.read().last_left_dir`/`last_right_dir` (Explorer's
/// remembered pane directory) here instead, which can silently differ from
/// the roots actually being compared.
fn left_then_right(entry: &RecEntry, left_root: &Path, right_root: &Path) -> (PathBuf, PathBuf) {
    (
        left_root.join(&entry.rel_path),
        right_root.join(&entry.rel_path),
    )
}

/// The swapped pair, for "Copy to left": source under `right_root`,
/// destination under `left_root`.
fn right_then_left(entry: &RecEntry, left_root: &Path, right_root: &Path) -> (PathBuf, PathBuf) {
    (
        right_root.join(&entry.rel_path),
        left_root.join(&entry.rel_path),
    )
}

/// F68: whether the per-row "Copy to right"/"Copy to left" buttons appear
/// depends on `entry.status` alone - these take no `Store`/`AppSettings`
/// parameter, so there is nothing left for `remember_explorer_dirs` (or any
/// other setting) to gate. The old code additionally required
/// `store.settings.read().last_left_dir`/`last_right_dir` to be `Some`,
/// which is exactly what `remember_explorer_dirs` off left `None`.
fn can_copy_left_to_right(status: RecStatus) -> bool {
    matches!(status, RecStatus::Changed | RecStatus::LeftOnly)
}

fn can_copy_right_to_left(status: RecStatus) -> bool {
    matches!(status, RecStatus::Changed | RecStatus::RightOnly)
}

fn size_label(e: &RecEntry) -> String {
    match (e.left_size, e.right_size) {
        (Some(l), Some(r)) if l != r => format!("{} → {}", fmt(l), fmt(r)),
        (Some(s), _) | (_, Some(s)) => fmt(s),
        _ => String::new(),
    }
}
fn fmt(n: u64) -> String {
    if n < 1024 {
        format!("{n}B")
    } else if n < 1_048_576 {
        format!("{:.1}KB", n as f64 / 1024.0)
    } else {
        format!("{:.1}MB", n as f64 / 1_048_576.0)
    }
}

// ─── Batch copy buttons ───────────────────────────────────────────────────────

#[component]
fn BatchCopyButtons(
    entries: Signal<Vec<RecEntry>>,
    left_root: PathBuf,
    right_root: PathBuf,
) -> Element {
    let mut store = use_context::<Store>();
    let lang = store.lang();
    let snap = entries.read();
    let has_changes = snap
        .iter()
        .any(|e| !matches!(e.status, RecStatus::Equal | RecStatus::Computing));
    if !has_changes {
        return rsx! {};
    }

    let lr = left_root.clone();
    let rr = right_root.clone();
    let lr2 = left_root;
    let rr2 = right_root;

    // "Copy all →" = copy left-only and changed files to the right tree
    let to_right: Vec<(PathBuf, PathBuf)> = snap
        .iter()
        .filter(|e| matches!(e.status, RecStatus::Changed | RecStatus::LeftOnly))
        .map(|e| (lr.join(&e.rel_path), rr.join(&e.rel_path)))
        .collect();
    // "Copy all ←" = copy right-only and changed files to the left tree
    let to_left: Vec<(PathBuf, PathBuf)> = snap
        .iter()
        .filter(|e| matches!(e.status, RecStatus::Changed | RecStatus::RightOnly))
        .map(|e| (rr2.join(&e.rel_path), lr2.join(&e.rel_path)))
        .collect();
    drop(snap);

    let tr_count = to_right.len();
    let tl_count = to_left.len();
    rsx! {
        if tr_count > 0 {
            button {
                class: "filter-btn",
                title: format!("{} {tr_count} {}", t(lang, "Copy to right"), t(lang, "files")),
                onclick: move |_| {
                    use crate::state::{BatchCopySpec, Modal};
                    store.modal.set(Modal::ConfirmBatchCopy(BatchCopySpec {
                        items: to_right.clone(),
                        label: format!("{} {tr_count} {}", t(lang, "Copy to right"), t(lang, "files")),
                    }));
                },
                {format!("{} {tr_count}", t(lang, "Copy to right"))}
            }
        }
        if tl_count > 0 {
            button {
                class: "filter-btn",
                title: format!("{} {tl_count} {}", t(lang, "Copy to left"), t(lang, "files")),
                onclick: move |_| {
                    use crate::state::{BatchCopySpec, Modal};
                    store.modal.set(Modal::ConfirmBatchCopy(BatchCopySpec {
                        items: to_left.clone(),
                        label: format!("{} {tl_count} {}", t(lang, "Copy to left"), t(lang, "files")),
                    }));
                },
                {format!("{} {tl_count}", t(lang, "Copy to left"))}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(rel: &str) -> RecEntry {
        RecEntry {
            rel_path: PathBuf::from(rel),
            status: RecStatus::Changed,
            left_size: None,
            right_size: None,
        }
    }

    fn entry_with_status(rel: &str, status: RecStatus) -> RecEntry {
        RecEntry {
            rel_path: PathBuf::from(rel),
            status,
            left_size: None,
            right_size: None,
        }
    }

    // F73: a per-row copy or Compare must land under the actual compare
    // roots, never under Explorer's remembered pane directory -
    // `last_left_dir`/`last_right_dir` can silently differ from these, and
    // the old code read those instead of the roots it was never given.
    // These roots are deliberately distinct from any plausible
    // `last_left_dir`/`last_right_dir` value a test could invent, so a
    // regression that substituted a different directory back in would show
    // up as the wrong assertion here, not a coincidentally-matching one.

    #[test]
    fn left_then_right_joins_the_relative_path_onto_each_actual_root() {
        let e = entry("sub/file.txt");
        let left_root = Path::new("/compare/left-root");
        let right_root = Path::new("/compare/right-root");

        let (src, dst) = left_then_right(&e, left_root, right_root);

        assert_eq!(src, PathBuf::from("/compare/left-root/sub/file.txt"));
        assert_eq!(dst, PathBuf::from("/compare/right-root/sub/file.txt"));
    }

    #[test]
    fn right_then_left_swaps_source_and_destination() {
        let e = entry("sub/file.txt");
        let left_root = Path::new("/compare/left-root");
        let right_root = Path::new("/compare/right-root");

        let (src, dst) = right_then_left(&e, left_root, right_root);

        assert_eq!(src, PathBuf::from("/compare/right-root/sub/file.txt"));
        assert_eq!(dst, PathBuf::from("/compare/left-root/sub/file.txt"));
    }

    #[test]
    fn copy_targets_do_not_depend_on_any_remembered_explorer_directory() {
        // The whole point of F73's fix: these functions take the compare
        // roots as plain parameters and have no access to `store.settings`
        // at all, so there is no `last_left_dir`/`last_right_dir` for them
        // to substitute even by accident. A `last_right_dir` that differs
        // from `right_root` (the exact condition that produced F73's
        // silent wrong-location write) cannot influence this result,
        // because it is never in scope here.
        let e = entry("changed.txt");
        let compare_root_left = Path::new("/tmp/root-a");
        let compare_root_right = Path::new("/tmp/root-b");
        let remembered_explorer_dir = Path::new("/home/user"); // never passed in

        let (_src, dst) = left_then_right(&e, compare_root_left, compare_root_right);

        assert_eq!(dst, PathBuf::from("/tmp/root-b/changed.txt"));
        assert_ne!(dst, remembered_explorer_dir.join("changed.txt"));
    }

    // F68: with the old `has_left_root && has_right_root && ...` gate, an
    // entry that should show a copy button (Changed/LeftOnly/RightOnly)
    // would not, once `remember_explorer_dirs` left `last_left_dir`/
    // `last_right_dir` as `None`. These functions take only `RecStatus`,
    // so there is no settings state left to test against - a `Some`/`None`
    // `last_left_dir` simply cannot appear in their signature. Confirmed by
    // status alone, matching every status this component's caller can
    // produce.

    #[test]
    fn copy_to_right_is_available_for_left_only_and_changed_entries() {
        assert!(can_copy_left_to_right(RecStatus::LeftOnly));
        assert!(can_copy_left_to_right(RecStatus::Changed));
        assert!(!can_copy_left_to_right(RecStatus::RightOnly));
        assert!(!can_copy_left_to_right(RecStatus::Equal));
        assert!(!can_copy_left_to_right(RecStatus::Computing));
        assert!(!can_copy_left_to_right(RecStatus::Symlink));
    }

    #[test]
    fn copy_to_left_is_available_for_right_only_and_changed_entries() {
        assert!(can_copy_right_to_left(RecStatus::RightOnly));
        assert!(can_copy_right_to_left(RecStatus::Changed));
        assert!(!can_copy_right_to_left(RecStatus::LeftOnly));
        assert!(!can_copy_right_to_left(RecStatus::Equal));
        assert!(!can_copy_right_to_left(RecStatus::Computing));
        assert!(!can_copy_right_to_left(RecStatus::Symlink));
    }

    // F78 §8.1: drives `apply_epoch_result` directly against a real
    // `Vec<RecEntry>` and a real `DigestEpoch` - no Dioxus runtime needed.
    // This is the guard `deep_compare.rs` lacked entirely before F78: a
    // phase-2 result located its entry by `rel_path` after `ent.set(initial)`
    // may already have replaced the whole list with a later run's listing,
    // and nothing stopped a superseded verdict from landing on whatever now
    // occupies that path. Given `can_copy_left_to_right`/`BatchCopyButtons`/
    // `can_cmp` all key off `entry.status` (F78 §2), a stale `Equal` here
    // silently drops a genuinely-differing file from "copy all changed".
    //
    // Falsified by temporarily removing the `if !epoch.is_current(stamp) {
    // return; }` guard in `apply_epoch_result` - the first assertion below
    // then fails (the stale `Equal` lands). Restored.
    #[test]
    fn a_stale_stamp_result_does_not_mutate_deep_compares_entries() {
        let mut epoch = DigestEpoch::new(4);
        let (stale_stamp, _, _) = epoch.begin_task();
        // The roots changed after `stale_stamp` was taken.
        epoch.restart();
        let (current_stamp, _, _) = epoch.begin_task();

        let mut entries = vec![entry_with_status("a.txt", RecStatus::Computing)];

        apply_epoch_result(
            &mut entries,
            Path::new("a.txt"),
            RecStatus::Equal,
            stale_stamp,
            &epoch,
        );
        assert_eq!(
            entries[0].status,
            RecStatus::Computing,
            "a stale-stamp result must not mutate the entry"
        );

        apply_epoch_result(
            &mut entries,
            Path::new("a.txt"),
            RecStatus::Equal,
            current_stamp,
            &epoch,
        );
        assert_eq!(entries[0].status, RecStatus::Equal);
    }
}
