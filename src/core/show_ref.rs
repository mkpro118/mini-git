use crate::utils::argparse::{ArgumentParser, ArgumentType, Namespace};
use crate::utils::collections::ordered_map::OrderedMap;
use crate::utils::path;

use crate::core::GitRepository;

const REF_DIR: &str = "refs";

/// List references
/// This handles the subcommand
///
/// ```bash
/// mini_git show-ref [--head] [--tags] [--heads] [--dereference] ref
/// ```
///
/// # Errors
///
/// If file system operations fail, or if input paths are not valid.
/// A [`String`] message describing the error is returned.
#[allow(clippy::module_name_repetitions)]
pub fn show_ref(_args: &Namespace) -> Result<String, String> {
    todo!()
}

fn resolve_ref(
    repo: &GitRepository,
    r#ref: &str,
) -> Result<Option<String>, String> {
    let Some(path) = path::repo_file(repo.gitdir(), &[r#ref], false)? else {
        unreachable!();
    };

    if !path.is_file() {
        return Ok(None);
    }

    let Ok(contents) = std::fs::read_to_string(&path) else {
        return Err(format!("Failed to read file at {:?}", path.as_os_str()));
    };

    let contents = contents.trim();
    if let Some(stripped) = contents.strip_prefix("ref: ") {
        resolve_ref(repo, stripped)
    } else {
        Ok(Some(contents.to_owned()))
    }
}

fn list_refs(
    repo: &GitRepository,
    path: Option<&std::path::Path>,
) -> Result<Vec<String>, String> {
    let Some(default_dir) = path::repo_dir(repo.gitdir(), &[REF_DIR], false)?
    else {
        return Err(
            "Fatal error: refs directory not found. This indicates the \
            repository is likely corrupted"
                .to_owned(),
        );
    };

    let path = path.unwrap_or(&default_dir);

    let Ok(ls) = std::fs::read_dir(path) else {
        return Err(format!("failed to read dir {:?}", path.as_os_str()));
    };

    let mut ls = ls
        .flatten()
        .filter_map(|x| x.path().as_os_str().to_str().map(String::from))
        .collect::<Vec<String>>();
    ls.sort();

    for entry in ls {}

    todo!()
}

/// Make `show-ref` parser
#[must_use]
pub fn make_parser() -> ArgumentParser {
    let mut parser = ArgumentParser::new("List references.");

    parser
        .add_argument("tags", ArgumentType::Boolean)
        .optional()
        .add_help("Only show tags");

    parser
        .add_argument("heads", ArgumentType::Boolean)
        .optional()
        .add_help("Only show heads");

    parser
        .add_argument("head", ArgumentType::Boolean)
        .optional()
        .add_help("Show the HEAD reference, even if it would be filtered out");

    parser
        .add_argument("dereference", ArgumentType::Boolean)
        .optional()
        .short('d')
        .add_help("Dereference tags into object IDs");

    parser
        .add_argument("ref", ArgumentType::String)
        .required()
        .add_help("Ref pattern to show");

    parser
}
