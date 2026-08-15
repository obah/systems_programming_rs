use bytes::{Buf, Bytes, BytesMut};
use std::io;
use tokio_util::codec::{Decoder, Encoder};

pub struct LengthPrefixedCodec {}

const MAX: usize = 4 * 1024 * 1024; //max size allowed for my payload

impl Encoder<Bytes> for LengthPrefixedCodec {
    type Error = io::Error;

    fn encode(&mut self, item: Bytes, dst: &mut BytesMut) -> Result<(), Self::Error> {
        if item.len() > MAX {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Frame of length {} is too large.", item.len()),
            ));
        }

        let len_slice = u32::to_be_bytes(item.len() as u32);

        dst.reserve(4 + item.len());

        dst.extend_from_slice(&len_slice);
        dst.extend_from_slice(&item);

        Ok(())
    }
}

impl Decoder for LengthPrefixedCodec {
    type Item = Bytes;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 4 {
            return Ok(None);
        }

        let mut length_bytes = [0u8; 4];
        length_bytes.copy_from_slice(&src[..4]);
        let length = u32::from_be_bytes(length_bytes) as usize;

        if length > MAX {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Frame of {} too large", length),
            ));
        }

        if src.len() < 4 + length {
            src.reserve(4 + length - src.len());
            return Ok(None);
        }

        src.advance(4);
        Ok(Some(src.split_to(length).freeze()))
    }
}

#[test]
fn roundtrip_and_partial() {
    let mut buf = BytesMut::new();
    LengthPrefixedCodec {}
        .encode(Bytes::from_static(b"\xff\xfe not utf8"), &mut buf)
        .unwrap();

    // one byte short -> no frame yet, buffer untouched
    let mut partial = buf.split_to(buf.len() - 1);
    assert!(
        LengthPrefixedCodec {}
            .decode(&mut partial)
            .unwrap()
            .is_none()
    );

    partial.unsplit(buf);
    assert_eq!(
        LengthPrefixedCodec {}
            .decode(&mut partial)
            .unwrap()
            .unwrap(),
        &b"\xff\xfe not utf8"[..]
    );
    assert!(partial.is_empty());
}
