use futures::{future, pin_mut, StreamExt, TryStreamExt};
use futures_channel::mpsc::{unbounded, UnboundedSender};
use std::{
    collections::HashMap,
    env, io,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;

type Sender = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Sender>>>;

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    let listener = TcpListener::bind(addr)
        .await
        .expect("Failed to start WebSocket server!");

    let state = PeerMap::new(Mutex::new(HashMap::new()));

    println!("Websocket server started at ws://127.0.0.1:8080");

    while let Ok((stream, addr)) = listener.accept().await {
        // Handle each client connection asynchronously
        tokio::spawn(handle_connection(state.clone(), stream, addr));
    }

    Ok(())
}

async fn handle_connection(peer_map: PeerMap, raw_stream: TcpStream, addr: SocketAddr) {
    println!("Incoming TCP connection from: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("Error during websocket handshake ocurred");

    println!("WebSocket connection established with {}", addr);

    let (sender, receiver) = unbounded();

    peer_map.lock().unwrap().insert(addr, sender);

    let (outgoing, incoming) = ws_stream.split();

    let broadcast_incoming = incoming.try_for_each(|msg| {
        let text = msg.to_text().unwrap();

        println!("Received a message from {}: {}", addr, text);

        let peers = peer_map.lock().unwrap();

        let broadcast_recipients = peers
            .iter()
            .filter(|(peer_addr, _)| peer_addr != &&addr)
            .map(|(_, ws_sink)| ws_sink);

        for recp in broadcast_recipients {
            recp.unbounded_send(msg.clone()).unwrap();
        }

        future::ok(())
    });

    let receive_from_others = receiver.map(Ok).forward(outgoing);

    pin_mut!(broadcast_incoming, receive_from_others);

    future::select(broadcast_incoming, receive_from_others).await;

    println!("{} disconnected", &addr);
    peer_map.lock().unwrap().remove(&addr);
}
