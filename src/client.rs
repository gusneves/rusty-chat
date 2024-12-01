use std::{env, io::Write};

use futures_channel::mpsc::UnboundedSender;
use futures_util::{future, pin_mut, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

type Sender = UnboundedSender<Message>;

#[tokio::main]
async fn main() {
    print!("Enter your username: ");
    std::io::stdout().flush().expect("Failed to flush stdout");
    let mut username = String::new();
    let _ = std::io::stdin()
        .read_line(&mut username)
        .expect("Failed to read username from stdin");
    let mut username = username.trim().to_string();
    username.push_str(": ");

    let url = env::args()
        .nth(1)
        .unwrap_or_else(|| panic!("this program requires at least one argument"));

    let (sender, receiver) = futures_channel::mpsc::unbounded();
    let (ws_stream, _) = connect_async(&url).await.expect("Failed to connect");
    println!("WebSocket handshake has been successfully completed");

    tokio::spawn(read_stdin(username.as_bytes().to_vec(), sender));

    let (outgoing, incoming) = ws_stream.split();

    let stdin_to_ws = receiver.map(Ok).forward(outgoing);
    let ws_to_stdout = {
        incoming.for_each(|message| async {
            let data = message.unwrap().into_data();
            tokio::io::stdout().write_all(&data).await.unwrap();
        })
    };

    pin_mut!(stdin_to_ws, ws_to_stdout);
    future::select(stdin_to_ws, ws_to_stdout).await;
}

// Our helper method which will read data from stdin and send it along the
// sender provided.
async fn read_stdin(username: Vec<u8>, sender: Sender) {
    let mut stdin = tokio::io::stdin();
    loop {
        let mut buf = vec![0; 1024];
        let n = match stdin.read(&mut buf).await {
            Err(_) | Ok(0) => break,
            Ok(n) => n,
        };
        let username = username.clone();
        buf.truncate(username.len() + n);
        buf.splice(0..0, username);
        sender.unbounded_send(Message::Binary(buf)).unwrap();
    }
}
