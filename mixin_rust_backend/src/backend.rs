use std::{
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

pub enum BuildMessage {
    Line(String),
    Finished(bool),
}

pub struct BuildHandle {
    pub receiver: Receiver<BuildMessage>,
}

pub fn start_cargo_check(project: &Path) -> BuildHandle {
    let (sender, receiver) = mpsc::channel();
    let project = project.to_path_buf();

    thread::spawn(move || {
        let success = run_cargo_command(&project, ["check", "--release"], &sender);
        let _ = sender.send(BuildMessage::Finished(success));
    });

    BuildHandle { receiver }
}

pub fn start_cargo_build(project: &Path) -> BuildHandle {
    let (sender, receiver) = mpsc::channel();
    let project = project.to_path_buf();

    thread::spawn(move || {
        let success = run_cargo_command(&project, ["build", "--release"], &sender);
        let _ = sender.send(BuildMessage::Finished(success));
    });

    BuildHandle { receiver }
}

pub fn start_cargo_run(project: &Path) -> Result<(), String> {
    Command::new("cargo")
        .args(["run", "--release"])
        .current_dir(project)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Failed to start cargo run: {error}"))
}

pub fn find_manifest(path: &Path) -> Option<PathBuf> {
    if path.is_file() && path.file_name().is_some_and(|name| name == "Cargo.toml") {
        return Some(path.to_path_buf());
    }

    let mut current = path.to_path_buf();
    if current.is_file() {
        current.pop();
    }

    let manifest = current.join("mixin_rust_gui/Cargo.toml");
    manifest.exists().then_some(manifest)
}

fn run_cargo_command<const N: usize>(
    project: &Path,
    args: [&str; N],
    sender: &Sender<BuildMessage>,
) -> bool {
    let mut child = match Command::new("cargo")
        .args(args)
        .current_dir(project)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            let _ = sender.send(BuildMessage::Line(format!("Failed to start cargo: {error}")));
            return false;
        }
    };

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let mut readers = Vec::new();

    if let Some(stdout) = stdout {
        readers.push(thread::spawn({
            let sender = sender.clone();
            move || forward_lines(stdout, sender)
        }));
    }

    if let Some(stderr) = stderr {
        readers.push(thread::spawn({
            let sender = sender.clone();
            move || forward_lines(stderr, sender)
        }));
    }

    let success = child.wait().map(|status| status.success()).unwrap_or(false);

    for reader in readers {
        let _ = reader.join();
    }

    success
}

fn forward_lines<R: std::io::Read + Send + 'static>(reader: R, sender: Sender<BuildMessage>) {
    let reader = BufReader::new(reader);
    for line in reader.lines() {
        match line {
            Ok(line) => {
                let _ = sender.send(BuildMessage::Line(line));
            }
            Err(error) => {
                let _ = sender.send(BuildMessage::Line(format!("Output read error: {error}")));
                break;
            }
        }
    }
}
