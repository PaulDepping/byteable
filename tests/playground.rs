use std::io::Cursor;

use byteable::{Byteable, WriteValue};

#[derive(Byteable, Debug, PartialEq)]
#[byteable(io_only)]
struct MixedStruct {
    #[byteable(big_endian)]
    port: u16,
    payload: Vec<u8>,
}

#[test]
fn example() {
    let a = MixedStruct {
        port: 30,
        payload: vec![1, 2, 3],
    };

    let mut c = Cursor::new(Vec::new());
    c.write_value(&a).unwrap();
    println!("{:?}", c);
}
