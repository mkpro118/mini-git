use crate::utils::argparse::{ArgumentParser, ArgumentType, Namespace};
use crate::utils::path;

use crate::core::objects::traits::{Deserialize, KVLM};
use crate::core::objects::{blob::Blob, commit::Commit, tag::Tag, tree::Tree};
use crate::core::objects::{hash_object, write_object, GitObject};
use crate::core::GitRepository;

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
#[allow(clippy::module_name_repetitions)]
pub fn cmd_hash_object(args: &Namespace) -> Result<String, String> {
    let Ok(data) = std::fs::read(&args["path"]) else {
        return Err(format!("failed to read file at {}", args["path"]));
    };

    let obj = make_object(&args["type"].to_lowercase(), &data)?;

    let sha = if matches!(args.get("write"), Some(..)) {
        let Ok(cwd) = std::env::current_dir() else {
            return Err(
                "Could not determined current working directory".to_owned()
            );
        };

        let repo = path::repo_find(cwd)?;
        let repo = GitRepository::new(&repo)?;
        write_object(&obj, &repo)?
    } else {
        let (_, mut sha) = hash_object(&obj);
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
        .add_argument("path", ArgumentType::String)
        .required()
        .short('p')
        .add_help("Read object from <file>");

    parser
        .add_argument("type", ArgumentType::String)
        .required()
        .short('t')
        .default("blob")
        .add_help("Specify the type of object");

    parser
        .add_argument("write", ArgumentType::Boolean)
        .optional()
        .short('w')
        .add_help("Actually write the object into the database");

    parser
}
