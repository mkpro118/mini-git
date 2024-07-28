use std::path::PathBuf;

use crate::core::objects::read_object;
use crate::core::objects::traits::KVLM;
use crate::core::objects::GitObject;
use crate::utils::argparse::{ArgumentParser, ArgumentType, Namespace};
use crate::utils::collections::ordered_map::OrderedMap;
use crate::utils::path::{self, repo_find};

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
pub fn show_ref(args: &Namespace) -> Result<String, String> {
    let repo = repo_find(".")?;
    let repo = GitRepository::new(&repo)?;

    let filter = args.get("pattern").map_or(None, |x| {
        if x == "*" {
            None
        } else {
            Some(x.as_str())
        }
    });

    let check_exists = args.get("exists").is_some();

    if check_exists && filter.is_none() {
        return Err("--exists requires a reference".to_owned());
    }

    if check_exists {
        let result = list_resolved_refs(args, &repo, None)?;
        let filter = filter.expect("Should exist, already checked");
        if result.into_iter().any(|x| x.ends_with(filter)) {
            Ok("".to_owned())
        } else {
            Err("error: reference not found".to_owned())
        }
    } else {
        let result = list_resolved_refs(args, &repo, filter)?;
        Ok(result.join("\n"))
    }
}

fn list_resolved_refs(
    args: &Namespace,
    repo: &GitRepository,
    filter: Option<&str>,
) -> Result<Vec<String>, String> {
    let dereference = args.get("dereference").is_some();

    let mut result = vec![];
    if args.get("head").is_some() {
        if let Some(sha) = resolve_ref(&repo, "HEAD")? {
            result.insert(0, format!("{sha} HEAD"));
        }
    }

    let refs = list_refs(&repo, filter)?;

    let pred = make_predicate(args);
    let refs_iter = refs.into_iter().filter(move |(x, _)| pred(&x));

    let relevant = refs_iter.map(|(name, resolved)| {
        let mut res = format!("{resolved} {name}");
        if !(dereference && name.starts_with("refs/tags")) {
            return res;
        }

        let tag = match read_object(&repo, &resolved) {
            Ok(GitObject::Tag(tag)) => tag,
            _ => return res,
        };

        let tag_kvlm = tag.kvlm();
        if let Some(object_sha) = tag_kvlm.get_key(b"object") {
            if object_sha.len() != 1 {
                return res;
            }
            let sha = object_sha[0]
                .iter()
                .map(|x| char::from(*x))
                .collect::<String>();

            res.push('\n');
            res.push_str(format!("{sha} {name}^{{}}").as_str());
        }

        res
    });

    result.extend(relevant);

    Ok(result)
}

fn make_predicate(args: &Namespace) -> Box<dyn Fn(&str) -> bool + '_> {
    match (args.get("heads"), args.get("tags")) {
        (None, None) => Box::new(|_: &str| true),
        (None, Some(_)) => Box::new(move |x: &str| x.starts_with("refs/tags")),
        (Some(_), None) => Box::new(move |x: &str| x.starts_with("refs/heads")),
        (Some(_), Some(_)) => Box::new(move |x: &str| {
            x.starts_with("refs/heads") || x.starts_with("refs/tags")
        }),
    }
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
    filter: Option<&str>,
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

            // If looking for a specific ref
            match (filter, r#ref.last()) {
                (Some(x), Some(y)) if x == y => {}
                (None, _) => {}
                _ => continue,
            };

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
        .add_argument("exists", ArgumentType::Boolean)
        .optional()
        .add_help("Check for reference existence without resolving");

    parser
        .add_argument("pattern", ArgumentType::String)
        .required()
        .default("*") // * is not a valid branch name
        .add_help("Pattern to filter");

    parser
}
