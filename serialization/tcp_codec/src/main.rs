use futures::{SinkExt, StreamExt};
use std::error;
use tcp_codec::LengthPrefixedCodec;
use tokio::net::TcpListener;
use tokio_util::codec::Framed;

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Server listening on 127.0.0.1:8080");

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("New connection from: {}", addr);

        tokio::spawn(async move {
            let mut framed = Framed::new(socket, LengthPrefixedCodec {});

            while let Some(frame_res) = framed.next().await {
                match frame_res {
                    Ok(bytes) => {
                        println!("From {}, received: {:?}", addr, bytes);

                        if let Err(e) = framed.send(bytes).await {
                            eprintln!("Failed to send frame {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("⚠️ Frame decoding error: {}", e);
                        break;
                    }
                }
            }

            println!("Connection closed");
        });
    }
}
