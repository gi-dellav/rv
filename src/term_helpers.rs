use crossterm::{
    execute,
    terminal::{Clear, ClearType},
};
use dialoguer::Select;
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use std::io::stdout;
use std::process;

pub fn clear_term() {
    execute!(stdout(), Clear(ClearType::All)).unwrap();
}

pub fn get_terminal_input(prompt: String) -> String {
    let mut rl = DefaultEditor::new().unwrap();
    match rl.readline(&prompt) {
        Ok(line) => {
            // Don't exit here, let the caller handle /quit and /exit
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

pub enum ActionSelection {
    /// Enter Chat Mode
    EnterChatMode,
    /// `git add . && git commit --fixup $COMMIT`
    GitAddAndFixup,
    /// `git add . && git commit`
    GitAddAndCommit,
    /// `git revert $COMMIT-1`
    GitRevertLast,
    /// Exit program
    Quit,
}

pub fn select_action_menu() -> ActionSelection {
    let items = vec![
        "Enter Chat Mode",
        "Fix Git commit",
        "Create new Git commit",
        "Revert to previous Git commit",
        "Exit rv",
    ];

    let selection = Select::new()
        //.with_prompt(">")
        .items(&items)
        .interact()
        .unwrap();

    match selection {
        0 => ActionSelection::EnterChatMode,
        1 => ActionSelection::GitAddAndFixup,
        2 => ActionSelection::GitAddAndCommit,
        3 => ActionSelection::GitRevertLast,
        4 => ActionSelection::Quit,
        _ => {
            panic!("ActionMenu selection failed");
        }
    }
}
