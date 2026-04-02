//! Binary messaging protocol using a field enum.
//!
//! In a pub/sub protocol, each message type carries different data. A
//! `#[derive(Byteable)]` field enum handles this naturally: the discriminant
//! byte is written first, followed by the variant's fields in declaration order.
//! Variants can freely mix unit variants with fields containing `String`,
//! `Vec<u8>`, or any other `Writable` / `Readable` type.
//!
//! Wire layout per message:
//! ```text
//!   Ping / Pong  →  [0x01] / [0x02]                        (1 byte)
//!   Subscribe    →  [0x10] [u64 topic_len] [topic bytes]
//!   Publish      →  [0x11] [u64 topic_len] [topic bytes]
//!                          [u64 len] [payload bytes]
//!   Error        →  [0xFF] [u16 BE code] [u64 desc_len] [desc bytes]
//! ```

use byteable::{Byteable, ReadValue, WriteValue};
use std::io::{self, Cursor};

/// Messages exchanged between client and broker.
#[derive(Debug, PartialEq, Byteable)]
#[repr(u8)]
enum Message {
    Ping = 0x01,
    Pong = 0x02,
    Subscribe {
        topic: String,
    } = 0x10,
    Publish {
        topic: String,
        payload: Vec<u8>,
    } = 0x11,
    Error {
        #[byteable(big_endian)]
        code: u16,
        description: String,
    } = 0xFF,
}

fn main() -> io::Result<()> {
    let outgoing = vec![
        Message::Ping,
        Message::Subscribe {
            topic: "sensors/temperature".into(),
        },
        Message::Publish {
            topic: "sensors/temperature".into(),
            payload: vec![0x41, 0x20, 0x00, 0x00], // f32 10.0 LE
        },
        Message::Pong,
        Message::Error {
            code: 403,
            description: "not authorized".into(),
        },
    ];

    // ── Encode ────────────────────────────────────────────────────────────────

    let mut stream = Cursor::new(Vec::<u8>::new());
    stream.write_value(&outgoing)?;
    println!(
        "Encoded {} messages into {} bytes\n",
        outgoing.len(),
        stream.position()
    );

    // ── Decode ────────────────────────────────────────────────────────────────

    stream.set_position(0);
    let received: Vec<Message> = stream.read_value()?;

    for msg in &received {
        match msg {
            Message::Ping => println!("→ Ping"),
            Message::Pong => println!("→ Pong"),
            Message::Subscribe { topic } => println!("→ Subscribe({topic})"),
            Message::Publish { topic, payload } => {
                println!("→ Publish({topic}, {} bytes)", payload.len())
            }
            Message::Error { code, description } => {
                println!("→ Error({code}: {description})")
            }
        }
    }

    assert_eq!(outgoing, received);
    println!(
        "\nAll {} messages round-tripped successfully.",
        received.len()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn main() {
        super::main().unwrap();
    }
}
