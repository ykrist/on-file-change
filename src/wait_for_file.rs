use notify::{DebouncedEvent, watcher, Watcher, RecursiveMode};
use std::sync::mpsc::channel;
use std::time::Duration;
use structopt::StructOpt;
use std::path::{PathBuf, Path};
use anyhow::Context;
use path_absolutize::Absolutize;

#[derive(StructOpt)]
#[structopt(name="wait-for-file", about="Block until a file exists.")]
/// A convenience wrapper around inotify to wait until a file exists.  You need permission to start an
/// inotify watch on the closest existing parent directory of the target file.
struct Args {
  /// Filepath to watch
  filepath: PathBuf,

  #[structopt(short="i")]
  /// Wait for an explicit CREATE event, ignoring the file if it existed before `wait-for-file` is run.
  ignore_existing: bool,

  #[structopt(short="p", value_name="N")]
  /// Use polling instead of inotify, with a poll interval of N milliseconds
  poll: Option<u64>
}

fn poll(path: impl AsRef<Path>, millis: u64) -> anyhow::Result<()> {
  let path = path.as_ref();
  let duration = std::time::Duration::from_millis(millis);
  while !path.exists() {
    std::thread::yield_now();
    std::thread::sleep(duration);
  }
  Ok(())
}

fn main() -> anyhow::Result<()> {
  let args: Args = StructOpt::from_args();
  let (tx, rx) = channel();

  if let Some(millis) = args.poll {
    return poll(&args.filepath, millis)
  }

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
