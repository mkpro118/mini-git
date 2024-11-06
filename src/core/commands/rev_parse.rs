use crate::core::{objects, GitRepository};
use crate::utils::argparse::{ArgumentParser, ArgumentType, Namespace};
use crate::utils::path;

#[allow(clippy::module_name_repetitions)]
pub fn rev_parse(args: &Namespace) -> Result<String, String> {
    let cwd = std::env::current_dir().map_err(|_| {
        "Could not determine current working directory".to_owned()
    })?;

    let repo_path = path::repo_find(&cwd)?
        .canonicalize()
        .map_err(|_| "Could not determine repository path".to_owned())?;
    let repo = GitRepository::new(&repo_path)?;

    if args.get("show-toplevel").is_some() {
        return repo_path
            .into_os_string()
            .into_string()
            .map_err(|_| "Could not determine repository toplevel".to_owned());
    }

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
        .add_argument("show-toplevel", ArgumentType::Boolean)
        .add_help(
        "Show the absolute path of the top-level directory of the working tree",
    );

    parser
        .add_argument("revision", ArgumentType::String)
        .required()
        .default("*")
        .add_help("The revision to parse");

    parser
}
