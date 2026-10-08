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
//! HTTP/1.1, framed by `Content-Length`, when measured against the real
//! endpoint while writing this — but that is a measurement of GitHub's
//! current deployment, not a property of the protocol HTTP/1.1 lets a
//! server choose freely (RFC 9112 §7.1). **The client accepts both framings
//! HTTP/1.1 allows**: `Content-Length`/connection-close as measured, and
//! `Transfer-Encoding: chunked` decoded by [`decode_chunked_body`] — review
//! 154's correction, after review found this module's first draft claimed
//! chunked replies could not happen rather than that they had not been
//! observed.

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
/// trust or even read `Content-Length` — a `Content-Length`-framed or
/// connection-closed body is simply "everything after the header
/// terminator that arrived before the read loop above stopped". A short
/// body (truncated mid-response) is not specially detected here; it fails
/// at the next step instead, when [`extract_tag_name`]'s JSON parse
/// rejects an incomplete document — exactly the "reply that does not
/// parse" outcome §2 already has a state for, not a new failure mode to
/// invent. `Transfer-Encoding: chunked` is the one framing that needs its
/// own step first (review 154): [`decode_chunked_body`] turns it back into
/// a plain body, or fails this whole parse (`None`) on a malformed chunk —
/// which reaches the caller the same way a truncated non-chunked body
/// does, through [`extract_tag_name`]'s JSON parse never seeing a tag.
fn parse_http_response(raw: &[u8]) -> Option<RawResponse> {
    let header_end = find_double_crlf(raw)?;
    let header_text = std::str::from_utf8(&raw[..header_end]).ok()?;
    let mut lines = header_text.lines();
    let status_line = lines.next()?;
    let status = parse_status_code(status_line)?;
    let is_chunked = lines.any(|line| {
        line.split_once(':').is_some_and(|(name, value)| {
            name.trim().eq_ignore_ascii_case("transfer-encoding")
                && value.trim().eq_ignore_ascii_case("chunked")
        })
    });
    let raw_body = &raw[header_end + 4..];
    let body = if is_chunked {
        decode_chunked_body(raw_body)?
    } else {
        raw_body.to_vec()
    };
    Some(RawResponse { status, body })
}

fn find_double_crlf(raw: &[u8]) -> Option<usize> {
    raw.windows(4).position(|w| w == b"\r\n\r\n")
}

/// Decodes a `Transfer-Encoding: chunked` body (RFC 9112 §7.1): a sequence
/// of `<hex-size>[;extension]\r\n<data>\r\n` chunks, terminated by a
/// zero-size chunk. Chunk extensions are ignored (nothing here needs one);
/// trailers after the terminating chunk are ignored too, by returning as
/// soon as it is seen. Fails (`None`), rather than returning a partial
/// body, on a bad hex size, a chunk whose declared length reaches past
/// what arrived, a missing chunk-terminating CRLF, or a decoded body that
/// would exceed [`MAX_RESPONSE_BYTES`] — checked here explicitly, not left
/// to follow only as a side effect of the raw-byte cap already applied to
/// what the read loop in [`fetch`] accepted.
fn decode_chunked_body(raw: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut pos = 0;
    loop {
        let line_end = pos + find_crlf(&raw[pos..])?;
        let size_line = std::str::from_utf8(&raw[pos..line_end]).ok()?;
        let size_hex = size_line.split(';').next()?.trim();
        let size = usize::from_str_radix(size_hex, 16).ok()?;
        pos = line_end + 2;
        if size == 0 {
            return Some(out);
        }
        let chunk_end = pos.checked_add(size)?;
        if chunk_end.checked_add(2)? > raw.len() {
            return None; // declared length reaches past what arrived
        }
        if &raw[chunk_end..chunk_end + 2] != b"\r\n" {
            return None;
        }
        out.extend_from_slice(&raw[pos..chunk_end]);
        if out.len() > MAX_RESPONSE_BYTES {
            return None;
        }
        pos = chunk_end + 2;
    }
}

fn find_crlf(buf: &[u8]) -> Option<usize> {
    buf.windows(2).position(|w| w == b"\r\n")
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

    // ── chunked transfer encoding (review 154) ──────────────────────────────

    /// A chunked reply whose chunk boundaries split the JSON mid-token
    /// still yields the tag — through `parse_http_response`, the same
    /// function `fetch` calls, then `extract_tag_name` on its body, the
    /// same function `check_for_updates` calls.
    #[test]
    fn a_chunked_reply_split_mid_token_still_yields_the_tag() {
        let full_body = br#"{"tag_name":"0.187.0","other":"x"}"#;
        // Splits "0.187.0" as "0.1" | "87.0" - genuinely mid-token, not at
        // a field boundary (checked: byte 16 of this exact literal).
        let (first, second) = full_body.split_at(16);
        let mut raw = Vec::new();
        raw.extend_from_slice(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n");
        raw.extend_from_slice(format!("{:x}\r\n", first.len()).as_bytes());
        raw.extend_from_slice(first);
        raw.extend_from_slice(b"\r\n");
        raw.extend_from_slice(format!("{:x}\r\n", second.len()).as_bytes());
        raw.extend_from_slice(second);
        raw.extend_from_slice(b"\r\n0\r\n\r\n");

        let r = parse_http_response(&raw).expect("must parse");
        assert_eq!(r.status, 200);
        assert_eq!(r.body, full_body);
        assert_eq!(extract_tag_name(&r.body), Some("0.187.0".to_string()));
    }

    #[test]
    fn chunk_extensions_and_trailers_are_ignored() {
        let body = br#"{"tag_name":"0.187.0"}"#;
        let mut raw = Vec::new();
        raw.extend_from_slice(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n");
        raw.extend_from_slice(format!("{:x};some-extension=1\r\n", body.len()).as_bytes());
        raw.extend_from_slice(body);
        raw.extend_from_slice(b"\r\n0\r\nX-Trailer: ignored\r\n\r\n");

        let r = parse_http_response(&raw).expect("must parse");
        assert_eq!(r.body, body);
    }

    #[test]
    fn a_bad_hex_chunk_size_fails_to_parse_and_never_yields_a_tag() {
        let mut raw = Vec::new();
        raw.extend_from_slice(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n");
        raw.extend_from_slice(b"not-hex\r\nxxxxxxx\r\n0\r\n\r\n");
        assert!(parse_http_response(&raw).is_none());
    }

    #[test]
    fn a_truncated_chunk_fails_to_parse_and_never_yields_a_tag() {
        let mut raw = Vec::new();
        raw.extend_from_slice(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n");
        // Declares 100 (0x64) bytes but the connection stopped after 10.
        raw.extend_from_slice(b"64\r\n");
        raw.extend_from_slice(br#"{"tag_na"#);
        assert!(parse_http_response(&raw).is_none());
    }

    #[test]
    fn a_chunked_body_over_the_cap_is_refused() {
        let mut raw = Vec::new();
        raw.extend_from_slice(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n");
        // Two chunks that together exceed MAX_RESPONSE_BYTES, each
        // individually well under it - the cap must apply to the decoded
        // total, not be checkable by looking at any one chunk alone.
        let half = vec![b'a'; (MAX_RESPONSE_BYTES / 2) + 1];
        for _ in 0..2 {
            raw.extend_from_slice(format!("{:x}\r\n", half.len()).as_bytes());
            raw.extend_from_slice(&half);
            raw.extend_from_slice(b"\r\n");
        }
        raw.extend_from_slice(b"0\r\n\r\n");
        assert!(parse_http_response(&raw).is_none());
    }

    #[test]
    fn content_length_framed_and_connection_close_framed_bodies_still_work_unchanged() {
        // No Transfer-Encoding header at all: the pre-existing path.
        let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 23\r\n\r\n{\"tag_name\":\"0.187.0\"}";
        let r = parse_http_response(raw).expect("must parse");
        assert_eq!(r.body, br#"{"tag_name":"0.187.0"}"#);
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
