use crate::utils::argparse::{ArgumentParser, ArgumentType, Namespace};
use crate::utils::path;

use crate::core::objects;
use crate::core::objects::GitObject;
use crate::core::GitRepository;

/// Pretty-print a tree object.
/// This handles the subcommand
///
/// ```bash
/// mini_git ls-tree [--recursive] tree
/// ```
///
/// # Errors
///
/// If file system operations fail, or if input paths are not valid.
/// A [`String`] message describing the error is returned.
#[allow(clippy::module_name_repetitions)]
pub fn ls_tree(args: &Namespace) -> Result<String, String> {
    let Ok(cwd) = std::env::current_dir() else {
        return Err("Could not determined current working directory".to_owned());
    };

    let repo = path::repo_find(cwd)?;
    let repo = GitRepository::new(&repo)?;
    let recursive = args.get("recursive").is_some();
    let tree_ref = &args["tree"];
    let show_trees = args.get("show-trees").is_some();
    let only_trees = args.get("only-trees").is_some();
    let mut res = String::new();
    tree(
        &mut res, &repo, tree_ref, "", recursive, show_trees, only_trees,
    )?;
    Ok(res)
}
#[allow(unused_variables)]
fn tree(
    acc: &mut String,
    repo: &GitRepository,
    tree_ref: &str,
    prefix: &str,
    recursive: bool,
    show_trees: bool,
    only_trees: bool,
) -> Result<(), String> {
    let sha = objects::find_object(repo, tree_ref, Some("tree"), false);
    let GitObject::Tree(obj) = objects::read_object(repo, &sha)? else {
        unreachable!();
    };

    for leaf in obj.leaves() {
        let mode = leaf.mode_as_string();
        let Some(obj_type) = leaf.obj_type() else {
            return Err(format!("Unknown object mode {mode}"));
        };

        let sha = leaf.sha();
        let path = join_path(prefix, &leaf.path_as_string());

        if recursive && obj_type == "tree" {
            if show_trees {
                acc.push_str(&repr_leaf(&mode, sha, obj_type, &path));
            }
            tree(acc, repo, sha, &path, recursive, show_trees, only_trees)?;
        } else {
            // Base case
            if only_trees && obj_type != "tree" {
                continue;
            }

            acc.push_str(&repr_leaf(&mode, sha, obj_type, &path));
        };
    }
    Ok(())
}

#[inline]
fn join_path(prefix: &str, next: &str) -> String {
    let path = std::path::Path::new(prefix).join(next);
    path.as_os_str().to_str().expect("utf-8 path").to_owned()
}

#[inline]
fn repr_leaf(mode: &str, obj_type: &str, sha: &str, path: &str) -> String {
    format!("{mode} {obj_type} {sha}\t{path}\n")
}

/// Make `ls-tree` parser
#[must_use]
pub fn make_parser() -> ArgumentParser {
    let mut parser = ArgumentParser::new("Pretty-print a tree object.");

    parser
        .add_argument("only-trees", ArgumentType::Boolean)
        .optional()
        .short('d')
        .add_help("Only show trees");

    parser
        .add_argument("recursive", ArgumentType::Boolean)
        .optional()
        .short('r')
        .add_help("Recurse into sub-trees");

    parser
        .add_argument("show-trees", ArgumentType::Boolean)
        .optional()
        .short('t')
        .add_help("Show trees when recursing");

    parser
        .add_argument("tree", ArgumentType::String)
        .required()
        .add_help("A tree-ish object.");

    parser
}
