use crate::core::{objects, GitRepository};
use crate::utils::argparse::{ArgumentParser, ArgumentType, Namespace};
use crate::utils::path;

#[allow(clippy::module_name_repetitions)]
pub fn rev_parse(args: &Namespace) -> Result<String, String> {
    let Ok(cwd) = std::env::current_dir() else {
        return Err("Could not determined current working directory".to_owned());
    };
    let path = path::repo_find(cwd)?;
    let repo = GitRepository::new(&path)?;

    let type_ = args.get("type").map(|x| x.as_str());
    let revision = &args["revision"];

    objects::find_object(&repo, revision, type_, true)
}

/// Make `rev-parse` parser
#[must_use]
pub fn make_parser() -> ArgumentParser {
    let mut parser =
        ArgumentParser::new("Parse revision (or other objects) identifiers");
    parser
        .add_argument("type", ArgumentType::String)
        .choices(&["blob", "commit", "tag", "tree"])
        .add_help("Specify the type of object");

    parser
        .add_argument("revision", ArgumentType::String)
        .required()
        .add_help("The revision to parse");

    parser
}
