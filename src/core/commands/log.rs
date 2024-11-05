use std::fmt::Write;

use crate::core::objects::{
    commit::Commit, find_object, read_object, traits::KVLM, GitObject,
};
use crate::core::GitRepository;

use crate::utils::argparse::{ArgumentParser, ArgumentType, Namespace};
use crate::utils::datetime::DateTime;
use crate::utils::path;

const RESET: &str = "\x1b[0m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";

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

macro_rules! kvlm_val_to_string {
    ($kvlm_val:expr) => {
        String::from_utf8($kvlm_val[0].to_vec()).map_err(|e| e.to_string())?
    };
}

macro_rules! kvlm_msg_to_string {
    ($kvlm_msg:expr) => {
        String::from_utf8($kvlm_msg.to_vec()).map_err(|e| e.to_string())?
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
        return Err("Could not determine current working directory".to_owned());
    };
    let path = path::repo_find(cwd)?;
    let repo = GitRepository::new(&path)?;

    let max_commits = option_to_int!(args.get("max"), usize::MAX, "max");
    let oneline = args.get("oneline").is_some();
    let show_author = args.get("no-author").is_none();
    let revision = &args["revision"];

    _log(&repo, revision, max_commits, oneline, show_author)
}

fn format_commit(
    hash: &str,
    commit: &Commit,
    oneline: bool,
    show_author: bool,
) -> Result<String, String> {
    let kvlm = commit.kvlm();
    let mut output = String::new();
    let short_hash = &hash[..7];

    if oneline {
        write!(output, "{YELLOW}{short_hash}{RESET} ")
            .map_err(|e| e.to_string())?;

        let Some(msg) = kvlm.get_msg() else {
            return Ok(output);
        };
        let msg = kvlm_msg_to_string!(msg);

        let Some(first_line) = msg.lines().next() else {
            return Ok(output);
        };
        writeln!(output, "{first_line}").map_err(|e| e.to_string())?;

        return Ok(output);
    }

    writeln!(output, "commit {YELLOW}{hash}{RESET}")
        .map_err(|e| e.to_string())?;

    if show_author {
        if let Some(author) = kvlm.get_key(b"author") {
            let author = kvlm_val_to_string!(author);
            let name = extract_name(&author)
                .expect("Author should exist for a commit");
            writeln!(output, "Author: {CYAN}{name}{RESET}")
                .map_err(|e| e.to_string())?;
        }
    }

    if let Some(committer) = kvlm.get_key(b"committer") {
        let committer = kvlm_val_to_string!(committer);
        if let Some(date) = DateTime::from_git_timestamp(&committer) {
            writeln!(output, "Date:   {}", date.format_git())
                .map_err(|e| e.to_string())?;
        } else {
            writeln!(output, "Date:   {committer}")
                .map_err(|e| e.to_string())?;
        }
    }

    writeln!(output).map_err(|e| e.to_string())?;

    if let Some(msg) = kvlm.get_msg() {
        let msg = kvlm_msg_to_string!(msg);
        for line in msg.lines() {
            writeln!(output, "    {line}").map_err(|e| e.to_string())?;
        }
    }
    writeln!(output).map_err(|e| e.to_string())?;

    Ok(output)
}

fn extract_name(author_string: &str) -> Option<&str> {
    // Format is typically "Name <email@example.com> timestamp timezone"
    let end = author_string.find('<').unwrap_or(author_string.len());
    let name = author_string[..end].trim();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn _log(
    repo: &GitRepository,
    revision: &str,
    max_commits: usize,
    oneline: bool,
    show_author: bool,
) -> Result<String, String> {
    let mut current = find_object(repo, revision, None, false)?;
    let mut output = String::new();
    let mut count = 0;

    while count < max_commits {
        let object = read_object(repo, &current)?;

        let GitObject::Commit(commit) = &object else {
            break;
        };

        let mut parents = Vec::new();

        // Collect all parents
        if let Some(parent_commits) = commit.kvlm().get_key(b"parent") {
            for parent in parent_commits {
                parents.push(kvlm_msg_to_string!(parent));
            }
        }

        output.push_str(&format_commit(
            &current,
            commit,
            oneline,
            show_author,
        )?);

        if let Some(parent) = parents.first() {
            current.clone_from(parent);
            count += 1;
        } else {
            break;
        }
    }

    Ok(output)
}

/// Make `log` parser
#[must_use]
pub fn make_parser() -> ArgumentParser {
    let mut parser = ArgumentParser::new("Shows the history of commit logs.");
    parser
        .add_argument("max", ArgumentType::Integer)
        .short('n')
        .optional()
        .add_help("Limit the number of commits to output");
    parser
        .add_argument("oneline", ArgumentType::Boolean)
        .optional()
        .add_help("Show each commit on a single line");
    parser
        .add_argument("no-author", ArgumentType::Boolean)
        .optional()
        .add_help("Don't show author information");
    parser
        .add_argument("revision", ArgumentType::String)
        .required()
        .default("HEAD")
        .add_help("Start from this commit or tag");

    parser
}
