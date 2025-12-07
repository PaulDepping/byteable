use tokio::io::{AsyncWriteExt, AsyncReadExt};
use std::io::Cursor;

#[tokio::main]
async fn main() {
    let mut cursor = Cursor::new(vec![]);
    cursor.write_all(b"test").await.unwrap();
}
