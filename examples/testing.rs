use std::io::{self, Cursor};

use byteable::{Byteable, WriteValue};

#[derive(Clone, Copy, Debug, Byteable)]
enum Temperature {
    Fahrenheit(#[byteable(big_endian)] f64),
    Celsius(#[byteable(little_endian)] f64),
    Kelvin(#[byteable(little_endian)] f64),
}

#[derive(Clone, Debug, Byteable)]
#[byteable(io_only)]
struct Location {
    s: String,
    t: Temperature,
}

fn main() -> io::Result<()> {
    let t = Temperature::Celsius(16.8);
    let l = Location {
        s: String::from("Hannover"),
        t,
    };
    let mut buf = Cursor::new(Vec::new());
    buf.write_value(&l)?;
    let v = buf.into_inner();
    dbg!(v.len(), v);
    Ok(())
}
