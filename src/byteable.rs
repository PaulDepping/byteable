use crate::byte_array::ByteArray;

pub trait Byteable {
    const BYTE_SIZE: usize = Self::ByteArray::BYTE_SIZE;
    type ByteArray: ByteArray;
    fn as_byte_array(self) -> Self::ByteArray;
    fn from_byte_array(byte_array: Self::ByteArray) -> Self;
}

impl<T: Byteable, const SIZE: usize> Byteable for [T; SIZE] {
    type ByteArray = [T::ByteArray; SIZE];

    fn as_byte_array(self) -> Self::ByteArray {
        self.map(T::as_byte_array)
    }

    fn from_byte_array(byte_array: Self::ByteArray) -> Self {
        byte_array.map(T::from_byte_array)
    }
}

#[macro_export]
macro_rules! unsafe_byteable_transmute {
    ($($type:ty),+) => {
        $(
            impl $crate::Byteable for $type {
                type ByteArray = [u8; ::std::mem::size_of::<Self>()];
                fn as_byte_array(self) -> Self::ByteArray {
                    unsafe { ::std::mem::transmute(self) }
                }
                fn from_byte_array(byte_array: Self::ByteArray) -> Self {
                    unsafe { ::std::mem::transmute(byte_array) }
                }
            }
        )+
    };
}

#[macro_export]
macro_rules! impl_byteable_via {
    ($regular_type:ty => $raw_type:ty) => {
        impl $crate::Byteable for $regular_type {
            type ByteArray = <$raw_type as Byteable>::ByteArray;

            fn as_byte_array(self) -> Self::ByteArray {
                let raw: $raw_type = self.into();
                raw.as_byte_array()
            }

            fn from_byte_array(byte_array: Self::ByteArray) -> Self {
                let raw = <$raw_type>::from_byte_array(byte_array);
                raw.into()
            }
        }
    };
}

macro_rules! impl_byteable_primitive {
    ($($type:ty),+) => {
        $(
            impl $crate::Byteable for $type {
                type ByteArray = [u8; ::std::mem::size_of::<Self>()];

                fn as_byte_array(self) -> Self::ByteArray {
                    <$type>::to_ne_bytes(self)
                }

                fn from_byte_array(byte_array: Self::ByteArray) -> Self {
                    <$type>::from_ne_bytes(byte_array)
                }
            }
        )+
    };
}

impl_byteable_primitive!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);

#[cfg(test)]
mod tests {
    use crate::{BigEndian, Byteable, LittleEndian};
    use byteable_derive::UnsafeByteable;

    #[derive(Clone, Copy, PartialEq, Debug, UnsafeByteable)]
    #[repr(C, packed)]
    struct ABC {
        a: LittleEndian<u16>,
        b: LittleEndian<u16>,
        c: BigEndian<u16>,
    }

    #[test]
    fn test_impl() {
        let a = ABC {
            a: LittleEndian::new(1),
            b: LittleEndian::new(2),
            c: BigEndian::new(3),
        };

        let expected_bytes = [1, 0, 2, 0, 0, 3];
        assert_eq!(a.as_byte_array(), expected_bytes);

        let read_a = ABC::from_byte_array(expected_bytes);
        assert_eq!(read_a.a.get(), 1);
        assert_eq!(read_a.b.get(), 2);
        assert_eq!(read_a.c.get(), 3);
        assert_eq!(read_a, a);
    }

    #[test]
    fn test_cursor() {
        let a = ABC {
            a: LittleEndian::new(1),
            b: LittleEndian::new(2),
            c: BigEndian::new(3),
        };

        let expected_bytes = [1, 0, 2, 0, 0, 3];
        assert_eq!(a.as_byte_array(), expected_bytes);

        let read = ABC::from_byte_array(expected_bytes);
        assert_eq!(read.a.get(), 1);
        assert_eq!(read.b.get(), 2);
        assert_eq!(read.c.get(), 3);
        assert_eq!(read, a);
    }

    #[derive(Clone, Copy, PartialEq, Debug, UnsafeByteable)]
    #[repr(C, packed)]
    struct MyRawStruct {
        a: u8,
        b: LittleEndian<u32>,
        c: LittleEndian<u16>,
        d: u8,
        e: u8,
    }

    #[derive(Clone, Copy, PartialEq, Debug)]
    struct MyRegularStruct {
        a: u8,
        b: u32,
        c: u16,
        d: u8,
        e: u8,
    }

    impl From<MyRawStruct> for MyRegularStruct {
        fn from(value: MyRawStruct) -> Self {
            Self {
                a: value.a,
                b: value.b.get(),
                c: value.c.get(),
                d: value.d,
                e: value.e,
            }
        }
    }

    impl From<MyRegularStruct> for MyRawStruct {
        fn from(value: MyRegularStruct) -> Self {
            Self {
                a: value.a,
                b: value.b.into(),
                c: value.c.into(),
                d: value.d,
                e: value.e,
            }
        }
    }

    impl_byteable_via!(MyRegularStruct => MyRawStruct);

    #[test]
    fn test_byteable_regular() {
        let my_struct = MyRegularStruct {
            a: 192,
            b: 168,
            c: 1,
            d: 1,
            e: 2,
        };

        let bytes = my_struct.as_byte_array();
        assert_eq!(bytes, [192, 168, 0, 0, 0, 1, 0, 1, 2]);

        let struct_from_bytes = MyRegularStruct::from_byte_array([192, 168, 0, 0, 0, 1, 0, 1, 2]);
        assert_eq!(struct_from_bytes, my_struct);

        assert_eq!(MyRegularStruct::BYTE_SIZE, 9);
    }
}
