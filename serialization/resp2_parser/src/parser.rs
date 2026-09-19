use crate::{Error, frame::Frame};

pub(crate) const MAX_BULK_LEN: usize = 512 * 1024 * 1024;

fn read_simple_data(input: &[u8]) -> Result<Option<(&str, usize)>, Error> {
    let Some(crlf_pos) = input.windows(2).position(|w| w == b"\r\n") else {
        return Ok(None);
    };

    let data = std::str::from_utf8(&input[1..crlf_pos])?;

    Ok(Some((data, crlf_pos + 2)))
}

fn read_bulk_string(input: &[u8]) -> Result<Option<(Option<&[u8]>, usize)>, Error> {
    let Some((length_str, body_start)) = read_simple_data(input)? else {
        return Ok(None);
    };

    if length_str == "-1" {
        return Ok(Some((None, body_start)));
    }

    let length = length_str.parse::<usize>()?;

    if length > MAX_BULK_LEN {
        return Err(Error::LengthTooLarge(length));
    }

    let body_end = body_start.saturating_add(length);

    match input.get(body_end..body_end.saturating_add(2)) {
        None => Ok(None),
        Some(b"\r\n") => Ok(Some((Some(&input[body_start..body_end]), body_end + 2))),
        Some(_) => Err(Error::MissingCrlf),
    }
}

fn read_array(input: &[u8]) -> Result<Option<(Frame, usize)>, Error> {
    let Some((count_str, mut consumed)) = read_simple_data(input)? else {
        return Ok(None);
    };

    if count_str == "-1" {
        return Ok(Some((Frame::Null, consumed)));
    }

    let count = count_str.parse::<usize>()?;

    let mut items = Vec::new();

    for _ in 0..count {
        let Some((frame, n)) = parse(&input[consumed..])? else {
            return Ok(None);
        };

        items.push(frame);
        consumed += n;
    }

    Ok(Some((Frame::Array(items), consumed)))
}

pub(crate) fn parse(input: &[u8]) -> Result<Option<(Frame, usize)>, Error> {
    let Some(&first) = input.first() else {
        return Ok(None);
    };

    match first {
        b'+' => Ok(read_simple_data(input)?.map(|(d, n)| (Frame::SimpleString(d.to_owned()), n))),
        b'-' => Ok(read_simple_data(input)?.map(|(d, n)| (Frame::Error(d.to_owned()), n))),
        b':' => match read_simple_data(input)? {
            None => Ok(None),
            Some((d, n)) => Ok(Some((Frame::Integer(d.parse::<i64>()?), n))),
        },
        b'$' => Ok(read_bulk_string(input)?.map(|(d, n)| match d {
            None => (Frame::Null, n),
            Some(bytes) => (Frame::BulkString(bytes.to_vec()), n),
        })),
        b'*' => read_array(input),
        _ => Err(Error::UnknownType(first)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_frames() {
        assert_eq!(
            parse(b"+OK\r\n").unwrap(),
            Some((Frame::SimpleString("OK".into()), 5))
        );
        assert_eq!(
            parse(b"+\r\n").unwrap(),
            Some((Frame::SimpleString("".into()), 3))
        );
        assert_eq!(
            parse(b"-ERR unknown\r\n").unwrap(),
            Some((Frame::Error("ERR unknown".into()), 14))
        );
        assert_eq!(parse(b":42\r\n").unwrap(), Some((Frame::Integer(42), 5)));
        assert_eq!(parse(b":-1\r\n").unwrap(), Some((Frame::Integer(-1), 5)));

        let (frame, n) = parse(b"+FIRST\r\n+SECOND\r\n").unwrap().unwrap();
        assert_eq!((frame, n), (Frame::SimpleString("FIRST".into()), 8));
        assert_eq!(
            parse(&b"+FIRST\r\n+SECOND\r\n"[n..]).unwrap(),
            Some((Frame::SimpleString("SECOND".into()), 9))
        );
    }

    #[test]
    fn parses_aggregate_frames() {
        let bulk = |s: &str| Frame::BulkString(s.as_bytes().to_vec());

        assert_eq!(
            parse(b"$5\r\nhello\r\n").unwrap(),
            Some((bulk("hello"), 11))
        );
        assert_eq!(parse(b"$0\r\n\r\n").unwrap(), Some((bulk(""), 6)));
        assert_eq!(parse(b"$-1\r\n").unwrap(), Some((Frame::Null, 5)));
        assert_eq!(parse(b"*0\r\n").unwrap(), Some((Frame::Array(vec![]), 4)));
        assert_eq!(
            parse(b"*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n").unwrap(),
            Some((Frame::Array(vec![bulk("hello"), bulk("world")]), 26))
        );
        assert_eq!(
            parse(b"*5\r\n:1\r\n:2\r\n:3\r\n:4\r\n$5\r\nhello\r\n").unwrap(),
            Some((
                Frame::Array(vec![
                    Frame::Integer(1),
                    Frame::Integer(2),
                    Frame::Integer(3),
                    Frame::Integer(4),
                    bulk("hello"),
                ]),
                31
            ))
        );

        // a payload is bytes, not text
        assert_eq!(
            parse(b"$3\r\n\xff\xfe\xfd\r\n").unwrap(),
            Some((Frame::BulkString(vec![0xff, 0xfe, 0xfd]), 9))
        );

        // every byte short of a whole frame is incomplete, never a partial frame
        let full = b"*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n";
        for n in 0..full.len() {
            assert_eq!(parse(&full[..n]).unwrap(), None, "{n}");
        }
    }

    #[test]
    fn incomplete_waits_and_invalid_rejects() {
        for partial in [&b""[..], b"+", b"+OK", b"+OK\r", b":42"] {
            assert_eq!(parse(partial).unwrap(), None, "{partial:?}");
        }

        assert_eq!(parse(b"+\xff\xfe\r\n"), Err(Error::InvalidUtf8));
        assert_eq!(parse(b":4x2\r\n"), Err(Error::InvalidInteger));
        assert_eq!(parse(b"?huh\r\n"), Err(Error::UnknownType(b'?')));
        // the length lies: the byte after the payload is not the terminator
        assert_eq!(parse(b"$1\r\nabc\r\n"), Err(Error::MissingCrlf));
        assert_eq!(
            parse(b"$999999999999\r\n"),
            Err(Error::LengthTooLarge(999_999_999_999))
        );
    }
}
