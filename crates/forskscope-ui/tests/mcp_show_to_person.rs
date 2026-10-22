//! RFC-096 §3: `show_to_person` starts `std::env::current_exe()` — the
//! same executable — with the two paths as separate arguments and no
//! subcommand. A test spawning the real compiled `forskscope` binary as
//! `mcp --allow-show` cannot fake that by placing a stand-in sibling
//! binary next to a copy any more (RFC-094's "beside it" lookup, and the
//! separate sibling binary it found, are both gone — see
//! `forskscope-commands`'s own `mcp/tools.rs` for the seam this uses
//! instead: one environment variable, read only by `call_show_to_person`,
//! overriding what executable it starts). Every test below sets it to a
//! tiny stand-in script, so this file never opens a real GUI window.
//!
//! RFC-096: moved here from `forskscope-cli`'s own `tests/`, adapted —
//! no more `fs::copy`/sibling-binary setup (the real compiled binary is
//! run directly, the same as `mcp_protocol.rs` already does). The old
//! `fs::copy`-then-spawn race is gone, but review 188's own follow-up
//! found an analogous one: the real server process can transiently fail
//! to exec a stand-in script this test just wrote and `chmod`'d, under
//! full workspace-test load, the same `ExecutableFileBusy` ("Text file
//! busy") the old retry targeted. `call_show_to_person` itself never
//! retries — a real launch failure stays a tool error — so the retry
//! lives here instead, around the whole `tools/call` round trip
//! ([`call_show_to_person_retrying_busy`]).
//!
//! Unix only: the stand-in "GUI" is a shell script, and `PermissionsExt`
//! is Unix-specific. `show_to_person`'s own code has no Unix-only branch
//! (`Command::new`/`arg`/`spawn`/`env` are all cross-platform); this
//! test's own stand-in script is the only reason for the `cfg`.

#![cfg(unix)]

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_forskscope")
}

fn send(stdin: &mut std::process::ChildStdin, value: serde_json::Value) {
    writeln!(stdin, "{}", serde_json::to_string(&value).unwrap()).unwrap();
}

fn recv(stdout: &mut BufReader<std::process::ChildStdout>) -> serde_json::Value {
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    serde_json::from_str(&line).expect("a JSON-RPC line")
}

/// Starts `forskscope mcp --allow-show --root <root>`, with
/// `FORSKSCOPE_SHOW_TO_PERSON_EXE_FOR_TEST` pointed at `stand_in_gui` —
/// `show_to_person` starts that script instead of `current_exe()`, so
/// this never opens a real window.
fn start_server(root: &std::path::Path, stand_in_gui: &std::path::Path) -> std::process::Child {
    Command::new(bin())
        .args(["mcp", "--root", root.to_str().unwrap(), "--allow-show"])
        .env("FORSKSCOPE_SHOW_TO_PERSON_EXE_FOR_TEST", stand_in_gui)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("forskscope mcp should start")
}

fn write_stand_in_gui(path: &std::path::Path, script: &str) {
    fs::write(path, script).unwrap();
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}

/// Review 188's own follow-up: sends `show_to_person` and, if the server
/// reports a launch failure that is the transient `ExecutableFileBusy`
/// ("Text file busy") race rather than a real one, sends it again —
/// bounded at 5 attempts with a short backoff, the same shape as
/// `forskscope-commands`'s own
/// `call_show_to_person_starting_retrying_busy`. The server itself never
/// retries internally; this is the test retrying the call, not the
/// product retrying the launch.
fn call_show_to_person_retrying_busy(
    stdin: &mut std::process::ChildStdin,
    stdout: &mut BufReader<std::process::ChildStdout>,
    id: i64,
    left: &str,
    right: &str,
) -> serde_json::Value {
    let busy = std::io::Error::from_raw_os_error(26).to_string();
    for attempt in 1..=5u64 {
        send(
            stdin,
            serde_json::json!({
                "jsonrpc": "2.0", "id": id, "method": "tools/call",
                "params": {"name": "show_to_person", "arguments": {"left": left, "right": right}}
            }),
        );
        let response = recv(stdout);
        let is_busy = response["result"]["isError"] == true
            && response["result"]["content"][0]["text"]
                .as_str()
                .is_some_and(|t| t.contains(&busy));
        if is_busy && attempt < 5 {
            std::thread::sleep(std::time::Duration::from_millis(20 * attempt));
            continue;
        }
        return response;
    }
    unreachable!()
}

#[test]
fn show_to_person_starts_the_stand_in_with_the_two_paths_no_shell() {
    let stage = tempfile::tempdir().unwrap();
    let log_path = stage.path().join("fake-gui.log");
    let fake_gui = stage.path().join("fake-gui.sh");
    // Logs argv with one marker line per argument, so a shell-injection
    // attempt in a path (e.g. containing `;` or spaces) would show up as
    // extra or malformed lines here rather than being silently absorbed
    // by a shell this call never goes through.
    write_stand_in_gui(
        &fake_gui,
        &format!("#!/bin/sh\nfor a in \"$@\"; do echo \"ARG:$a\"; done > {log_path:?}\n"),
    );

    let root = tempfile::tempdir().unwrap();
    let left = root.path().join("a.txt");
    let right = root.path().join("b.txt");
    fs::write(&left, "x").unwrap();
    fs::write(&right, "y").unwrap();

    let mut child = start_server(root.path(), &fake_gui);
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    send(
        &mut stdin,
        serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
    );
    recv(&mut stdout);

    let response = call_show_to_person_retrying_busy(
        &mut stdin,
        &mut stdout,
        2,
        left.to_str().unwrap(),
        right.to_str().unwrap(),
    );
    assert_eq!(
        response["result"]["isError"], false,
        "show_to_person should succeed: {response}"
    );

    let mut logged = String::new();
    for _ in 0..100 {
        if let Ok(contents) = fs::read_to_string(&log_path) {
            logged = contents;
            if !logged.is_empty() {
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!(
        logged,
        format!("ARG:{}\nARG:{}\n", left.display(), right.display()),
        "the stand-in GUI should have received exactly the two paths, as separate arguments"
    );

    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn a_gui_that_writes_to_its_own_stdout_never_corrupts_the_protocol_stream() {
    // Review 183 §1: the child used to inherit the server's own stdout -
    // its protocol channel. A stand-in GUI that writes noise to stdout
    // (the way a stray println!, a GLib warning, or g_print could) must
    // never have that noise read back as a JSON-RPC message.
    let stage = tempfile::tempdir().unwrap();
    let fake_gui = stage.path().join("fake-gui.sh");
    write_stand_in_gui(
        &fake_gui,
        "#!/bin/sh\necho GUI-STDOUT-NOISE this is not json\n",
    );

    let root = tempfile::tempdir().unwrap();
    let left = root.path().join("a.txt");
    let right = root.path().join("b.txt");
    fs::write(&left, "x").unwrap();
    fs::write(&right, "y").unwrap();

    let mut child = start_server(root.path(), &fake_gui);
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    send(
        &mut stdin,
        serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
    );
    recv(&mut stdout);

    let show_response = call_show_to_person_retrying_busy(
        &mut stdin,
        &mut stdout,
        2,
        left.to_str().unwrap(),
        right.to_str().unwrap(),
    );
    assert_eq!(show_response["id"], 2);
    assert_eq!(show_response["result"]["isError"], false, "{show_response}");

    // The very next line read, after whatever the stand-in GUI printed,
    // must be the ping's own answer - not the GUI's noise, and not
    // garbage.
    send(
        &mut stdin,
        serde_json::json!({"jsonrpc": "2.0", "id": 3, "method": "ping"}),
    );
    let ping_response = recv(&mut stdout);
    assert_eq!(
        ping_response,
        serde_json::json!({"jsonrpc": "2.0", "id": 3, "result": {}}),
        "the GUI's stdout noise must never reach the protocol stream"
    );

    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn a_gui_that_exits_non_zero_quickly_is_reported_as_an_error() {
    let stage = tempfile::tempdir().unwrap();
    let fake_gui = stage.path().join("fake-gui.sh");
    write_stand_in_gui(&fake_gui, "#!/bin/sh\nexit 1\n");

    let root = tempfile::tempdir().unwrap();
    let left = root.path().join("a.txt");
    let right = root.path().join("b.txt");
    fs::write(&left, "x").unwrap();
    fs::write(&right, "y").unwrap();

    let mut child = start_server(root.path(), &fake_gui);
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    send(
        &mut stdin,
        serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
    );
    recv(&mut stdout);

    let response = call_show_to_person_retrying_busy(
        &mut stdin,
        &mut stdout,
        2,
        left.to_str().unwrap(),
        right.to_str().unwrap(),
    );
    assert_eq!(response["result"]["isError"], true, "{response}");
    assert!(
        response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("exit code 1"),
        "{response}"
    );

    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn a_ping_sent_right_after_show_to_person_is_answered_well_under_its_own_early_exit_window() {
    // Review 184 §3.1: call_show_to_person's own early-exit wait (up to
    // EARLY_EXIT_WINDOW, 2s) used to run on the dispatch thread - the
    // same thread that answers every other message - so a ping sent
    // right after it waited out the window too. A stand-in GUI that
    // stays alive (never exits within the window) forces the full wait;
    // this proves ping is answered immediately regardless, not merely
    // that it happens to be fast in the common case.
    let stage = tempfile::tempdir().unwrap();
    let fake_gui = stage.path().join("fake-gui.sh");
    write_stand_in_gui(&fake_gui, "#!/bin/sh\nsleep 10\n");

    let root = tempfile::tempdir().unwrap();
    let left = root.path().join("a.txt");
    let right = root.path().join("b.txt");
    fs::write(&left, "x").unwrap();
    fs::write(&right, "y").unwrap();

    let mut child = start_server(root.path(), &fake_gui);
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    send(
        &mut stdin,
        serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
    );
    recv(&mut stdout);

    send(
        &mut stdin,
        serde_json::json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": {"name": "show_to_person", "arguments": {
                "left": left.to_str().unwrap(), "right": right.to_str().unwrap()
            }}
        }),
    );
    let start = std::time::Instant::now();
    send(
        &mut stdin,
        serde_json::json!({"jsonrpc": "2.0", "id": 3, "method": "ping"}),
    );
    let response = recv(&mut stdout);
    let elapsed = start.elapsed();

    assert_eq!(
        response,
        serde_json::json!({"jsonrpc": "2.0", "id": 3, "result": {}}),
        "the first line back must be ping's own answer, not show_to_person's \
         (still running against a GUI that stays alive): {response}"
    );
    assert!(
        elapsed < std::time::Duration::from_millis(500),
        "ping must not wait out show_to_person's own early-exit window: {elapsed:?}"
    );

    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn show_to_person_is_absent_from_tools_list_without_allow_show() {
    let root = tempfile::tempdir().unwrap();
    let mut child = Command::new(bin())
        .args(["mcp", "--root", root.path().to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    send(
        &mut stdin,
        serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
    );
    recv(&mut stdout);

    send(
        &mut stdin,
        serde_json::json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
    );
    let response = recv(&mut stdout);
    let names: Vec<&str> = response["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["compare"], "no --allow-show, no show_to_person");

    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn show_to_person_is_refused_when_called_without_allow_show() {
    let root = tempfile::tempdir().unwrap();
    let mut child = Command::new(bin())
        .args(["mcp", "--root", root.path().to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    send(
        &mut stdin,
        serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
    );
    recv(&mut stdout);

    send(
        &mut stdin,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {"name": "show_to_person", "arguments": {"left": "a", "right": "b"}}
        }),
    );
    let response = recv(&mut stdout);
    assert!(response.get("error").is_some(), "{response}");
    assert_eq!(response["error"]["code"], -32602);

    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
}
