// Uncomment this block to pass the first stage
// use std::net::TcpListener;

use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage
    //
    let listener = TcpListener::bind("127.0.0.1:4221").await?;
    loop {
        // The second item contains the IP and port of the new connection.
        let (socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            process(socket).await;
        });
    }
}

async fn process(mut stream: TcpStream) {
    let _ = stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n").await;
}
