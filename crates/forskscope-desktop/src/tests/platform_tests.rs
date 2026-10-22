//! Tests for platform diagnostic info (RFC-026) and Flatpak detection
//! (RFC-095 §4.1).

use crate::platform::{FlatpakInfo, PlatformInfo, flatpak_at, parse_flatpak_info};

#[test]
fn collect_does_not_panic() {
    let _ = PlatformInfo::collect();
}

#[test]
fn app_version_is_non_empty() {
    let info = PlatformInfo::collect();
    assert!(!info.app_version.is_empty());
}

#[test]
fn os_is_non_empty() {
    let info = PlatformInfo::collect();
    assert!(!info.os.is_empty(), "OS field must be non-empty");
}

#[test]
fn arch_is_non_empty() {
    let info = PlatformInfo::collect();
    assert!(!info.arch.is_empty(), "arch field must be non-empty");
}

#[test]
fn to_report_contains_version_and_os() {
    let info = PlatformInfo::collect();
    let report = info.to_report();
    assert!(
        report.contains("ForskScope"),
        "report must include app name"
    );
    assert!(report.contains("OS:"), "report must include OS label");
    assert!(report.contains("Arch:"), "report must include arch label");
}

#[test]
fn home_redacted_does_not_contain_username() {
    let info = PlatformInfo::collect();
    // The redacted home should end with *** or be "unknown"
    if info.home_redacted != "unknown" {
        assert!(
            info.home_redacted.ends_with("***"),
            "home field must be redacted: {:?}",
            info.home_redacted
        );
    }
}

#[test]
fn to_report_is_stable_across_calls() {
    // Two calls must produce identical output (no random components)
    let a = PlatformInfo::collect().to_report();
    let b = PlatformInfo::collect().to_report();
    assert_eq!(a, b, "to_report() must be deterministic");
}

#[test]
fn logical_cpus_is_positive_number_or_unknown() {
    let info = PlatformInfo::collect();
    if info.logical_cpus != "unknown" {
        let n: usize = info
            .logical_cpus
            .parse()
            .expect("logical_cpus must be a number or 'unknown'");
        assert!(n >= 1, "must have at least 1 logical CPU");
    }
}

#[test]
fn config_dir_hint_is_non_empty() {
    let info = PlatformInfo::collect();
    assert!(!info.config_dir_hint.is_empty());
}

// ── RFC-095 §4.1: Flatpak detection ──────────────────────────────────────────

#[test]
fn no_flatpak_info_file_means_not_in_a_flatpak() {
    let info = flatpak_at("/this/path/does/not/exist/.flatpak-info");
    assert_eq!(
        info,
        FlatpakInfo {
            in_flatpak: false,
            app_id: None,
            home_granted: false,
        }
    );
}

#[test]
fn a_real_file_with_no_context_section_grants_no_home() {
    let dir = tmp("no-context");
    let path = write_flatpak_info(
        &dir,
        "flatpak-info",
        "[Application]\nname=io.github.forskscope.forskscope\n",
    );
    let info = flatpak_at(&path);
    assert!(info.in_flatpak);
    assert_eq!(info.app_id, Some("io.github.forskscope.forskscope".into()));
    assert!(!info.home_granted);
}

#[test]
fn empty_filesystems_grants_no_home() {
    assert!(!parse_flatpak_info("[Context]\nfilesystems=\n").home_granted);
}

#[test]
fn filesystems_home_grants_home() {
    assert!(parse_flatpak_info("[Context]\nfilesystems=home\n").home_granted);
}

#[test]
fn filesystems_home_ro_grants_home() {
    assert!(parse_flatpak_info("[Context]\nfilesystems=home:ro\n").home_granted);
}

#[test]
fn filesystems_host_grants_home() {
    assert!(parse_flatpak_info("[Context]\nfilesystems=host\n").home_granted);
}

#[test]
fn filesystems_xdg_documents_only_grants_no_home() {
    assert!(!parse_flatpak_info("[Context]\nfilesystems=xdg-documents\n").home_granted);
}

#[test]
fn a_filesystems_line_outside_context_is_ignored() {
    // The same key name in an unrelated section must not be read as the
    // home grant - a key file's section is whichever `[...]` header most
    // recently preceded the line, not the key name alone.
    let info = parse_flatpak_info("[Instance]\nfilesystems=home\n");
    assert!(!info.home_granted);
}

#[test]
fn a_real_flatpak_info_reflects_a_run_time_override() {
    // RFC-095 §4.1: "`--filesystem=home` adds `home`; `--nofilesystem=home`
    // empties it" - verified on a real file read, not only the parser.
    let dir = tmp("override");
    let granted = write_flatpak_info(
        &dir,
        "granted",
        "[Application]\nname=io.github.forskscope.forskscope\n[Context]\nfilesystems=home;xdg-run/dconf:ro;\n",
    );
    assert!(flatpak_at(&granted).home_granted);

    let revoked = write_flatpak_info(
        &dir,
        "revoked",
        "[Application]\nname=io.github.forskscope.forskscope\n[Context]\nfilesystems=;xdg-run/dconf:ro;\n",
    );
    assert!(!flatpak_at(&revoked).home_granted);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn tmp(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("fsk-flatpak-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn write_flatpak_info(dir: &std::path::Path, name: &str, contents: &str) -> String {
    let path = dir.join(name);
    std::fs::write(&path, contents).unwrap();
    path.to_str().unwrap().to_string()
}
