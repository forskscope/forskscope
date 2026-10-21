//! RFC-094 §4: `show_to_person` "starts the program beside
//! `forskscope-cli`, in the same folder, never one found on `PATH`. It
//! passes the two paths as separate arguments, with no shell." Tested
//! with a fake sibling `forskscope` — a tiny script that logs its own
//! argv — copied beside a copy of the real `forskscope-cli` binary, so
//! this test never opens a real GUI window.
//!
//! Unix only: the fake "GUI" is a shell script, and `PermissionsExt` is
//! Unix-specific. `show_to_person`'s own code has no Unix-only branch
//! (`Command::new`/`arg`/`spawn` are all cross-platform); this test's
//! fake binary is the only reason for the `cfg`.

#![cfg(unix)]

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio};

fn real_bin() -> &'static str {
    env!("CARGO_BIN_EXE_forskscope-cli")
}

fn send(stdin: &mut std::process::ChildStdin, value: serde_json::Value) {
    writeln!(stdin, "{}", serde_json::to_string(&value).unwrap()).unwrap();
}

fn recv(stdout: &mut BufReader<std::process::ChildStdout>) -> serde_json::Value {
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    serde_json::from_str(&line).expect("a JSON-RPC line")
}

#[test]
fn show_to_person_spawns_the_sibling_gui_binary_with_the_two_paths_no_shell() {
    let stage = tempfile::tempdir().unwrap();
    let cli_copy = stage.path().join("forskscope-cli");
    fs::copy(real_bin(), &cli_copy).unwrap();

    let log_path = stage.path().join("fake-gui.log");
    let fake_gui = stage.path().join("forskscope");
    // Logs argv with one marker line per argument, so a shell-injection
    // attempt in a path (e.g. containing `;` or spaces) would show up as
    // extra or malformed lines here rather than being silently absorbed
    // by a shell this call never goes through.
    fs::write(
        &fake_gui,
        format!("#!/bin/sh\nfor a in \"$@\"; do echo \"ARG:$a\"; done > {log_path:?}\n"),
    )
    .unwrap();
    let mut perms = fs::metadata(&fake_gui).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&fake_gui, perms).unwrap();

    let root = tempfile::tempdir().unwrap();
    let left = root.path().join("a.txt");
    let right = root.path().join("b.txt");
    fs::write(&left, "x").unwrap();
    fs::write(&right, "y").unwrap();

    let mut child = Command::new(&cli_copy)
        .args([
            "mcp",
            "--root",
            root.path().to_str().unwrap(),
            "--allow-show",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the copied forskscope-cli should run");

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
            "params": {
                "name": "show_to_person",
                "arguments": {
                    "left": left.to_str().unwrap(),
                    "right": right.to_str().unwrap(),
                }
            }
        }),
    );
    let response = recv(&mut stdout);
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
        "the fake GUI should have received exactly the two paths, as separate arguments"
    );

    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn show_to_person_is_absent_from_tools_list_without_allow_show() {
    let root = tempfile::tempdir().unwrap();
    let mut child = Command::new(real_bin())
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
    let mut child = Command::new(real_bin())
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
