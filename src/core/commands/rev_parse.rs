use crate::core::commands::show_ref;
use crate::core::objects;
use crate::core::{
    resolve_repository_context, GitRepository, RepositoryContext,
};

use crate::utils::argparse::{ArgumentParser, ArgumentType, Namespace};
use crate::utils::path;

macro_rules! path_to_string {
    ($path:expr, $err:literal) => {
        match $path.to_str() {
            Some(s) => Ok(String::from(s)),
            None => Err($err.to_owned()),
        }
    };
}

type PathFunc = fn(&GitRepository) -> Result<String, String>;

const OPTION_MAP: &[(&str, PathFunc)] = &[
    ("all", all_refs),
    ("git-dir", gitdir),
    ("is-inside-git-dir", |repo| is_cwd_inside(repo.gitdir())),
    ("is-inside-work-tree", |repo| is_cwd_inside(repo.worktree())),
    ("show-toplevel", show_toplevel),
];

/// Parse revision (or other objects) identifiers
///
/// This handles the subcommand
///
/// ```bash
/// mini_git rev-parse [--type TREE] [ --revision REVISION ]
/// mini_git rev-parse --all
/// mini_git rev-parse --git-dir
/// mini_git rev-parse --is-inside-git-dir
/// mini_git rev-parse --is-inside-work-tree
/// mini_git rev-parse --show-toplevel
/// ```
///
/// # Errors
///
/// If file system operations fail, or if input paths are not valid.
/// A [`String`] message describing the error is returned.
pub fn rev_parse(args: &Namespace) -> Result<String, String> {
    let RepositoryContext { repo, .. } = resolve_repository_context()?;

    let mut output = String::new();

    for option in &args.order {
        let Ok(res) = OPTION_MAP
            .binary_search_by(|opt| opt.0.cmp(option))
            .map(|x| (OPTION_MAP[x].1)(&repo))
        else {
            continue;
        };
        let Ok(res) = res else {
            return res;
        };
        output.push_str(&res);
        output.push('\n');
    }

    let type_ = args.get("type").map(std::string::String::as_str);
    let revision = &args["revision"];

    if revision == "*" {
        return Ok(output);
    }

    let res = objects::find_object(&repo, revision, type_, true)?;

    output.push_str(&res);
    output.push('\n');
    Ok(output)
}

fn all_refs(repo: &GitRepository) -> Result<String, String> {
    show_ref::list_resolved_refs(&Namespace::new(), repo, None).map(|x| {
        x.iter()
            .filter_map(|s| s.split_whitespace().next())
            .collect::<Vec<_>>()
            .join("\n")
    })
}

fn gitdir(repo: &GitRepository) -> Result<String, String> {
    path_to_string!(repo.gitdir(), "Could not determine repository gitdir")
}

fn is_cwd_inside(top: &std::path::Path) -> Result<String, String> {
    Ok(format!("{}", path::current_dir()?.starts_with(top)))
}

fn show_toplevel(repo: &GitRepository) -> Result<String, String> {
    path_to_string!(repo.worktree(), "Could not determine repository toplevel")
}

/// Make `rev-parse` parser
#[must_use]
pub fn make_parser() -> ArgumentParser {
    let mut parser =
        ArgumentParser::new("Parse revision (or other objects) identifiers");
    parser
        .add_argument("all", ArgumentType::Boolean)
        .add_help("Show all refs found in `refs/`");

    parser
        .add_argument("is-inside-git-dir", ArgumentType::Boolean)
        .add_help("When the current working directory is below the repository directory print \"true\", otherwise \"false\"");

    parser
        .add_argument("is-inside-work-tree", ArgumentType::Boolean)
        .add_help("When the current working directory is inside the work tree of the repository print \"true\", otherwise \"false\"");

    parser
        .add_argument("type", ArgumentType::String)
        .choices(&["blob", "commit", "tag", "tree"])
        .add_help("Specify the type of object");

    parser
        .add_argument("show-toplevel", ArgumentType::Boolean)
        .add_help(
        "Show the absolute path of the top-level directory of the working tree",
    );

    parser
        .add_argument("git-dir", ArgumentType::Boolean)
        .add_help("Show the absolute path to the .git directory.");

    parser
        .add_argument("revision", ArgumentType::String)
        .required()
        .default("*")
        .add_help("The revision to parse");

    parser
}
