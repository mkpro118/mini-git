use crate::{kvlm_msg_to_string, kvlm_val_to_string, parse_arg_as_int};
use std::fmt::Write;

use crate::core::objects::{commit::Commit, traits::KVLM};
use crate::core::objects::{find_object, read_object, GitObject};
use crate::core::{
    resolve_repository_context, GitRepository, RepositoryContext,
};

use crate::utils::argparse::{ArgumentParser, ArgumentType, Namespace};
use crate::utils::datetime::DateTime;

const RESET: &str = "\x1b[0m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";

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
pub fn log(args: &Namespace) -> Result<String, String> {
    let RepositoryContext { repo, .. } = resolve_repository_context()?;

    let max_commits = parse_arg_as_int!(args.get("max"), usize::MAX, "max");
    let oneline = args.get("oneline").is_some();
    let show_author = args.get("no-author").is_none();
    let revision = &args["revision"];

    #[expect(clippy::used_underscore_items)]
    _log(&repo, revision, max_commits, oneline, show_author)
}

fn _log(
    repo: &GitRepository,
    revision: &str,
    max_commits: usize,
    oneline: bool,
    show_author: bool,
) -> Result<String, String> {
    let mut current = find_object(repo, revision, None, true)?;
    let mut output = String::new();
    let mut count = 0;

    while count < max_commits {
        let object = read_object(repo, &current)?;

        let commit = match &object {
            GitObject::Blob(_) => {
                return Err(format!(
                    "Cannot show history for a blob (sha {current})"
                ))
            }
            GitObject::Tree(_) => {
                return Err(format!(
                    "Cannot show history for a tree (sha {current})"
                ))
            }
            GitObject::Commit(commit) => commit,
            GitObject::Tag(tag) => {
                let Some(object) = tag.kvlm().get_key(b"object") else {
                    return Err(format!(
                        "Bad tag {current} does not have an object"
                    ));
                };
                current = kvlm_val_to_string!(object);
                continue;
            }
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
