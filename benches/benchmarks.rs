use byteable::{Byteable, IntoByteArray};
use core::f32;
use core::hint::black_box;
use criterion::{Criterion, criterion_group, criterion_main};

#[derive(Clone, Copy, Debug, Byteable)]
struct MyStruct {
    a: u8,
    #[byteable(little_endian)]
    b: u32,
    #[byteable(big_endian)]
    c: u16,
    #[byteable(big_endian)]
    d: f32,
    #[byteable(little_endian)]
    e: u128,
}

fn benchmarks(c: &mut Criterion) {
    c.bench_function("as_bytearray_mystruct", |b| {
        b.iter(|| {
            black_box([
                MyStruct {
                    a: 1,
                    b: 2,
                    c: 3,
                    d: 4.0,
                    e: 5,
                },
                MyStruct {
                    a: 2,
                    b: 67,
                    c: 128,
                    d: f32::consts::PI,
                    e: 0,
                },
            ])
            .into_byte_array()
        })
    });
}

criterion_group!(all_benchmarks, benchmarks);
criterion_main!(all_benchmarks);
