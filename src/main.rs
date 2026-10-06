use std::{
    error::Error,
    io::{self, stdout, Write},
    process::Command,
};

use crossterm::{
    cursor::MoveTo,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{self, Clear, ClearType},
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
    let raw_mode = RawModeGuard::new()?;
    let mut stdout = stdout().lock();
    let (mut branches, max_branch_name_len) = local_git_branches();
    let mut selection = Selection::new(branches.len() - 1);

    // Clear the screen only once to avoid flicker
    execute!(&mut stdout, Clear(ClearType::All))?;
    loop {
        execute!(&mut stdout, MoveTo(0, 0))?;

        print_branches(&mut stdout, &branches, selection.index, max_branch_name_len)?;

        let selected_branch = branches.get_mut(selection.index).unwrap();

        stdout.flush().unwrap();

        let action = match event::read()? {
            Event::Key(key) => key_to_action(key),
            _ => Action::None,
        };
        match action {
            Action::MoveUp => selection.move_up(),
            Action::MoveDown => selection.move_down(),
            Action::Delete => selected_branch.delete("-d"),
            Action::ForceDelete => selected_branch.delete("-D"),
            Action::Checkout => {
                execute!(&mut stdout, Clear(ClearType::All), MoveTo(0, 0))?;
                stdout.flush()?;
                drop(stdout);
                drop(raw_mode);
                selected_branch.checkout()?;
                std::io::stdout().lock().flush()?;
                break; // Auto-quit
            }
            Action::Quit => break,
            Action::None => {}
        }
    }

    Ok(())
}

fn print_branches(
    stdout: &mut dyn std::io::Write,
    branches: &[Branch],
    selected: usize,
    max_branch_name_len: usize,
) -> std::io::Result<()> {
    writeln!(stdout, "BRANCHES")?;
    writeln!(stdout)?;
    for (index, branch) in branches.iter().enumerate() {
        write!(
            stdout,
            "{}{}{}{MARGIN}{}",
            if selected == index { "-> " } else { "   " },
            branch.name,
            " ".repeat(max_branch_name_len - branch.name.len()),
            branch.status
        )?;
        execute!(stdout, Clear(ClearType::UntilNewLine))?;
        writeln!(stdout)?;
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
        (KeyCode::Down | KeyCode::Right, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
            Action::MoveDown
        }
        (KeyCode::Up | KeyCode::Left, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
            Action::MoveUp
        }
        (KeyCode::Esc, _)
        | (KeyCode::Char('q'), _)
        | (KeyCode::Char('c'), KeyModifiers::CONTROL) => Action::Quit,
        (KeyCode::Delete, _) | (KeyCode::Char('d'), _) => Action::Delete,
        (KeyCode::Char('D'), _) => Action::ForceDelete,
        (KeyCode::Char('c'), _) | (KeyCode::Enter, _) => Action::Checkout,
        (KeyCode::Char('j'), _) => Action::MoveDown,
        (KeyCode::Char('k'), _) => Action::MoveUp,
        _ => Action::None,
    }
}

struct RawModeGuard;

impl RawModeGuard {
    fn new() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}

impl Branch {
    fn from_line(line: impl AsRef<str>) -> Self {
        let status = line
            .as_ref()
            .starts_with('*')
            .then(|| "(current branch)".to_owned())
            .unwrap_or_default();

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
