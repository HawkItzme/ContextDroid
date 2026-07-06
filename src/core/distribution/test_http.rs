//! Minimal in-process HTTP fixture server for distribution tests.
//! Serves canned responses on an ephemeral port; the daemon thread dies
//! with the test process.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

type Route = (String, u16, Vec<(String, String)>, Vec<u8>);

pub fn serve(routes: Vec<(String, u16, Vec<u8>)>) -> String {
    serve_with_headers(
        routes
            .into_iter()
            .map(|(path, status, body)| (path, status, Vec::new(), body))
            .collect(),
    )
}

pub fn serve_with_headers(routes: Vec<Route>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture server");
    let base = format!("http://{}", listener.local_addr().unwrap());

    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            handle(stream, &routes);
        }
    });
    base
}

fn handle(stream: std::net::TcpStream, routes: &[Route]) {
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() {
        return;
    }
    // Drain headers so the client sees a well-behaved server.
    let mut line = String::new();
    while reader.read_line(&mut line).is_ok() && line.trim() != "" {
        line.clear();
    }

    let path = request_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .split('?')
        .next()
        .unwrap_or("/")
        .to_string();

    let mut stream = reader.into_inner();
    match routes.iter().find(|(p, ..)| *p == path) {
        Some((_, status, headers, body)) => {
            let mut response = format!(
                "HTTP/1.1 {} X\r\nContent-Length: {}\r\n",
                status,
                body.len()
            );
            for (k, v) in headers {
                response.push_str(&format!("{}: {}\r\n", k, v));
            }
            response.push_str("Connection: close\r\n\r\n");
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.write_all(body);
        }
        None => {
            let _ = stream.write_all(
                b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
        }
    }
}
