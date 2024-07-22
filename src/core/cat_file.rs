use crate::utils::argparse::{ArgumentParser, ArgumentType, Namespace};
use crate::utils::path;

use crate::core::objects::{find_object, read_object};
use crate::core::GitRepository;

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
pub fn cat_file(args: &Namespace) -> Result<String, String> {
    let Ok(cwd) = std::env::current_dir() else {
        return Err("Could not determined current working directory".to_owned());
    };
    let path = path::repo_find(cwd)?;
    let repo = GitRepository::new(&path)?;

    let obj_type = &args["type"];
    let name = &args["object"];

    let object = find_object(&repo, name, Some(obj_type), true);
    let object = read_object(&repo, &object)?;
    let Ok(s) = String::from_utf8(object.serialize()) else {
        return Err("Failed to serialize object!".to_owned());
    };
    Ok(s)
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
