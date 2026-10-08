//! Update-check decision logic (F180/F181, handoff 073).
//!
//! Pure mapping from *(current version, outcome of the network request)*
//! to what the About dialog's *Check for updates* button shows. The
//! network call itself is a thin adapter in `forskscope-ui`
//! (`crate::update_check` there, §3) — this module never does I/O, so
//! [`decide`] is the one function both the app and its tests call (F169,
//! review 151's lesson: a test on a copy of the decision is not a test on
//! the app's own decision).
//!
//! ## The rule that matters most
//!
//! "Up to date" must only ever come from a version that was actually read
//! and parsed. Every failure — unreachable, rate-limited, any other HTTP
//! status, or a reply/tag that does not parse — becomes [`CheckFailureReason`],
//! never silently read as "up to date". [`decide`]'s own tests falsify this
//! directly: mapping any [`CheckOutcome`] failure variant to
//! [`UpdateCheckState::UpToDate`] is asserted never to happen.

use std::cmp::Ordering;
use std::fmt;

// ── Version ───────────────────────────────────────────────────────────────────

/// A released version, shaped exactly as our tags are: plain `X.Y.Z`, no
/// `v` prefix, no pre-release or build suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    /// Parses a tag exactly shaped `X.Y.Z` — three decimal components
    /// separated by `.`, nothing else. Anything else (a `v` prefix, a
    /// pre-release suffix, a missing or extra component, non-digits) is
    /// rejected rather than guessed: a reply this cannot read is "Could not
    /// check", never a silent best-effort parse (§2's rule).
    pub fn parse(s: &str) -> Option<Self> {
        let mut parts = s.split('.');
        let major = parts.next()?;
        let minor = parts.next()?;
        let patch = parts.next()?;
        if parts.next().is_some() {
            return None;
        }
        Some(Self {
            major: major.parse().ok()?,
            minor: minor.parse().ok()?,
            patch: patch.parse().ok()?,
        })
    }

    /// This build's own version, from `CARGO_PKG_VERSION` — always a
    /// well-formed `X.Y.Z`, since `cargo xtask version-sync` enforces that
    /// shape workspace-wide.
    pub fn current() -> Self {
        Self::parse(env!("CARGO_PKG_VERSION")).expect("CARGO_PKG_VERSION is always X.Y.Z")
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

// ── CheckOutcome / decide ──────────────────────────────────────────────────────

/// What the network adapter learned, before this module interprets it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckOutcome {
    /// The reply parsed as JSON and had a `tag_name` string field — raw,
    /// not yet validated as a version. The adapter's whole job is to read
    /// `tag_name` and nothing else from the reply (§3); this carries
    /// exactly that, and no other field ever reaches this module.
    TagFound(String),
    /// Connect, TLS or read failed, or the request timed out: no HTTP
    /// response was ever received.
    Unreachable,
    /// GitHub answered 403 or 429.
    RateLimited,
    /// Any other non-200 HTTP status.
    HttpStatus(u16),
    /// A 200 arrived, but the body was not JSON, or had no `tag_name`
    /// string field.
    BodyUnparseable,
}

/// Why "Could not check" is being shown — the reason the table (§2) says
/// must be stated plainly, not folded into one generic message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckFailureReason {
    Unreachable,
    RateLimited,
    HttpStatus(u16),
    /// The reply's body did not parse ([`CheckOutcome::BodyUnparseable`]),
    /// or its tag was not a plain `X.Y.Z` version ([`Version::parse`]
    /// failing inside [`decide`]).
    Unparseable,
}

/// Everything the About dialog's update-check area can show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateCheckState {
    /// A request is in flight. Not produced by [`decide`] — the UI sets
    /// this directly before awaiting the adapter, since it is not a
    /// function of any [`CheckOutcome`].
    Checking,
    UpToDate {
        current: Version,
    },
    NewerAvailable {
        current: Version,
        latest: Version,
    },
    AheadOfRelease {
        current: Version,
        latest: Version,
    },
    CouldNotCheck(CheckFailureReason),
}

/// The one function the app and its tests both call (F169, review 151).
/// Pure: no I/O, no clock, no randomness — the same `(current, outcome)`
/// always produces the same state.
pub fn decide(current: Version, outcome: CheckOutcome) -> UpdateCheckState {
    let latest = match outcome {
        CheckOutcome::Unreachable => {
            return UpdateCheckState::CouldNotCheck(CheckFailureReason::Unreachable);
        }
        CheckOutcome::RateLimited => {
            return UpdateCheckState::CouldNotCheck(CheckFailureReason::RateLimited);
        }
        CheckOutcome::HttpStatus(code) => {
            return UpdateCheckState::CouldNotCheck(CheckFailureReason::HttpStatus(code));
        }
        CheckOutcome::BodyUnparseable => {
            return UpdateCheckState::CouldNotCheck(CheckFailureReason::Unparseable);
        }
        CheckOutcome::TagFound(tag) => match Version::parse(&tag) {
            Some(v) => v,
            None => return UpdateCheckState::CouldNotCheck(CheckFailureReason::Unparseable),
        },
    };
    match latest.cmp(&current) {
        Ordering::Equal => UpdateCheckState::UpToDate { current },
        Ordering::Greater => UpdateCheckState::NewerAvailable { current, latest },
        Ordering::Less => UpdateCheckState::AheadOfRelease { current, latest },
    }
}

// ── Where the app came from (§4) ───────────────────────────────────────────────

/// How this build was obtained — read at compile time from
/// `FORSKSCOPE_CHANNEL`, since a copy installed from the AUR or the Store
/// is updated by that store, and pointing it at a GitHub download would be
/// wrong advice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppChannel {
    /// A release tarball/zip/dmg downloaded from GitHub. Also the fallback
    /// for an unset or unrecognised channel label — a plain download is
    /// the only thing this build can safely be assumed to be.
    Direct,
    Aur,
    Store,
}

impl AppChannel {
    /// `label` is `option_env!("FORSKSCOPE_CHANNEL")`'s value, read by the
    /// caller at compile time — a `const` macro invocation can't be fed
    /// synthetic values, so the mapping it feeds is kept separately
    /// testable here, against a plain `Option<&str>`.
    pub fn from_label(label: Option<&str>) -> Self {
        match label {
            Some("aur") => Self::Aur,
            Some("store") => Self::Store,
            _ => Self::Direct,
        }
    }
}

/// What a *Newer available* row asks the user to do, per channel (§4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateAction {
    /// Open the GitHub release page in the system browser.
    OpenReleasePage,
    UseAurHelper,
    UseMicrosoftStore,
}

pub fn update_action(channel: AppChannel) -> UpdateAction {
    match channel {
        AppChannel::Direct => UpdateAction::OpenReleasePage,
        AppChannel::Aur => UpdateAction::UseAurHelper,
        AppChannel::Store => UpdateAction::UseMicrosoftStore,
    }
}

/// The release page to open for `version`.
///
/// Built only from `version`'s already-validated numeric fields, never
/// from raw reply text (§4's security bullet): the tag came from the
/// network, and this URL is handed to a launched subprocess
/// (`forskscope_core::external_tool::open_url`). A [`Version`] can only
/// exist by [`Version::parse`] having already accepted a plain `X.Y.Z`
/// shape — there is no path from a hostile reply into this string, because
/// there is no path from a hostile reply into a `Version` at all.
pub fn release_page_url(version: &Version) -> String {
    format!("https://github.com/forskscope/forskscope/releases/tag/{version}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(major: u32, minor: u32, patch: u32) -> Version {
        Version {
            major,
            minor,
            patch,
        }
    }

    // ── Version::parse ─────────────────────────────────────────────────────

    #[test]
    fn parses_a_plain_version() {
        assert_eq!(Version::parse("0.187.0"), Some(v(0, 187, 0)));
        assert_eq!(Version::parse("12.3.456"), Some(v(12, 3, 456)));
    }

    #[test]
    fn rejects_shapes_that_are_not_plain_x_y_z() {
        for tag in [
            "v0.187.0",
            "0.187",
            "0.187.0.1",
            "0.187.0-beta",
            "0.187.0+build",
            "",
            "...",
            "a.b.c",
            "0.187.0 ",
            " 0.187.0",
            "0.187.0\n",
        ] {
            assert_eq!(Version::parse(tag), None, "{tag:?} must not parse");
        }
    }

    /// A reply field is attacker-influenced text, not a trusted version
    /// string. These are not merely malformed — they are the kind of
    /// string a hostile reply would carry if it were trying to reach a
    /// launched command line through this value. All of them must be
    /// rejected exactly like any other unparseable tag, never specially
    /// sanitised or partially accepted.
    #[test]
    fn rejects_hostile_tag_shapes() {
        for tag in [
            "0.187.0; rm -rf /",
            "0.187.0 && touch pwned",
            "0.187.0\"; evil",
            "$(whoami)",
            "../../etc/passwd",
            "0.187.0/../../x",
            "javascript:alert(1)",
        ] {
            assert_eq!(Version::parse(tag), None, "{tag:?} must not parse");
        }
    }

    #[test]
    fn current_parses_its_own_cargo_pkg_version() {
        // Smoke test: whatever the workspace version is right now, it must
        // be readable by the same parser a reply's tag goes through.
        let _ = Version::current();
    }

    #[test]
    fn version_displays_as_plain_x_y_z() {
        assert_eq!(v(0, 187, 0).to_string(), "0.187.0");
    }

    #[test]
    fn versions_compare_numerically_not_lexically() {
        assert!(v(0, 10, 0) > v(0, 9, 0), "0.10.0 must be newer than 0.9.0");
        assert!(v(1, 0, 0) > v(0, 187, 0));
        assert_eq!(v(0, 187, 0), v(0, 187, 0));
    }

    // ── decide ──────────────────────────────────────────────────────────────

    #[test]
    fn equal_version_is_up_to_date() {
        let current = v(0, 187, 0);
        let state = decide(current, CheckOutcome::TagFound("0.187.0".into()));
        assert_eq!(state, UpdateCheckState::UpToDate { current });
    }

    #[test]
    fn newer_tag_is_newer_available() {
        let current = v(0, 187, 0);
        let latest = v(0, 188, 0);
        let state = decide(current, CheckOutcome::TagFound("0.188.0".into()));
        assert_eq!(state, UpdateCheckState::NewerAvailable { current, latest });
    }

    #[test]
    fn older_tag_is_ahead_of_release() {
        let current = v(0, 188, 0);
        let latest = v(0, 187, 0);
        let state = decide(current, CheckOutcome::TagFound("0.187.0".into()));
        assert_eq!(state, UpdateCheckState::AheadOfRelease { current, latest });
    }

    #[test]
    fn unreachable_is_could_not_check_unreachable() {
        let state = decide(v(0, 187, 0), CheckOutcome::Unreachable);
        assert_eq!(
            state,
            UpdateCheckState::CouldNotCheck(CheckFailureReason::Unreachable)
        );
    }

    #[test]
    fn rate_limited_is_could_not_check_rate_limited() {
        let state = decide(v(0, 187, 0), CheckOutcome::RateLimited);
        assert_eq!(
            state,
            UpdateCheckState::CouldNotCheck(CheckFailureReason::RateLimited)
        );
    }

    #[test]
    fn other_http_status_carries_its_code() {
        let state = decide(v(0, 187, 0), CheckOutcome::HttpStatus(500));
        assert_eq!(
            state,
            UpdateCheckState::CouldNotCheck(CheckFailureReason::HttpStatus(500))
        );
    }

    #[test]
    fn unparseable_body_is_could_not_check_unparseable() {
        let state = decide(v(0, 187, 0), CheckOutcome::BodyUnparseable);
        assert_eq!(
            state,
            UpdateCheckState::CouldNotCheck(CheckFailureReason::Unparseable)
        );
    }

    #[test]
    fn a_tag_that_is_not_a_version_is_could_not_check_unparseable_not_a_guess() {
        for tag in ["not-a-version", "v0.187.0", "0.187", "0.187.0-rc1"] {
            let state = decide(v(0, 187, 0), CheckOutcome::TagFound(tag.into()));
            assert_eq!(
                state,
                UpdateCheckState::CouldNotCheck(CheckFailureReason::Unparseable),
                "{tag:?} must be Could not check, not a guessed version"
            );
        }
    }

    /// §2's falsification, run directly against the production function:
    /// every failure outcome must decide to `CouldNotCheck`, and none of
    /// them may ever decide to `UpToDate`. F166: our own AUR check mapped
    /// a failure to "up to date" three times in one day; here that would
    /// mislead a user, not a maintainer.
    #[test]
    fn no_failure_outcome_ever_decides_up_to_date() {
        let current = v(0, 187, 0);
        let failures = [
            CheckOutcome::Unreachable,
            CheckOutcome::RateLimited,
            CheckOutcome::HttpStatus(500),
            CheckOutcome::HttpStatus(404),
            CheckOutcome::BodyUnparseable,
            CheckOutcome::TagFound("not-a-version".into()),
            CheckOutcome::TagFound("v0.187.0".into()),
        ];
        for outcome in failures {
            let state = decide(current, outcome.clone());
            assert!(
                !matches!(state, UpdateCheckState::UpToDate { .. }),
                "{outcome:?} must never decide UpToDate, got {state:?}"
            );
            assert!(
                matches!(state, UpdateCheckState::CouldNotCheck(_)),
                "{outcome:?} must decide CouldNotCheck, got {state:?}"
            );
        }
    }

    // ── AppChannel ──────────────────────────────────────────────────────────

    #[test]
    fn unset_channel_is_direct() {
        assert_eq!(AppChannel::from_label(None), AppChannel::Direct);
    }

    #[test]
    fn known_channel_labels_map_correctly() {
        assert_eq!(AppChannel::from_label(Some("aur")), AppChannel::Aur);
        assert_eq!(AppChannel::from_label(Some("store")), AppChannel::Store);
    }

    #[test]
    fn an_unrecognised_channel_label_is_treated_as_direct() {
        for bogus in ["", "AUR", "Store", "github", "unknown"] {
            assert_eq!(
                AppChannel::from_label(Some(bogus)),
                AppChannel::Direct,
                "{bogus:?} must fall back to Direct"
            );
        }
    }

    // ── update_action ───────────────────────────────────────────────────────

    #[test]
    fn each_channel_has_its_own_action() {
        assert_eq!(
            update_action(AppChannel::Direct),
            UpdateAction::OpenReleasePage
        );
        assert_eq!(update_action(AppChannel::Aur), UpdateAction::UseAurHelper);
        assert_eq!(
            update_action(AppChannel::Store),
            UpdateAction::UseMicrosoftStore
        );
    }

    // ── release_page_url ────────────────────────────────────────────────────

    #[test]
    fn release_page_url_is_built_from_the_version_alone() {
        assert_eq!(
            release_page_url(&v(0, 188, 0)),
            "https://github.com/forskscope/forskscope/releases/tag/0.188.0"
        );
    }

    /// Structural, not a string-content check: a `Version` can only ever
    /// contain three `u32`s, so its `Display` can only ever emit digits
    /// and `.` — this is true for *any* `Version`, including one built
    /// from the largest fields possible, not just ordinary-looking ones.
    #[test]
    fn release_page_url_can_only_ever_contain_digits_and_dots_in_its_version_segment() {
        let hostile_shaped = v(u32::MAX, u32::MAX, u32::MAX);
        let url = release_page_url(&hostile_shaped);
        let segment = url.rsplit('/').next().unwrap();
        assert!(
            segment.chars().all(|c| c.is_ascii_digit() || c == '.'),
            "version segment {segment:?} must be only digits and dots"
        );
    }
}
