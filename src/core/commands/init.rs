use crate::core::GitRepository;
use crate::utils::argparse::{ArgumentParser, ArgumentType, Namespace};
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
pub fn init(args: &Namespace) -> Result<String, String> {
    let path = &args["path"];

    let Ok(cwd) = std::env::current_dir() else {
        return Err("failed to get cwd".to_owned());
    };

    let path = if path == DEFAULT_PATH {
        cwd
    } else {
        let temp = Path::new(&path).to_owned();
        if temp.is_absolute() {
            temp
        } else {
            cwd.join(temp)
        }
    };

    let repo = GitRepository::create(&path)?;
    Ok(format!(
        "initialized empty repository in {:?}\n",
        repo.worktree().as_os_str()
    ))
}

/// Make `init` parser
#[must_use]
pub fn make_parser() -> ArgumentParser {
    let mut parser = ArgumentParser::new("Initializes a new repository");
    parser
        .add_argument("path", ArgumentType::String)
        .required()
        .default(".")
        .add_help("The folder to initialize the new repository in");

    parser
}
