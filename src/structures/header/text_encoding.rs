use zerocopy::big_endian::U32;

use crate::create_enum;

create_enum!(TextEncoding(u32) => TextEncodingRaw(U32) {
    Utf8 = 1,
    Utf16Le = 2,
    Utf16Be = 3,
} [TextEncodingError = "texte encoding"]);
