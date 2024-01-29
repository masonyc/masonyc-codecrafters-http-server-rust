use anyhow::{Context, Ok};
use clap::Parser;
use core::fmt;
use std::{collections::HashMap, fs, path::Path, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    directory: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let arg = Arc::new(Args::parse());
    let listener = TcpListener::bind("127.0.0.1:4221").await?;
    loop {
        // The second item contains the IP and port of the new connection.
        let (mut socket, _) = listener.accept().await?;
        let cloned_arg = arg.clone();

        tokio::spawn(async move {
            if let Err(e) = process(&mut socket, cloned_arg).await {
                eprintln!("Error handling request {:?}", e)
            };
        });
    }
}

async fn process(stream: &mut TcpStream, arg: Arc<Args>) -> anyhow::Result<()> {
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
        let response = HttpResponse::new(echo_content.to_string(), request.protocol, 200);
        response.prepare_plain_text_headers()
    } else if request.verb == "GET" && request.path == "/user-agent" {
        let response = HttpResponse::new(
            request.headers.get("User-Agent").unwrap().to_string(),
            request.protocol,
            200,
        );
        response.prepare_plain_text_headers()
    } else if request.verb == "GET" && request.path.starts_with("/files/") {
        let filename = request.path.split_once("/files").unwrap().1;
        dbg!("file name {}", filename);
        let path_str = format!("{}/{}", arg.directory.to_owned().unwrap(), filename);
        let path = Path::new(&path_str);

        if path.exists() {
            let body = fs::read_to_string(path).expect("Read file always sucuess");
            let response = HttpResponse::new(body, request.protocol, 200);
            response.prepare_octet_stream_headers()
        } else {
            HttpResponse::new("".to_string(), request.protocol, 404)
        }
    } else if request.verb == "POST" && request.path.starts_with("/files/") {
        let filename = request.path.split_once("/files").unwrap().1;
        dbg!("file name {}", filename);
        let path_str = format!("{}/{}", arg.directory.to_owned().unwrap(), filename);
        let path = Path::new(&path_str);

        let _ = fs::write(path, request.body);

        HttpResponse::new("".to_string(), request.protocol, 201)
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
    headers: HashMap<String, String>,
}

impl HttpResponse {
    fn new(body: String, protocol: String, status: u16) -> Self {
        let status = if status == 200 {
            "200 OK"
        } else if status == 404 {
            "404 Not Found"
        } else if status == 201 {
            "201 Created"
        } else {
            panic!("HTTP status code not supported")
        }
        .to_string();

        Self {
            body,
            protocol,
            status,
            headers: HashMap::new(),
        }
    }

    fn prepare_plain_text_headers(mut self) -> HttpResponse {
        self.headers
            .insert("Content-Type".to_string(), "text/plain".to_string());
        self.headers
            .insert("Content-Length".to_string(), self.body.len().to_string());
        self
    }

    fn prepare_octet_stream_headers(mut self) -> HttpResponse {
        self.headers.insert(
            "Content-Type".to_string(),
            "application/octet-stream".to_string(),
        );
        self.headers
            .insert("Content-Length".to_string(), self.body.len().to_string());
        self
    }
}

impl fmt::Display for HttpResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut response = format!("{} {}", self.protocol, self.status);

        self.headers.clone().into_iter().for_each(|header| {
            response = format!("{}\r\n{}: {}", response, header.0, header.1);
        });

        if !self.body.is_empty() {
            response = format!("{}\r\n\r\n{}", response, self.body);
        }

        write!(f, "{}\r\n\r\n", response)
    }
}

struct HttpRequest {
    verb: String,
    path: String,
    protocol: String,
    headers: HashMap<String, String>,
    body: String,
}

impl HttpRequest {
    fn from_byte_array(buf: &[u8; 1024]) -> Self {
        let data = String::from_utf8_lossy(&buf[..]);

        let (parts, body) = data.split_once("\r\n\r\n").unwrap_or_default();
        let mut parts = parts.split_whitespace();

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
            .map(|x| {
                (
                    x[0].split_once(':').unwrap().0.to_string(),
                    x[1].to_string(),
                )
            })
            .collect::<HashMap<_, _>>();

        dbg!("{:#?}", headers.clone());
        let body = body.to_string();
        dbg!("body {:#?}", body.clone());
        Self {
            verb,
            path,
            protocol,
            headers,
            body,
        }
    }
}
