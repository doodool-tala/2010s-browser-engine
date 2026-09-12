//! HTTP transport integration tests: a real loopback TCP server per
//! test with canned responses, exercised by the real client. This is
//! the Transport seam end to end over actual sockets.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;

use nbe_net::http::HttpTransport;
use nbe_net::loader::Transport;
use nbe_net::url::parse_url;

/// A one-shot loopback server: serves the responses in order, one
/// per connection, then stops. Records each full request.
struct Loopback {
    port: u16,
    requests: mpsc::Receiver<String>,
}

impl Loopback {
    fn base(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    fn take_requests(&self, count: usize) -> Vec<String> {
        (0..count)
            .map(|_| self.requests.recv().expect("server recorded a request"))
            .collect()
    }

    fn start_lines(&self, count: usize) -> Vec<String> {
        self.take_requests(count)
            .into_iter()
            .map(|request| request.lines().next().unwrap_or_default().to_string())
            .collect()
    }
}

fn read_request(stream: &mut TcpStream) -> String {
    let mut request = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let read = stream.read(&mut chunk).unwrap();
        if read == 0 {
            break;
        }
        request.extend_from_slice(&chunk[..read]);
        if request.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8_lossy(&request).to_string()
}

fn serve<F>(responses: F) -> Loopback
where
    F: FnOnce(u16) -> Vec<Vec<u8>> + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, requests) = mpsc::channel();
    std::thread::spawn(move || {
        for response in responses(port) {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let request = read_request(&mut stream);
            if tx.send(request).is_err() {
                return;
            }
            stream.write_all(&response).unwrap();
        }
    });
    Loopback { port, requests }
}

fn response_with_length(status: u16, content_type: &str, body: &[u8]) -> Vec<u8> {
    let length = body.len();
    let mut response = format!(
        "HTTP/1.1 {status} X\r\nContent-Type: {content_type}\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n"
    )
    .into_bytes();
    response.extend_from_slice(body);
    response
}

fn redirect_response(location: &str) -> Vec<u8> {
    format!("HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\n\r\n").into_bytes()
}

#[test]
fn simple_get_roundtrips() {
    let server = serve(|_| vec![response_with_length(200, "text/plain", b"hello")]);
    let url = parse_url(&format!("{}/path?x=1", server.base())).unwrap();
    let mut transport = HttpTransport::new();
    let resource = transport.fetch(&url).unwrap();
    assert_eq!(resource.bytes(), b"hello");
    assert_eq!(resource.mime_type(), Some("text/plain"));

    let requests = server.take_requests(1);
    assert!(requests[0].starts_with("GET /path?x=1 HTTP/1.1\r\n"));
    assert!(requests[0].contains(&format!("Host: 127.0.0.1:{}\r\n", server.port)));
    assert!(requests[0].contains("Connection: close\r\n"));
}

#[test]
fn chunked_bodies_are_decoded() {
    let server = serve(|_| {
        vec![b"HTTP/1.1 200 X\r\nContent-Type: text/plain\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n".to_vec()]
    });
    let url = parse_url(&format!("{}/chunked", server.base())).unwrap();
    let mut transport = HttpTransport::new();
    let resource = transport.fetch(&url).unwrap();
    assert_eq!(resource.bytes(), b"hello world");
}

#[test]
fn bodies_without_length_read_to_close() {
    let server = serve(|_| {
        let mut response =
            b"HTTP/1.1 200 X\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n".to_vec();
        response.extend_from_slice(b"until the wire goes quiet");
        vec![response]
    });
    let url = parse_url(&format!("{}/quiet", server.base())).unwrap();
    let mut transport = HttpTransport::new();
    let resource = transport.fetch(&url).unwrap();
    assert_eq!(resource.bytes(), b"until the wire goes quiet");
}

#[test]
fn zero_length_body_is_empty() {
    let server = serve(|_| vec![response_with_length(200, "text/plain", b"")]);
    let url = parse_url(&format!("{}/empty", server.base())).unwrap();
    let mut transport = HttpTransport::new();
    let resource = transport.fetch(&url).unwrap();
    assert_eq!(resource.bytes(), b"");
}

#[test]
fn root_relative_redirects_are_followed() {
    let server = serve(|_| {
        vec![
            redirect_response("/next"),
            response_with_length(200, "text/html", b"<html>arrived</html>"),
        ]
    });
    let url = parse_url(&format!("{}/start", server.base())).unwrap();
    let mut transport = HttpTransport::new();
    let resource = transport.fetch(&url).unwrap();
    assert_eq!(resource.bytes(), b"<html>arrived</html>");
    assert_eq!(
        server.start_lines(2),
        vec![
            String::from("GET /start HTTP/1.1"),
            String::from("GET /next HTTP/1.1"),
        ]
    );
}

#[test]
fn absolute_redirects_are_followed() {
    let server = serve(|port| {
        vec![
            redirect_response(&format!("http://127.0.0.1:{port}/abs")),
            response_with_length(200, "text/plain", b"landed"),
        ]
    });
    let url = parse_url(&format!("{}/rel", server.base())).unwrap();
    let mut transport = HttpTransport::new();
    let resource = transport.fetch(&url).unwrap();
    assert_eq!(resource.bytes(), b"landed");
    assert_eq!(
        server.start_lines(2),
        vec![
            String::from("GET /rel HTTP/1.1"),
            String::from("GET /abs HTTP/1.1"),
        ]
    );
}

#[test]
fn redirect_chains_are_capped() {
    let server = serve(|_| {
        std::iter::repeat(redirect_response("/loop"))
            .take(5)
            .collect()
    });
    let url = parse_url(&format!("{}/start", server.base())).unwrap();
    let mut transport = HttpTransport::new().with_max_redirects(2);
    let error = transport.fetch(&url).unwrap_err();
    assert!(error.to_string().contains("too many redirects"));
}

#[test]
fn non_2xx_statuses_are_module_errors() {
    let server = serve(|_| vec![response_with_length(404, "text/plain", b"missing")]);
    let url = parse_url(&format!("{}/gone", server.base())).unwrap();
    let mut transport = HttpTransport::new();
    let error = transport.fetch(&url).unwrap_err();
    assert!(error.to_string().contains("status 404"));
}

#[test]
fn https_is_a_module_error_until_tls_lands() {
    let url = parse_url("https://example.com/").unwrap();
    let mut transport = HttpTransport::new();
    let error = transport.fetch(&url).unwrap_err();
    assert!(error.to_string().contains("TLS not yet supported"));
}

#[test]
fn connection_refusal_is_a_module_error() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let url = parse_url(&format!("http://127.0.0.1:{port}/x")).unwrap();
    let mut transport = HttpTransport::new();
    assert!(transport.fetch(&url).is_err());
}
