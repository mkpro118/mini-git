use std::collections::VecDeque;
use std::path::PathBuf;

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
) -> Result<OrderedMap<String, String>, String> {
    let Some(initial_path) = path::repo_dir(repo.gitdir(), &[REF_DIR], false)?
    else {
        return Err(
            "Fatal error: refs directory not found. This indicates the \
            repository is likely corrupted"
                .to_owned(),
        );
    };

    let n_comps = repo.gitdir().components().count();
    let initial_entries = sorted_dir(&initial_path)?;

    let mut stack = Vec::<Vec<PathBuf>>::new();
    stack.push(initial_entries);

    let mut res = OrderedMap::new();
    while let Some(entries) = stack.pop() {
        for (i, entry) in entries.iter().enumerate() {
            if entry.is_dir() {
                let remaining = entries[(i + 1)..].to_vec();

                stack.push(remaining); // this will pop second
                stack.push(sorted_dir(&entry)?); // this will pop first

                break;
            }

            // is file
            let r#ref = entry
                .components() // make path relative
                .skip(n_comps)
                .map(std::path::Component::as_os_str)
                .map(std::ffi::OsStr::to_string_lossy)
                .map(|x| x.into())
                .collect::<Vec<String>>();

            // For operations, we use OS specific path separator
            let rec_ref = r#ref.join(std::path::MAIN_SEPARATOR_STR);
            let resolved =
                resolve_ref(repo, &rec_ref)?.unwrap_or("".to_owned());

            // For display we use the posix path separator '/'.
            let key_ref = r#ref.join("/");
            res.insert(key_ref, resolved);
        }
    }
    Ok(res)
}

fn sorted_dir(
    path: &std::path::Path,
) -> Result<Vec<std::path::PathBuf>, String> {
    let Ok(ls) = std::fs::read_dir(path) else {
        return Err(format!("failed to read dir {:?}", path.as_os_str()));
    };

    let mut ls = ls
        .flatten()
        .map(|x| x.path())
        .collect::<Vec<std::path::PathBuf>>();
    ls.sort_unstable();
    Ok(ls)
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
