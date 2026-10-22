//! Tests for platform diagnostic info (RFC-026), Flatpak detection
//! (RFC-095 §4.1) and document-portal host paths (RFC-095 §4.4).

use crate::platform::{FlatpakInfo, PlatformInfo, flatpak_at, parse_flatpak_info};
#[cfg(target_os = "linux")]
use crate::platform::{display_path, document_portal_host_path};

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

// ── RFC-095 §4.4: document-portal host paths ────────────────────────────────

#[test]
#[cfg(target_os = "linux")]
fn the_attribute_absent_falls_back_to_the_path_itself() {
    let dir = tmp("doc-portal-absent");
    let path = dir.join("plain-file.txt");
    std::fs::write(&path, "x").unwrap();
    assert_eq!(document_portal_host_path(&path), None);
    assert_eq!(display_path(&path), path);
}

#[test]
#[cfg(target_os = "linux")]
fn the_attribute_present_is_read_back() {
    let dir = tmp("doc-portal-present");
    let path = dir.join("forwarded.txt");
    std::fs::write(&path, "x").unwrap();
    let host_path = "/home/alice/Documents/forwarded.txt";
    rustix::fs::setxattr(
        &path,
        "user.document-portal.host-path",
        host_path.as_bytes(),
        rustix::fs::XattrFlags::empty(),
    )
    .unwrap();
    assert_eq!(
        document_portal_host_path(&path),
        Some(std::path::PathBuf::from(host_path))
    );
    assert_eq!(display_path(&path), std::path::PathBuf::from(host_path));
}

#[test]
#[cfg(target_os = "linux")]
fn a_non_utf8_value_is_still_returned_losslessly() {
    use std::os::unix::ffi::OsStrExt;

    let dir = tmp("doc-portal-non-utf8");
    let path = dir.join("forwarded.txt");
    std::fs::write(&path, "x").unwrap();
    // A lone 0xFF byte is not valid UTF-8 in any position - but Path on
    // Unix makes no UTF-8 promise, so this must still round-trip exactly.
    let raw: &[u8] = b"/home/alice/\xffbroken";
    rustix::fs::setxattr(
        &path,
        "user.document-portal.host-path",
        raw,
        rustix::fs::XattrFlags::empty(),
    )
    .unwrap();
    let got = document_portal_host_path(&path).unwrap();
    assert_eq!(got.as_os_str().as_bytes(), raw);
}

#[test]
#[cfg(target_os = "linux")]
fn a_path_that_is_not_a_document_portal_path_has_no_attribute() {
    // Same precondition as "the attribute absent" above, from a
    // differently-named test matching RFC-095 §4.1's own planned-input
    // list ("a path that is not a document-portal path") - a plain file
    // this process itself created has never been near the portal.
    let dir = tmp("doc-portal-not-a-portal-path");
    let path = dir.join("never-forwarded.txt");
    std::fs::write(&path, "x").unwrap();
    assert_eq!(document_portal_host_path(&path), None);
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
