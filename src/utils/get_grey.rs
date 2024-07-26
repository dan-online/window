pub fn get_grey(r: u8, g: u8, b: u8) -> u8 {
    // (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32) as u8
    0.2126f32.mul_add(r as f32, 0.7152f32.mul_add(g as f32, 0.0722 * b as f32)) as u8
}
