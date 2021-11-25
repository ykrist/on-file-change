use notify::{DebouncedEvent, watcher, Watcher, RecursiveMode};
use std::sync::mpsc::channel;
use std::time::Duration;
use structopt::StructOpt;
use std::path::{PathBuf};
use anyhow::Context;
use path_absolutize::Absolutize;

#[derive(StructOpt)]
#[structopt(name="wait-for-file", about="Block until a file exists.")]
/// A convenience wrapper around inotify to wait until a file exists
struct Args {
  /// Filepath to watch
  filepath: PathBuf,

  #[structopt(short="i")]
  /// Wait for an explicit CREATE event, ignoring the file if it existed before `wait-for-file` is run.
  ignore_existing: bool
}

fn main() -> anyhow::Result<()> {
  let args: Args = StructOpt::from_args();
  let (tx, rx) = channel();

  let target = args.filepath.absolutize()?;
  let directory = target.ancestors().find(|d| d.is_dir())
    .ok_or_else(|| anyhow::anyhow!("Unable to find existing parent directory for {:?}", target))?;


  // Start the watcher first
  let mut w = watcher(tx, Duration::from_millis(250))?;
  w.watch(directory, RecursiveMode::Recursive)
    .context("failed to set up watch ")?;

  // Now check if the file already exists
  if !args.ignore_existing && args.filepath.exists() {
    return Ok(());
  }

  for event in rx {
    match event {
      DebouncedEvent::Create(path) => {
        if path == target { break }
      },
      _ => {}
    }
  }

  Ok(())
}
