use async_gen::stream;
use futures_util::StreamExt;
use std::pin::pin;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

    let mut incoming = pin!(stream! {
        loop {
            let (socket, _) = listener.accept().await.unwrap();
            yield socket;
        }
    });

    while let Some(v) = incoming.next().await {
        println!("handle = {:?}", v);
    }
}
