pub fn rgb_distance((r1, g1, b1): (u8, u8, u8), (r2, g2, b2): (u8, u8, u8)) -> f32 {
    let r = r1 as f32 - r2 as f32;
    let g = g1 as f32 - g2 as f32;
    let b = b1 as f32 - b2 as f32;

    // (r * r + g * g + b * b).sqrt()
    r.mul_add(r, g.mul_add(g, b * b)).sqrt()
}
