use crossterm::{
    execute,
    terminal::{Clear, ClearType},
};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::io::stdout;
use std::process;

pub fn clear_term() {
    execute!(stdout(), Clear(ClearType::All)).unwrap();
}

pub fn get_terminal_input(prompt: String) -> String {
    let mut rl = DefaultEditor::new().unwrap();
    match rl.readline(&prompt) {
        Ok(line) => {
            if line == "/quit" {
                process::exit(0);
            }
            line
        }
        Err(ReadlineError::Interrupted) => {
            // User pressed Ctrl-C
            println!("Press Ctrl+D to close");
            String::new()
        }
        Err(ReadlineError::Eof) => {
            // User pressed Ctrl-D
            process::exit(0);
        }
        Err(err) => {
            eprintln!("Error reading input: {:?}", err);
            String::new()
        }
    }
}
