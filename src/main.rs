use std::{
    error::Error,
    io::{stdout, Write},
    process::Command,
};

use crossterm::{
    cursor::MoveTo,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};

/// Info about a git branch
#[derive(Debug)]
struct Branch {
    /// Name of the git branch
    name: String,

    /// The latest stdout or stderr message from `git branch -d/-D {name}`
    status: String,
}

/// Keyboard input is mapped to one of these actions
#[derive(Debug)]
enum Action {
    Delete,
    ForceDelete,
    Checkout,
    Quit,
    MoveDown,
    MoveUp,
    None,
}

#[derive(Debug)]
struct Selection {
    index: usize,
    max: usize,
}

/// To line things up nicely
const MARGIN: &str = "   ";

fn main() -> Result<(), Box<dyn Error>> {
    let mut terminal = Terminal::new()?;
    let (mut branches, max_branch_name_len) = local_git_branches();
    let mut selection = Selection::new(branches.len().saturating_sub(1));

    terminal.clear()?;
    loop {
        execute!(terminal.stdout, MoveTo(0, 0))?;

        print_branches(
            &mut terminal.stdout,
            &branches,
            selection.index,
            max_branch_name_len,
        )?;

        terminal.stdout.flush()?;

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind == KeyEventKind::Release {
            continue;
        }

        match key_to_action(key) {
            Action::MoveUp => selection.move_up(),
            Action::MoveDown => selection.move_down(),
            Action::Delete => {
                if let Some(selected_branch) = branches.get_mut(selection.index) {
                    selected_branch.delete("-d");
                }
            }
            Action::ForceDelete => {
                if let Some(selected_branch) = branches.get_mut(selection.index) {
                    selected_branch.delete("-D");
                }
            }
            Action::Checkout => {
                if let Some(selected_branch) = branches.get_mut(selection.index) {
                    terminal.clear()?;
                    terminal.restore()?;
                    selected_branch.checkout()?;
                    stdout().lock().flush()?;
                    break;
                }
            }
            Action::Quit => break,
            Action::None => {}
        }
    }

    Ok(())
}

fn print_branches<W: Write>(
    stdout: &mut W,
    branches: &[Branch],
    selected: usize,
    max_branch_name_len: usize,
) -> std::io::Result<()> {
    writeln!(stdout, "BRANCHES\r")?;
    writeln!(stdout, "\r")?;
    for (index, branch) in branches.iter().enumerate() {
        writeln!(
            stdout,
            "{}{}{}{MARGIN}{}\r",
            if selected == index { "-> " } else { "   " },
            branch.name,
            " ".repeat(max_branch_name_len - branch.name.len()),
            branch.status,
        )?;
        execute!(stdout, Clear(ClearType::UntilNewLine))?;
    }

    Ok(())
}

fn local_git_branches() -> (Vec<Branch>, usize) {
    // Do not set e.g. GIT_CONFIG_NOSYSTEM, because we want the branch order to
    // match what the user is used to
    let stdout = Command::new("git")
        .args(["branch", "--list", "--color=never"])
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

fn key_to_action(key: KeyEvent) -> Action {
    match (key.code, key.modifiers) {
        (KeyCode::Down | KeyCode::Right | KeyCode::Char('j'), _)
        | (KeyCode::Char('n'), KeyModifiers::CONTROL) => Action::MoveDown,
        (KeyCode::Up | KeyCode::Left | KeyCode::Char('k'), _)
        | (KeyCode::Char('p'), KeyModifiers::CONTROL) => Action::MoveUp,
        (KeyCode::Esc | KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            Action::Quit
        }
        (KeyCode::Delete | KeyCode::Char('d'), _) => Action::Delete,
        (KeyCode::Char('D'), _) => Action::ForceDelete,
        (KeyCode::Char('c') | KeyCode::Enter, _) => Action::Checkout,
        _ => Action::None,
    }
}

struct Terminal {
    stdout: std::io::Stdout,
    raw_mode_enabled: bool,
}

impl Terminal {
    fn new() -> std::io::Result<Self> {
        enable_raw_mode()?;
        Ok(Self {
            stdout: stdout(),
            raw_mode_enabled: true,
        })
    }

    fn clear(&mut self) -> std::io::Result<()> {
        execute!(self.stdout, Clear(ClearType::All), MoveTo(0, 0))
    }

    fn restore(&mut self) -> std::io::Result<()> {
        if self.raw_mode_enabled {
            disable_raw_mode()?;
            self.raw_mode_enabled = false;
        }
        Ok(())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if self.raw_mode_enabled {
            let _ = disable_raw_mode();
        }
    }
}

impl Branch {
    fn from_line(line: impl AsRef<str>) -> Self {
        let status = if line.as_ref().starts_with('*') {
            "(current branch)".to_owned()
        } else {
            String::new()
        };

        Self {
            name: line.as_ref().split_at(2).1.to_owned(),
            status,
        }
    }

    fn run_cmd(&mut self, mut cmd: Command) {
        let output = cmd.output().unwrap();
        self.status = String::from_utf8_lossy(&output.stderr).to_string()
            + &String::from_utf8_lossy(&output.stdout);
        self.status = self.status.replace('\n', " ");
    }

    fn delete(&mut self, delete_arg: &str) {
        let mut cmd = Command::new("git");
        cmd.arg("branch").arg(delete_arg).arg(self.name.as_str());
        self.run_cmd(cmd);
    }

    fn checkout(&mut self) -> std::io::Result<()> {
        let mut cmd = Command::new("git");
        cmd.arg("checkout")
            .arg("--progress")
            .arg(self.name.as_str());

        // Run cmd which will make it print its output
        cmd.spawn()?.wait()?;
        Ok(())
    }
}

impl Selection {
    fn new(max: usize) -> Self {
        Self { index: 0, max }
    }

    fn move_up(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        }
    }

    fn move_down(&mut self) {
        if self.index < self.max {
            self.index += 1;
        }
    }
}
