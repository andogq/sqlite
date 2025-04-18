use zerocopy::big_endian::U32;

use crate::create_enum;

create_enum!(SchemaFormat(u32) => SchemaFormatRaw(U32) {
    V1 = 1,
    V2 = 2,
    V3 = 3,
    V4 = 4,
} [SchemaFormatError = "schema format"]);
