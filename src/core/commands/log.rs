use std::fmt::Write;

use crate::core::objects::{find_object, read_object, traits::KVLM, GitObject};
use crate::core::GitRepository;

use crate::utils::argparse::{ArgumentParser, ArgumentType, Namespace};
use crate::utils::datetime::DateTime;
use crate::utils::path;

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
    let show_graph = args.get("graph").is_some();
    let oneline = args.get("oneline").is_some();
    let show_author = args.get("no-author").is_none();
    let treeish = &args["treeish"];

    _log(
        &repo,
        treeish,
        max_commits,
        show_graph,
        oneline,
        show_author,
    )
}

fn format_commit(
    commit: &GitObject,
    oneline: bool,
    show_author: bool,
) -> Result<String, String> {
    match commit {
        GitObject::Commit(commit) => {
            let kvlm = commit.kvlm();
            let mut output = String::new();

            let commit_hash = kvlm_val_to_string!(kvlm
                .get_key(b"tree")
                .expect("Commit object has a tree"));
            let short_hash = &commit_hash[..7]; // First 7 characters of hash

            if oneline {
                write!(output, "{short_hash:#} ").map_err(|e| e.to_string())?;

                // Get first line of message
                if let Some(msg) = kvlm.get_msg() {
                    let msg = String::from_utf8(msg.clone())
                        .map_err(|e| e.to_string())?;
                    if let Some(first_line) = msg.lines().next() {
                        writeln!(output, "{first_line}")
                            .map_err(|e| e.to_string())?;
                    }
                }
            } else {
                writeln!(output, "commit {commit_hash:#}")
                    .map_err(|e| e.to_string())?;

                if let Some(parent) = kvlm.get_key(b"parent") {
                    writeln!(
                        output,
                        "parent {:#}",
                        kvlm_val_to_string!(parent)
                    )
                    .map_err(|e| e.to_string())?;
                }

                if show_author {
                    if let Some(author) = kvlm.get_key(b"author") {
                        let author = kvlm_val_to_string!(author);
                        let author = extract_name(&author)
                            .expect("Author should exist for a commit");
                        writeln!(output, "Author: {author:#}")
                            .map_err(|e| e.to_string())?;
                    }
                }

                if let Some(committer) = kvlm.get_key(b"committer") {
                    let committer = kvlm_val_to_string!(committer);
                    if let Some(date) = DateTime::from_git_timestamp(&committer)
                    {
                        writeln!(output, "Date:   {:#}", date.format_git())
                            .map_err(|e| e.to_string())?;
                    } else {
                        writeln!(output, "Date:   {committer:#}")
                            .map_err(|e| e.to_string())?;
                    }
                }

                writeln!(output).map_err(|e| e.to_string())?;

                // Message is stored with empty key
                if let Some(msg) = kvlm.get_msg() {
                    let msg = kvlm_msg_to_string!(msg);
                    for line in msg.lines() {
                        writeln!(output, "    {line}")
                            .map_err(|e| e.to_string())?;
                    }
                }
                writeln!(output).map_err(|e| e.to_string())?;
            }

            Ok(output)
        }
        _ => Err("Not a commit object".to_string()),
    }
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
    treeish: &str,
    max_commits: usize,
    show_graph: bool,
    oneline: bool,
    show_author: bool,
) -> Result<String, String> {
    let mut current = find_object(repo, treeish, None, false)?;
    let mut output = String::new();
    let mut count = 0;

    while count < max_commits {
        let object = read_object(repo, &current)?;

        if show_graph {
            // Add basic ASCII graph visualization
            write!(output, "* ").map_err(|e| e.to_string())?;
        }

        output.push_str(&format_commit(&object, oneline, show_author)?);

        // Get parent commit
        if let GitObject::Commit(commit) = object {
            if let Some(parent) = commit.kvlm().get_key(b"parent") {
                current = kvlm_val_to_string!(parent);
                count += 1;
            } else {
                break;
            }
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
        .add_argument("graph", ArgumentType::Boolean)
        .optional()
        .add_help("Show ASCII graph representing the branch and merge history");
    parser
        .add_argument("no-author", ArgumentType::Boolean)
        .optional()
        .add_help("Don't show author information");
    parser
        .add_argument("treeish", ArgumentType::String)
        .required()
        .default("HEAD")
        .add_help("Start from this commit or tag");

    parser
}
