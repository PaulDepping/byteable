//! Binary file format combining a fixed-size header with variable-length records.
//!
//! The file header is a fixed-size struct: `BYTE_SIZE` is available at compile
//! time and `write_value` / `read_value` use zero-copy transmute under the hood.
//!
//! Each log record contains a `String` message and a `Vec<String>` tag list, so
//! its size is not known at compile time. `#[byteable(io_only)]` generates
//! `Readable` / `Writable` implementations that serialize fields sequentially.
//! `String` and `Vec` fields are prefixed with a `u64` little-endian length.
//!
//! File layout:
//! ```text
//!   [FileHeader: 9 bytes]
//!   [Record 0]
//!   [Record 1]
//!   ...
//! ```

use byteable::{ByteRepr, Byteable, ReadValue, WriteValue};
use std::fs::File;
use std::io::{self, BufReader, BufWriter};
use std::path::Path;

// ── Types ─────────────────────────────────────────────────────────────────────

/// Fixed-size file header. `FileHeader::BYTE_SIZE` is a compile-time constant.
#[derive(Clone, Copy, Debug, PartialEq, Byteable)]
struct FileHeader {
    magic: [u8; 4], // b"BREC"
    version: u8,
    #[byteable(little_endian)]
    record_count: u32,
}

/// A single log record. `String` and `Vec<String>` make the size dynamic,
/// so this type gets `Readable` / `Writable` rather than byte-array traits.
#[derive(Debug, PartialEq, Byteable)]
#[byteable(io_only)]
struct Record {
    timestamp: u64,
    level: u8, // 0 = INFO, 1 = WARN, 2 = ERROR
    message: String,
    tags: Vec<String>,
}

// ── I/O helpers ───────────────────────────────────────────────────────────────

fn write_records(path: &Path, records: &[Record]) -> io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);

    let header = FileHeader {
        magic: *b"BREC",
        version: 1,
        record_count: records.len() as u32,
    };

    // FileHeader is fixed-size: BYTE_SIZE bytes, zero-copy transmute.
    println!(
        "Writing header ({} bytes): {:?}",
        FileHeader::BYTE_SIZE,
        header
    );
    file.write_value(&header)?;

    for record in records {
        file.write_value(record)?;
    }
    Ok(())
}

fn read_records(path: &Path) -> io::Result<(FileHeader, Vec<Record>)> {
    let mut file = BufReader::new(File::open(path)?);

    let header: FileHeader = file.read_value()?;
    println!(
        "Read header: version={}, {} record(s)",
        header.version, header.record_count
    );

    let mut records = Vec::with_capacity(header.record_count as usize);
    for _ in 0..header.record_count {
        records.push(file.read_value()?);
    }
    Ok((header, records))
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() -> io::Result<()> {
    let records = vec![
        Record {
            timestamp: 1_700_000_000,
            level: 0,
            message: "server started on :8080".into(),
            tags: vec!["startup".into(), "http".into()],
        },
        Record {
            timestamp: 1_700_000_042,
            level: 0,
            message: "accepted connection from 192.168.1.5".into(),
            tags: vec!["network".into()],
        },
        Record {
            timestamp: 1_700_000_105,
            level: 1,
            message: "disk usage above 90%".into(),
            tags: vec!["disk".into(), "warning".into(), "ops".into()],
        },
        Record {
            timestamp: 1_700_000_210,
            level: 2,
            message: "failed to write to /var/log/app.log: permission denied".into(),
            tags: vec!["disk".into(), "error".into()],
        },
    ];

    let path = std::env::temp_dir().join("byteable_records.bin");

    // Write to file
    write_records(&path, &records)?;
    println!("Wrote {} records to {}\n", records.len(), path.display());

    // Read back from file
    let (_, loaded) = read_records(&path)?;
    println!();

    let level_name = |l| match l {
        0 => "INFO ",
        1 => "WARN ",
        _ => "ERROR",
    };

    for record in &loaded {
        println!(
            "[{}] {} {} — tags: {:?}",
            record.timestamp,
            level_name(record.level),
            record.message,
            record.tags,
        );
    }

    assert_eq!(records, loaded);
    println!("\nAll {} records verified.", loaded.len());

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn main() {
        super::main().unwrap();
    }
}
