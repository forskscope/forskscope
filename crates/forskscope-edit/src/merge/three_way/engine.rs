//! Line-oriented three-way merge engine (RFC-033 §"Algorithm Boundary").
//!
//! This is the conservative initial implementation the RFC anticipates: a
//! classic diff3 over lines. It aligns `left` and `right` against `base`
//! with LCS matching, then walks all three in lockstep, emitting stable
//! regions (all sides agree) and change regions. Each change region is
//! classified into a clean auto-merge or a conflict; no risky automatic
//! decision is ever made for two-sided divergent edits.
//!
//! The engine holds no `similar` types in its output — it returns plain
//! [`MergeRegion`]s consumed by the session layer.

use similar::{Algorithm, TextDiffConfig};

use super::line::{MergeLine, key};

/// How a change region was reconciled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionKind {
    /// All three sides are identical here.
    Stable,
    /// Only the left side changed relative to base; left content is taken.
    LeftChanged,
    /// Only the right side changed relative to base; right content is taken.
    RightChanged,
    /// Left and right made the *same* change; either content is taken.
    BothSame,
    /// Left and right diverged; this is a conflict.
    Conflict,
}

impl RegionKind {
    pub fn is_conflict(self) -> bool {
        matches!(self, Self::Conflict)
    }

    /// `true` when this region differs from base on at least one side.
    pub fn is_change(self) -> bool {
        !matches!(self, Self::Stable)
    }
}

/// One reconciled region of the three-way merge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeRegion {
    pub kind: RegionKind,
    pub base: Vec<MergeLine>,
    pub left: Vec<MergeLine>,
    pub right: Vec<MergeLine>,
}

impl MergeRegion {
    /// Content chosen automatically for a non-conflict region.
    pub(super) fn auto_content(&self) -> Vec<MergeLine> {
        match self.kind {
            RegionKind::Stable | RegionKind::LeftChanged => self.left.clone(),
            RegionKind::RightChanged => self.right.clone(),
            // BothSame: left == right by construction; either is correct.
            RegionKind::BothSame => self.left.clone(),
            // Conflict has no automatic content; the session leaves it
            // unresolved. As a defined default we surface the base.
            RegionKind::Conflict => self.base.clone(),
        }
    }
}

/// A matched-anchor alignment of one variant against base: a list of
/// `(base_index, variant_index)` pairs for lines that are equal, in order.
fn lcs_anchors(base: &[MergeLine], variant: &[MergeLine]) -> Vec<(usize, usize)> {
    let base_keys: Vec<String> = base.iter().map(key).collect();
    let var_keys: Vec<String> = variant.iter().map(key).collect();
    let base_refs: Vec<&str> = base_keys.iter().map(String::as_str).collect();
    let var_refs: Vec<&str> = var_keys.iter().map(String::as_str).collect();

    let mut config = TextDiffConfig::default();
    config.algorithm(Algorithm::Myers);
    let diff = config.diff_slices(&base_refs, &var_refs);

    let mut anchors = Vec::new();
    for op in diff.ops() {
        if let similar::DiffOp::Equal {
            old_index,
            new_index,
            len,
        } = op
        {
            for k in 0..*len {
                anchors.push((old_index + k, new_index + k));
            }
        }
    }
    anchors
}

/// Run the three-way merge, returning the reconciled region list.
pub fn diff3(base: &[MergeLine], left: &[MergeLine], right: &[MergeLine]) -> Vec<MergeRegion> {
    // Anchors that survive in *both* variants pin synchronization points.
    let left_anchors = lcs_anchors(base, left);
    let right_anchors = lcs_anchors(base, right);

    // Build base-index -> variant-index maps for quick lookup.
    let mut left_map = vec![None; base.len()];
    for (b, l) in &left_anchors {
        left_map[*b] = Some(*l);
    }
    let mut right_map = vec![None; base.len()];
    for (b, r) in &right_anchors {
        right_map[*b] = Some(*r);
    }

    // A base line is a *common anchor* when both variants kept it.
    let common: Vec<usize> = (0..base.len())
        .filter(|&b| left_map[b].is_some() && right_map[b].is_some())
        .collect();

    let mut regions: Vec<MergeRegion> = Vec::new();
    let mut bi = 0usize; // cursor into base
    let mut li = 0usize; // cursor into left
    let mut ri = 0usize; // cursor into right

    for &anchor_b in &common {
        let anchor_l = left_map[anchor_b].unwrap();
        let anchor_r = right_map[anchor_b].unwrap();

        // Emit the change region preceding this anchor (the slices between
        // the previous synchronization point and this anchor).
        let base_seg = &base[bi..anchor_b];
        let left_seg = &left[li..anchor_l];
        let right_seg = &right[ri..anchor_r];
        if !(base_seg.is_empty() && left_seg.is_empty() && right_seg.is_empty()) {
            regions.push(classify(base_seg, left_seg, right_seg));
        }

        // Emit the anchor line itself as a stable single-line region,
        // coalescing with a preceding stable region for compactness.
        push_stable_line(&mut regions, base[anchor_b].clone());

        bi = anchor_b + 1;
        li = anchor_l + 1;
        ri = anchor_r + 1;
    }

    // Trailing region after the last common anchor.
    let base_seg = &base[bi..];
    let left_seg = &left[li..];
    let right_seg = &right[ri..];
    if !(base_seg.is_empty() && left_seg.is_empty() && right_seg.is_empty()) {
        regions.push(classify(base_seg, left_seg, right_seg));
    }

    regions
}

fn push_stable_line(regions: &mut Vec<MergeRegion>, line: MergeLine) {
    if let Some(last) = regions.last_mut()
        && last.kind == RegionKind::Stable
    {
        last.base.push(line.clone());
        last.left.push(line.clone());
        last.right.push(line);
        return;
    }
    regions.push(MergeRegion {
        kind: RegionKind::Stable,
        base: vec![line.clone()],
        left: vec![line.clone()],
        right: vec![line],
    });
}

/// Classify a change region by comparing each variant against base.
fn classify(base: &[MergeLine], left: &[MergeLine], right: &[MergeLine]) -> MergeRegion {
    let left_changed = !lines_equal(base, left);
    let right_changed = !lines_equal(base, right);
    let kind = match (left_changed, right_changed) {
        (false, false) => RegionKind::Stable,
        (true, false) => RegionKind::LeftChanged,
        (false, true) => RegionKind::RightChanged,
        (true, true) => {
            if lines_equal(left, right) {
                RegionKind::BothSame
            } else {
                RegionKind::Conflict
            }
        }
    };
    MergeRegion {
        kind,
        base: base.to_vec(),
        left: left.to_vec(),
        right: right.to_vec(),
    }
}

fn lines_equal(a: &[MergeLine], b: &[MergeLine]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| key(x) == key(y))
}
