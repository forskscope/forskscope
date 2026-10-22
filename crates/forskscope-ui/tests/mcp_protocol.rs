//! `forskscope mcp` protocol tests (RFC-094 §4, review 182 §1):
//! stdio framing, the hostile-input cases review 182 lists by name, root
//! enforcement, and "one comparison runs at a time per server" —
//! including cancellation and the busy rejection. Runs the actual
//! compiled binary and talks JSON-RPC to it over real pipes, the same
//! "run the real program" discipline `compare_exit_codes.rs` already
//! uses for `compare`.
//!
//! RFC-096: moved here from `forskscope-cli`'s own `tests/` — `mcp` is
//! reached as a subcommand of the one program, `forskscope`.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_forskscope")
}

struct Server {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl Server {
    fn start(root: &str) -> Self {
        Self::start_with_args(&["--root", root])
    }

    fn start_with_args(extra: &[&str]) -> Self {
        Self::start_with_args_and_cwd(extra, None)
    }

    fn start_with_args_and_cwd(extra: &[&str], cwd: Option<&std::path::Path>) -> Self {
        let mut args = vec!["mcp"];
        args.extend_from_slice(extra);
        let mut command = Command::new(bin());
        command
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        }
        let mut child = command.spawn().expect("forskscope mcp should start");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            stdin: Some(stdin),
            stdout,
        }
    }

    fn send(&mut self, value: serde_json::Value) {
        let stdin = self.stdin.as_mut().expect("stdin was already closed");
        writeln!(stdin, "{}", serde_json::to_string(&value).unwrap()).unwrap();
    }

    fn send_raw(&mut self, line: &str) {
        let stdin = self.stdin.as_mut().expect("stdin was already closed");
        writeln!(stdin, "{line}").unwrap();
    }

    fn recv(&mut self) -> serde_json::Value {
        let mut line = String::new();
        self.stdout.read_line(&mut line).expect("a response line");
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("not valid JSON: {e}: {line:?}"))
    }

    fn initialize(&mut self) {
        self.send(serde_json::json!({
            "jsonrpc": "2.0", "id": "init", "method": "initialize",
            "params": {"protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"name": "test", "version": "0"}}
        }));
        let resp = self.recv();
        assert_eq!(resp["result"]["protocolVersion"], "2025-11-25");
        self.send(serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
    }

    /// Closes the server's stdin, the way a real client does when it is
    /// finished with it - review 183 §2.4's own trigger ("when standard
    /// input reaches end of file").
    fn close_stdin(&mut self) {
        self.stdin = None;
    }

    /// Polls for the child's own exit, without `kill`ing it, up to
    /// `timeout`. `None` means it was still running when the budget ran
    /// out.
    fn wait_for_exit(&mut self, timeout: std::time::Duration) -> Option<std::process::ExitStatus> {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            if let Ok(Some(status)) = self.child.try_wait() {
                return Some(status);
            }
            if std::time::Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn file_pair(dir: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let left = dir.join("left.txt");
    let right = dir.join("right.txt");
    fs::write(&left, "one\ntwo\nthree\n").unwrap();
    fs::write(&right, "one\nTWO\nthree\n").unwrap();
    (left, right)
}

// ── Protocol basics ──────────────────────────────────────────────────────────

#[test]
fn initialize_names_the_decided_protocol_revision_and_the_server() {
    let root = tempfile::tempdir().unwrap();
    let mut server = Server::start(root.path().to_str().unwrap());
    server.send(serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}
    }));
    let resp = server.recv();
    assert_eq!(resp["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(resp["result"]["serverInfo"]["name"], "forskscope");
    assert_eq!(
        resp["result"]["capabilities"]["tools"],
        serde_json::json!({})
    );
}

#[test]
fn a_client_asking_for_2025_06_18_is_answered_with_2025_06_18() {
    let root = tempfile::tempdir().unwrap();
    let mut server = Server::start(root.path().to_str().unwrap());
    server.send(serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "t", "version": "0"}}
    }));
    let resp = server.recv();
    assert_eq!(resp["result"]["protocolVersion"], "2025-06-18", "{resp}");
}

#[test]
fn a_client_asking_for_anything_else_is_answered_with_2025_11_25() {
    let root = tempfile::tempdir().unwrap();
    for requested in ["2024-11-05", "2025-03-26", "2026-07-28", "nonsense"] {
        let mut server = Server::start(root.path().to_str().unwrap());
        server.send(serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {"protocolVersion": requested, "capabilities": {}, "clientInfo": {"name": "t", "version": "0"}}
        }));
        let resp = server.recv();
        assert_eq!(
            resp["result"]["protocolVersion"], "2025-11-25",
            "requested {requested}: {resp}"
        );
    }
}

#[test]
fn ping_answers_with_an_empty_result() {
    let root = tempfile::tempdir().unwrap();
    let mut server = Server::start(root.path().to_str().unwrap());
    server.initialize();
    server.send(serde_json::json!({"jsonrpc": "2.0", "id": 2, "method": "ping"}));
    let resp = server.recv();
    assert_eq!(resp["result"], serde_json::json!({}));
}

#[test]
fn tools_list_describes_compare_with_a_plain_untrusted_content_notice() {
    let root = tempfile::tempdir().unwrap();
    let mut server = Server::start(root.path().to_str().unwrap());
    server.initialize();
    server.send(serde_json::json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}));
    let resp = server.recv();
    let tools = resp["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 1);
    let compare = &tools[0];
    assert_eq!(compare["name"], "compare");
    let description = compare["description"].as_str().unwrap();
    assert!(
        description.contains("data") && description.contains("never instructions"),
        "description must say plainly that compared content is data, not instructions: {description}"
    );
    let required = compare["inputSchema"]["required"].as_array().unwrap();
    assert!(required.iter().any(|v| v == "left"));
    assert!(required.iter().any(|v| v == "right"));
}

#[test]
fn compare_tool_call_returns_structured_content_for_json_and_the_same_text() {
    let root = tempfile::tempdir().unwrap();
    let (left, right) = file_pair(root.path());
    let mut server = Server::start(root.path().to_str().unwrap());
    server.initialize();
    server.send(serde_json::json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": {"name": "compare", "arguments": {
            "left": left.to_str().unwrap(), "right": right.to_str().unwrap()
        }}
    }));
    let resp = server.recv();
    assert_eq!(resp["result"]["isError"], false, "{resp}");
    let structured = &resp["result"]["structuredContent"];
    assert_eq!(structured["schema"], "forskscope.comparison/1");
    assert_eq!(structured["identical"], false);
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed_from_text: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(
        &parsed_from_text, structured,
        "content text and structuredContent must agree"
    );
}

#[test]
fn compare_tool_call_with_format_text_has_no_structured_content() {
    let root = tempfile::tempdir().unwrap();
    let (left, right) = file_pair(root.path());
    let mut server = Server::start(root.path().to_str().unwrap());
    server.initialize();
    server.send(serde_json::json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": {"name": "compare", "arguments": {
            "left": left.to_str().unwrap(), "right": right.to_str().unwrap(), "format": "text"
        }}
    }));
    let resp = server.recv();
    assert_eq!(resp["result"]["isError"], false, "{resp}");
    assert!(resp["result"].get("structuredContent").is_none());
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("− two"));
    assert!(text.contains("+ TWO"));
}

// ── Hostile input (review 182 §1): never crashes the server ───────────────────

#[test]
fn a_malformed_line_gets_a_parse_error_and_the_server_keeps_running() {
    let root = tempfile::tempdir().unwrap();
    let mut server = Server::start(root.path().to_str().unwrap());
    server.initialize();
    server.send_raw("{ this is not json");
    let resp = server.recv();
    assert_eq!(resp["error"]["code"], -32700);

    server.send(serde_json::json!({"jsonrpc": "2.0", "id": 9, "method": "ping"}));
    let resp = server.recv();
    assert_eq!(
        resp["result"],
        serde_json::json!({}),
        "server must still answer: {resp}"
    );
}

#[test]
fn an_unknown_method_gets_method_not_found() {
    let root = tempfile::tempdir().unwrap();
    let mut server = Server::start(root.path().to_str().unwrap());
    server.initialize();
    server.send(serde_json::json!({"jsonrpc": "2.0", "id": 9, "method": "no/such/method"}));
    let resp = server.recv();
    assert_eq!(resp["error"]["code"], -32601);
}

#[test]
fn an_unknown_notification_with_no_id_is_silently_ignored() {
    let root = tempfile::tempdir().unwrap();
    let mut server = Server::start(root.path().to_str().unwrap());
    server.initialize();
    server.send(serde_json::json!({"jsonrpc": "2.0", "method": "no/such/notification"}));
    // No response should ever arrive for it - the very next line read
    // must be the ping's own response, not an error about the notification.
    server.send(serde_json::json!({"jsonrpc": "2.0", "id": 9, "method": "ping"}));
    let resp = server.recv();
    assert_eq!(resp["id"], 9);
}

#[test]
fn wrong_parameter_types_are_reported_as_a_tool_error_not_a_crash() {
    let root = tempfile::tempdir().unwrap();
    let mut server = Server::start(root.path().to_str().unwrap());
    server.initialize();
    server.send(serde_json::json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": {"name": "compare", "arguments": {"left": 42, "right": "x"}}
    }));
    let resp = server.recv();
    assert_eq!(resp["result"]["isError"], true, "{resp}");
    assert!(
        resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("must be a string")
    );
}

#[test]
fn a_message_over_the_4mib_cap_is_a_parse_error_and_the_server_keeps_running() {
    let root = tempfile::tempdir().unwrap();
    let mut server = Server::start(root.path().to_str().unwrap());
    server.initialize();
    let huge = "x".repeat(5 * 1024 * 1024);
    server.send_raw(&huge);
    let resp = server.recv();
    assert_eq!(resp["error"]["code"], -32700);
    assert!(resp["error"]["message"].as_str().unwrap().contains("4 MiB"));

    server.send(serde_json::json!({"jsonrpc": "2.0", "id": 9, "method": "ping"}));
    let resp = server.recv();
    assert_eq!(
        resp["result"],
        serde_json::json!({}),
        "server must still answer: {resp}"
    );
}

// ── Roots (RFC-094 §4, acceptance criterion 4) ─────────────────────────────────

#[test]
fn a_path_outside_every_root_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    let (_, right) = file_pair(root.path());
    let outside_left = elsewhere.path().join("left.txt");
    fs::write(&outside_left, "x").unwrap();

    let mut server = Server::start(root.path().to_str().unwrap());
    server.initialize();
    server.send(serde_json::json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": {"name": "compare", "arguments": {
            "left": outside_left.to_str().unwrap(), "right": right.to_str().unwrap()
        }}
    }));
    let resp = server.recv();
    assert_eq!(resp["result"]["isError"], true, "{resp}");
    assert!(
        resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("outside every configured --root directory")
    );
}

#[test]
fn a_path_escaping_the_root_through_dot_dot_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let inside = root.path().join("inside");
    fs::create_dir(&inside).unwrap();
    let (left, right) = file_pair(&inside);
    let escaping_right = inside.join("..").join(right.file_name().unwrap());

    let mut server = Server::start(inside.to_str().unwrap());
    server.initialize();
    server.send(serde_json::json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": {"name": "compare", "arguments": {
            "left": left.to_str().unwrap(), "right": escaping_right.to_str().unwrap()
        }}
    }));
    let resp = server.recv();
    assert_eq!(resp["result"]["isError"], true, "{resp}");
}

#[cfg(unix)]
#[test]
fn a_path_escaping_the_root_through_a_symlink_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let (left, _) = file_pair(root.path());
    let secret = outside.path().join("secret.txt");
    fs::write(&secret, "s").unwrap();
    let link = root.path().join("escape.txt");
    std::os::unix::fs::symlink(&secret, &link).unwrap();

    let mut server = Server::start(root.path().to_str().unwrap());
    server.initialize();
    server.send(serde_json::json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": {"name": "compare", "arguments": {
            "left": left.to_str().unwrap(), "right": link.to_str().unwrap()
        }}
    }));
    let resp = server.recv();
    assert_eq!(resp["result"]["isError"], true, "{resp}");
}

#[cfg(unix)]
#[test]
fn a_symlink_inside_a_compared_folder_is_not_followed_and_its_target_content_is_absent() {
    // Review 183 §0/§2.5: the engine's own F80 rule ("ForskScope does not
    // follow cross-root symlinks") already holds for a folder compared
    // over MCP, checked live by hand - this is the test for it. The
    // symlink is one-sided (left only) on purpose: `RecStatus::Symlink`
    // takes precedence over "left only" for a path that is a symlink on
    // either side, per its own doc comment.
    //
    // Unix only: `std::os::unix::fs::symlink` has no portable
    // equivalent in this test file (Windows needs
    // `symlink_file`/`symlink_dir` chosen by target type, and needs the
    // process to hold the privilege to create one) - the engine's own
    // F80 behavior this proves is platform-independent; only the test's
    // own setup is Unix-specific.
    let root = tempfile::tempdir().unwrap();
    let left_dir = root.path().join("left");
    let right_dir = root.path().join("right");
    fs::create_dir(&left_dir).unwrap();
    fs::create_dir(&right_dir).unwrap();
    fs::write(left_dir.join("ok.txt"), "same\n").unwrap();
    fs::write(right_dir.join("ok.txt"), "same\n").unwrap();

    let outside = tempfile::tempdir().unwrap();
    let secret = outside.path().join("secret.txt");
    fs::write(&secret, "SECRET-CONTENT-MUST-NOT-APPEAR\n").unwrap();
    let link = left_dir.join("link.txt");
    std::os::unix::fs::symlink(&secret, &link).unwrap();

    let mut server = Server::start(root.path().to_str().unwrap());
    server.initialize();
    server.send(serde_json::json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": {"name": "compare", "arguments": {
            "left": left_dir.to_str().unwrap(), "right": right_dir.to_str().unwrap()
        }}
    }));
    let resp = server.recv();
    assert_eq!(resp["result"]["isError"], false, "{resp}");
    let structured = &resp["result"]["structuredContent"];
    let entries = structured["entries"].as_array().unwrap();
    let link_entry = entries
        .iter()
        .find(|e| e["path"] == "link.txt")
        .unwrap_or_else(|| panic!("the symlink must be listed as its own entry: {structured}"));
    assert_eq!(link_entry["status"], "symlink_not_followed", "{link_entry}");

    let whole_document = serde_json::to_string(structured).unwrap();
    assert!(
        !whole_document.contains("SECRET-CONTENT-MUST-NOT-APPEAR"),
        "the symlink's target content must never appear in the result: {whole_document}"
    );
}

#[test]
fn a_relative_path_resolves_against_the_servers_cwd_and_escaping_that_way_is_refused() {
    // RFC-094 §4 / review 183 §2.5: a relative path is resolved the same
    // way `canonicalize` resolves any relative path - against the
    // process's own current working directory, never against a root.
    // One that escapes every root that way must be refused exactly like
    // an absolute one.
    let root = tempfile::tempdir().unwrap();
    let (left, _) = file_pair(root.path());

    let cwd = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret.txt"), "s").unwrap();
    let relative_escape = format!(
        "../{}/secret.txt",
        outside.path().file_name().unwrap().to_str().unwrap()
    );
    // `cwd` and `outside` are siblings (both direct children of the
    // system temp directory), so this really does resolve outside the
    // sandbox root above once canonicalized.
    assert!(
        cwd.path().parent() == outside.path().parent(),
        "test assumption: cwd and outside must be siblings"
    );

    let mut server = Server::start_with_args_and_cwd(
        &["--root", root.path().to_str().unwrap()],
        Some(cwd.path()),
    );
    server.initialize();
    server.send(serde_json::json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": {"name": "compare", "arguments": {
            "left": left.to_str().unwrap(), "right": relative_escape
        }}
    }));
    let resp = server.recv();
    assert_eq!(resp["result"]["isError"], true, "{resp}");
    assert!(
        resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("outside every configured --root directory"),
        "{resp}"
    );
}

// ── "One comparison runs at a time" + cancellation ─────────────────────────────

/// Enough files that a folder comparison takes a real, measurable amount
/// of time - otherwise a race against a second request or a cancellation
/// notification would be flaky by construction, not a real test.
fn slow_folder_pair() -> (tempfile::TempDir, tempfile::TempDir) {
    let left = tempfile::tempdir().unwrap();
    let right = tempfile::tempdir().unwrap();
    for i in 0..6000 {
        fs::write(
            left.path().join(format!("f{i}.txt")),
            format!("original {i}\n"),
        )
        .unwrap();
        fs::write(
            right.path().join(format!("f{i}.txt")),
            format!("changed {i}\n"),
        )
        .unwrap();
    }
    (left, right)
}

#[test]
fn a_second_compare_while_one_runs_is_rejected_busy_and_the_first_still_completes() {
    let (left, right) = slow_folder_pair();
    let root = tempfile::tempdir().unwrap();
    let mut server = Server::start_with_args(&[
        "--root",
        left.path().to_str().unwrap(),
        "--root",
        right.path().to_str().unwrap(),
        "--root",
        root.path().to_str().unwrap(),
    ]);
    server.initialize();

    server.send(serde_json::json!({
        "jsonrpc": "2.0", "id": "first", "method": "tools/call",
        "params": {"name": "compare", "arguments": {
            "left": left.path().to_str().unwrap(), "right": right.path().to_str().unwrap()
        }}
    }));
    server.send(serde_json::json!({
        "jsonrpc": "2.0", "id": "second", "method": "tools/call",
        "params": {"name": "compare", "arguments": {
            "left": left.path().to_str().unwrap(), "right": right.path().to_str().unwrap()
        }}
    }));

    let r1 = server.recv();
    let r2 = server.recv();
    let (busy, completed) = if r1["id"] == "second" {
        (r1, r2)
    } else {
        (r2, r1)
    };

    assert_eq!(busy["error"]["code"], -32000, "{busy}");
    assert_eq!(completed["id"], "first");
    assert_eq!(completed["result"]["isError"], false, "{completed}");
}

#[test]
fn cancelling_an_in_flight_comparison_stops_it_with_no_response_at_all() {
    // Review 183 §2.2: 2025-11-25 says a receiver of
    // `notifications/cancelled` SHOULD NOT send a response for the
    // cancelled request - so there is nothing to `recv()` for "cmp1"
    // itself. What proves the comparison actually stopped (not just that
    // the server went quiet) is everything *after* the cancellation:
    // `ping` is still answered, and a brand new `compare` is accepted
    // (not refused as "Busy") well before the slow pair would have
    // finished on its own.
    let (left, right) = slow_folder_pair();
    let mut server = Server::start_with_args(&[
        "--root",
        left.path().to_str().unwrap(),
        "--root",
        right.path().to_str().unwrap(),
    ]);
    server.initialize();

    server.send(serde_json::json!({
        "jsonrpc": "2.0", "id": "cmp1", "method": "tools/call",
        "params": {"name": "compare", "arguments": {
            "left": left.path().to_str().unwrap(), "right": right.path().to_str().unwrap()
        }}
    }));
    server.send(serde_json::json!({
        "jsonrpc": "2.0", "method": "notifications/cancelled",
        "params": {"requestId": "cmp1"}
    }));
    server.send(serde_json::json!({"jsonrpc": "2.0", "id": "ping1", "method": "ping"}));

    let after_cancel = server.recv();
    assert_eq!(
        after_cancel["id"], "ping1",
        "ping must still be answered right after cancelling: {after_cancel}"
    );

    // The worker thread notices the cancellation and releases the slot
    // on its own schedule, not instantly - poll with short-lived probe
    // calls (each either an immediate "Busy" or a real, accepted
    // comparison) until one is accepted, bounded well under the time the
    // slow pair takes to run to completion on its own (~0.4-0.5s
    // locally), so this proves the slot freed promptly, not merely
    // "eventually, after the original walk would have finished anyway."
    let budget = std::time::Instant::now() + std::time::Duration::from_millis(300);
    let mut accepted_id = None;
    let mut probe = 0;
    while std::time::Instant::now() < budget {
        probe += 1;
        let id = format!("probe{probe}");
        let ping_id = format!("ping-{id}");
        server.send(serde_json::json!({
            "jsonrpc": "2.0", "id": id, "method": "tools/call",
            "params": {"name": "compare", "arguments": {
                "left": left.path().to_str().unwrap(), "right": right.path().to_str().unwrap()
            }}
        }));
        server.send(serde_json::json!({"jsonrpc": "2.0", "id": ping_id.clone(), "method": "ping"}));

        let first = server.recv();
        if first["id"] == id {
            // Refused as Busy, synchronously - its own ping answers next.
            assert_eq!(first["error"]["code"], -32000, "{first}");
            let ping_answer = server.recv();
            assert_eq!(ping_answer["id"], ping_id);
            continue;
        }
        // Accepted: nothing was written for `id` yet, so the ping
        // dispatched right behind it answered first.
        assert_eq!(first["id"], ping_id, "{first}");
        accepted_id = Some(id);
        break;
    }
    let accepted_id =
        accepted_id.expect("a compare should have been accepted well within the budget");

    let result = server.recv();
    assert_eq!(result["id"], accepted_id);
    assert_eq!(result["result"]["isError"], false, "{result}");
    assert_ne!(after_cancel["id"], "cmp1");
    assert_ne!(result["id"], "cmp1");
}

#[test]
fn closing_stdin_cancels_a_running_comparison_so_the_server_exits_promptly() {
    // Review 183 §2.4: "the client has gone, so nobody will read the
    // answer. A large folder walk keeps the process alive for nothing."
    // Without cancelling on EOF, this test would take as long as the
    // slow pair's own full walk (~0.4-0.5s locally) to exit, if it exited
    // at all before the process-level `kill` in `Server::drop` stepped
    // in; with it, closing stdin should make the server notice almost
    // immediately.
    let (left, right) = slow_folder_pair();
    let mut server = Server::start_with_args(&[
        "--root",
        left.path().to_str().unwrap(),
        "--root",
        right.path().to_str().unwrap(),
    ]);
    server.initialize();

    server.send(serde_json::json!({
        "jsonrpc": "2.0", "id": "cmp", "method": "tools/call",
        "params": {"name": "compare", "arguments": {
            "left": left.path().to_str().unwrap(), "right": right.path().to_str().unwrap()
        }}
    }));
    // Give the worker thread a moment to actually start walking, so this
    // proves cancelling a comparison already in flight, not merely one
    // that never got the chance to begin.
    std::thread::sleep(std::time::Duration::from_millis(5));

    let start = std::time::Instant::now();
    server.close_stdin();
    let status = server.wait_for_exit(std::time::Duration::from_secs(2));
    let elapsed = start.elapsed();

    assert!(
        status.is_some(),
        "the server must exit on its own once stdin closes, not hang until killed"
    );
    assert!(
        elapsed < std::time::Duration::from_millis(300),
        "closing stdin should cancel the running comparison and exit promptly, took {elapsed:?}"
    );
}

#[test]
fn a_comparison_completes_normally_once_the_previous_one_is_done() {
    // Guards the busy test above: confirms the slot really is freed after
    // a comparison finishes, not left permanently "busy".
    let root = tempfile::tempdir().unwrap();
    let (left, right) = file_pair(root.path());
    let mut server = Server::start(root.path().to_str().unwrap());
    server.initialize();

    for id in 0..2 {
        server.send(serde_json::json!({
            "jsonrpc": "2.0", "id": id, "method": "tools/call",
            "params": {"name": "compare", "arguments": {
                "left": left.to_str().unwrap(), "right": right.to_str().unwrap()
            }}
        }));
        let resp = server.recv();
        assert_eq!(resp["result"]["isError"], false, "run {id}: {resp}");
    }
}

#[test]
fn a_tight_back_to_back_loop_of_compares_never_sees_busy() {
    // Review 183 §2.3: "a client that sends its next compare the moment
    // it reads the answer can be refused as Busy, even though nothing is
    // running" was the defect - the slot used to be released *after* the
    // response was written, so a client fast enough to react to its own
    // answer could race the server's own cleanup. Releasing the slot
    // first (this reply's fix) means sending the next call the instant
    // this one's answer arrives must never be refused.
    let root = tempfile::tempdir().unwrap();
    let (left, right) = file_pair(root.path());
    let mut server = Server::start(root.path().to_str().unwrap());
    server.initialize();

    for id in 0..50 {
        server.send(serde_json::json!({
            "jsonrpc": "2.0", "id": id, "method": "tools/call",
            "params": {"name": "compare", "arguments": {
                "left": left.to_str().unwrap(), "right": right.to_str().unwrap()
            }}
        }));
        let resp = server.recv();
        assert_eq!(resp["id"], id);
        assert!(resp.get("error").is_none(), "run {id} was refused: {resp}");
        assert_eq!(resp["result"]["isError"], false, "run {id}: {resp}");
    }
}

// ── Startup ──────────────────────────────────────────────────────────────────

#[test]
fn a_nonexistent_root_exits_two_before_the_server_ever_starts() {
    let output = Command::new(bin())
        .args(["mcp", "--root", "/definitely/does/not/exist/anywhere"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(
        output.stdout.is_empty(),
        "nothing but protocol messages on stdout, and the server never started"
    );
}

#[test]
fn no_root_at_all_exits_two() {
    let output = Command::new(bin()).args(["mcp"]).output().unwrap();
    assert_eq!(output.status.code(), Some(2));
}
