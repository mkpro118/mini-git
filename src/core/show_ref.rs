use crate::utils::argparse::{ArgumentParser, ArgumentType, Namespace};

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
pub fn show_ref(_args: &Namespace) -> Result<String, String> {
    todo!()
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
        .add_argument("ref", ArgumentType::String)
        .required()
        .add_help("Ref pattern to show");

    parser
}