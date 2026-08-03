use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};
use std::{
    assert_eq, assert_ne,
    io::{self, Cursor},
    println,
};

fn write_endian<E: ByteOrder>(vals: (u32, i64, f64)) -> io::Result<Vec<u8>> {
    let mut c_buf = Cursor::new(Vec::with_capacity(20));

    c_buf.write_u32::<E>(vals.0)?;
    c_buf.write_i64::<E>(vals.1)?;
    c_buf.write_f64::<E>(vals.2)?;

    Ok(c_buf.into_inner())
}

fn read_endian<E: ByteOrder>(buf: &[u8]) -> io::Result<((u32, i64, f64), u64)> {
    let mut c_buf = Cursor::new(buf);

    let val_u32 = c_buf.read_u32::<E>()?;
    let val_i64 = c_buf.read_i64::<E>()?;
    let val_f64 = c_buf.read_f64::<E>()?;

    Ok(((val_u32, val_i64, val_f64), c_buf.position()))
}

fn main() -> io::Result<()> {
    let nums = (12, -12, 12.12);
    let expected_le = [
        0x0C, 0x00, 0x00, 0x00, // u32 12
        0xF4, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, // i64 -12, two's complement
        0x3D, 0x0A, 0xD7, 0xA3, 0x70, 0x3D, 0x28, 0x40, // f64 12.12
    ];

    let le = write_endian::<LittleEndian>(nums)?;
    assert_eq!(le, expected_le);
    let (l_vals, l_pos) = read_endian::<LittleEndian>(&le)?;
    assert_eq!(l_vals, nums);
    assert_eq!(le.len(), 20);
    assert_eq!(l_pos, le.len().try_into().unwrap());

    let be = write_endian::<BigEndian>(nums)?;
    let (b_vals, b_pos) = read_endian::<BigEndian>(&be)?;
    assert_eq!(b_vals, nums);
    assert_eq!(be.len(), 20);
    assert_eq!(b_pos, be.len().try_into().unwrap());
    assert_ne!(le, be);

    // SHOULD return false because of NAN, to_bits() or is_nan() will help though
    let special_nums = (u32::MAX, i64::MIN, f64::NAN);
    let le = write_endian::<LittleEndian>(special_nums)?;
    println!(
        "special nums are ({},{},{}) and in little endian form: {:?}",
        special_nums.0, special_nums.1, special_nums.2, le
    );
    assert_eq!(read_endian::<LittleEndian>(&le)?.0, special_nums);

    println!("b position is {b_pos}");

    Ok(())

    // for (v, k) in [
    //     (ValType::UInt32(0xDEADBEEF), Kind::UInt32),
    //     (ValType::Int64(-42), Kind::Int64),
    //     (ValType::Float64(3.25), Kind::Float64),
    // ] {
    //     let bytes = write_little_endian(&v);
    //     assert_eq!(read_little_endian(&bytes, k), v);
    // }

    // for (v, k) in [
    //     (ValType::UInt32(0xDEADBEEF), Kind::UInt32),
    //     (ValType::Int64(-42), Kind::Int64),
    //     (ValType::Float64(3.25), Kind::Float64),
    // ] {
    //     let bytes = write_big_endian(&v);
    //     assert_eq!(read_big_endian(&bytes, k), v);
    // }
}

// fn read_big_endian(value: &[u8], kind: Kind) -> ValType {
//     match kind {
//         Kind::UInt32 => ValType::UInt32(u32::from_be_bytes(value.try_into().unwrap())),
//         Kind::Int64 => ValType::Int64(i64::from_be_bytes(value.try_into().unwrap())),
//         Kind::Float64 => ValType::Float64(f64::from_be_bytes(value.try_into().unwrap())),
//     }
// }

// fn write_big_endian(value: &ValType) -> Vec<u8> {
//     match value {
//         ValType::UInt32(v) => v.to_be_bytes().to_vec(),
//         ValType::Int64(v) => v.to_be_bytes().to_vec(),
//         ValType::Float64(v) => v.to_be_bytes().to_vec(),
//     }
// }

// //using byteorder
// fn read_little_endian_1(value: &[u8], kind: Kind) -> ValType {
//     match kind {
//         Kind::UInt32 => ValType::UInt32(LittleEndian::read_u32(&value)),
//         Kind::Int64 => ValType::Int64(LittleEndian::read_i64(&value)),
//         Kind::Float64 => ValType::Float64(LittleEndian::read_f64(&value)),
//     }
// }

// //using stdlib
// fn read_little_endian(value: &[u8], kind: Kind) -> ValType {
//     match kind {
//         Kind::UInt32 => ValType::UInt32(u32::from_le_bytes(value.try_into().unwrap())),
//         Kind::Int64 => ValType::Int64(i64::from_le_bytes(value.try_into().unwrap())),
//         Kind::Float64 => ValType::Float64(f64::from_le_bytes(value.try_into().unwrap())),
//     }
// }

// //using byteorder
// fn write_little_endian_1(value: ValType) -> Vec<u8> {
//     match value {
//         ValType::UInt32(val) => {
//             let mut buf = [0; 4];
//             LittleEndian::write_u32(&mut buf, val);

//             return buf.to_vec();
//         }
//         ValType::Int64(val) => {
//             let mut buf = [0; 8];
//             LittleEndian::write_i64(&mut buf, val);

//             return buf.to_vec();
//         }
//         ValType::Float64(val) => {
//             let mut buf = [0; 8];
//             LittleEndian::write_f64(&mut buf, val);

//             return buf.to_vec();
//         }
//     }
// }

// // using stdlib
// fn write_little_endian(value: &ValType) -> Vec<u8> {
//     match value {
//         ValType::UInt32(v) => v.to_le_bytes().to_vec(),
//         ValType::Int64(v) => v.to_le_bytes().to_vec(),
//         ValType::Float64(v) => v.to_le_bytes().to_vec(),
//     }
// }
