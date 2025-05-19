use crate::utils::argparse::{ArgumentParser, ArgumentType, Namespace};

use crate::core::objects::traits::{Deserialize, KVLM};
use crate::core::objects::{self, write_object, GitObject};
use crate::core::objects::{blob::Blob, commit::Commit, tag::Tag, tree::Tree};
use crate::core::{resolve_repository_context, RepositoryContext};

/// Computes the hash for a git object
///
/// This handles the subcommand
///
/// ```bash
/// mini_git hash-object [--type TYPE] [--write] path
/// ```
///
/// # Errors
///
/// If file system operations fail, or if input paths are not valid.
/// A [`String`] message describing the error is returned.
pub fn hash_object(args: &Namespace) -> Result<String, String> {
    let Ok(data) = std::fs::read(&args["path"]) else {
        return Err(format!("failed to read file at {}", args["path"]));
    };

    let obj = make_object(&args["type"].to_lowercase(), &data)?;

    let sha = if matches!(args.get("write"), Some(..)) {
        let RepositoryContext { repo, .. } = resolve_repository_context()?;
        write_object(&obj, &repo)?
    } else {
        let (_, mut sha) = objects::hash_object(&obj);
        sha.hex_digest()
    };

    Ok(sha)
}

fn make_object(obj_type: &str, data: &[u8]) -> Result<GitObject, String> {
    Ok(match obj_type {
        "blob" => GitObject::Blob(Blob::deserialize(data)?),
        "commit" => GitObject::Commit(Commit::deserialize(data)?),
        "tag" => GitObject::Tag(Tag::deserialize(data)?),
        "tree" => GitObject::Tree(Tree::deserialize(data)?),
        _ => return Err(format!("{obj_type} is not a known object type")),
    })
}

/// Make `hash-object` parser
#[must_use]
pub fn make_parser() -> ArgumentParser {
    let mut parser = ArgumentParser::new(
        "Compute object ID and optionally creates a blob from a file",
    );

    parser
        .add_argument("type", ArgumentType::String)
        .optional()
        .short('t')
        .choices(&["blob", "commit", "tag", "tree"])
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
        .short('p')
        .add_help("Read object from <file>");

    parser
}
