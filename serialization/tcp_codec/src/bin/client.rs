use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use tcp_codec::LengthPrefixedCodec;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stream = TcpStream::connect("127.0.0.1:8080").await?;
    println!("Connected to server!");

    let mut framed = Framed::new(stream, LengthPrefixedCodec {});

    let message = Bytes::from("Hello, Length-Prefixed World!");
    println!("Sending: {:?}", message);
    framed.send(message).await?;

    if let Some(Ok(response)) = framed.next().await {
        println!("Received Echo: {:?}", response);
    }

    Ok(())
}
