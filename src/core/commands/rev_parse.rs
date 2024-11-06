use crate::utils::argparse::{ArgumentParser, ArgumentType, Namespace};

#[allow(clippy::module_name_repetitions)]
pub fn rev_parse(_args: &Namespace) -> Result<String, String> {
    todo!()
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
