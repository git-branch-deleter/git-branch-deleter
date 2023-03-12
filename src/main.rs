use std::{
    error::Error,
    io::{stdin, stdout},
    process::Command,
};

use std::io::Write;

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

/// Info about a git branch
#[derive(Debug, Clone)]
struct Branch {
    /// Name of the git branch
    name: String,

    /// The latest stdout or stderr message from `git branch -d/-D {name}`
    status: String,
}

/// Keyboard input is mapped to one of these actions
enum Action {
    Delete,
    ForceDelete,
    Quit,
    MoveDown,
    MoveUp,
    None,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut keys = stdin().lock().keys();
    let mut stdout = stdout().lock().into_raw_mode()?;
    let (mut branches, max_branch_name_len) = get_local_branches();

    // Clear the screen once to avoid flicker
    write!(stdout, "{}", termion::clear::All)?;

    let mut selected = 0;
    loop {
        write!(stdout, "{}", termion::cursor::Goto::default())?;

        writeln!(stdout, "BRANCHES\r")?;
        writeln!(stdout, "\r")?;
        for (index, branch) in branches.iter().enumerate() {
            let prefix = if selected == index { "-> " } else { "   " };

            write!(stdout, "{prefix}{}", branch.name)?;

            let padding_len = max_branch_name_len - branch.name.len();
            for _ in 0..padding_len {
                write!(stdout, "     {}", branch.status)?;
            }

            write!(stdout, "{}\n\r", termion::clear::AfterCursor)?;
        }

        let selected_branch = branches.get_mut(selected).unwrap();
        let branch_name = selected_branch.name.clone();

        print_help(&mut stdout, max_branch_name_len, &branch_name)?;
        stdout.flush().unwrap();

        match key_to_action(keys.next().unwrap()?) {
            Action::Quit => break,
            Action::MoveUp => {
                if selected > 0 {
                    selected -= 1;
                }
            }
            Action::MoveDown => {
                if selected < branches.len() - 1 {
                    selected += 1;
                }
            }
            Action::Delete => {
                selected_branch.delete("-d");
            }
            Action::ForceDelete => {
                selected_branch.delete("-D");
            }
            _ => {}
        }
    }

    Ok(())
}

fn print_help(
    mut stdout: impl std::io::Write,
    indentation: usize,
    branch_name: &str,
) -> std::io::Result<()> {
    let indentation = " ".repeat(indentation as usize);
    writeln!(stdout, "\r")?;
    writeln!(stdout, "\r")?;
    writeln!(stdout, "COMMANDS\r")?;
    writeln!(stdout, "\r")?;
    writeln!(stdout, "    d{indentation}   git branch -d {branch_name}\r")?;
    writeln!(stdout, "\r")?;
    writeln!(stdout, "    D{indentation}   git branch -D {branch_name}\r")?;
    writeln!(stdout, "\r")?;
    writeln!(stdout, "    q{indentation}   Quit app\r")?;
    writeln!(stdout, "\r")?;

    Ok(())
}

impl Branch {
    fn from_line(line: impl AsRef<str>) -> Self {
        let status = line
            .as_ref()
            .starts_with("*")
            .then(|| "(current branch)".to_owned())
            .unwrap_or_default();

        Self {
            name: line.as_ref().split_at(2).1.to_owned(),
            status,
        }
    }

    fn delete(&mut self, delete_arg: &str) {
        let output = Command::new("git")
            .arg("branch")
            .arg(delete_arg)
            .arg(self.name.as_str())
            .output()
            .unwrap();
        self.status = String::from_utf8_lossy(if output.status.success() {
            &output.stdout
        } else {
            &output.stderr
        })
        .into();
    }
}

fn get_local_branches() -> (Vec<Branch>, usize) {
    let stdout = Command::new("git")
        .args(["branch", "--list", "--color=never"])
        .env("HOME", "/no-config")
        .env("XDG_CONFIG_HOME", "/no-config")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap()
        .stdout;

    let stdout: String = String::from_utf8_lossy(&stdout).into_owned();

    let branches: Vec<Branch> = stdout.lines().map(Branch::from_line).collect();

    let max_branch_name_len = branches
        .iter()
        .map(|branch| branch.name.len())
        .max()
        .unwrap_or(0);

    (branches, max_branch_name_len)
}

fn key_to_action(key: Key) -> Action {
    match key {
        Key::Down | Key::Ctrl('n') | Key::Char('j') => Action::MoveDown,
        Key::Up | Key::Ctrl('p') | Key::Char('k') => Action::MoveUp,
        Key::Esc | Key::Char('q') | Key::Ctrl('c') => Action::Quit,
        Key::Delete | Key::Char('d') => Action::Delete,
        Key::Char('D') => Action::ForceDelete,
        _ => Action::None,
    }
}
