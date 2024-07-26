pub fn size() -> ((u16, u16), (u16, u16)) {
    let (width, height) = termion::terminal_size().unwrap();
    let (width_pixels, height_pixels) = termion::terminal_size_pixels().unwrap();
    ((width, height), (width_pixels, height_pixels))
}

// pub fn character_size() -> (u16, u16) {
//     let ((width, height), (width_pixels, height_pixels)) = size();
//     let character_width = width_pixels / width;
//     let character_height = height_pixels / height;

//     (character_width, character_height)
// }
