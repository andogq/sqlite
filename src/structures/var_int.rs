use derive_more::{Deref, DerefMut};

#[derive(Clone, Copy, Deref, DerefMut)]
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
}
