//! Simple example demonstrating basic usage of the `UnsafeByteable` derive macro.
//!
//! This example shows the most straightforward use case: converting structs
//! to and from byte arrays for serialization.

use byteable::{BigEndian, Byteable, LittleEndian, UnsafeByteable};

/// A simple sensor reading structure
#[derive(Clone, Copy, Debug, UnsafeByteable)]
#[repr(C, packed)]
struct SensorReadingRaw {
    sensor_id: u8,
    temperature: LittleEndian<u16>, // Temperature in 0.01°C units
    humidity: LittleEndian<u16>,    // Humidity in 0.01% units
    pressure: BigEndian<u32>,       // Pressure in Pascals
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SensorReading {
    sensor_id: u8,
    temperature: u16, // Temperature in 0.01°C units
    humidity: u16,    // Humidity in 0.01% units
    pressure: u32,    // Pressure in Pascals
}

impl Byteable for SensorReading {
    type ByteArray = <SensorReadingRaw as Byteable>::ByteArray;

    fn as_bytearray(self) -> Self::ByteArray {
        SensorReadingRaw {
            sensor_id: self.sensor_id,
            temperature: LittleEndian::new(self.temperature),
            humidity: LittleEndian::new(self.humidity),
            pressure: BigEndian::new(self.pressure),
        }
        .as_bytearray()
    }

    fn from_bytearray(ba: Self::ByteArray) -> Self {
        let raw = SensorReadingRaw::from_bytearray(ba);
        Self {
            sensor_id: raw.sensor_id,
            temperature: raw.temperature.get(),
            humidity: raw.humidity.get(),
            pressure: raw.pressure.get(),
        }
    }
}

/// A compact RGB color structure
#[derive(Clone, Copy, Debug, UnsafeByteable)]
#[repr(C, packed)]
struct RgbColorRaw {
    red: u8,
    green: u8,
    blue: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct RgbColor {
    red: u8,
    green: u8,
    blue: u8,
}

impl Byteable for RgbColor {
    type ByteArray = <RgbColorRaw as Byteable>::ByteArray;

    fn as_bytearray(self) -> Self::ByteArray {
        RgbColorRaw {
            red: self.red,
            green: self.green,
            blue: self.blue,
        }
        .as_bytearray()
    }

    fn from_bytearray(ba: Self::ByteArray) -> Self {
        let raw = RgbColorRaw::from_bytearray(ba);
        Self {
            red: raw.red,
            green: raw.green,
            blue: raw.blue,
        }
    }
}

fn main() {
    println!("=== Simple Byteable Usage Example ===\n");

    // Example 1: Create a sensor reading
    let reading = SensorReading {
        sensor_id: 5,
        temperature: 2547, // 25.47°C
        humidity: 6523,    // 65.23%
        pressure: 101325,  // Standard atmospheric pressure
    };

    println!("1. Sensor Reading:");
    println!("   Sensor ID: {}", reading.sensor_id);
    println!(
        "   Temperature: {:.2}°C",
        reading.temperature as f32 / 100.0
    );
    println!("   Humidity: {:.2}%", reading.humidity as f32 / 100.0);
    println!("   Pressure: {} Pa", reading.pressure);

    // Convert to bytes
    let bytes = reading.as_bytearray();
    println!("   Byte representation: {:?}", bytes);
    println!("   Size: {} bytes\n", bytes.len());

    // Example 2: Reconstruct from bytes
    println!("2. Reconstructing from bytes:");
    let reconstructed = SensorReading::from_bytearray(bytes);
    println!("   Reconstructed: {:?}", reconstructed);
    println!("   Matches original: {}\n", reconstructed == reading);

    // Example 3: Working with colors
    println!("3. RGB Color:");
    let cyan = RgbColor {
        red: 0,
        green: 255,
        blue: 255,
    };

    println!("   Color: RGB({}, {}, {})", cyan.red, cyan.green, cyan.blue);
    let color_bytes = cyan.as_bytearray();
    println!("   Bytes: {:?}", color_bytes);
    println!(
        "   Hex representation: #{:02X}{:02X}{:02X}\n",
        color_bytes[0], color_bytes[1], color_bytes[2]
    );

    // Example 4: Array of byteable structs
    println!("4. Working with arrays:");
    let color_palette = [
        RgbColor {
            red: 255,
            green: 0,
            blue: 0,
        }, // Red
        RgbColor {
            red: 0,
            green: 255,
            blue: 0,
        }, // Green
        RgbColor {
            red: 0,
            green: 0,
            blue: 255,
        }, // Blue
    ];

    println!("   Color palette:");
    for (i, color) in color_palette.iter().enumerate() {
        let bytes = color.as_bytearray();
        println!(
            "   Color {}: RGB({:3}, {:3}, {:3}) = {:?}",
            i + 1,
            color.red,
            color.green,
            color.blue,
            bytes
        );
    }

    // Convert entire palette to bytes
    let total_size = RgbColor::BINARY_SIZE * color_palette.len();
    println!("   Total palette size: {} bytes", total_size);

    println!("\n=== Example completed! ===");
}
