use byteable::{BigEndian, Byteable, ByteableRegular, LittleEndian};
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

#[derive(Clone, Copy, Debug, Byteable)]
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

impl ByteableRegular for MyStruct {
    type Raw = MyStructRaw;

    fn to_raw(self) -> Self::Raw {
        Self::Raw {
            a: self.a,
            b: BigEndian::new(self.b),
            c: LittleEndian::new(self.c),
            d: LittleEndian::new(self.d),
            e: BigEndian::new(self.e),
        }
    }

    fn from_raw(raw: Self::Raw) -> Self {
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
