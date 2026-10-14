//! Tab-list index arithmetic (RFC-087): the Explorer, every file comparison
//! tab, and every directory compare tab form one ordered list for keyboard
//! navigation — the Explorer first, then file tabs in their existing order,
//! then directory tabs in theirs, matching how `TabBar` already renders
//! them. Pure, so wrap/Home/End/the flat↔kind mapping can be tested without
//! a Dioxus runtime; `ui/layout/tabs.rs` and `keyboard.rs` are the only
//! callers.

/// Which tab a flat position names. `File`/`Dir` carry the same index
/// `store.active`/`store.active_dir` and `store.tabs`/`store.dir_tabs` use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TabKind {
    Explorer,
    File(usize),
    Dir(usize),
}

/// Total tabs in the list: the Explorer, always present, plus every file
/// and directory tab.
pub(crate) fn tab_count(file_count: usize, dir_count: usize) -> usize {
    1 + file_count + dir_count
}

/// The flat position of the active tab. `active_file` and `active_dir` are
/// kept mutually exclusive by every caller that sets them (never both
/// `Some`); if that invariant is ever violated, `active_file` wins, the
/// same precedence `TabBar`'s own `is_active` checks already use.
pub(crate) fn position_of(
    active_file: Option<usize>,
    active_dir: Option<usize>,
    file_count: usize,
) -> usize {
    match (active_file, active_dir) {
        (Some(i), _) => 1 + i,
        (None, Some(j)) => 1 + file_count + j,
        (None, None) => 0,
    }
}

/// The tab named by a flat position. Panics if `position >= tab_count(file_count, dir_count)`
/// — every caller derives `position` from [`tab_count`]'s own range (wrap/Home/End never
/// escape it), so an out-of-range value here is a bug at the call site, not a state to handle.
pub(crate) fn kind_at(position: usize, file_count: usize, dir_count: usize) -> TabKind {
    if position == 0 {
        TabKind::Explorer
    } else if position <= file_count {
        TabKind::File(position - 1)
    } else if position <= file_count + dir_count {
        TabKind::Dir(position - 1 - file_count)
    } else {
        panic!("tab position {position} out of range for {file_count} file + {dir_count} dir tabs");
    }
}

/// Wrapping next/previous position among `total` tabs. `total` is always at
/// least 1 (the Explorer), so this never divides by zero.
pub(crate) fn next_position(current: usize, total: usize) -> usize {
    (current + 1) % total
}

pub(crate) fn prev_position(current: usize, total: usize) -> usize {
    (current + total - 1) % total
}

pub(crate) fn home_position() -> usize {
    0
}

pub(crate) fn end_position(total: usize) -> usize {
    total - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_count_is_one_plus_both_kinds() {
        assert_eq!(tab_count(0, 0), 1);
        assert_eq!(tab_count(3, 0), 4);
        assert_eq!(tab_count(0, 2), 3);
        assert_eq!(tab_count(3, 2), 6);
    }

    #[test]
    fn position_of_explorer_is_zero() {
        assert_eq!(position_of(None, None, 3), 0);
    }

    #[test]
    fn position_of_a_file_tab_is_one_plus_its_index() {
        assert_eq!(position_of(Some(0), None, 3), 1);
        assert_eq!(position_of(Some(2), None, 3), 3);
    }

    #[test]
    fn position_of_a_dir_tab_is_after_every_file_tab() {
        assert_eq!(position_of(None, Some(0), 3), 4);
        assert_eq!(position_of(None, Some(1), 3), 5);
    }

    #[test]
    fn kind_at_round_trips_with_position_of() {
        // Every (active_file, active_dir) a real Store can hold, round-tripped
        // through position_of -> kind_at, for every file/dir count up to 3.
        for file_count in 0..=3usize {
            for dir_count in 0..=3usize {
                let total = tab_count(file_count, dir_count);
                for position in 0..total {
                    let kind = kind_at(position, file_count, dir_count);
                    let (af, ad) = match kind {
                        TabKind::Explorer => (None, None),
                        TabKind::File(i) => (Some(i), None),
                        TabKind::Dir(j) => (None, Some(j)),
                    };
                    assert_eq!(
                        position_of(af, ad, file_count),
                        position,
                        "file_count={file_count} dir_count={dir_count} position={position}"
                    );
                }
            }
        }
    }

    /// Falsify by removing the modulo: `next_position` at the last tab must
    /// wrap to 0 (the Explorer), not go out of range.
    #[test]
    fn next_position_wraps_from_the_last_tab_to_the_explorer() {
        assert_eq!(next_position(4, 5), 0);
    }

    #[test]
    fn next_position_does_not_wrap_before_the_end() {
        assert_eq!(next_position(1, 5), 2);
    }

    /// Falsify the same way: `prev_position` at the Explorer (0) must wrap
    /// to the last tab, not underflow.
    #[test]
    fn prev_position_wraps_from_the_explorer_to_the_last_tab() {
        assert_eq!(prev_position(0, 5), 4);
    }

    #[test]
    fn prev_position_does_not_wrap_after_the_start() {
        assert_eq!(prev_position(3, 5), 2);
    }

    #[test]
    fn home_is_always_the_explorer() {
        assert_eq!(home_position(), 0);
    }

    #[test]
    fn end_is_the_last_tab() {
        assert_eq!(end_position(1), 0);
        assert_eq!(end_position(6), 5);
    }

    #[test]
    #[should_panic(expected = "out of range")]
    fn kind_at_panics_past_the_last_tab() {
        kind_at(4, 1, 2); // tab_count(1, 2) == 4, so position 4 is out of range
    }
}
