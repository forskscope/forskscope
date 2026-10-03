//! F138: a 512-byte crafted CFB header used to abort the process with a
//! 9.26 GB allocation. `sheets-diff` 3.2.0 fixed it by declining anything
//! that is not ZIP-magic before the parser sees it. See
//! `tests/fixtures/f138/README.md` for the exact byte layout.
//!
//! **This is its own test binary, not a `#[cfg(test)] mod` in the library —
//! deliberately**, so [`RLIMIT_AS`](libc::RLIMIT_AS) is capped in only this
//! process. `setrlimit` applies to the whole process; setting it in the
//! shared unit-test binary would cap every other test running alongside this
//! one, several of which legitimately allocate hundreds of MB (the F123/F130
//! spreadsheet fixtures) or more.
//!
//! ## Why the cap matters — read this before trusting a green run of this
//! ## test without it
//!
//! Without a capped address space, **this exact regression is invisible**:
//! Linux overcommit grants a 9.26 GB reservation untouched (it is virtual
//! address space, not physical memory, and 64-bit address space is not
//! scarce), and calamine's own parse fails for an unrelated reason a few
//! bytes later — producing the *same* `Err` text as the fixed version. Both
//! confirmed by hand against `sheets-diff` 3.0.0 (the vulnerable version)
//! outside this test: with no cap, the crafted file returns an ordinary `Err`
//! either way; with the address space capped to 2 GB (`ulimit -v 2000000`),
//! 3.0.0 aborts with `memory allocation of 9261023232 bytes failed`, SIGABRT.
//! So the cap below is not a nicety — it is the entire discriminating power
//! of this test. Unlike a `chmod`-based guard, there is no known, legitimate
//! reason `setrlimit` should fail here to *lower* this process's own limit —
//! POSIX permits that unconditionally, root included, unlike raising one
//! past its hard limit — so F143 (handoff 063 §1) makes this fail loudly
//! rather than skip: a refusal here is itself the finding, not a precondition
//! to shrug off.

use std::path::PathBuf;

// 2 GB — comfortably under the 9.26 GB the regression would request,
// comfortably over what an ordinary small-workbook comparison needs. Only
// used by the unix-only test below, which is the only place the cap can be
// applied (`cap_address_space` is itself `#[cfg(unix)]`); cfg-gated the same
// way so a non-unix build (checked on the Windows GNU target in CI) does not
// see it as dead code.
#[cfg(unix)]
const CAP_BYTES: u64 = 2 * 1024 * 1024 * 1024;

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

/// Rebuilds the fixture's bytes from the layout `tests/fixtures/f138/README.md`
/// documents, so the committed binary cannot silently drift from what that
/// table says it is.
fn expected_bytes() -> [u8; 512] {
    let mut b = [0u8; 512];
    b[0..8].copy_from_slice(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
    b[26..28].copy_from_slice(&0x0003u16.to_le_bytes());
    b[30..32].copy_from_slice(&0x0009u16.to_le_bytes());
    b[32..34].copy_from_slice(&0x0006u16.to_le_bytes());
    b[44..48].copy_from_slice(&0x8A000000u32.to_le_bytes());
    b[48..52].copy_from_slice(&0xFFFFFFFEu32.to_le_bytes());
    b[68..72].copy_from_slice(&0xFFFFFFFEu32.to_le_bytes());
    b
}

#[test]
fn the_committed_fixture_matches_its_documented_byte_layout() {
    let on_disk = std::fs::read(fixture("tests/fixtures/f138/oom-artifact-512b.bin")).unwrap();
    assert_eq!(on_disk, expected_bytes());
}

/// Sets this process's own `RLIMIT_AS` (unix only). Returns `false` if the
/// kernel refused it — the caller panics on that (F143): *lowering* a
/// process's own `RLIMIT_AS` never requires privilege and is never refused
/// on a conforming Unix kernel, so there is no known legitimate reason for
/// this to fail, unlike a `chmod`-based guard that root can validly bypass.
#[cfg(unix)]
fn cap_address_space(bytes: u64) -> bool {
    let limit = libc::rlimit {
        rlim_cur: bytes,
        rlim_max: bytes,
    };
    // SAFETY: a plain `setrlimit(2)` call with a stack-local `rlimit` value;
    // no pointers escape, no aliasing. Lowering `RLIMIT_AS` is process-wide
    // and cannot be undone within the process afterwards — exactly why this
    // is an integration test in a file of its own (see the module doc).
    let rc = unsafe { libc::setrlimit(libc::RLIMIT_AS, &limit) };
    rc == 0
}

#[cfg(unix)]
#[test]
fn a_crafted_cfb_header_is_declined_not_allocated() {
    assert!(
        cap_address_space(CAP_BYTES),
        "setrlimit(RLIMIT_AS) was refused - lowering a process's own address \
         space limit should never require privilege or be refused on a \
         conforming Unix kernel, so this is the finding, not a precondition \
         to skip past silently (F143)"
    );

    let bad = fixture("tests/fixtures/f138/oom-artifact-512b.bin");
    // Any small, real workbook; only `bad`'s side matters here. Read through
    // forskscope-core's own xlsx tests fixtures, not a scratch file, so
    // nothing else in this test allocates unexpectedly under the cap.
    let good = fixture("src/tests/fixtures/xlsx/basic/old.xlsx");

    let result = forskscope_core::xlsx::compare_pair(&bad, &good);

    // If the regression reappeared, the attempted 9.26 GB allocation would
    // fail against the 2 GB cap and abort *this test binary* — no assertion
    // downstream of that point would ever run. Reaching this line at all is
    // half the proof; the other half is that the error is the ordinary,
    // pre-parse decline, not some other failure the cap itself provoked.
    let message = match result {
        Ok(_) => panic!("a 512-byte crafted CFB header must never be accepted as a workbook"),
        Err(e) => e.to_string(),
    };
    assert!(
        message.contains("not an xlsx file"),
        "expected the pre-parse ZIP-magic decline, got: {message}"
    );
}
