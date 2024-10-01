use crate::utils::argparse::{ArgumentParser, ArgumentType, Namespace};
use crate::utils::path;

use crate::core::objects::{find_object, read_object};
use crate::core::GitRepository;

macro_rules! option_to_int {
    ($value:expr, $default:expr, $err_msg:literal) => {
        match $value {
            None => $default,
            Some(count) => {
                let Ok(x) = count.parse::<usize>() else {
                    return Err(format!("{} is not a number", $err_msg));
                };
                x
            }
        }
    };
}

/// Shows the history of commit logs
/// This handles the subcommand
///
/// ```bash
/// mini_git log [options] [ --count COUNT ] [ --treeish TREEISH ]
/// ```
///
/// # Errors
///
/// If file system operations fail, or if input paths are not valid.
/// A [`String`] message describing the error is returned.
#[allow(clippy::module_name_repetitions)]
pub fn log(args: &Namespace) -> Result<String, String> {
    let Ok(cwd) = std::env::current_dir() else {
        return Err("Could not determined current working directory".to_owned());
    };
    let path = path::repo_find(cwd)?;
    let repo = GitRepository::new(&path)?;

    let max_count =
        option_to_int!(args.get("max-count"), usize::MAX, "Max count");
    let skip = option_to_int!(args.get("skip"), usize::MAX, "skip");
    let count = option_to_int!(args.get("count"), usize::MAX, "count");
    let oneline = args.get("oneline").is_some();
    let treeish = &args["treeish"];
    _log(&repo, &treeish);
    todo!()
}

fn _log(repo: &GitRepository, treeish: &str) -> Result<String, String> {
    let treeish = find_object(repo, treeish, None, false)?;
    let stuff = read_object(repo, &treeish)?;
    println!("{:?}", stuff);
    Ok("".to_owned())
}

/// Make `log` parser
#[must_use]
pub fn make_parser() -> ArgumentParser {
    let mut parser = ArgumentParser::new("Shows the history of commit logs.");
    parser
        .add_argument("max-count", ArgumentType::Integer)
        .short('n')
        .optional()
        .add_help("Display at most N commits");
    parser
        .add_argument("oneline", ArgumentType::Boolean)
        .optional()
        .add_help(
            "Display commits on one line in the format <hash> <title-line>",
        );
    parser
        .add_argument("skip", ArgumentType::Integer)
        .optional()
        .default("0")
        .add_help("Skip the first N commits");
    parser
        .add_argument("count", ArgumentType::Integer)
        .required()
        .default("5")
        .add_help("Display at least COUNT commits");
    parser
        .add_argument("treeish", ArgumentType::String)
        .required()
        .default("HEAD")
        .add_help("Start from this commit or tag");

    parser
}
