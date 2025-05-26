use derive_more::{Deref, DerefMut};

use crate::memory::MemoryPage;

#[derive(Clone, Copy, Debug, Deref, DerefMut)]
pub struct VarInt(i64);

impl VarInt {
    pub fn from_buffer(mut buf: &[u8]) -> (Self, &[u8]) {
        let mut value: i64 = 0;

        for (i, b) in buf.iter().take(9).enumerate() {
            let mask = 0xffu8 >> (1 - (i / 8));
            let shift = 7 + (i / 8);
            buf = &buf[1..];

            value = (value << shift) + (b & mask) as i64;

            if b >> 7 == 0 {
                break;
            }
        }

        (Self(value), buf)
    }

    pub fn from_page(page: MemoryPage) -> (Self, MemoryPage) {
        let buf = page.buffer();
        let original_length = buf.len();

        let (value, buf) = Self::from_buffer(&buf);

        (value, page.slice(original_length - buf.len()..))
    }

    #[allow(unused)]
    fn to_bytes(self) -> Vec<u8> {
        if self.0 == 0 {
            return vec![0x00];
        }

        let mut bytes = Vec::with_capacity(9);

        let mut n = self.0;
        while n > 0 {
            bytes.push((n as u8 & 0b0111_1111) + 0b1000_0000);
            n >>= 7;
        }

        bytes.reverse();

        *bytes.last_mut().expect("0 already handled") &= 0b0111_1111;

        bytes
    }
}

#[cfg(test)]
mod test {

    use super::*;

    mod from_bytes {
        use super::*;

        #[test]
        fn n0() {
            assert_eq!(*VarInt::from_buffer(&[0b0000_0000]).0, 0);
        }

        #[test]
        fn n127() {
            assert_eq!(*VarInt::from_buffer(&[0b0111_1111]).0, 127);
        }

        #[test]
        fn n255() {
            assert_eq!(*VarInt::from_buffer(&[0b1000_0001, 0b0111_1111]).0, 255);
        }
    }

    mod to_bytes {
        use super::*;

        #[test]
        fn n0() {
            assert_eq!(VarInt::to_bytes(VarInt(0)), &[0b0000_0000]);
        }

        #[test]
        fn n127() {
            assert_eq!(VarInt::to_bytes(VarInt(127)), &[0b0111_1111]);
        }

        #[test]
        fn n255() {
            assert_eq!(VarInt::to_bytes(VarInt(255)), &[0b1000_0001, 0b0111_1111]);
        }

        #[test]
        fn i64_max() {
            assert_eq!(
                VarInt::to_bytes(VarInt(i64::MAX)),
                &[
                    0b1111_1111,
                    0b1111_1111,
                    0b1111_1111,
                    0b1111_1111,
                    0b1111_1111,
                    0b1111_1111,
                    0b1111_1111,
                    0b1111_1111,
                    0b0111_1111,
                ]
            );
        }
    }
}
