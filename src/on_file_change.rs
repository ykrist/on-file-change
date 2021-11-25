use notify::{DebouncedEvent, watcher, Watcher, RecursiveMode};
use std::sync::mpsc::channel;
use std::time::Duration;
use std::process::{Command, Stdio};
use structopt::StructOpt;
use std::path::{PathBuf, Path};
use anyhow::Context;

#[derive(StructOpt)]
#[structopt(name="on-file-change", about="A convenience wrapper around inotify to run a command whenever a file changes.")]
struct Args {
    /// Filepaths to watch
    #[structopt(min_values=1)]
    filepaths: Vec<PathBuf>,

    /// Command to run.  Will be run using the shell interpreter in the $SHELL env var.  The full
    /// filepath of the file which triggered the command is supplied in the $F env var.
    #[structopt(short="c")]
    cmd: String,

    /// Exit if command errors
    #[structopt(short="e")]
    exit_on_error: bool,
}

#[derive(Debug)]
struct UserCommand {
    exit_on_error: bool,
    cmd: Command
}

impl UserCommand {
    fn new(args: &Args) -> anyhow::Result<Self> {
        let mut cmd = Command::new(std::env::var("SHELL")?);
        cmd.args(&["-c", &args.cmd])
          .stderr(Stdio::inherit())
          .stdout(Stdio::inherit());
        Ok(UserCommand { exit_on_error: args.exit_on_error, cmd })
    }

    fn run(&mut self, filepath: impl AsRef<Path>) -> anyhow::Result<()> {
        let filepath = filepath.as_ref();
        let output = self.cmd.env("F", filepath).spawn()?.wait_with_output()?;
        if !output.status.success() && self.exit_on_error {
            anyhow::bail!("command failed with exit code {}", output.status)
        }
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let args : Args = StructOpt::from_args();
    let (tx, rx) = channel();
    let mut w = watcher(tx, Duration::from_millis(250))?;
    for p in &args.filepaths {
        w.watch(p, RecursiveMode::NonRecursive)
          .with_context(|| format!("add watch for {:?}", p))?;
    }
    let mut cmd = UserCommand::new(&args)?;

    for event in rx {
        match event {
            DebouncedEvent::Write(p) | DebouncedEvent::Create(p) => {
                cmd.run(&p)?;
            }
            _ => {}
        }
    }

    Ok(())
}
