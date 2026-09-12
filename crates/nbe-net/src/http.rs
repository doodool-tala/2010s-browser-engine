//! An HTTP/1.1 client over TCP, implementing the Transport seam.
//! Normative reference: ADR-0011.
//!
//! Hand-rolled by design: zero new dependencies, an era-appropriate
//! surface (GET-only, `Connection: close`, one TCP connection per
//! request hop), full auditability. Redirects resolve inside the
//! transport (absolute and root-relative targets, DV-007) up to a
//! hop limit. `https` returns a clean Module error until TLS lands
//! (DV-005). Body framing: Content-Length, chunked, or read-to-close,
//! capped at MAX_BODY_BYTES — a hostile server's headers are
//! untrusted input.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

use nbe_core::error::ModuleError;

use crate::loader::{Resource, Transport};
use crate::url::{parse_url, Url};

/// The default redirect hop limit (Chrome-2011-era value).
pub const DEFAULT_MAX_REDIRECTS: u32 = 20;

/// Hard cap on any single response body, against allocation bombs
/// from hostile servers.
const MAX_BODY_BYTES: usize = 64 * 1024 * 1024;

/// An HTTP/1.1 transport: GET-only, one TCP connection per hop.
pub struct HttpTransport {
    max_redirects: u32,
    user_agent: String,
}

impl HttpTransport {
    /// A transport with the default redirect limit.
    #[must_use]
    pub fn new() -> Self {
        Self {
            max_redirects: DEFAULT_MAX_REDIRECTS,
            user_agent: String::from("nbe/0.1"),
        }
    }

    /// Override the redirect hop limit.
    #[must_use]
    pub fn with_max_redirects(mut self, max_redirects: u32) -> Self {
        self.max_redirects = max_redirects;
        self
    }
}

impl Default for HttpTransport {
    fn default() -> Self {
        Self::new()
    }
}

/// What one request hop produced.
enum Outcome {
    /// A final (2xx) response body.
    Body(Resource),
    /// A 3xx with a Location to follow.
    Redirect(String),
}

impl Transport for HttpTransport {
    fn fetch(&mut self, url: &Url) -> Result<Resource, ModuleError> {
        let mut current = url.clone();
        let mut hops: u32 = 0;
        loop {
            match fetch_once(&current, &self.user_agent)? {
                Outcome::Body(resource) => return Ok(resource),
                Outcome::Redirect(location) => {
                    if hops >= self.max_redirects {
                        return Err(ModuleError::new("http", "too many redirects"));
                    }
                    hops += 1;
                    current = resolve_redirect(&current, &location)?;
                }
            }
        }
    }
}

fn fetch_once(url: &Url, user_agent: &str) -> Result<Outcome, ModuleError> {
    if url.scheme().is_tls() {
        return Err(ModuleError::new(
            "http",
            "TLS not yet supported (DV-005); rustls arrives in a later package",
        ));
    }
    let port = url.port().unwrap_or_else(|| url.scheme().default_port());
    let connect_to = format!("{}:{port}", url.host());
    let host_header = match url.port() {
        Some(port) => format!("{}:{port}", url.host()),
        None => url.host().to_string(),
    };
    let target = request_target(url);
    let request = format!(
        "GET {target} HTTP/1.1\r\nHost: {host_header}\r\nUser-Agent: {user_agent}\r\nAccept: */*\r\nAccept-Encoding: identity\r\nConnection: close\r\n\r\n"
    );
    let mut stream = TcpStream::connect(connect_to.as_str())
        .map_err(|e| ModuleError::new("http", format!("connect failed: {e}")))?;
    stream
        .write_all(request.as_bytes())
        .map_err(|e| ModuleError::new("http", format!("request write failed: {e}")))?;
    stream
        .flush()
        .map_err(|e| ModuleError::new("http", format!("request flush failed: {e}")))?;
    let mut reader = BufReader::new(stream);
    let status = parse_status(&read_crlf_line(&mut reader)?)?;
    let headers = parse_headers(&mut reader)?;
    let content_type = header_value(&headers, "content-type").map(str::to_string);
    let body = read_body(&mut reader, &headers)?;
    if (200..300).contains(&status) {
        Ok(Outcome::Body(Resource::new(body, content_type)))
    } else if (300..400).contains(&status) {
        match header_value(&headers, "location") {
            Some(location) => Ok(Outcome::Redirect(location.to_string())),
            None => Err(ModuleError::new(
                "http",
                "redirect without a location header",
            )),
        }
    } else {
        Err(ModuleError::new("http", format!("status {status}")))
    }
}

/// Resolve a redirect target: absolute URLs, or root-relative paths
/// against the current authority (DV-007 covers the rest).
fn resolve_redirect(base: &Url, location: &str) -> Result<Url, ModuleError> {
    if let Some(absolute) = parse_url(location) {
        return Ok(absolute);
    }
    if location.starts_with('/') {
        let prefix = authority_prefix(base)
            .ok_or_else(|| ModuleError::new("http", "malformed redirect base"))?;
        let target = format!("{prefix}{location}");
        return parse_url(&target)
            .ok_or_else(|| ModuleError::new("http", "invalid redirect target"));
    }
    Err(ModuleError::new(
        "http",
        "unsupported redirect target (absolute or root-relative only; DV-007)",
    ))
}

/// scheme://[userinfo@]host[:port] — everything before the path.
fn authority_prefix(url: &Url) -> Option<String> {
    let text = url.to_string();
    let scheme_end = text.find("://")?;
    let authority_start = scheme_end + 3;
    // The serialized path always begins with '/', and neither userinfo
    // (rejects '/') nor hosts (domains, IPv4, bracketed IPv6) can
    // contain one — so the first '/' after the scheme is the path.
    let path_offset = text[authority_start..].find('/')?;
    Some(text[..authority_start + path_offset].to_string())
}

/// The request target: path plus query.
fn request_target(url: &Url) -> String {
    let mut target = String::new();
    target.push('/');
    target.push_str(&url.path_segments().join("/"));
    if let Some(query) = url.query() {
        target.push('?');
        target.push_str(query);
    }
    target
}

fn read_crlf_line(reader: &mut BufReader<TcpStream>) -> Result<String, ModuleError> {
    let mut line = String::new();
    let read = reader
        .read_line(&mut line)
        .map_err(|e| ModuleError::new("http", format!("read failed: {e}")))?;
    if read == 0 {
        return Err(ModuleError::new("http", "connection closed mid-header"));
    }
    while line.ends_with('\n') || line.ends_with('\r') {
        line.pop();
    }
    Ok(line)
}

fn parse_status(line: &str) -> Result<u16, ModuleError> {
    let mut parts = line.splitn(3, ' ');
    let version = parts
        .next()
        .ok_or_else(|| ModuleError::new("http", "empty status line"))?;
    if !version.starts_with("HTTP/1.") {
        return Err(ModuleError::new(
            "http",
            format!("unsupported HTTP version: {version}"),
        ));
    }
    let code = parts
        .next()
        .ok_or_else(|| ModuleError::new("http", format!("status line without code: {line}")))?;
    code.trim()
        .parse::<u16>()
        .map_err(|_| ModuleError::new("http", format!("unparseable status code: {code}")))
}

fn parse_headers(reader: &mut BufReader<TcpStream>) -> Result<Vec<(String, String)>, ModuleError> {
    let mut headers = Vec::new();
    loop {
        let line = read_crlf_line(reader)?;
        if line.is_empty() {
            return Ok(headers);
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err(ModuleError::new(
                "http",
                format!("malformed header line: {line}"),
            ));
        };
        headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
    }
}

fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}

fn read_body(
    reader: &mut BufReader<TcpStream>,
    headers: &[(String, String)],
) -> Result<Vec<u8>, ModuleError> {
    let chunked = header_value(headers, "transfer-encoding")
        .is_some_and(|value| value.to_ascii_lowercase().contains("chunked"));
    if chunked {
        return read_chunked(reader);
    }
    match header_value(headers, "content-length") {
        Some(value) => {
            let length: usize = value
                .parse()
                .map_err(|_| ModuleError::new("http", format!("bad content-length: {value}")))?;
            if length > MAX_BODY_BYTES {
                return Err(ModuleError::new("http", "body exceeds the size cap"));
            }
            let mut body = vec![0u8; length];
            reader
                .read_exact(&mut body)
                .map_err(|e| ModuleError::new("http", format!("body read failed: {e}")))?;
            Ok(body)
        }
        None => {
            let mut body = Vec::new();
            reader
                .read_to_end(&mut body)
                .map_err(|e| ModuleError::new("http", format!("body read failed: {e}")))?;
            Ok(body)
        }
    }
}

fn read_chunked(reader: &mut BufReader<TcpStream>) -> Result<Vec<u8>, ModuleError> {
    let mut body = Vec::new();
    loop {
        let size_line = read_crlf_line(reader)?;
        let size_text = size_line.split(';').next().unwrap_or_default().trim();
        let size = usize::from_str_radix(size_text, 16)
            .map_err(|_| ModuleError::new("http", format!("bad chunk size: {size_line}")))?;
        if size == 0 {
            return Ok(body);
        }
        if body.len().saturating_add(size) > MAX_BODY_BYTES {
            return Err(ModuleError::new(
                "http",
                "chunked body exceeds the size cap",
            ));
        }
        let mut chunk = vec![0u8; size];
        reader
            .read_exact(&mut chunk)
            .map_err(|e| ModuleError::new("http", format!("chunk read failed: {e}")))?;
        body.extend_from_slice(&chunk);
        // Consume the CRLF that terminates the chunk data.
        read_crlf_line(reader)?;
    }
}
