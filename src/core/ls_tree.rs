use crate::utils::argparse::{ArgumentParser, ArgumentType, Namespace};

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
pub fn ls_tree(_args: &Namespace) -> Result<String, String> {
    todo!()
}

/// Make `ls-tree` parser
#[must_use]
pub fn make_parser() -> ArgumentParser {
    let mut parser = ArgumentParser::new("Pretty-print a tree object.");
    parser
        .add_argument("recursive", ArgumentType::Boolean)
        .optional()
        .short('r')
        .add_help("Recurse into sub-trees");

    parser
        .add_argument("tree", ArgumentType::String)
        .required()
        .add_help("A tree-ish object.");

    parser
}
