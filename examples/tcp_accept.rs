use async_gen::stream;
use futures::StreamExt;
use nio::net::TcpListener;
use std::pin::pin;

#[nio::main]
async fn main() {
    let mut listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

    let mut incoming = pin!(stream! {
        loop {
            let socket = listener.accept().await.unwrap();
            yield socket.connect().await;
        }
    });

    while let Some(Ok(v)) = incoming.next().await {
        println!("handle = {:?}", v);
    }
}
