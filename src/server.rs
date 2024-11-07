mod client;

use futures_util::{future, StreamExt, TryStreamExt};
use log::*;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{ AsyncWriteExt};

async fn accept_connection(stream: TcpStream) {
    let addr = stream.peer_addr().expect("Connected streams should have a peer address");

    info!("Peer address: {}", addr);

    let web_socket_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    info!("New WebSocket connection: {}", addr);

    let (write, read) = web_socket_stream.split();


    read.try_filter(|msg| future::ready(msg.is_text() || msg.is_binary()))
        .forward(write)
        .await
        .expect("Failed to forward messages")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let addr = "127.0.0.1:9002";

    let listener = TcpListener::bind(&addr).await.expect("Can't listen");

    info!("Listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await{
        tokio::spawn(accept_connection(stream));
    }

    Ok(())
}

