use crossterm::terminal;

pub fn step_size() -> u32 {
    let (width, height) = terminal::size().unwrap();

    (((width / (height - 2)) as u32).saturating_sub(2)).max(2)
}
