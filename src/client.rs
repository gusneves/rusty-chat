use std::env;
use futures::pin_mut;
use futures_util::{future, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::tungstenite::Message;

async fn read_stdin(tx: futures_channel::mpsc::UnboundedSender<Message>) {
    let mut stdin = tokio::io::stdin();

    loop{
        let mut buf = vec![0; 1024];
        let n = match stdin.read(&mut buf).await {
            Err(_) | Ok(0) => break,
            Ok(n) => n
        };

        buf.truncate(n);
        tx.unbounded_send(Message::binary(buf)).unwrap();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = env::args().nth(1).expect("Missing url argument");

    let (stdin_tx, stdin_rx) = futures_channel::mpsc::unbounded();

    tokio::spawn(read_stdin(stdin_tx));

    let (web_socket_stream, _) = tokio_tungstenite::connect_async(url).await.expect("Failed to connect");

    let(write, read) = web_socket_stream.split();

    let stdin_to_web_socket = stdin_rx.map(Ok).forward(write);

    let web_socket_to_stdout = {
        read.for_each(|message| async {
            let data = message.unwrap().into_data();
            tokio::io::stdout().write_all(&data).await.unwrap();
        })
    };

    pin_mut!(stdin_to_web_socket, web_socket_to_stdout);

    future::select(stdin_to_web_socket, web_socket_to_stdout).await;

    Ok(())
}

