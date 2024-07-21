use crate::utils::argparse::{ArgumentParser, ArgumentType, Namespace};

/// Provide content of repository objects
/// This handles the subcommand
///
/// ```bash
/// mini_git cat-file [type] [path]
/// ```
///
/// # Errors
///
/// If file system operations fail, or if input paths are not valid.
/// A [`String`] message describing the error is returned.
#[allow(clippy::module_name_repetitions)]
pub fn cmd_cat_file(_args: &Namespace) -> Result<String, String> {
    todo!();
}

/// Make `cat-file` parser
#[must_use]
pub fn make_parser() -> ArgumentParser {
    let mut parser =
        ArgumentParser::new("Provide content of repository objects");
    parser
        .add_argument("type", ArgumentType::String)
        .choices(&["blob", "commit", "tag", "tree"])
        .required()
        .add_help("Specify the type of object");

    parser
        .add_argument("object", ArgumentType::String)
        .required()
        .add_help("The object to display");

    parser
}
