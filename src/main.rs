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
    let data = String::from_utf8_lossy(&buf[..]);
    let mut parts = data.split_whitespace();
    let _ = parts.next();
    let path = parts.next();
    let response = match path {
        Some("/") => "HTTP/1.1 200 OK\r\n\r\n",
        _ => "HTTP/1.1 404 Not Found\r\n\r\n",
    };
    stream.write_all(response.as_bytes()).await?;
    Ok(())
}
