use crate::core::GitRepository;
use std::path::Path;

const DEFAULT_PATH: &str = ".";

/// Initializes a new git repository
/// This handles the subcommand
///
/// ```bash
/// mini_git init [path]
/// ```
///
/// # Errors
///
/// If file system operations fail, or if input paths are not valid.
/// A [`String`] message describing the error is returned.
#[allow(clippy::module_name_repetitions)]
pub fn cmd_init(
    args: impl IntoIterator<Item = impl AsRef<str>>,
) -> Result<String, String> {
    let path = args.into_iter().next();
    let path = match path {
        Some(ref arg) => arg.as_ref(),
        None => DEFAULT_PATH,
    };

    if path == "-h" || path == "--help" {
        return Ok(help().to_owned());
    }

    let repo = GitRepository::create(Path::new(&path))?;
    Ok(format!(
        "initialized empty repository in {:?}\n",
        repo.worktree().as_os_str()
    ))
}

/// Display a help message for the init command
const fn help() -> &'static str {
    "mini_git init [path]
  Initializes a new repository

Options:
  -h, --help: Display this help message

Arguments:
  path: The folder to initialize the new repository in,
        defaults to the current working directory.
        If provided, must be an existing directory."
}
