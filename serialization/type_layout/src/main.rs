//define the same struct three ways (default, repr(C), repr(packed)), print size_of and offset_of for each, and explain the padding you see.

use std::mem::offset_of;

#[repr(C)]
struct CStruct {
    data: i8,
    tag: i16,
}

#[repr(packed)]
struct PackedStruct {
    data: i8,
    tag: i16,
}

#[repr(packed(2))]
struct PackedStruct2 {
    data: i8,
    tag: i16,
}

struct RustStruct {
    data: i8,
    tag: i16,
}

fn main() {
    println!(
        "Size of CStruct is {} and its offset of data is {}, offset of tag is {}",
        size_of::<CStruct>(),
        offset_of!(CStruct, data),
        offset_of!(CStruct, tag)
    );
    println!(
        "Size of PackedStruct is {} and its offset of data is {}, offset of tag is {}",
        size_of::<PackedStruct>(),
        offset_of!(PackedStruct, data),
        offset_of!(PackedStruct, tag)
    );
    println!(
        "Size of PackedStruct2 is {} and its offset of data is {}, offset of tag is {}",
        size_of::<PackedStruct2>(),
        offset_of!(PackedStruct2, data),
        offset_of!(PackedStruct2, tag)
    );
    println!(
        "Size of RustStruct is {} and its offset of data is {}, offset of tag is {}",
        size_of::<RustStruct>(),
        offset_of!(RustStruct, data),
        offset_of!(RustStruct, tag)
    );
    let packed_struct = PackedStruct { data: 10, tag: 11 };
    let r = &packed_struct.tag; //wont compile because of alignment issues with the packed representation

    // OUTPUTS
    // Size of CStruct is 4 and its offset of data is 0, offset of tag is 2
    // Size of PackedStruct is 3 and its offset of data is 0, offset of tag is 1
    // Size of PackedStruct2 is 4 and its offset of data is 2, offset of tag is 0
    // Size of RustStruct is 4 and its offset of data is 2, offset of tag is 0

    // EXPLANATION
    // `tag: i16` has alignment 2, the highest in the struct, so every version has
    // struct alignment 2. Total size must be a multiple of that.
    //
    // CStruct (repr(C)): field order is fixed as declared, so `data` sits at 0 and
    // one byte of INTERIOR padding is inserted at offset 1 to push `tag` onto a
    // 2-byte boundary. tag = 2..4. Size is already 4, no trailing padding needed.
    //
    // RustStruct (repr(Rust)): the compiler is free to reorder, and sorts by
    // decreasing alignment, so tag = 0..2, data = 2..3, then 1 byte of TRAILING
    // padding to round 3 up to 4. Same size as repr(C), padding in a different
    // place, for a different reason. Reordering saved nothing here — with only
    // two fields there was nothing to win.
    //
    // PackedStruct (repr(packed)): alignment forced to 1, so no padding anywhere.
    // data = 0, tag = 1, size 3.
    //
    // PackedStruct2 (repr(packed(2))): identical to RustStruct. packed(N) *caps*
    // alignment at N, and the max field alignment is already 2, so the cap never
    // binds. And because there's no `C`, reordering is still allowed — that, not
    // packing, is why it differs from CStruct. To see packed(2) actually do
    // something, add an i64 field; its alignment 8 would get clamped to 2.
    //
    // THE POINT: repr(Rust) layout is UNSPECIFIED. These offsets are what today's
    // rustc emits for this target, not a guarantee — a future version may differ.
    // That's why a binary codec never reads bytes straight out of a repr(Rust)
    // struct: only repr(C) gives a layout you can write down and depend on.
    //
    // Cost of packed: `&packed_struct.tag` won't compile. References must be
    // aligned, and a packed field might not be.
}
