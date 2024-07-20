use crate::utils::argparse::{ArgumentParser, ArgumentType, Namespace};

#[allow(clippy::module_name_repetitions)]
pub fn cmd_hash_object(_args: &Namespace) -> Result<String, String> {
    todo!();
}

/// Make `hash-object` parser
#[must_use]
pub fn make_parser() -> ArgumentParser {
    let mut parser = ArgumentParser::new(
        "Compute object ID and optionally creates a blob from a file",
    );
    parser
        .add_argument("type", ArgumentType::String)
        .required()
        .default("blob")
        .add_help("Specify the type of object");

    parser
        .add_argument("write", ArgumentType::Boolean)
        .optional()
        .short('w')
        .add_help("Actually write the object into the database");

    parser
        .add_argument("path", ArgumentType::String)
        .required()
        .add_help("Read object from <file>");

    parser
}
