// to extract bits at the LSB (rightmost), mask but to extract bits at the left (MSB), shift then mask
// private field => the only way to get a Tag is through new(), so every Tag in
// existence already fits in 4 bits. pack() can't be handed a bad one.
mod tag {
    const BITS: u32 = 4;
    const MAX: u8 = (1 << BITS) - 1; // 0x0F

    #[derive(Clone, Copy, Debug)]
    pub struct Tag(u8);

    impl Tag {
        pub fn new(value: u8) -> Option<Self> {
            (value <= MAX).then_some(Tag(value))
        }

        pub fn get(self) -> u8 {
            self.0
        }
    }
}

mod length {
    const BITS: u32 = 11;
    const MAX: u16 = (1 << BITS) - 1; //0b0000_0111_1111_1111

    #[derive(Clone, Copy, Debug)]
    pub struct Length(u16);

    impl Length {
        pub fn new(value: u16) -> Option<Self> {
            (value <= MAX).then_some(Length(value))
        }

        pub fn get(self) -> u16 {
            self.0
        }
    }
}

use length::Length;
use tag::Tag;

struct RecordHeader {
    type_tag: Tag,
    compressed: bool,
    length: Length,
}

impl RecordHeader {
    fn pack(&self) -> u16 {
        // tag and length need no check or mask: Tag can't hold anything wider than 4 bits and Length 11 bits

        // record header should be 16 bits: [15..5(length),4(compressed),3..0(type)]
        let record_header: u16 =
            self.type_tag.get() as u16 | (self.compressed as u16) << 4 | self.length.get() << 5;

        record_header
    }

    fn unpack(packed_header: u16) -> Self {
        // to unpack, we use shift and masking for the MSB
        Self {
            type_tag: Tag::new(packed_header as u8 & 0xF).expect("fits in 4 bits"), //casting down truncates, removing the MSB and leaving 8 bits which we unmask
            compressed: (packed_header >> 4 & 0b1) != 0,
            length: Length::new(packed_header >> 5 & 0b111_1111_1111).expect("fits in 11 bits"), //unmasking here isnt really needed, because after shifting, 11 bits is all that's left
        }
    }
}

fn main() -> Result<(), String> {
    // the boundary: one check, here, and the rest of the program can't get it wrong
    let tester = RecordHeader {
        type_tag: Tag::new(0b111).ok_or("tag exceeds 4 bits")?,
        compressed: true,
        length: Length::new(0b10_1111).ok_or("length exceeds 11 bits")?,
    };

    let packed = tester.pack();

    println!(
        "tester packed is {:b}, from {:b} + {:b} / {} + {:b}",
        packed,
        tester.type_tag.get(),
        tester.compressed as u16,
        tester.compressed,
        tester.length.get()
    );

    let unpacked = RecordHeader::unpack(packed);

    println!(
        "packed tester unpacked is tag: {:b}, compressed: {}, length: {:b}",
        unpacked.type_tag.get(),
        unpacked.compressed,
        unpacked.length.get()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // 4 + 1 + 11 = 16 bits exactly, so every u16 is a valid header and the whole
    // state space is only 65_536 values. Exhaustive is cheaper to write than a
    // random generator and can't miss a corner.
    #[test]
    fn round_trip_every_header() {
        for bits in 0..=u16::MAX {
            let header = RecordHeader::unpack(bits);
            assert_eq!(header.pack(), bits, "lost bits on {bits:#018b}");
        }
    }

    // Round-trip alone can't catch pack/unpack being wrong in mirrored ways
    // (both using the wrong shift). One golden value pins the actual layout.
    #[test]
    fn layout_is_length_compressed_tag() {
        let header = RecordHeader {
            type_tag: Tag::new(0b111).unwrap(),
            compressed: true,
            length: Length::new(0b10_1111).unwrap(),
        };

        assert_eq!(header.pack(), 0b0000_0101_1111_0111);
    }
}
