//! Tests for platform diagnostic info (RFC-026).

use crate::platform::PlatformInfo;

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
    assert!(report.contains("ForskScope"), "report must include app name");
    assert!(report.contains("OS:"),        "report must include OS label");
    assert!(report.contains("Arch:"),      "report must include arch label");
}

#[test]
fn home_redacted_does_not_contain_username() {
    let info = PlatformInfo::collect();
    // The redacted home should end with *** or be "unknown"
    if info.home_redacted != "unknown" {
        assert!(info.home_redacted.ends_with("***"),
            "home field must be redacted: {:?}", info.home_redacted);
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
        let n: usize = info.logical_cpus.parse()
            .expect("logical_cpus must be a number or 'unknown'");
        assert!(n >= 1, "must have at least 1 logical CPU");
    }
}

#[test]
fn config_dir_hint_is_non_empty() {
    let info = PlatformInfo::collect();
    assert!(!info.config_dir_hint.is_empty());
}
