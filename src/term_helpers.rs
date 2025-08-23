use crossterm::{execute, terminal::{Clear, ClearType}};
use std::io::{stdout, Write};

pub fn clear_term() {
    execute!(stdout(), Clear(ClearType::All)).unwrap();
}
