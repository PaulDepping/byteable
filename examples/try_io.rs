//! Example demonstrating fallible I/O operations with TryFromByteArray and TryIntoByteArray.
//!
//! This example shows how to implement types with validation that can fail during
//! byte conversion, and how to handle both I/O and conversion errors separately.

use byteable::{
    AssociatedByteArray, ReadTryByteable, TryByteableError, TryFromByteArray, TryIntoByteArray,
    WriteTryByteable,
};
use std::error::Error;
use std::fmt;
use std::io::Cursor;

/// A temperature value that must be within a valid range (-100°C to 100°C)
#[derive(Debug, PartialEq, Copy, Clone)]
struct Temperature {
    celsius: i32,
}

/// Error type for invalid temperature values
#[derive(Debug)]
enum TemperatureError {
    TooHot(i32),
    TooCold(i32),
}

impl fmt::Display for TemperatureError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TemperatureError::TooHot(temp) => {
                write!(f, "Temperature {}°C is too hot (max: 100°C)", temp)
            }
            TemperatureError::TooCold(temp) => {
                write!(f, "Temperature {}°C is too cold (min: -100°C)", temp)
            }
        }
    }
}

impl Error for TemperatureError {}

impl Temperature {
    const MIN: i32 = -100;
    const MAX: i32 = 100;

    fn new(celsius: i32) -> Result<Self, TemperatureError> {
        if celsius > Self::MAX {
            Err(TemperatureError::TooHot(celsius))
        } else if celsius < Self::MIN {
            Err(TemperatureError::TooCold(celsius))
        } else {
            Ok(Temperature { celsius })
        }
    }
}

// Implement the byteable traits for Temperature
impl AssociatedByteArray for Temperature {
    type ByteArray = [u8; 4];
}

impl TryFromByteArray for Temperature {
    type Error = TemperatureError;

    fn try_from_byte_array(bytes: [u8; 4]) -> Result<Self, Self::Error> {
        let value = i32::from_le_bytes(bytes);
        Temperature::new(value)
    }
}

impl TryIntoByteArray for Temperature {
    type Error = TemperatureError;

    fn try_into_byte_array(self) -> Result<[u8; 4], Self::Error> {
        // Validate before serialization (in case value was modified)
        if self.celsius > Self::MAX {
            return Err(TemperatureError::TooHot(self.celsius));
        } else if self.celsius < Self::MIN {
            return Err(TemperatureError::TooCold(self.celsius));
        }
        Ok(self.celsius.to_le_bytes())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("=== Byteable Try-Trait I/O Example ===\n");

    // Example 1: Writing and reading valid temperature
    println!("Example 1: Valid temperature");
    let temp = Temperature::new(25)?;
    println!("  Created temperature: {}°C", temp.celsius);

    let mut buffer = Cursor::new(Vec::new());
    buffer.write_try_byteable(temp)?;
    println!("  Written to buffer: {:?}", buffer.get_ref());

    buffer.set_position(0);
    let read_temp: Temperature = buffer.read_try_byteable()?;
    println!("  Read from buffer: {}°C", read_temp.celsius);
    assert_eq!(temp.celsius, read_temp.celsius);
    println!("  ✓ Success!\n");

    // Example 2: Reading invalid temperature from bytes
    println!("Example 2: Invalid temperature (too hot)");
    let invalid_bytes = 150i32.to_le_bytes(); // 150°C is too hot
    println!("  Bytes represent: 150°C");

    let mut cursor = Cursor::new(invalid_bytes);
    let result: Result<Temperature, TryByteableError<TemperatureError>> =
        cursor.read_try_byteable();

    match result {
        Ok(_) => println!("  Unexpected success"),
        Err(TryByteableError::Io(err)) => {
            println!("  ✗ I/O error: {}", err);
        }
        Err(TryByteableError::Conversion(err)) => {
            println!("  ✗ Conversion error: {}", err);
            println!("  ✓ Error handled correctly!\n");
        }
    }

    // Example 3: Reading invalid temperature (too cold)
    println!("Example 3: Invalid temperature (too cold)");
    let invalid_bytes = (-150i32).to_le_bytes(); // -150°C is too cold
    println!("  Bytes represent: -150°C");

    let mut cursor = Cursor::new(invalid_bytes);
    let result: Result<Temperature, TryByteableError<TemperatureError>> =
        cursor.read_try_byteable();

    match result {
        Ok(_) => println!("  Unexpected success"),
        Err(TryByteableError::Io(err)) => {
            println!("  ✗ I/O error: {}", err);
        }
        Err(TryByteableError::Conversion(err)) => {
            println!("  ✗ Conversion error: {}", err);
            println!("  ✓ Error handled correctly!\n");
        }
    }

    // Example 4: I/O error (not enough bytes)
    println!("Example 4: I/O error (incomplete data)");
    let incomplete = vec![1, 2]; // Only 2 bytes, need 4
    println!("  Buffer has only {} bytes (need 4)", incomplete.len());

    let mut cursor = Cursor::new(incomplete);
    let result: Result<Temperature, TryByteableError<TemperatureError>> =
        cursor.read_try_byteable();

    match result {
        Ok(_) => println!("  Unexpected success"),
        Err(TryByteableError::Io(err)) => {
            println!("  ✗ I/O error: {}", err);
            println!("  ✓ Error handled correctly!\n");
        }
        Err(TryByteableError::Conversion(err)) => {
            println!("  ✗ Conversion error: {}", err);
        }
    }

    // Example 5: Multiple valid temperatures
    println!("Example 5: Writing multiple temperatures");
    let temps = vec![
        Temperature::new(-50)?,
        Temperature::new(0)?,
        Temperature::new(25)?,
        Temperature::new(100)?,
    ];

    let mut buffer = Cursor::new(Vec::new());
    for temp in &temps {
        buffer.write_try_byteable(*temp)?;
        println!("  Written: {}°C", temp.celsius);
    }

    println!("\n  Reading back:");
    buffer.set_position(0);
    for expected in &temps {
        let temp: Temperature = buffer.read_try_byteable()?;
        println!("  Read: {}°C", temp.celsius);
        assert_eq!(temp.celsius, expected.celsius);
    }
    println!("  ✓ All temperatures match!\n");

    // Example 6: Error handling with pattern matching
    println!("Example 6: Comprehensive error handling");
    let test_values: Vec<(i32, &str)> = vec![(50, "valid"), (150, "too hot"), (-150, "too cold")];

    for (value, description) in test_values {
        println!("  Testing {}°C ({})", value, description);
        let bytes = value.to_le_bytes();
        let mut cursor = Cursor::new(bytes);

        match cursor.read_try_byteable() {
            Ok(temp) => {
                let t: Temperature = temp;
                println!("    ✓ Success: {}°C", t.celsius);
            }
            Err(TryByteableError::Io(err)) => {
                println!("    ✗ I/O error: {}", err);
            }
            Err(TryByteableError::Conversion(TemperatureError::TooHot(t))) => {
                println!("    ✗ Too hot: {}°C > {}°C", t, Temperature::MAX);
            }
            Err(TryByteableError::Conversion(TemperatureError::TooCold(t))) => {
                println!("    ✗ Too cold: {}°C < {}°C", t, Temperature::MIN);
            }
        }
    }

    println!("\n=== All examples completed successfully! ===");
    Ok(())
}
