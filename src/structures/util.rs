use std::marker::PhantomData;

use thiserror::Error;
use zerocopy::{ByteOrder, FromBytes, Immutable, IntoBytes, Unaligned};

use super::{Valid, Validate, ValidityMarker};

/// A constant [`u8`] value. During verification, if the deserialised value doesn't match the
/// constant, then an error will be raised.
#[derive(Clone, Copy, Debug, IntoBytes, FromBytes, Immutable, Unaligned)]
#[repr(transparent)]
pub struct ConstU8<const N: u8, V: ValidityMarker = Valid>(u8, PhantomData<fn() -> V>);

impl<const N: u8, V: ValidityMarker> ConstU8<N, V> {
    pub const fn value() -> u8 {
        N
    }
}

impl<const N: u8, V: ValidityMarker> Validate<V> for ConstU8<N, V> {
    type Valid = u8;
    type Error = ConstU8Error;

    fn try_get(&self) -> Result<Self::Valid, Self::Error> {
        if self.0 != N {
            return Err(ConstU8Error {
                expected: N,
                found: self.0,
            });
        }

        Ok(N)
    }
}

impl<const N: u8, V: ValidityMarker> PartialEq for ConstU8<N, V> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<const N: u8, V: ValidityMarker> Eq for ConstU8<N, V> {}

/// Error produced during [`ConstU8::validate`].
#[derive(Clone, Debug, Error)]
#[error("expected const u8 value {expected} (found {found})")]
pub struct ConstU8Error {
    /// Expected constant value.
    pub expected: u8,
    /// Value that was deserialised.
    pub found: u8,
}

/// A byte sequence of `N` bytes used to represent a boolean. If all of the bytes are `0x00`, then
/// `false` will be assumed. Otherwise, `true` is the value.
#[derive(Clone, Debug, IntoBytes, FromBytes, Immutable, Unaligned)]
#[repr(transparent)]
pub struct ByteBoolean<const N: usize>([u8; N]);

impl<const N: usize> ByteBoolean<N> {
    /// Retrieve the value of the byte boolean.
    pub fn get(&self) -> bool {
        !self.0.iter().all(|b| *b == 0)
    }
}

/// A reserved sequence of bytes, which will enforce all bytes are `0x00`. If the deserialised
/// value contains non-zero bytes, an error will be raised during validation.
#[derive(Clone, Debug, IntoBytes, FromBytes, Immutable, Unaligned)]
#[repr(transparent)]
pub struct Reserved<const N: usize, V: ValidityMarker = Valid>([u8; N], PhantomData<fn() -> V>);

impl<const N: usize, V: ValidityMarker> Validate<V> for Reserved<N, V> {
    type Valid = ();
    type Error = ReservedError;

    fn try_get(&self) -> Result<Self::Valid, Self::Error> {
        if !self.0.iter().all(|b| *b == 0) {
            return Err(ReservedError(N));
        }

        Ok(())
    }
}

/// An error produced during [`Reserved::validate`].
#[derive(Clone, Debug, Error)]
#[error("expected {0} bytes of reserved zero bytes")]
pub struct ReservedError(pub usize);

/// A value that cannot be zero. If the deserialised value is zero, an error will be raised during
/// validation.
#[derive(Clone, Debug, IntoBytes, FromBytes, Immutable, Unaligned)]
#[repr(transparent)]
pub struct NonZero<T: ByteOrderNumber, V: ValidityMarker = Valid>(T, PhantomData<fn() -> V>);

impl<T: ByteOrderNumber, V: ValidityMarker> Validate<V> for NonZero<T, V> {
    type Valid = ();
    type Error = NonZeroError;

    fn try_get(&self) -> Result<Self::Valid, Self::Error> {
        self.0.get_non_zero().ok_or(NonZeroError)?;
        Ok(())
    }
}

/// An error produced during [`NonZero::validate`].
#[derive(Clone, Debug, Error)]
#[error("expected non-zero number, but found zero")]
pub struct NonZeroError;

/// A value that is optional. If it is deserialised as zero, then it will be assumed that the value
/// is not present (null).
#[derive(Clone, Copy, Debug, IntoBytes, FromBytes, Immutable, Unaligned, Eq, PartialEq)]
#[repr(transparent)]
pub struct Optional<T: ByteOrderNumber>(T);

impl<T: ByteOrderNumber> Optional<T> {
    /// Get the optional value.
    pub fn get(&self) -> Option<T::NonZero> {
        self.0.get_non_zero()
    }
}

/// A number that can be expressed with a [`ByteOrder`]. This is intended to contain all required
/// functionality and associated types for number types that may be used with number-adjacent types
/// ([`NonZero`], [`Optional`], etc).
pub trait ByteOrderNumber {
    /// The primitive representation of the number.
    type Inner;
    /// A type-safe non-zero container for the number (should be
    /// [`NonZero<Self::Inner>`](std::num::NonZero)).
    type NonZero;

    fn new(inner: Self::Inner) -> Self;

    /// Fetch the primitive representation of the value. This will handle any byte-order
    /// transformations.
    fn get(&self) -> Self::Inner;

    /// Fetch the non-zero representation of the primitive.
    fn get_non_zero(&self) -> Option<Self::NonZero>;
}

macro_rules! impl_byte_order_number {
    ($primitive:ty => $byte_order:path) => {
        impl<B: ByteOrder> ByteOrderNumber for $byte_order {
            type Inner = $primitive;
            type NonZero = std::num::NonZero<$primitive>;

            fn new(inner: Self::Inner) -> Self {
                Self::new(inner)
            }

            fn get(&self) -> Self::Inner {
                Self::get(*self)
            }

            fn get_non_zero(&self) -> Option<Self::NonZero> {
                Self::NonZero::try_from(self.get()).ok()
            }
        }
    };

    ($primitive:ty) => {
        impl ByteOrderNumber for $primitive {
            type Inner = $primitive;
            type NonZero = std::num::NonZero<$primitive>;

            fn new(inner: Self::Inner) -> Self {
                inner
            }

            fn get(&self) -> Self::Inner {
                *self
            }

            fn get_non_zero(&self) -> Option<Self::NonZero> {
                Self::NonZero::try_from(self.get()).ok()
            }
        }
    };
}

impl_byte_order_number!(u8);
impl_byte_order_number!(u16 => zerocopy::U16<B>);
impl_byte_order_number!(u32 => zerocopy::U32<B>);
impl_byte_order_number!(u64 => zerocopy::U64<B>);
impl_byte_order_number!(u128 => zerocopy::U128<B>);
impl_byte_order_number!(i8);
impl_byte_order_number!(i16 => zerocopy::I16<B>);
impl_byte_order_number!(i32 => zerocopy::I32<B>);
impl_byte_order_number!(i64 => zerocopy::I64<B>);
impl_byte_order_number!(i128 => zerocopy::I128<B>);

#[macro_export]
macro_rules! create_enum {
    ($ty:ident($repr:ty) => $raw:ident($raw_repr:ty) { $($variant:ident = $discriminant:expr,)* } [$error:ident = $hint:expr]) => {
        #[derive(Clone, Copy, Debug, zerocopy::IntoBytes, zerocopy::TryFromBytes, zerocopy::Immutable, zerocopy::Unaligned)]
        #[repr(transparent)]
        pub struct $raw<V: $crate::structures::ValidityMarker = $crate::structures::Valid>(
            $raw_repr,
            ::std::marker::PhantomData<fn() -> V>,
        );

        impl<V: $crate::structures::ValidityMarker> $crate::structures::Validate<V> for $raw<V> {
            type Valid = $ty;
            type Error = $error;

            fn try_get(&self) -> Result<Self::Valid, Self::Error> {
                Self::Valid::try_from(self)
            }
        }

        impl<V: $crate::structures::ValidityMarker> From<$ty> for $raw<V> {
            fn from(encoding: $ty) -> Self {
                Self(<$raw_repr as $crate::structures::util::ByteOrderNumber>::new(encoding.into()), ::std::marker::PhantomData)
            }
        }

        impl<V: $crate::structures::ValidityMarker> TryFrom<&$raw<V>> for $ty {
            type Error = $error;

            fn try_from(encoding: &$raw<V>) -> Result<Self, Self::Error> {
                <Self as num_enum::TryFromPrimitive>::try_from_primitive(
                    <$raw_repr as $crate::structures::util::ByteOrderNumber>::get(&encoding.0)
                )
                    .map_err(|e| $error(e.number))
            }
        }

        #[derive(Clone, Copy, Debug, num_enum::IntoPrimitive, num_enum::TryFromPrimitive)]
        #[repr($repr)]
        pub enum $ty {
             $($variant = $discriminant,)*
        }

        #[derive(Clone, Debug, thiserror::Error)]
        #[error("unknown {hint} value: {0}", hint = $hint)]
        pub struct $error($repr);
    };
}
