//! Simple example demonstrating basic usage of the `Byteable` derive macro.
//!
//! This example shows the most straightforward use case: converting structs
//! to and from byte arrays for serialization.

use byteable::Byteable;

/// A simple sensor reading structure
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Byteable)]
struct SensorReading {
    sensor_id: u8,
    #[byteable(little_endian)]
    temperature: u16, // Temperature in 0.01°C units
    #[byteable(little_endian)]
    humidity: u16, // Humidity in 0.01% units
    #[byteable(big_endian)]
    pressure: u32, // Pressure in Pascals
}

/// A compact RGB color structure
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Byteable)]
struct RgbColor {
    red: u8,
    green: u8,
    blue: u8,
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
    let bytes = reading.to_byte_array();
    println!("   Byte representation: {:?}", bytes);
    println!("   Size: {} bytes\n", bytes.len());

    // Example 2: Reconstruct from bytes
    println!("2. Reconstructing from bytes:");
    let reconstructed = SensorReading::from_byte_array(bytes);
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
    let color_bytes = cyan.to_byte_array();
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
        let bytes = color.to_byte_array();
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
    let total_size = RgbColor::BYTE_SIZE * color_palette.len();
    println!("   Total palette size: {} bytes", total_size);

    println!("\n=== Example completed! ===");
}
