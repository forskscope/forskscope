//! RFC-095 §4.3: what the Explorer shows, decided from Flatpak detection
//! alone. A pure function of [`FlatpakInfo`] — the caller
//! (`forskscope-ui`'s `Explorer` component) supplies that by calling
//! `forskscope_desktop::platform::flatpak()` itself, the same split
//! `compare::data_location` uses for RFC-091's decision: no filesystem
//! read happens here, so this is testable with a plain, in-memory
//! [`FlatpakInfo`] value, never a real `/.flatpak-info` or a real
//! sandbox.

use forskscope_desktop::platform::FlatpakInfo;

/// What the Explorer should show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplorerAccess {
    /// Show the directory browser, exactly as today.
    Available,
    /// Replace it with RFC-095 §4.3's one message. Outside a Flatpak
    /// this never happens (`FlatpakInfo::in_flatpak` is always `false`
    /// there); inside one, only when the home folder is not granted.
    HomeRequired,
}

/// Decides between the two, from `flatpak` alone.
pub fn explorer_access(flatpak: &FlatpakInfo) -> ExplorerAccess {
    if flatpak.in_flatpak && !flatpak.home_granted {
        ExplorerAccess::HomeRequired
    } else {
        ExplorerAccess::Available
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flatpak(in_flatpak: bool, home_granted: bool) -> FlatpakInfo {
        FlatpakInfo {
            in_flatpak,
            app_id: None,
            home_granted,
        }
    }

    #[test]
    fn outside_a_flatpak_the_explorer_is_always_available() {
        assert_eq!(
            explorer_access(&flatpak(false, false)),
            ExplorerAccess::Available
        );
        // Even if home_granted were somehow true outside a Flatpak
        // (never produced by platform::flatpak(), but this function
        // does not trust that on its own) - in_flatpak alone gates it.
        assert_eq!(
            explorer_access(&flatpak(false, true)),
            ExplorerAccess::Available
        );
    }

    #[test]
    fn in_a_flatpak_without_the_home_grant_the_message_shows() {
        assert_eq!(
            explorer_access(&flatpak(true, false)),
            ExplorerAccess::HomeRequired
        );
    }

    #[test]
    fn in_a_flatpak_with_the_home_grant_the_explorer_is_available() {
        assert_eq!(
            explorer_access(&flatpak(true, true)),
            ExplorerAccess::Available
        );
    }
}
