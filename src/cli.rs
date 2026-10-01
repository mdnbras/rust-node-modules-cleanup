use std::{ffi::OsString, path::PathBuf};

pub const HELP: &str = "rust-node-modules-cleanup <path> [options]

Find node_modules directories, show their sizes, and confirm permanent deletion.

Options:
  --dry, --dry-run          Preview only; never delete or prompt
  --skip-confirmation, -y   Delete all listed directories without prompting
  --jobs <1..64>            Concurrent size checks/deletions (default: 5)
  --help, -h, --h           Show help
  --version, -V, --v        Show version
  --                       Treat remaining argument as the path

Hidden directories and symbolic links are not traversed.
Without confirmation (including EOF), nothing is deleted.";

#[derive(Debug, PartialEq, Eq)]
pub struct Options {
    pub path: PathBuf,
    pub dry: bool,
    pub yes: bool,
    pub jobs: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Help,
    Version,
    Run(Options),
}

pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut args = args.into_iter();
    let (mut path, mut dry, mut yes, mut literal) = (None, false, false, false);
    let mut jobs = 5;
    while let Some(arg) = args.next() {
        if !literal {
            match arg.to_str() {
                Some("--help" | "-h" | "--h") => return Ok(Command::Help),
                Some("--version" | "-V" | "--v") => return Ok(Command::Version),
                Some("--dry" | "--dry-run") => {
                    dry = true;
                    continue;
                }
                Some("--skip-confirmation" | "-y") => {
                    yes = true;
                    continue;
                }
                Some("--") => {
                    literal = true;
                    continue;
                }
                Some("--jobs") => {
                    jobs = args
                        .next()
                        .and_then(|s| s.to_str().and_then(|s| s.parse().ok()))
                        .filter(|n| (1..=64).contains(n))
                        .ok_or("--jobs requires an integer from 1 to 64")?;
                    continue;
                }
                Some(s) if s.starts_with('-') => return Err(format!("Unknown option: {s}")),
                _ => {}
            }
        }
        if path.replace(PathBuf::from(arg)).is_some() {
            return Err("Provide exactly one search path".into());
        }
    }
    Ok(Command::Run(Options {
        path: path.ok_or("Missing search path; use --help for usage")?,
        dry,
        yes,
        jobs,
    }))
}

pub fn confirmed(answer: &str) -> bool {
    matches!(
        answer.trim().to_lowercase().as_str(),
        "y" | "yes" | "s" | "sim" | "kör bara kör!"
    )
}
