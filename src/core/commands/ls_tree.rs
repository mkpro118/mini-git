use crate::core::objects::traits::KVLM;
use crate::core::objects::{self, GitObject};
use crate::core::{
    resolve_repository_context, GitRepository, RepositoryContext,
};

use crate::utils::argparse::{ArgumentParser, ArgumentType, Namespace};
use crate::utils::collections::kvlm;

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
pub fn ls_tree(args: &Namespace) -> Result<String, String> {
    let RepositoryContext { repo, .. } = resolve_repository_context()?;
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

fn tree(
    acc: &mut String,
    repo: &GitRepository,
    tree_ref: &str,
    prefix: &str,
    recursive: bool,
    show_trees: bool,
    only_trees: bool,
) -> Result<(), String> {
    let sha = objects::find_object(repo, tree_ref, None, false)?;
    let obj = objects::read_object(repo, &sha)?;

    let mut f = |obj_type: &str, kvlm: &kvlm::KVLM| {
        let Some(obj_tree) = kvlm.get_key(b"tree") else {
            return Err(format!(
                "{obj_type} {tree_ref} has no associated tree"
            ));
        };
        for subtree in obj_tree {
            let subtree =
                subtree.iter().map(|x| char::from(*x)).collect::<String>();
            tree(
                acc, repo, &subtree, prefix, recursive, show_trees, only_trees,
            )?;
        }
        Ok(())
    };

    let obj = match obj {
        GitObject::Commit(commit) => return f("commit", commit.kvlm()),
        GitObject::Blob(_) => {
            return Err(format!("{tree_ref} is not a tree_object"))
        }
        GitObject::Tag(tag) => return f("tag", tag.kvlm()),
        GitObject::Tree(obj) => obj,
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
                acc.push_str(&repr_leaf(&mode, obj_type, sha, &path));
            }
            tree(acc, repo, sha, &path, recursive, show_trees, only_trees)?;
        } else {
            // Base case
            if only_trees && obj_type != "tree" {
                continue;
            }

            acc.push_str(&repr_leaf(&mode, obj_type, sha, &path));
        }
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
