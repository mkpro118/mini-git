use crate::core::commands::show_ref;
use crate::core::{objects, GitRepository};
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

/// List differences
/// This handles the subcommand
///
/// ```bash
/// mini_git rev-parse [--type TREE] [ --revision REVISION ]
/// mini_git rev-parse --show-toplevel
/// mini_git rev-parse --git-dir
/// mini_git rev-parse --all
/// ```
///
/// # Errors
///
/// If file system operations fail, or if input paths are not valid.
/// A [`String`] message describing the error is returned.
#[allow(clippy::module_name_repetitions)]
pub fn rev_parse(args: &Namespace) -> Result<String, String> {
    let cwd = path::current_dir()?;

    let repo_path = path::repo_find(&cwd)?
        .canonicalize()
        .map_err(|_| "Could not determine repository path".to_owned())?;
    let repo = GitRepository::new(&repo_path)?;

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

fn show_toplevel(repo: &GitRepository) -> Result<String, String> {
    path_to_string!(repo.worktree(), "Could not determine repository toplevel")
}

fn gitdir(repo: &GitRepository) -> Result<String, String> {
    path_to_string!(repo.gitdir(), "Could not determine repository gitdir")
}

fn all_refs(repo: &GitRepository) -> Result<String, String> {
    show_ref::list_resolved_refs(&Namespace::new(), repo, None).map(|x| {
        x.iter()
            .filter_map(|s| s.split_whitespace().next())
            .collect::<Vec<_>>()
            .join("\n")
    })
}

fn is_cwd_inside(top: &std::path::Path) -> Result<String, String> {
    let cwd = path_to_string!(path::current_dir()?, "Could not determine cwd")?;
    let top = path_to_string!(top, "Could not determine top")?;
    dbg!(&cwd);
    dbg!(&top);
    Ok(format!("{}", cwd.starts_with(&top)))
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
