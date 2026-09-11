//! URL parsing and serialization for absolute http/https URLs.
//! Normative reference: ADR-0009.
//!
//! WHATWG-informed subset (v1): scheme, authority (userinfo, host,
//! port), path with dot-segment removal, query, fragment. Hosts are
//! domains (ASCII), IPv4 (hex/octal/decimal and short number forms),
//! or IPv6 (full grammar with `::` compression and embedded IPv4).
//! Serialization is exact: a parsed URL re-serializes to its
//! canonical form. Malformed input returns `None` — the
//! RecoverableInput class (ADR-0002).

use std::fmt;

use crate::origin::Origin;

/// The URL schemes v1 understands. Other schemes are a v1 parse
/// failure (divergence DV-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scheme {
    /// Plain HTTP, default port 80.
    Http,
    /// HTTP over TLS, default port 443.
    Https,
}

impl Scheme {
    /// The scheme's name as serialized.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Scheme::Http => "http",
            Scheme::Https => "https",
        }
    }

    /// The scheme's default port.
    #[must_use]
    pub fn default_port(self) -> u16 {
        match self {
            Scheme::Http => 80,
            Scheme::Https => 443,
        }
    }

    /// True when the scheme is TLS-protected.
    #[must_use]
    pub fn is_tls(self) -> bool {
        matches!(self, Scheme::Https)
    }
}

impl fmt::Display for Scheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A parsed host: domain, IPv4 address, or IPv6 address.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Host {
    /// A lowercased ASCII domain name.
    Domain(String),
    /// An IPv4 address as a raw 32-bit value.
    Ipv4(u32),
    /// An IPv6 address as eight 16-bit pieces.
    Ipv6([u16; 8]),
}

impl fmt::Display for Host {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Host::Domain(domain) => f.write_str(domain),
            Host::Ipv4(v) => write!(
                f,
                "{}.{}.{}.{}",
                v >> 24,
                (v >> 16) & 0xFF,
                (v >> 8) & 0xFF,
                v & 0xFF
            ),
            // WHATWG host serialization wraps IPv6 in brackets.
            Host::Ipv6(pieces) => write!(f, "[{}]", serialize_ipv6(pieces)),
        }
    }
}

/// A parsed absolute http/https URL.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Url {
    scheme: Scheme,
    username: String,
    password: Option<String>,
    host: Host,
    port: Option<u16>,
    segments: Vec<String>,
    query: Option<String>,
    fragment: Option<String>,
}

impl Url {
    /// The scheme.
    #[must_use]
    pub fn scheme(&self) -> Scheme {
        self.scheme
    }

    /// The host.
    #[must_use]
    pub fn host(&self) -> &Host {
        &self.host
    }

    /// The effective port; `None` for the scheme default.
    #[must_use]
    pub fn port(&self) -> Option<u16> {
        self.port
    }

    /// The userinfo username (empty when absent).
    #[must_use]
    pub fn username(&self) -> &str {
        &self.username
    }

    /// The userinfo password.
    #[must_use]
    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    /// Path segments without the leading slash; empty means "/".
    #[must_use]
    pub fn path_segments(&self) -> &[String] {
        &self.segments
    }

    /// The query, without its '?'.
    #[must_use]
    pub fn query(&self) -> Option<&str> {
        self.query.as_deref()
    }

    /// The fragment, without its '#'.
    #[must_use]
    pub fn fragment(&self) -> Option<&str> {
        self.fragment.as_deref()
    }

    /// The URL's origin: scheme, host, and effective port.
    #[must_use]
    pub fn origin(&self) -> Origin {
        Origin::new(self.scheme, self.host.clone(), self.port)
    }
}

impl fmt::Display for Url {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://", self.scheme)?;
        if !self.username.is_empty() || self.password.is_some() {
            write!(f, "{}", self.username)?;
            if let Some(password) = &self.password {
                write!(f, ":{password}")?;
            }
            f.write_str("@")?;
        }
        write!(f, "{}", self.host)?;
        if let Some(port) = self.port {
            write!(f, ":{port}")?;
        }
        f.write_str("/")?;
        f.write_str(&self.segments.join("/"))?;
        if let Some(query) = &self.query {
            write!(f, "?{query}")?;
        }
        if let Some(fragment) = &self.fragment {
            write!(f, "#{fragment}")?;
        }
        Ok(())
    }
}

/// Parse an absolute http/https URL. Returns `None` on any malformed
/// input: the RecoverableInput class (ADR-0002) — the caller shows an
/// error, the engine never crashes.
#[must_use]
pub fn parse_url(input: &str) -> Option<Url> {
    // WHATWG pre-processing: strip leading/trailing C0 controls and
    // space; remove all tab, LF, CR.
    let cleaned: String = input
        .trim_matches(|c: char| c <= ' ')
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();
    if cleaned.is_empty() {
        return None;
    }

    // Scheme. v1 recognizes exactly two scheme names (DV-002); the
    // literal match subsumes the scheme-name grammar.
    let colon = cleaned.find(':')?;
    let scheme = match cleaned[..colon].to_ascii_lowercase().as_str() {
        "http" => Scheme::Http,
        "https" => Scheme::Https,
        _ => return None,
    };
    let rest = cleaned[colon + 1..].strip_prefix("//")?;

    // Authority ends at the first '/', '?', or '#'.
    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    let tail = &rest[authority_end..];

    // Userinfo is everything before the LAST '@' in the authority.
    let (userinfo, host_port) = match authority.rfind('@') {
        Some(at) => (Some(&authority[..at]), &authority[at + 1..]),
        None => (None, authority),
    };
    if host_port.is_empty() {
        return None;
    }

    let (host_text, port_text) = split_host_port(host_port)?;

    // Port: digits only, <= 65535, leading zeros dropped, default
    // ports removed, a bare trailing ':' allowed.
    let port = match port_text {
        None => None,
        Some("") => None,
        Some(text) => {
            if !text.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            let trimmed = text.trim_start_matches('0');
            let value: u32 = if trimmed.is_empty() {
                0
            } else {
                trimmed.parse().ok()?
            };
            if value > 65535 {
                return None;
            }
            let port = value as u16;
            if port == scheme.default_port() {
                None
            } else {
                Some(port)
            }
        }
    };

    let host = parse_host(host_text)?;

    // Userinfo.
    let (username, password) = match userinfo {
        None => (String::new(), None),
        Some(info) => {
            if !is_valid_userinfo(info) {
                return None;
            }
            match info.find(':') {
                Some(c) => (info[..c].to_string(), Some(info[c + 1..].to_string())),
                None => (info.to_string(), None),
            }
        }
    };

    // Fragment, then query, then path.
    let (before_fragment, fragment) = match tail.find('#') {
        Some(hash) => (&tail[..hash], Some(tail[hash + 1..].to_string())),
        None => (tail, None),
    };
    let (path_text, query) = match before_fragment.find('?') {
        Some(question) => (
            &before_fragment[..question],
            Some(before_fragment[question + 1..].to_string()),
        ),
        None => (before_fragment, None),
    };
    if let Some(query) = &query {
        if !is_valid_component(query) {
            return None;
        }
    }
    if let Some(fragment) = &fragment {
        if !is_valid_component(fragment) {
            return None;
        }
    }
    let segments = parse_path(path_text)?;

    Some(Url {
        scheme,
        username,
        password,
        host,
        port,
        segments,
        query,
        fragment,
    })
}

/// Split host from optional port text. IPv6 hosts carry their colons
/// inside brackets.
fn split_host_port(host_port: &str) -> Option<(&str, Option<&str>)> {
    if host_port.starts_with('[') {
        let close = host_port.find(']')?;
        let host = &host_port[..close + 1];
        let after = &host_port[close + 1..];
        let port = if after.is_empty() {
            None
        } else {
            Some(after.strip_prefix(':')?)
        };
        return Some((host, port));
    }
    match host_port.find(':') {
        Some(c) => Some((&host_port[..c], Some(&host_port[c + 1..]))),
        None => Some((host_port, None)),
    }
}

/// Parse a host: bracketed IPv6, IPv4 (when the host ends in a
/// number), or an ASCII domain.
fn parse_host(input: &str) -> Option<Host> {
    if input.is_empty() {
        return None;
    }
    if input.starts_with('[') {
        if !input.ends_with(']') {
            return None;
        }
        let inner = &input[1..input.len() - 1];
        return Some(Host::Ipv6(parse_ipv6(inner)?));
    }
    let host = input.strip_suffix('.').unwrap_or(input);
    if host.is_empty() {
        return None;
    }
    if ends_in_number(host) {
        return Some(Host::Ipv4(parse_ipv4(host)?));
    }
    let lowered = host.to_ascii_lowercase();
    if !is_valid_domain(&lowered) {
        return None;
    }
    Some(Host::Domain(lowered))
}

/// A valid ASCII domain (ADR-0009 per-label validation): URL
/// characters except the authority delimiters and '%' (v1 does not
/// percent-encode domains).
fn is_valid_domain(domain: &str) -> bool {
    !domain.is_empty()
        && domain.bytes().all(|b| {
            is_url_char(b)
                && !matches!(
                    b,
                    b'/' | b'?' | b'#' | b'@' | b':' | b'[' | b']' | b'%' | b'\\'
                )
        })
}

/// True when the host's final label is an ASCII-decimal or 0x-hex
/// number: such hosts must parse as IPv4 (WHATWG).
fn ends_in_number(host: &str) -> bool {
    let mut parts: Vec<&str> = host.split('.').collect();
    if parts.len() > 1 && parts.last().is_some_and(|part| part.is_empty()) {
        parts.pop();
    }
    if let Some(last) = parts.last() {
        is_number(last)
    } else {
        false
    }
}

/// A number per the ends-in-a-number check: all ASCII digits, or
/// 0x/0X followed by at least one hex digit.
fn is_number(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    if text.bytes().all(|b| b.is_ascii_digit()) {
        return true;
    }
    match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(digits) => !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_hexdigit()),
        None => false,
    }
}

/// The WHATWG IPv4 number parser: decimal, 0x-hex (<= 4 digits), or
/// leading-0 octal. "0x" and bare "0" styles yield 0.
fn parse_ipv4_number(input: &str) -> Option<u32> {
    if input.is_empty() {
        return None;
    }
    let (radix, digits) = if let Some(rest) = input
        .strip_prefix("0x")
        .or_else(|| input.strip_prefix("0X"))
    {
        (16u32, rest)
    } else if input.len() > 1 && input.starts_with('0') {
        (8, &input[1..])
    } else {
        (10, input)
    };
    if digits.is_empty() {
        return Some(0);
    }
    if radix == 16 && digits.len() > 4 {
        return None;
    }
    let mut value: u64 = 0;
    for b in digits.bytes() {
        let digit = u64::from((b as char).to_digit(radix)?);
        value = value * u64::from(radix) + digit;
        if value > u64::from(u32::MAX) {
            return None;
        }
    }
    Some(value as u32)
}

/// The WHATWG IPv4 parser: up to 4 dot-separated numbers, each part
/// shifted left by 8 bits except the last, which occupies
/// 256^(5-count) — short forms ("127.1" is 127.0.0.1).
fn parse_ipv4(input: &str) -> Option<u32> {
    let mut parts: Vec<&str> = input.split('.').collect();
    if parts.len() > 1 && parts.last().is_some_and(|part| part.is_empty()) {
        parts.pop();
    }
    let count = parts.len();
    if count == 0 || count > 4 {
        return None;
    }
    let last_multiplier = 256u64.pow(u32::try_from(5 - count).ok()?);
    let mut value: u64 = 0;
    for (index, part) in parts.iter().enumerate() {
        let number = u64::from(parse_ipv4_number(part)?);
        let multiplier = if index + 1 == count {
            last_multiplier
        } else {
            256
        };
        if number >= multiplier {
            return None;
        }
        value = value * multiplier + number;
    }
    if value > u64::from(u32::MAX) {
        return None;
    }
    Some(value as u32)
}

/// Full IPv6 grammar: 4-hex-digit pieces separated by ':', exactly
/// one "::" standing for at least one zero piece, optional embedded
/// IPv4 in the final position.
fn parse_ipv6(input: &str) -> Option<[u16; 8]> {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut address = [0u16; 8];
    let mut piece_index = 0usize;
    let mut compress: Option<usize> = None;
    let mut p = 0usize;

    if len >= 2 && bytes[0] == b':' && bytes[1] == b':' {
        p = 2;
        compress = Some(0);
        if p == len {
            return Some(address); // "::" — all zeros
        }
    } else if bytes.first() == Some(&b':') {
        return None; // single leading ':'
    }

    while p < len {
        if piece_index == 8 {
            return None; // too many pieces
        }
        let piece_start = p;
        let mut value: u32 = 0;
        let mut digits = 0;
        while p < len && digits < 4 && (bytes[p] as char).is_ascii_hexdigit() {
            value = value * 16 + (bytes[p] as char).to_digit(16)?;
            digits += 1;
            p += 1;
        }
        // Embedded IPv4: a '.' appeared where a piece was being read.
        if p < len && bytes[p] == b'.' {
            let ipv4 = parse_ipv4(&input[piece_start..])?;
            if piece_index + 2 > 8 {
                return None;
            }
            address[piece_index] = (ipv4 >> 16) as u16;
            address[piece_index + 1] = (ipv4 & 0xFFFF) as u16;
            return finish_ipv6(address, piece_index + 2, compress);
        }
        if digits == 0 {
            return None; // expected a hex piece
        }
        address[piece_index] = value as u16;
        piece_index += 1;
        if p == len {
            break;
        }
        if bytes[p] != b':' {
            return None; // junk after a piece
        }
        p += 1;
        if p == len {
            return None; // trailing single ':'
        }
        if bytes[p] == b':' {
            if compress.is_some() {
                return None; // two compressions
            }
            p += 1;
            compress = Some(piece_index);
            if p == len {
                break; // trailing "::"
            }
        }
    }

    finish_ipv6(address, piece_index, compress)
}

/// Expand the compression around the recorded index and enforce the
/// exactly-8-pieces rule.
fn finish_ipv6(address: [u16; 8], piece_index: usize, compress: Option<usize>) -> Option<[u16; 8]> {
    match compress {
        Some(c) => {
            let before = c;
            let after = piece_index - c;
            if before + after > 7 {
                return None; // "::" must stand for >= 1 zero piece
            }
            let mut result = [0u16; 8];
            result[..before].copy_from_slice(&address[..before]);
            result[8 - after..].copy_from_slice(&address[c..piece_index]);
            Some(result)
        }
        None => {
            if piece_index == 8 {
                Some(address)
            } else {
                None
            }
        }
    }
}

/// RFC 5952 serialization: compress the leftmost longest run of >= 2
/// zero pieces; lowercase hex, no leading zeros.
fn serialize_ipv6(pieces: &[u16; 8]) -> String {
    let mut best_start = None;
    let mut best_len = 0;
    let mut i = 0;
    while i < 8 {
        if pieces[i] == 0 {
            let start = i;
            while i < 8 && pieces[i] == 0 {
                i += 1;
            }
            let len = i - start;
            if len > best_len {
                best_len = len;
                best_start = Some(start);
            }
        } else {
            i += 1;
        }
    }
    let hex =
        |slice: &[u16]| -> Vec<String> { slice.iter().map(|piece| format!("{piece:x}")).collect() };
    match best_start {
        Some(start) if best_len >= 2 => {
            let end = start + best_len;
            let left = hex(&pieces[..start]);
            let right = hex(&pieces[end..]);
            match (left.is_empty(), right.is_empty()) {
                (true, true) => String::from("::"),
                (true, false) => format!("::{}", right.join(":")),
                (false, true) => format!("{}::", left.join(":")),
                (false, false) => format!("{}::{}", left.join(":"), right.join(":")),
            }
        }
        _ => hex(pieces).join(":"),
    }
}

/// Split the path into segments, remove dot segments, and validate.
fn parse_path(path_text: &str) -> Option<Vec<String>> {
    let mut segments: Vec<String> = Vec::new();
    if path_text.is_empty() {
        return Some(segments); // "" serializes as "/"
    }
    if !path_text.starts_with('/') {
        return None;
    }
    for segment in path_text[1..].split('/') {
        if !is_valid_component(segment) {
            return None;
        }
        if segment == "." {
            // dropped
        } else if segment == ".." {
            segments.pop(); // popping at the root is a no-op
        } else {
            segments.push(segment.to_string());
        }
    }
    if segments.len() == 1 && segments[0].is_empty() {
        segments.clear(); // "/" and "" both serialize to "/"
    }
    Some(segments)
}

/// An unreserved/sub-delimited URL character (v1's accepted set
/// before percent-encoding normalization, DV-004).
fn is_url_char(b: u8) -> bool {
    if b.is_ascii_alphanumeric() {
        return true;
    }
    matches!(
        b,
        b'-' | b'.'
            | b'_'
            | b'~'
            | b'!'
            | b'$'
            | b'&'
            | b'\''
            | b'('
            | b')'
            | b'*'
            | b'+'
            | b','
            | b';'
            | b'='
            | b':'
            | b'@'
            | b'/'
    )
}

/// A component (path segment, query, fragment): URL characters with
/// well-formed percent-escapes only.
fn is_valid_component(s: &str) -> bool {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            match valid_percent_at(bytes, i) {
                Some(next) => i = next,
                None => return false,
            }
        } else if is_url_char(bytes[i]) {
            i += 1;
        } else {
            return false;
        }
    }
    true
}

/// Userinfo: URL characters except '/', with well-formed
/// percent-escapes. (v1 allows a raw '@' inside userinfo; the split
/// uses the LAST '@' — see ADR-0009.)
fn is_valid_userinfo(s: &str) -> bool {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            match valid_percent_at(bytes, i) {
                Some(next) => i = next,
                None => return false,
            }
        } else if bytes[i] != b'/' && is_url_char(bytes[i]) {
            i += 1;
        } else {
            return false;
        }
    }
    true
}

/// A '%' at `i` followed by two hex digits; returns the next index.
fn valid_percent_at(bytes: &[u8], i: usize) -> Option<usize> {
    if i + 2 < bytes.len()
        && (bytes[i + 1] as char).is_ascii_hexdigit()
        && (bytes[i + 2] as char).is_ascii_hexdigit()
    {
        Some(i + 3)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nbe_core::snapshot::{check_golden, GoldenOutcome};

    fn roundtrip(input: &str) -> Option<String> {
        parse_url(input).map(|url| url.to_string())
    }

    #[test]
    fn roundtrips_canonical_urls() {
        assert_eq!(
            roundtrip("https://example.com/"),
            Some(String::from("https://example.com/"))
        );
        assert_eq!(
            roundtrip("http://example.com/path/to/page?name=ferret#fragment"),
            Some(String::from(
                "http://example.com/path/to/page?name=ferret#fragment"
            ))
        );
        assert_eq!(
            roundtrip("http://user:pass@example.com:8080/x"),
            Some(String::from("http://user:pass@example.com:8080/x"))
        );
        assert_eq!(
            roundtrip("http://user@example.com/"),
            Some(String::from("http://user@example.com/"))
        );
    }

    #[test]
    fn scheme_and_host_case_normalize() {
        assert_eq!(
            roundtrip("HTTP://ExAmPle.CoM/Path?Q=1"),
            Some(String::from("http://example.com/Path?Q=1"))
        );
    }

    #[test]
    fn default_ports_are_stripped() {
        assert_eq!(
            roundtrip("http://a.com:80/x"),
            Some(String::from("http://a.com/x"))
        );
        assert_eq!(
            roundtrip("https://a.com:443/x"),
            Some(String::from("https://a.com/x"))
        );
        assert_eq!(
            roundtrip("http://a.com:8080/x"),
            Some(String::from("http://a.com:8080/x"))
        );
        assert_eq!(
            roundtrip("http://a.com:/x"),
            Some(String::from("http://a.com/x"))
        );
        assert_eq!(
            roundtrip("http://a.com:00080/x"),
            Some(String::from("http://a.com/x"))
        );
    }

    #[test]
    fn empty_path_becomes_slash() {
        assert_eq!(
            roundtrip("http://example.com"),
            Some(String::from("http://example.com/"))
        );
        assert_eq!(
            roundtrip("http://example.com#frag"),
            Some(String::from("http://example.com/#frag"))
        );
    }

    #[test]
    fn ipv4_number_forms() {
        assert_eq!(
            roundtrip("http://127.0.0.1/"),
            Some(String::from("http://127.0.0.1/"))
        );
        assert_eq!(
            roundtrip("http://0x7f.1/"),
            Some(String::from("http://127.0.0.1/"))
        );
        assert_eq!(
            roundtrip("http://2130706433/"),
            Some(String::from("http://127.0.0.1/"))
        );
        assert_eq!(
            roundtrip("http://999/"),
            Some(String::from("http://0.0.3.231/"))
        );
        assert_eq!(roundtrip("http://256.1.1.1/"), None);
        assert_eq!(roundtrip("http://1.2.3.4.5/"), None);
        assert_eq!(
            roundtrip("http://1.2.3.4./"),
            Some(String::from("http://1.2.3.4/"))
        );
    }

    #[test]
    fn ipv6_forms() {
        assert_eq!(
            roundtrip("http://[::1]/"),
            Some(String::from("http://[::1]/"))
        );
        assert_eq!(
            roundtrip("http://[2001:db8::1]/"),
            Some(String::from("http://[2001:db8::1]/"))
        );
        assert_eq!(
            roundtrip("http://[1:2:3:4:5:6:7:8]/"),
            Some(String::from("http://[1:2:3:4:5:6:7:8]/"))
        );
        assert_eq!(
            roundtrip("http://[::ffff:1.2.3.4]/"),
            Some(String::from("http://[::ffff:102:304]/"))
        );
        assert_eq!(roundtrip("http://[1:2:3:4:5:6:7:8:9]/"), None);
        assert_eq!(roundtrip("http://[1::2::3]/"), None);
        assert_eq!(roundtrip("http://[:::]/"), None);
        assert_eq!(roundtrip("http://[1:2:3]/"), None);
    }

    #[test]
    fn dot_segments_are_removed() {
        assert_eq!(
            roundtrip("http://a/b/c/./../d"),
            Some(String::from("http://a/b/d"))
        );
        assert_eq!(
            roundtrip("http://a/b/../.."),
            Some(String::from("http://a/"))
        );
        assert_eq!(
            roundtrip("http://a/./b/"),
            Some(String::from("http://a/b/"))
        );
    }

    #[test]
    fn userinfo_roundtrips() {
        assert_eq!(
            roundtrip("http://user@host/"),
            Some(String::from("http://user@host/"))
        );
        assert_eq!(
            roundtrip("http://user:pass@host:8080/"),
            Some(String::from("http://user:pass@host:8080/"))
        );
    }

    #[test]
    fn rejects_malformed_urls() {
        assert_eq!(roundtrip("not a url"), None);
        assert_eq!(roundtrip("http://"), None);
        assert_eq!(roundtrip("http://a.com:99999/"), None);
        assert_eq!(roundtrip("http://a:b/"), None);
        assert_eq!(roundtrip("ftp://a.com/"), None);
        assert_eq!(roundtrip("http://a/b%zz/"), None);
        assert_eq!(roundtrip("http://a/b%2/"), None);
        assert_eq!(roundtrip("http://ex mple.com/"), None);
    }

    #[test]
    fn trailing_dot_host_is_normalized() {
        assert_eq!(
            roundtrip("http://example.com./"),
            Some(String::from("http://example.com/"))
        );
    }

    #[test]
    fn tabs_and_newlines_are_stripped() {
        assert_eq!(
            roundtrip("ht\ttp://example.com/\n"),
            Some(String::from("http://example.com/"))
        );
    }

    #[test]
    fn golden_url_parse_serialize() {
        let cases = [
            "https://example.com/",
            "http://example.com/path/to/page?name=ferret#fragment",
            "HTTP://ExAmPle.CoM/Path",
            "http://a.com:80/x",
            "http://a.com:8080/x",
            "http://0x7f.1/",
            "http://[::1]/",
            "http://[2001:db8::1]/",
            "http://[::ffff:1.2.3.4]/",
            "http://user:pass@example.com/x",
            "http://a/b/c/./../d",
            "http://a.com./",
        ];
        let mut text = String::new();
        for case in cases {
            match parse_url(case) {
                Some(url) => text.push_str(&format!("{case} => {url}\n")),
                None => text.push_str(&format!("{case} => <invalid>\n")),
            }
        }
        let outcome = check_golden("pp-07-url", text.as_bytes());
        assert!(
            matches!(outcome, GoldenOutcome::Matched | GoldenOutcome::Updated),
            "golden mismatch: {outcome:?}"
        );
    }
}
