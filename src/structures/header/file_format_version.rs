use crate::create_enum;

create_enum!(FileFormatVersion(u8) => FileFormatVersionRaw(u8) {
    Legacy = 1,
    Wal = 2,
} [FileFormatVersionError = "file format version"]);
