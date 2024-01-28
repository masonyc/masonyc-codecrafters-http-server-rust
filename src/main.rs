use core::fmt;
use std::{collections::HashMap, fmt::format};

use anyhow::{Context, Ok};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:4221").await?;
    loop {
        // The second item contains the IP and port of the new connection.
        let (mut socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(e) = process(&mut socket).await {
                eprintln!("Error handling request {:?}", e)
            };
        });
    }
}

async fn process(stream: &mut TcpStream) -> anyhow::Result<()> {
    let mut buf = [0u8; 1024];
    stream
        .read(&mut buf)
        .await
        .context("CTX: handle connection read buffer")?;

    let request = HttpRequest::from_byte_array(&buf);

    let response = if request.verb == "GET" && request.path == "/" {
        HttpResponse::new("".to_string(), request.protocol, 200)
    } else if request.verb == "GET" && request.path.starts_with("/echo/") {
        let echo_content = request
            .path
            .split_once("/echo/")
            .expect("Echo should contain content")
            .1;
        HttpResponse::new(echo_content.to_string(), request.protocol, 200)
    } else if request.verb == "GET" && request.path == "user-agent" {
        HttpResponse::new(
            request.headers.get("User-Agent").unwrap().to_string(),
            request.protocol,
            200,
        )
    } else {
        HttpResponse::new("".to_string(), request.protocol, 404)
    };
    stream.write_all(response.to_string().as_bytes()).await?;
    Ok(())
}

struct HttpResponse {
    body: String,
    protocol: String,
    status: String,
}

impl HttpResponse {
    fn new(body: String, protocol: String, status: u16) -> Self {
        let status = if status == 200 {
            "200 OK"
        } else if status == 404 {
            "404 Not Found"
        } else {
            panic!("HTTP status code not supported")
        }
        .to_string();

        Self {
            body,
            protocol,
            status,
        }
    }
}

impl fmt::Display for HttpResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut response = format!("{} {}", self.protocol, self.status);

        if !self.body.is_empty() {
            response = format!(
                "{}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                response,
                self.body.len(),
                self.body
            );
        }

        write!(f, "{}\r\n\r\n", response)
    }
}

struct HttpRequest {
    verb: String,
    path: String,
    protocol: String,
    headers: HashMap<String, String>,
}

impl HttpRequest {
    fn from_byte_array(buf: &[u8; 1024]) -> Self {
        let data = String::from_utf8_lossy(&buf[..]);
        let mut parts = data.split_whitespace();

        let verb = parts
            .next()
            .expect("Request should contains verb")
            .to_string();
        let path = parts
            .next()
            .expect("Request should contains path")
            .to_string();
        let protocol = parts
            .next()
            .expect("Request should contains protocol")
            .to_string();

        let headers = parts
            .collect::<Vec<_>>()
            .chunks(2)
            .filter(|x| x.len() == 2)
            .map(|x| (x[0].to_string(), x[1].to_string()))
            .collect::<HashMap<_, _>>();
        Self {
            verb,
            path,
            protocol,
            headers,
        }
    }
}
