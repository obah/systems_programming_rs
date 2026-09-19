use crate::{Error, frame::Frame, parser::parse};

const COMPACT_AT: usize = 4 * 1024;

#[derive(Default)]
pub struct Decoder {
    buf: Vec<u8>,
    cursor: usize,
}

impl Decoder {
    pub fn feed(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    pub fn next_frame(&mut self) -> Result<Option<Frame>, Error> {
        let Some((frame, consumed)) = parse(&self.buf[self.cursor..])? else {
            return Ok(None);
        };

        self.cursor += consumed;

        if self.cursor >= COMPACT_AT {
            self.buf.drain(..self.cursor);
            self.cursor = 0;
        }

        Ok(Some(frame))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoder_reassembles_frames_split_across_chunks() {
        let mut d = Decoder::default();

        d.feed(b"+OK\r\n+PA");
        assert_eq!(
            d.next_frame().unwrap(),
            Some(Frame::SimpleString("OK".into()))
        );
        assert_eq!(d.next_frame().unwrap(), None);

        d.feed(b"RTIAL\r\n");
        assert_eq!(
            d.next_frame().unwrap(),
            Some(Frame::SimpleString("PARTIAL".into()))
        );
        assert_eq!(d.next_frame().unwrap(), None);

        d.feed(b"?huh\r\n");
        assert_eq!(d.next_frame(), Err(Error::UnknownType(b'?')));
    }

    #[test]
    fn decoder_compacts_consumed_bytes() {
        let mut d = Decoder::default();
        for _ in 0..1000 {
            d.feed(b"+OK\r\n");
        }

        let mut frames = 0;
        while d.next_frame().unwrap().is_some() {
            frames += 1;
        }

        assert_eq!(frames, 1000);
        assert!(d.buf.len() < COMPACT_AT, "retained {} bytes", d.buf.len());
    }
}
