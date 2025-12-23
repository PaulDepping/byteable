use byteable::{BigEndian, Byteable, LittleEndian, UnsafeByteable};
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

#[derive(Clone, Copy, Debug, UnsafeByteable)]
#[repr(C, packed)]
struct MyStructRaw {
    a: u8,
    b: BigEndian<u32>,
    c: LittleEndian<u16>,
    d: LittleEndian<f32>,
    e: BigEndian<u16>,
}

#[derive(Clone, Copy, Debug)]
struct MyStruct {
    a: u8,
    b: u32,
    c: u16,
    d: f32,
    e: u16,
}

impl Byteable for MyStruct {
    type ByteArray = <MyStructRaw as Byteable>::ByteArray;

    fn as_bytearray(self) -> Self::ByteArray {
        MyStructRaw {
            a: self.a,
            b: self.b.into(),
            c: self.c.into(),
            d: self.d.into(),
            e: self.e.into(),
        }
        .as_bytearray()
    }

    fn from_bytearray(ba: Self::ByteArray) -> Self {
        let raw = MyStructRaw::from_bytearray(ba);
        Self {
            a: raw.a,
            b: raw.b.get(),
            c: raw.c.get(),
            d: raw.d.get(),
            e: raw.e.get(),
        }
    }
}

fn benchmarks(c: &mut Criterion) {
    c.bench_function("as_bytearray_mystruct", |b| {
        b.iter(|| {
            black_box(MyStruct {
                a: 1,
                b: 2,
                c: 3,
                d: 4.0,
                e: 5,
            })
            .as_bytearray()
        })
    });
}

criterion_group!(all_benchmarks, benchmarks);
criterion_main!(all_benchmarks);
