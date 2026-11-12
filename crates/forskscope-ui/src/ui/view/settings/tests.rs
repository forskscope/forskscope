//! RFC-076 patch 4: proves `persist`/`load`'s Store-independent halves
//! actually go through `SettingsRepository` — not just that the
//! lower-level core parsers work (handoff §6) — and that `persist`'s merge
//! preserves fields the UI does not own.

use std::fs;
use std::path::PathBuf;

use forskscope_desktop::persist::schema::PersistenceLoad;
use forskscope_desktop::persist::schema::settings::runtime::{
    MigrationCommitOutcome, SettingsRuntimeOutcome, SettingsRuntimeResolution,
};
use forskscope_desktop::persist::schema::settings::{PersistedSettings, SettingsRepository};

use super::{build_save_payload, load_settings, persist_settings, recovery_modal, reset_settings};
use crate::state::{AppSettings, Modal};

fn temp_path(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fsk-ui-settings-{tag}-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    dir.join("settings.json")
}

#[test]
fn build_save_payload_overlays_ui_fields_but_preserves_the_rest() {
    let base = PersistedSettings {
        density: forskscope_desktop::settings::display::Density::Spacious,
        show_line_numbers: false,
        ..PersistedSettings::default()
    };
    let mut edited = AppSettings::from_v2(&base);
    edited.context_lines = 9;

    let merged = build_save_payload(&edited, &base);

    assert_eq!(merged.context_lines, 9, "UI-editable field must apply");
    assert_eq!(
        merged.density,
        forskscope_desktop::settings::display::Density::Spacious,
        "non-UI-owned field must survive a save the UI has no control over"
    );
    assert!(
        !merged.show_line_numbers,
        "non-UI-owned field must survive a save the UI has no control over"
    );
}

/// F159(a): a newline mode written by hand into `settings.json` used to be
/// silently reset to `Significant` on the next save, because the UI's profile
/// type had no field to carry it. Falsify by hardcoding `Significant` back into
/// `DiffProfile::to_v2`: the assertion fails.
#[test]
fn a_newline_mode_set_by_hand_survives_a_settings_save() {
    use forskscope_compare::NewlineCompareMode;

    let mut base = PersistedSettings::default();
    let mut hand = base.profiles[0].clone();
    hand.name = "Hand-written".into();
    hand.built_in = false;
    hand.newlines = NewlineCompareMode::IgnoreDifference;
    base.profiles.push(hand);

    let edited = AppSettings::from_v2(&base);
    let merged = build_save_payload(&edited, &base);

    let kept = merged
        .profiles
        .iter()
        .find(|p| p.name == "Hand-written")
        .expect("the hand-written profile must survive the save");
    assert_eq!(kept.newlines, NewlineCompareMode::IgnoreDifference);
}

#[test]
fn persist_settings_writes_through_the_real_repository() {
    let path = temp_path("persist");
    let repo = SettingsRepository::new(path);
    let payload = PersistedSettings {
        context_lines: 11,
        ..PersistedSettings::default()
    };

    persist_settings(&payload, &repo).expect("persist_settings must succeed");

    match repo.load() {
        PersistenceLoad::Current { value } => assert_eq!(value.context_lines, 11),
        other => panic!("expected Current after persist_settings, got {other:?}"),
    }
}

#[test]
fn load_settings_resolves_missing_file_as_fresh_defaults() {
    let path = temp_path("load-missing");
    let _ = fs::remove_file(&path);
    let repo = SettingsRepository::new(path);

    let (settings, resolution, _initial_system_color_scheme) = load_settings(&repo);

    assert!(!resolution.write_disabled);
    assert_eq!(
        settings.diff_font_size,
        PersistedSettings::default().diff_font_size
    );
}

/// RFC-097 stage 2 §2.1: "System is the default only for a settings file
/// that never saved a theme" - a missing file is exactly that. Whether
/// the resolved theme is actually `System` depends on
/// `system_theme_offered`'s own answer for *this* host (`true` only on
/// Windows, synchronously - Linux's portal answer isn't known yet this
/// early), so this asserts the invariant symmetrically rather than
/// hardcoding a platform, which would only ever exercise one branch
/// depending on which OS runs the suite.
#[test]
fn a_fresh_install_defaults_to_system_theme_exactly_where_it_is_synchronously_offered() {
    let path = temp_path("load-missing-theme-default");
    let _ = fs::remove_file(&path);
    let repo = SettingsRepository::new(path);

    let (settings, _resolution, initial_system_color_scheme) = load_settings(&repo);

    // Whatever `load_settings` itself just read synchronously (Windows:
    // always `None` here, decided with no I/O; Linux: a real, bounded
    // portal read, whose answer depends on whether this environment has
    // one reachable - this asserts load_settings resolved the theme
    // consistently with its *own* reading, not a hardcoded expectation
    // that would only ever hold on one kind of CI runner).
    let offered = crate::state::settings::system_theme_offered(
        crate::state::settings::host_os(),
        initial_system_color_scheme.is_some(),
    );
    if offered {
        assert_eq!(settings.theme, crate::state::Theme::System);
    } else {
        assert_eq!(settings.theme, crate::state::Theme::Dark);
    }
}

/// The same rule must not apply to a file that merely failed to parse -
/// an actual theme choice may be sitting unreadable in it, so guessing
/// System (even where offered) would silently discard that choice rather
/// than merely resolving an absence.
#[test]
fn a_corrupt_file_does_not_get_the_fresh_install_system_default() {
    let path = temp_path("load-corrupt-theme-default");
    fs::write(&path, "not valid json").unwrap();
    let repo = SettingsRepository::new(path);

    let (settings, resolution, _initial_system_color_scheme) = load_settings(&repo);

    assert!(matches!(
        resolution.outcome,
        SettingsRuntimeOutcome::CorruptPreserved { .. }
    ));
    assert_eq!(
        settings.theme,
        crate::state::Theme::Dark,
        "a corrupt file must fall back to the universal default, never guess System"
    );
}

#[test]
fn load_settings_round_trips_through_persist_settings() {
    let path = temp_path("round-trip");
    let repo = SettingsRepository::new(path);
    let payload = PersistedSettings {
        context_lines: 8,
        explorer_compact: true,
        ..PersistedSettings::default()
    };
    persist_settings(&payload, &repo).expect("persist_settings must succeed");

    let (settings, _resolution, _initial_system_color_scheme) = load_settings(&repo);

    assert_eq!(settings.context_lines, 8);
    assert!(settings.explorer_compact);
}

/// Review 211 §3: the synchronous portal read only runs "when the saved
/// theme is System, or the install is new" - never on an ordinary launch
/// with an explicit Dark/Light/Night saved, which must stay as fast as
/// it always was, not pay a bounded-but-real wait on every single start.
/// A real `gio` D-Bus round trip (even one that fails fast, no portal
/// reachable) is not literally zero-cost, so this times an *ordinary*
/// load against the ~250ms budget `synchronous_linux_color_scheme_read`
/// allows itself, rather than asserting zero I/O directly (which would
/// need mocking `gio` itself, not attempted here).
#[test]
fn an_ordinary_saved_theme_does_not_pay_the_portal_reads_own_budget() {
    let path = temp_path("load-ordinary-theme-is-fast");
    let payload = PersistedSettings {
        theme: forskscope_desktop::settings::ThemeId::Night,
        ..PersistedSettings::default()
    };
    let repo = SettingsRepository::new(path);
    persist_settings(&payload, &repo).expect("persist_settings must succeed");

    let start = std::time::Instant::now();
    let (settings, _resolution, initial_system_color_scheme) = load_settings(&repo);
    let elapsed = start.elapsed();

    assert_eq!(settings.theme, crate::state::Theme::Night);
    assert_eq!(
        initial_system_color_scheme, None,
        "a saved Night theme must never trigger the portal read at all"
    );
    assert!(
        elapsed < std::time::Duration::from_millis(200),
        "an ordinary saved theme took {elapsed:?} - the portal read's own \
         250ms budget must not run at all for this case"
    );
}

// ── RFC-076 patch 6: reset_settings / recovery_modal ──────────────────────

fn resolution(outcome: SettingsRuntimeOutcome) -> SettingsRuntimeResolution {
    SettingsRuntimeResolution {
        value: PersistedSettings::default(),
        write_disabled: true,
        outcome,
        raw_bytes: None,
    }
}

#[test]
fn reset_settings_writes_backup_and_new_value_through_the_real_repository() {
    let path = temp_path("reset");
    fs::write(&path, "{not valid json").unwrap();
    let repo = SettingsRepository::new(path.clone());
    let value = PersistedSettings {
        context_lines: 5,
        ..PersistedSettings::default()
    };

    reset_settings(&value, b"{not valid json", &repo).expect("reset must succeed");

    let backup_path = path.with_file_name("settings.json.reset.bak");
    assert_eq!(
        fs::read_to_string(&backup_path).unwrap(),
        "{not valid json",
        "the corrupt original must be preserved under a distinct .reset.bak name"
    );
    match repo.load() {
        PersistenceLoad::Current { value } => assert_eq!(value.context_lines, 5),
        other => panic!("expected Current after reset_settings, got {other:?}"),
    }
}

#[test]
fn reset_settings_rejects_stale_bytes_after_external_change() {
    let path = temp_path("reset-stale");
    fs::write(&path, "{not valid json").unwrap();
    let repo = SettingsRepository::new(path.clone());

    let result = reset_settings(&PersistedSettings::default(), b"different bytes", &repo);

    assert!(
        result.is_err(),
        "must refuse to reset over bytes that no longer match what's on disk"
    );
}

#[test]
fn recovery_modal_is_none_for_outcomes_without_a_dialog() {
    for outcome in [
        SettingsRuntimeOutcome::Fresh,
        SettingsRuntimeOutcome::Current,
        SettingsRuntimeOutcome::Migrated(MigrationCommitOutcome::Committed { backup_path: None }),
        SettingsRuntimeOutcome::Migrated(MigrationCommitOutcome::DeferredByConflict),
    ] {
        assert!(recovery_modal(&resolution(outcome)).is_none());
    }
}

#[test]
fn recovery_modal_is_some_for_outcomes_with_a_dialog() {
    let res = resolution(SettingsRuntimeOutcome::Incompatible {
        schema: "settings".into(),
        version: 99,
    });
    assert!(matches!(
        recovery_modal(&res),
        Some(Modal::SettingsRecovery(_))
    ));
}
