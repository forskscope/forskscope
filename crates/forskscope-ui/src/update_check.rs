//! Update-check network adapter (F180, handoff 073).
//!
//! Everything here is I/O: one bounded HTTPS GET, off the UI thread. The
//! *decision* about what it means lives in `forskscope_ui_logic::update_check`
//! (`decide`), which this module never duplicates — it only ever produces a
//! [`forskscope_ui_logic::CheckOutcome`] for `decide` to interpret.
//!
//! ## Why a hand-written HTTP/1.1 client over `native-tls`
//!
//! `native-tls` is already linked (via `tungstenite` -> `dioxus-desktop`'s
//! loopback WebView transport), so using it directly here adds no new TLS
//! stack and no new system library — `readelf -d` on a release build lists
//! the same 16 libraries as 0.186.0 (checked in the review request). No
//! HTTP client crate (`ureq`, `reqwest`, …) is added either, so there is
//! nothing new in `cargo xtask audit-deps`'s external-network-crate list to
//! review. GitHub's REST API answers a plain `GET` with no ALPN offered in
//! HTTP/1.1 framed by `Content-Length`, never chunked or compressed
//! (measured directly against the real endpoint while writing this) —
//! simple enough to parse without a general-purpose client.

use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use forskscope_ui_logic::CheckOutcome;

const HOST: &str = "api.github.com";
const PORT: u16 = 443;
const PATH: &str = "/repos/forskscope/forskscope/releases/latest";

/// §3's bound: "a timeout of about 10 s". Applied to the connect phase and
/// to every subsequent read, so a server that accepts the connection and
/// then never answers cannot hang the button indefinitely either.
const IO_TIMEOUT: Duration = Duration::from_secs(10);

/// §3's bound: "a cap on the response size (say 1 MiB)". Checked against
/// the raw bytes received (headers included), so a misbehaving endpoint
/// cannot flood the app regardless of what it claims in `Content-Length` —
/// nothing here trusts that header at all; see [`parse_http_response`].
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;

/// Runs the bounded HTTPS GET and maps it to a [`CheckOutcome`]. Blocking —
/// callers run this inside `tokio::task::spawn_blocking`, the same pattern
/// `forskscope-ui` already uses for file loading and directory scans.
pub fn check_for_updates() -> CheckOutcome {
    let (host, port) = target();
    match fetch(&host, port, HOST, PATH) {
        Err(_) => CheckOutcome::Unreachable,
        Ok(response) => match response.status {
            200 => match extract_tag_name(&response.body) {
                Some(tag) => CheckOutcome::TagFound(tag),
                None => CheckOutcome::BodyUnparseable,
            },
            403 | 429 => CheckOutcome::RateLimited,
            other => CheckOutcome::HttpStatus(other),
        },
    }
}

/// The real target, unless a debug build has been asked to point at an
/// unreachable one instead (§7.2's verification override — not a feature,
/// a way to observe "Could not check" live without touching this
/// machine's real network configuration). `cfg(debug_assertions)` means a
/// `--release` build does not contain this branch at all: there is no env
/// var it could read, because the code that would read one does not exist
/// in that binary. See the review request for how this was confirmed, not
/// just asserted.
#[cfg(debug_assertions)]
fn target() -> (String, u16) {
    if std::env::var("FORSKSCOPE_FORCE_UNREACHABLE_UPDATE_CHECK").is_ok() {
        // Nothing listens here: the connection is refused immediately
        // rather than waiting out IO_TIMEOUT.
        return ("127.0.0.1".to_string(), 1);
    }
    (HOST.to_string(), PORT)
}

#[cfg(not(debug_assertions))]
fn target() -> (String, u16) {
    (HOST.to_string(), PORT)
}

/// Read exactly `tag_name` and nothing else (§3) — any other field in the
/// reply is never deserialized into anything, so there is no path from the
/// rest of the reply into the app at all, not even an ignored one.
#[derive(serde::Deserialize)]
struct LatestRelease {
    tag_name: String,
}

fn extract_tag_name(body: &[u8]) -> Option<String> {
    serde_json::from_slice::<LatestRelease>(body)
        .ok()
        .map(|r| r.tag_name)
}

struct RawResponse {
    status: u16,
    body: Vec<u8>,
}

/// Connects to `host:port`, requests `tls_domain`'s certificate and SNI
/// (always the real `api.github.com`, even when `host`/`port` have been
/// overridden for the debug verification path — not that it matters there,
/// since nothing listens on that address and the connection fails before
/// any TLS handshake begins), and GETs `path`.
fn fetch(host: &str, port: u16, tls_domain: &str, path: &str) -> io::Result<RawResponse> {
    let addr = (host, port)
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no address for host"))?;
    let stream = TcpStream::connect_timeout(&addr, IO_TIMEOUT)?;
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;

    let connector = native_tls::TlsConnector::new().map_err(io::Error::other)?;
    let mut tls = connector
        .connect(tls_domain, stream)
        .map_err(io::Error::other)?;

    let request = format!(
        "GET {path} HTTP/1.1\r\n\
         Host: {tls_domain}\r\n\
         User-Agent: ForskScope/{version}\r\n\
         Accept: application/json\r\n\
         Connection: close\r\n\
         \r\n",
        version = env!("CARGO_PKG_VERSION"),
    );
    tls.write_all(request.as_bytes())?;

    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let read_result = tls.read(&mut chunk);
        let n = match read_result {
            Ok(0) => break, // EOF: the server closed the connection.
            Ok(n) => n,
            Err(e) if is_timeout(&e) => break, // No more data in time; parse what arrived.
            Err(e) => return Err(e),
        };
        buf.extend_from_slice(&chunk[..n]);
        if buf.len() > MAX_RESPONSE_BYTES {
            return Err(io::Error::other("response exceeded the size cap"));
        }
    }

    parse_http_response(&buf)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "malformed HTTP response"))
}

fn is_timeout(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
    )
}

/// Parses a raw HTTP/1.1 response into a status code and body. Does not
/// trust or even read `Content-Length` — the body is simply "everything
/// after the header terminator that arrived before the read loop above
/// stopped". A short body (truncated mid-response) is not specially
/// detected here; it fails at the next step instead, when
/// [`extract_tag_name`]'s JSON parse rejects an incomplete document —
/// exactly the "reply that does not parse" outcome §2 already has a state
/// for, not a new failure mode to invent.
fn parse_http_response(raw: &[u8]) -> Option<RawResponse> {
    let header_end = find_double_crlf(raw)?;
    let header_text = std::str::from_utf8(&raw[..header_end]).ok()?;
    let status_line = header_text.lines().next()?;
    let status = parse_status_code(status_line)?;
    let body = raw[header_end + 4..].to_vec();
    Some(RawResponse { status, body })
}

fn find_double_crlf(raw: &[u8]) -> Option<usize> {
    raw.windows(4).position(|w| w == b"\r\n\r\n")
}

/// `"HTTP/1.1 200 OK"` -> `200`.
fn parse_status_code(status_line: &str) -> Option<u16> {
    let mut parts = status_line.split_whitespace();
    parts.next()?; // the HTTP version token
    parts.next()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_http_response / parse_status_code ────────────────────────────

    #[test]
    fn parses_a_normal_response() {
        let raw =
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"tag_name\":\"0.187.0\"}";
        let r = parse_http_response(raw).expect("must parse");
        assert_eq!(r.status, 200);
        assert_eq!(r.body, br#"{"tag_name":"0.187.0"}"#);
    }

    #[test]
    fn parses_a_non_200_status() {
        let raw = b"HTTP/1.1 403 Forbidden\r\nContent-Type: text/plain\r\n\r\nrate limited";
        let r = parse_http_response(raw).expect("must parse");
        assert_eq!(r.status, 403);
    }

    #[test]
    fn empty_body_parses_as_an_empty_body_not_a_failure() {
        let raw = b"HTTP/1.1 204 No Content\r\n\r\n";
        let r = parse_http_response(raw).expect("must parse");
        assert_eq!(r.status, 204);
        assert!(r.body.is_empty());
    }

    #[test]
    fn no_header_terminator_fails_to_parse() {
        // Truncated before the blank line ever arrived: there is no
        // complete response here at all.
        let raw = b"HTTP/1.1 200 OK\r\nContent-Type: applica";
        assert!(parse_http_response(raw).is_none());
    }

    #[test]
    fn garbage_that_is_not_http_at_all_fails_to_parse() {
        let raw = b"this is not an http response\r\n\r\nbody";
        assert!(parse_http_response(raw).is_none());
    }

    #[test]
    fn empty_input_fails_to_parse() {
        assert!(parse_http_response(b"").is_none());
    }

    #[test]
    fn status_code_parses_from_a_normal_status_line() {
        assert_eq!(parse_status_code("HTTP/1.1 200 OK"), Some(200));
        assert_eq!(
            parse_status_code("HTTP/1.1 429 Too Many Requests"),
            Some(429)
        );
    }

    #[test]
    fn status_code_rejects_a_non_numeric_or_missing_code() {
        assert_eq!(parse_status_code("HTTP/1.1"), None);
        assert_eq!(parse_status_code("HTTP/1.1 OK"), None);
        assert_eq!(parse_status_code(""), None);
    }

    // ── extract_tag_name ────────────────────────────────────────────────────

    #[test]
    fn extracts_the_tag_name_and_nothing_else() {
        let body = br#"{"tag_name":"0.187.0","other_field":"ignored","nested":{"x":1}}"#;
        assert_eq!(extract_tag_name(body), Some("0.187.0".to_string()));
    }

    #[test]
    fn missing_tag_name_is_unparseable() {
        let body = br#"{"name":"v0.187.0"}"#;
        assert_eq!(extract_tag_name(body), None);
    }

    #[test]
    fn non_json_body_is_unparseable() {
        assert_eq!(extract_tag_name(b"not json"), None);
    }

    #[test]
    fn empty_body_is_unparseable() {
        assert_eq!(extract_tag_name(b""), None);
    }

    #[test]
    fn a_tag_name_that_is_not_a_string_is_unparseable() {
        let body = br#"{"tag_name":187}"#;
        assert_eq!(extract_tag_name(body), None);
    }

    #[test]
    fn truncated_json_body_is_unparseable() {
        // What a response cut off mid-body by the read loop's EOF/timeout
        // exit looks like — the failure this is expected to surface as.
        let body = br#"{"tag_name":"0.18"#;
        assert_eq!(extract_tag_name(body), None);
    }

    // ── The debug-only override (§7.2) ──────────────────────────────────────

    #[test]
    fn the_override_target_is_refused_fast_not_a_10s_timeout() {
        // Nothing listens on 127.0.0.1:1: the connection is refused
        // immediately. This is the fast, deterministic path the real
        // override exercises when FORSKSCOPE_FORCE_UNREACHABLE_UPDATE_CHECK
        // is set — proven here without setting process-wide env state (a
        // hazard under `cargo test`'s multi-threaded runner), by calling
        // `fetch` directly against that same address.
        let started = std::time::Instant::now();
        let result = fetch("127.0.0.1", 1, HOST, PATH);
        assert!(result.is_err(), "nothing listens there; must fail");
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "a refused connection must not wait anywhere near the 10s timeout"
        );
    }

    // ── Real network ─────────────────────────────────────────────────────

    #[test]
    #[ignore = "hits the real network; run explicitly with --ignored"]
    fn check_for_updates_reaches_the_real_endpoint_and_finds_a_tag() {
        let outcome = check_for_updates();
        match outcome {
            CheckOutcome::TagFound(tag) => {
                assert!(
                    forskscope_ui_logic::Version::parse(&tag).is_some(),
                    "the real endpoint's tag_name must parse as a plain version: {tag:?}"
                );
            }
            other => panic!("expected TagFound from the real endpoint, got {other:?}"),
        }
    }
}
