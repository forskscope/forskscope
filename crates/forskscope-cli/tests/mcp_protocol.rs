//! `forskscope-cli mcp` protocol tests (RFC-094 §4, review 182 §1):
//! stdio framing, the hostile-input cases review 182 lists by name, root
//! enforcement, and "one comparison runs at a time per server" —
//! including cancellation and the busy rejection. Runs the actual
//! compiled binary and talks JSON-RPC to it over real pipes, the same
//! "run the real program" discipline `compare_exit_codes.rs` already
//! uses for `compare`.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_forskscope-cli")
}

struct Server {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Server {
    fn start(root: &str) -> Self {
        Self::start_with_args(&["--root", root])
    }

    fn start_with_args(extra: &[&str]) -> Self {
        let mut args = vec!["mcp"];
        args.extend_from_slice(extra);
        let mut child = Command::new(bin())
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("forskscope-cli mcp should start");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            stdin,
            stdout,
        }
    }

    fn send(&mut self, value: serde_json::Value) {
        writeln!(self.stdin, "{}", serde_json::to_string(&value).unwrap()).unwrap();
    }

    fn send_raw(&mut self, line: &str) {
        writeln!(self.stdin, "{line}").unwrap();
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
    assert_eq!(resp["result"]["serverInfo"]["name"], "forskscope-cli");
    assert_eq!(
        resp["result"]["capabilities"]["tools"],
        serde_json::json!({})
    );
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
fn cancelling_an_in_flight_comparison_stops_it() {
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
    server.send(serde_json::json!({
        "jsonrpc": "2.0", "method": "notifications/cancelled",
        "params": {"requestId": "cmp"}
    }));

    let resp = server.recv();
    assert_eq!(resp["id"], "cmp");
    assert_eq!(resp["result"]["isError"], true, "{resp}");
    assert!(
        resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("cancelled")
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
