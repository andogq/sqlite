use cuisiner::{Cuisiner, CuisinerError};
use zerocopy::ByteOrder;

pub struct VarInt(i64);

impl Cuisiner for VarInt {
    type Raw<B: zerocopy::ByteOrder> = ();

    fn try_from_raw<B: zerocopy::ByteOrder>(raw: Self::Raw<B>) -> Result<Self, CuisinerError> {
        todo!()
    }

    fn try_to_raw<B: zerocopy::ByteOrder>(self) -> Result<Self::Raw<B>, CuisinerError> {
        todo!()
    }

    fn from_bytes<B: ByteOrder>(bytes: &[u8]) -> Result<Self, CuisinerError> {
        let mut value: i64 = 0;

        for (i, b) in bytes.iter().take(9).enumerate() {
            let mask = 0xffu8 >> (1 - (i / 8));
            let shift = 7 + (i / 8);

            value = (value << shift) + (b & mask) as i64;

            if b >> 7 == 0 {
                break;
            }
        }

        Ok(Self(value))
    }

    fn to_bytes<B: ByteOrder>(self) -> Result<Vec<u8>, CuisinerError> {
        if self.0 == 0 {
            return Ok(vec![0x00]);
        }

        let mut bytes = Vec::with_capacity(9);

        let mut n = self.0;
        while n > 0 {
            bytes.push((n as u8 & 0b0111_1111) + 0b1000_0000);
            n >>= 7;
        }

        bytes.reverse();

        *bytes.last_mut().expect("0 already handled") &= 0b0111_1111;

        Ok(bytes)
    }
}

#[cfg(test)]
mod test {
    use zerocopy::BigEndian;

    use super::*;

    mod from_bytes {
        use super::*;

        #[test]
        fn n0() {
            assert_eq!(
                VarInt::from_bytes::<BigEndian>(&[0b0000_0000]).unwrap().0,
                0
            );
        }

        #[test]
        fn n127() {
            assert_eq!(
                VarInt::from_bytes::<BigEndian>(&[0b0111_1111]).unwrap().0,
                127
            );
        }

        #[test]
        fn n255() {
            assert_eq!(
                VarInt::from_bytes::<BigEndian>(&[0b1000_0001, 0b0111_1111])
                    .unwrap()
                    .0,
                255
            );
        }
    }

    mod to_bytes {
        use super::*;

        #[test]
        fn n0() {
            assert_eq!(
                VarInt::to_bytes::<BigEndian>(VarInt(0)).unwrap(),
                &[0b0000_0000]
            );
        }

        #[test]
        fn n127() {
            assert_eq!(
                VarInt::to_bytes::<BigEndian>(VarInt(127)).unwrap(),
                &[0b0111_1111]
            );
        }

        #[test]
        fn n255() {
            assert_eq!(
                VarInt::to_bytes::<BigEndian>(VarInt(255)).unwrap(),
                &[0b1000_0001, 0b0111_1111]
            );
        }

        #[test]
        fn i64_max() {
            assert_eq!(
                VarInt::to_bytes::<BigEndian>(VarInt(i64::MAX)).unwrap(),
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
