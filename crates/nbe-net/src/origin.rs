//! The origin model: the security boundary for same-origin checks.
//! Normative reference: ADR-0009.
//!
//! An origin is the (scheme, host, port) tuple; two URLs have the
//! same origin iff their tuples are equal. Default ports are already
//! stripped by the URL parser, so `http://a/` and `http://a:80/`
//! share an origin. Opaque ("null") origins arrive with sandboxed
//! documents in later packages.

use std::fmt;

use crate::url::{Host, Scheme};

/// A URL origin: scheme, host, and effective port.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Origin {
    scheme: Scheme,
    host: Host,
    port: Option<u16>,
}

impl Origin {
    /// Construct an origin.
    pub fn new(scheme: Scheme, host: Host, port: Option<u16>) -> Self {
        Self { scheme, host, port }
    }

    /// The origin's scheme.
    #[must_use]
    pub fn scheme(&self) -> Scheme {
        self.scheme
    }

    /// The origin's host.
    #[must_use]
    pub fn host(&self) -> &Host {
        &self.host
    }

    /// The origin's port; `None` means the scheme default.
    #[must_use]
    pub fn port(&self) -> Option<u16> {
        self.port
    }

    /// True when the origin is TLS-protected.
    #[must_use]
    pub fn is_tls(&self) -> bool {
        self.scheme.is_tls()
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}", self.scheme, self.host)?;
        if let Some(port) = self.port {
            write!(f, ":{port}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::url::parse_url;

    fn origin_of(input: &str) -> Option<Origin> {
        parse_url(input).map(|url| url.origin())
    }

    #[test]
    fn origin_equality_and_default_ports() {
        let a = origin_of("http://example.com/page").unwrap();
        let b = origin_of("http://example.com:80/other").unwrap();
        let c = origin_of("http://example.com:8080/").unwrap();
        let d = origin_of("https://example.com/").unwrap();
        let e = origin_of("http://www.example.com/").unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
        assert_ne!(a, e);
    }

    #[test]
    fn origin_serialization() {
        assert_eq!(
            origin_of("http://example.com/x").unwrap().to_string(),
            String::from("http://example.com")
        );
        assert_eq!(
            origin_of("https://example.com:8443/x").unwrap().to_string(),
            String::from("https://example.com:8443")
        );
        assert_eq!(
            origin_of("http://[::1]:8080/x").unwrap().to_string(),
            String::from("http://[::1]:8080")
        );
    }
}
