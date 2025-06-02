use std::iter;

use ux::{i24, i48};

use crate::disk::var_int::VarInt;

#[derive(Clone, Debug)]
#[allow(unused)]
pub enum RecordType {
    Null,
    I8(i8),
    I16(i16),
    I24(i24),
    I32(i32),
    I48(i48),
    I64(i64),
    F64(f64),
    Zero,
    One,
    Reserved,
    Blob(Vec<u8>),
    String(String),
}

impl RecordType {
    pub fn string(self) -> Option<String> {
        match self {
            RecordType::String(field) => Some(field),
            _ => None,
        }
    }

    pub fn integer(self) -> Option<i64> {
        Some(match self {
            RecordType::I8(i) => i.into(),
            RecordType::I16(i) => i.into(),
            RecordType::I24(i) => i.into(),
            RecordType::I32(i) => i.into(),
            RecordType::I48(i) => i.into(),
            RecordType::I64(i) => i,
            _ => return None,
        })
    }
}

#[allow(unused)]
#[derive(Clone, Debug)]
pub struct Record {
    pub id: i64,
    pub fields: Vec<RecordType>,
}

impl Record {
    pub fn from_buf(id: i64, buf: &[u8]) -> Self {
        Self {
            id,
            fields: {
                let buf_len = buf.len();
                let (header_length, buf) = VarInt::from_buffer(buf);
                let remaining_header = *header_length as usize - (buf_len - buf.len());

                let mut header = &buf[..remaining_header];
                let mut body = &buf[remaining_header..];

                iter::from_fn(|| {
                    if header.is_empty() {
                        assert!(body.is_empty());
                        return None;
                    }

                    let (serial_type, rest) = VarInt::from_buffer(header);
                    header = rest;

                    let mut take_bytes = |n| {
                        let bytes = &body[..n];
                        body = &body[n..];
                        bytes
                    };

                    let mut i64_from_bytes = |n| {
                        assert!(n <= 8);

                        take_bytes(n).iter().fold(0i64, |n, b| (n << 8) | *b as i64)
                    };

                    let field = match *serial_type {
                        0 => RecordType::Null,
                        1 => RecordType::I8(i64_from_bytes(1) as i8),
                        2 => RecordType::I16(i64_from_bytes(2) as i16),
                        3 => RecordType::I24(i24::new(i64_from_bytes(3) as i32)),
                        4 => RecordType::I32(i64_from_bytes(4) as i32),
                        5 => RecordType::I48(i48::new(i64_from_bytes(6))),
                        6 => RecordType::I64(i64_from_bytes(8)),
                        7 => RecordType::F64(f64::from_bits(i64_from_bytes(8) as u64)),
                        8 => RecordType::Zero,
                        9 => RecordType::One,
                        10 | 11 => RecordType::Reserved,
                        n @ 12.. if n % 2 == 0 => {
                            let length = (n as usize - 12) / 2;

                            let mut buf = vec![0; length];
                            buf.copy_from_slice(take_bytes(length));

                            RecordType::Blob(buf)
                        }
                        n @ 13.. if n % 2 == 1 => {
                            let length = (n as usize - 13) / 2;

                            let mut buf = vec![0; length];
                            buf.copy_from_slice(take_bytes(length));

                            RecordType::String(
                                // TODO: Use different encoding depending on DB config
                                String::from_utf8(buf).unwrap(),
                            )
                        }
                        _ => unreachable!(),
                    };

                    Some(field)
                })
                .collect()
            },
        }
    }
}
