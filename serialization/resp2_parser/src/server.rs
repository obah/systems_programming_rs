use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use crate::{decoder::Decoder, encoder::encode, frame::Frame};

const READ_BUF: usize = 1024;

pub fn run(addr: &str) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    println!("listening on {addr}");

    for incoming in listener.incoming() {
        match incoming {
            Ok(stream) => {
                let peer = stream.peer_addr().ok();
                println!("-- open  {peer:?}");

                if let Err(e) = handle(stream) {
                    eprintln!("-- io error: {e}");
                }

                println!("-- close {peer:?}");
            }
            Err(e) => eprintln!("-- accept failed: {e}"),
        }
    }

    Ok(())
}

fn handle(mut stream: TcpStream) -> std::io::Result<()> {
    let mut decoder = Decoder::default();
    let mut chunk = [0u8; READ_BUF];

    loop {
        let n = stream.read(&mut chunk)?;

        if n == 0 {
            return Ok(());
        }

        println!("<- {}", chunk[..n].escape_ascii());

        decoder.feed(&chunk[..n]);

        loop {
            let reply = match decoder.next_frame() {
                Ok(Some(frame)) => {
                    println!("   parsed {frame:?}");
                    encode(&Frame::SimpleString("OK".into()))
                }
                Ok(None) => break,
                Err(e) => {
                    let reply = encode(&Frame::Error(format!("ERR protocol error: {e}")));
                    println!("-> {}", reply.escape_ascii());
                    stream.write_all(&reply)?;
                    return Ok(());
                }
            };

            println!("-> {}", reply.escape_ascii());
            stream.write_all(&reply)?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoder::encode_command;

    #[test]
    fn answers_a_command_over_a_real_socket() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            handle(stream).unwrap();
        });

        let mut client = TcpStream::connect(addr).unwrap();
        client.write_all(&encode_command(&[&b"PING"[..]])).unwrap();

        let mut reply = [0u8; 5];
        client.read_exact(&mut reply).unwrap();

        assert_eq!(&reply, b"+OK\r\n");
    }

    #[test]
    fn a_protocol_error_is_answered_then_the_socket_closes() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            handle(stream).unwrap();
        });

        let mut client = TcpStream::connect(addr).unwrap();
        client.write_all(b"?garbage\r\n").unwrap();

        let mut reply = Vec::new();
        client.read_to_end(&mut reply).unwrap();

        assert_eq!(
            reply,
            b"-ERR protocol error: unknown frame type byte 0x3f\r\n".to_vec()
        );
    }
}
