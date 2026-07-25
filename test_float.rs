fn main() {
    let bytes: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x45, 0x40];
    let value = f64::from_le_bytes(bytes);
    println!("Parsed value: {}", value);
    println!("Expected: 42.5");
    println!("Difference: {}", (value - 42.5).abs());
}
