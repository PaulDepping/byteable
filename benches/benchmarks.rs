use byteable::{BigEndian, Byteable, LittleEndian, UnsafeByteable, impl_byteable_via};
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

impl From<MyStructRaw> for MyStruct {
    fn from(value: MyStructRaw) -> Self {
        Self {
            a: value.a,
            b: value.b.get(),
            c: value.c.get(),
            d: value.d.get(),
            e: value.e.get(),
        }
    }
}

impl From<MyStruct> for MyStructRaw {
    fn from(value: MyStruct) -> Self {
        Self {
            a: value.a,
            b: value.b.into(),
            c: value.c.into(),
            d: value.d.into(),
            e: value.e.into(),
        }
    }
}

impl_byteable_via!(MyStruct => MyStructRaw);

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
            .as_byte_array()
        })
    });
}

criterion_group!(all_benchmarks, benchmarks);
criterion_main!(all_benchmarks);
