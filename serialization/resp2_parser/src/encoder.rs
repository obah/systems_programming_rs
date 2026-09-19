use crate::frame::Frame;

const CRLF: &[u8] = b"\r\n";

pub fn encode_command(args: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();

    write_line(&mut out, b'*', args.len().to_string().as_bytes());

    for arg in args {
        write_bulk(&mut out, arg);
    }

    out
}

pub fn encode(frame: &Frame) -> Vec<u8> {
    let mut out = Vec::new();
    write_frame(&mut out, frame);
    out
}

fn write_frame(out: &mut Vec<u8>, frame: &Frame) {
    match frame {
        Frame::SimpleString(s) => {
            debug_assert!(!s.contains(['\r', '\n']));
            write_line(out, b'+', s.as_bytes());
        }
        Frame::Error(s) => {
            debug_assert!(!s.contains(['\r', '\n']));
            write_line(out, b'-', s.as_bytes());
        }
        Frame::Integer(n) => write_line(out, b':', n.to_string().as_bytes()),
        Frame::Null => out.extend_from_slice(b"$-1\r\n"),
        Frame::BulkString(bytes) => write_bulk(out, bytes),
        Frame::Array(items) => {
            write_line(out, b'*', items.len().to_string().as_bytes());
            for item in items {
                write_frame(out, item);
            }
        }
    }
}

fn write_line(out: &mut Vec<u8>, prefix: u8, payload: &[u8]) {
    out.push(prefix);
    out.extend_from_slice(payload);
    out.extend_from_slice(CRLF);
}

fn write_bulk(out: &mut Vec<u8>, bytes: &[u8]) {
    write_line(out, b'$', bytes.len().to_string().as_bytes());
    out.extend_from_slice(bytes);
    out.extend_from_slice(CRLF);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn encodes_a_command_as_an_array_of_bulk_strings() {
        assert_eq!(
            encode_command(&[&b"GET"[..], b"foo"]),
            b"*2\r\n$3\r\nGET\r\n$3\r\nfoo\r\n".to_vec()
        );
        assert_eq!(encode_command(&[]), b"*0\r\n".to_vec());
        assert_eq!(
            encode_command(&[&b"SET"[..], b"k", &[0xff, 0x00, b'\n']]),
            b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$3\r\n\xff\x00\n\r\n".to_vec()
        );
    }

    #[test]
    fn a_command_round_trips_back_into_bulk_string_frames() {
        let bytes = encode_command(&[&b"SET"[..], b"key", b"a\r\nb"]);

        assert_eq!(
            parse(&bytes).unwrap(),
            Some((
                Frame::Array(vec![
                    Frame::BulkString(b"SET".to_vec()),
                    Frame::BulkString(b"key".to_vec()),
                    Frame::BulkString(b"a\r\nb".to_vec()),
                ]),
                bytes.len()
            ))
        );
    }

    #[test]
    fn every_reply_frame_round_trips_through_the_parser() {
        let frames = [
            Frame::SimpleString("OK".into()),
            Frame::Error("ERR unknown command".into()),
            Frame::Integer(-42),
            Frame::Null,
            Frame::BulkString(vec![0xff, 0xfe, 0xfd]),
            Frame::Array(vec![
                Frame::Integer(1),
                Frame::BulkString(b"hello".to_vec()),
                Frame::Array(vec![Frame::Null]),
            ]),
        ];

        for frame in frames {
            let bytes = encode(&frame);
            assert_eq!(parse(&bytes).unwrap(), Some((frame, bytes.len())));
        }
    }
}
