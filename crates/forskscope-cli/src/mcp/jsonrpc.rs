//! The stdio JSON-RPC loop (RFC-094 §4, review 182 §1): one message per
//! line, UTF-8, nothing but protocol messages on stdout, diagnostics on
//! stderr. Hand-written, no async runtime — a background thread reads
//! stdin into an `mpsc` channel; the dispatch thread (this module's main
//! loop) handles everything synchronously except `compare`, which runs
//! on its own thread so a long folder walk never blocks `ping`/
//! `tools/list`/cancellation from being answered. "One comparison runs
//! at a time per server" (RFC-094 §4) is enforced by `in_flight` below,
//! not by blocking: a second `compare` while one is running is refused
//! immediately, never queued.

use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use forskscope_compare::CancellationToken;
use serde_json::{Value, json};

use super::roots::Roots;
use super::tools;

/// RFC-094 §4, 2026-10-21 clarification (review 182 §1): the newest
/// revision that still has an `initialize` handshake — the actual
/// latest, 2026-07-28, replaced it with per-request `_meta` (SEP-2567),
/// which a client built against this fixed revision cannot speak.
const PROTOCOL_VERSION: &str = "2025-11-25";

/// RFC-094 §4's hostile-input list (review 182 §1): "a message over a
/// size cap (say 4 MiB)."
const MAX_MESSAGE_BYTES: usize = 4 * 1024 * 1024;

enum Incoming {
    Line(Vec<u8>),
    TooLarge,
    Eof,
    IoError(String),
}

/// The one comparison a server may run at a time: the JSON-RPC id it
/// will answer, and the token `notifications/cancelled` can cancel it
/// through.
type InFlight = Arc<Mutex<Option<(Value, CancellationToken)>>>;

pub fn serve(roots: Vec<PathBuf>, allow_show: bool) {
    let roots = Roots::new(roots);
    let (tx, rx) = mpsc::channel::<Incoming>();

    thread::spawn(move || {
        let stdin = io::stdin();
        let mut reader = stdin.lock();
        loop {
            let message = match read_capped_line(&mut reader, MAX_MESSAGE_BYTES) {
                Ok(Some(LineRead::Line(bytes))) => Incoming::Line(bytes),
                Ok(Some(LineRead::TooLarge)) => Incoming::TooLarge,
                Ok(None) => Incoming::Eof,
                Err(e) => Incoming::IoError(e.to_string()),
            };
            let stop = matches!(message, Incoming::Eof | Incoming::IoError(_));
            if tx.send(message).is_err() || stop {
                break;
            }
        }
    });

    let in_flight: InFlight = Arc::new(Mutex::new(None));

    for message in rx {
        match message {
            Incoming::Eof => {
                wait_for_in_flight(&in_flight);
                break;
            }
            Incoming::IoError(e) => {
                eprintln!("forskscope-cli mcp: stdin read error: {e}");
                break;
            }
            Incoming::TooLarge => {
                send_error(None, -32700, "Parse error: message exceeds the 4 MiB limit");
            }
            Incoming::Line(bytes) => handle_line(&bytes, &roots, allow_show, &in_flight),
        }
    }
}

fn wait_for_in_flight(in_flight: &InFlight) {
    loop {
        if in_flight.lock().unwrap().is_none() {
            return;
        }
        thread::sleep(std::time::Duration::from_millis(20));
    }
}

fn handle_line(bytes: &[u8], roots: &Roots, allow_show: bool, in_flight: &InFlight) {
    let text = match std::str::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => {
            send_error(None, -32700, "Parse error: not valid UTF-8");
            return;
        }
    };
    if text.trim().is_empty() {
        return;
    }

    let value: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => {
            send_error(None, -32700, "Parse error: invalid JSON");
            return;
        }
    };
    let Some(obj) = value.as_object() else {
        send_error(None, -32600, "Invalid Request: not a JSON object");
        return;
    };
    let id = obj.get("id").cloned();
    let Some(method) = obj.get("method").and_then(Value::as_str) else {
        send_error(id, -32600, "Invalid Request: missing `method`");
        return;
    };
    let params = obj.get("params").cloned().unwrap_or(Value::Null);

    match method {
        "initialize" => respond_or_invalid_request(id, initialize_result()),
        "notifications/initialized" => {}
        "ping" => respond_or_invalid_request(id, json!({})),
        "tools/list" => respond_or_invalid_request(id, tools::list_result(allow_show)),
        "tools/call" => handle_tools_call(id, &params, roots, allow_show, in_flight),
        "notifications/cancelled" => handle_cancelled(&params, in_flight),
        other => {
            // An unknown notification (no id) is ignored, per spec; an
            // unknown request still gets Method not found.
            if id.is_some() {
                send_error(id, -32601, &format!("Method not found: {other}"));
            }
        }
    }
}

/// `initialize`/`ping`/`tools/list` all expect a response - `id` missing
/// means whatever sent this expected a notification's silence instead,
/// which is itself the malformed part.
fn respond_or_invalid_request(id: Option<Value>, result: Value) {
    match id {
        Some(id) => respond(id, result),
        None => send_error(None, -32600, "Invalid Request: this method requires an id"),
    }
}

fn handle_tools_call(
    id: Option<Value>,
    params: &Value,
    roots: &Roots,
    allow_show: bool,
    in_flight: &InFlight,
) {
    let Some(id) = id else {
        send_error(None, -32600, "Invalid Request: tools/call requires an id");
        return;
    };
    let Some(name) = params
        .get("name")
        .and_then(Value::as_str)
        .map(str::to_string)
    else {
        send_error(Some(id), -32602, "Invalid params: `name` must be a string");
        return;
    };
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    match name.as_str() {
        "compare" => {
            let mut guard = in_flight.lock().unwrap();
            if guard.is_some() {
                drop(guard);
                send_error(Some(id), -32000, "Busy: a comparison is already running");
                return;
            }
            let token = CancellationToken::new();
            *guard = Some((id.clone(), token.clone()));
            drop(guard);

            let roots = roots.clone();
            let in_flight = Arc::clone(in_flight);
            thread::spawn(move || {
                let result = tools::call_compare(&arguments, &roots, &token);
                respond_tool_result(id, result);
                *in_flight.lock().unwrap() = None;
            });
        }
        "show_to_person" if allow_show => {
            let result = tools::call_show_to_person(&arguments, roots);
            respond_tool_result(id, result);
        }
        "show_to_person" => send_error(
            Some(id),
            -32602,
            "Invalid params: unknown tool `show_to_person` (server started without --allow-show)",
        ),
        other => send_error(
            Some(id),
            -32602,
            &format!("Invalid params: unknown tool `{other}`"),
        ),
    }
}

/// A tool-level failure (a bad path, a missing file) is reported through
/// `isError: true` inside an ordinary successful result — MCP's own
/// convention for "the tool ran and failed," kept distinct from a
/// JSON-RPC protocol error (a malformed request, an unknown method).
fn respond_tool_result(id: Value, result: Result<tools::ToolCallResult, String>) {
    match result {
        Ok(tool_result) => respond(id, tool_result.to_json()),
        Err(message) => respond(
            id,
            json!({
                "content": [{"type": "text", "text": message}],
                "isError": true,
            }),
        ),
    }
}

fn handle_cancelled(params: &Value, in_flight: &InFlight) {
    let Some(request_id) = params.get("requestId") else {
        return;
    };
    if let Some((id, token)) = in_flight.lock().unwrap().as_ref()
        && id == request_id
    {
        token.cancel();
    }
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": { "tools": {} },
        "serverInfo": {
            "name": "forskscope-cli",
            "version": env!("CARGO_PKG_VERSION"),
        },
    })
}

fn respond(id: Value, result: Value) {
    write_stdout(&json!({ "jsonrpc": "2.0", "id": id, "result": result }));
}

fn send_error(id: Option<Value>, code: i64, message: &str) {
    write_stdout(&json!({
        "jsonrpc": "2.0",
        "id": id.unwrap_or(Value::Null),
        "error": { "code": code, "message": message },
    }));
}

/// The only place this module writes to stdout - one JSON-RPC message
/// per line, nothing else. `Stdout`'s own internal lock keeps two
/// threads' calls here from interleaving mid-line.
fn write_stdout(value: &Value) {
    let line = serde_json::to_string(value).expect("a JSON-RPC message is always serializable");
    let mut out = io::stdout().lock();
    let _ = writeln!(out, "{line}");
    let _ = out.flush();
}

// ── A size-capped line reader ────────────────────────────────────────────────

enum LineRead {
    Line(Vec<u8>),
    TooLarge,
}

/// Reads one `\n`-terminated line, never buffering past `cap` bytes even
/// when the line itself is far longer — a hostile line with no newline
/// at all must not be able to force unbounded memory growth before this
/// function ever gets to compare its length against `cap`. Bytes beyond
/// `cap` are still read and discarded (not buffered) so the stream stays
/// in sync for the next message. Returns `Ok(None)` at EOF with nothing
/// read.
fn read_capped_line(reader: &mut impl BufRead, cap: usize) -> io::Result<Option<LineRead>> {
    let mut buf: Vec<u8> = Vec::new();
    let mut overflowed = false;
    loop {
        let chunk = reader.fill_buf()?;
        if chunk.is_empty() {
            if buf.is_empty() && !overflowed {
                return Ok(None);
            }
            return Ok(Some(finish(buf, overflowed)));
        }
        if let Some(pos) = chunk.iter().position(|&b| b == b'\n') {
            if !overflowed {
                append_capped(&mut buf, &chunk[..pos], cap, &mut overflowed);
            }
            reader.consume(pos + 1);
            return Ok(Some(finish(buf, overflowed)));
        }
        let len = chunk.len();
        if !overflowed {
            append_capped(&mut buf, chunk, cap, &mut overflowed);
        }
        reader.consume(len);
    }
}

fn append_capped(buf: &mut Vec<u8>, bytes: &[u8], cap: usize, overflowed: &mut bool) {
    if buf.len() + bytes.len() > cap {
        *overflowed = true;
    } else {
        buf.extend_from_slice(bytes);
    }
}

fn finish(mut buf: Vec<u8>, overflowed: bool) -> LineRead {
    if overflowed {
        return LineRead::TooLarge;
    }
    if buf.last() == Some(&b'\r') {
        buf.pop();
    }
    LineRead::Line(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_all(input: &[u8], cap: usize) -> Vec<String> {
        let mut reader = io::Cursor::new(input);
        let mut out = Vec::new();
        loop {
            match read_capped_line(&mut reader, cap).unwrap() {
                None => break,
                Some(LineRead::Line(bytes)) => out.push(String::from_utf8(bytes).unwrap()),
                Some(LineRead::TooLarge) => out.push("<too large>".to_string()),
            }
        }
        out
    }

    #[test]
    fn reads_ordinary_lines() {
        assert_eq!(read_all(b"a\nbb\nccc\n", 100), vec!["a", "bb", "ccc"]);
    }

    #[test]
    fn a_final_line_with_no_trailing_newline_is_still_read() {
        assert_eq!(read_all(b"a\nb", 100), vec!["a", "b"]);
    }

    #[test]
    fn strips_a_trailing_carriage_return() {
        assert_eq!(read_all(b"a\r\nb\r\n", 100), vec!["a", "b"]);
    }

    #[test]
    fn a_line_over_the_cap_is_reported_too_large_and_the_stream_resynchronizes() {
        let input = [vec![b'x'; 50], b"\n".to_vec(), b"next\n".to_vec()].concat();
        assert_eq!(read_all(&input, 10), vec!["<too large>", "next"]);
    }

    #[test]
    fn an_oversized_line_with_no_trailing_newline_at_all_is_still_bounded() {
        // The hostile case this function exists for: no newline ever
        // arrives, so without a cap this would buffer without bound.
        let input = vec![b'x'; 10_000];
        assert_eq!(read_all(&input, 100), vec!["<too large>"]);
    }

    #[test]
    fn empty_input_yields_no_lines() {
        assert_eq!(read_all(b"", 100), Vec::<String>::new());
    }
}
