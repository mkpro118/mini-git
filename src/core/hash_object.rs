use crate::utils::argparse::{ArgumentParser, ArgumentType, Namespace};
use crate::utils::path;

use crate::core::objects::traits::{Deserialize, KVLM};
use crate::core::objects::{blob::Blob, commit::Commit, tag::Tag, tree::Tree};
use crate::core::objects::{hash_object, write_object, GitObject};
use crate::core::GitRepository;

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
        "blob" => GitObject::Blob(Blob::deserialize(&data)?),
        "commit" => GitObject::Commit(Commit::deserialize(&data)?),
        "tag" => GitObject::Tag(Tag::deserialize(&data)?),
        "tree" => GitObject::Tree(Tree::deserialize(&data)?),
        _ => return Err(format!("{} is not a known object type", obj_type)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test::TempDir;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    static FS_MUTEX: Mutex<Option<TempDir>> = Mutex::new(None);
    static OBJECT_DIR: fn() -> PathBuf = || Path::new(".git").join("objects");

    fn make_namespaces<'a>(
        args: &'a [&[&'a str]],
    ) -> impl Iterator<Item = Namespace> + 'a {
        let mut parser = make_parser();
        parser.compile();

        args.iter().flat_map(move |&x| parser.parse_args(x))
    }

    fn setup() {
        static CONTENT: &[(&'static str, &'static [u8])] =
            &[("readme", b"readme.md\n"), ("test.file", b"testfile\n")];
        let guard = FS_MUTEX.lock();
        match guard {
            Ok(mut inner) if inner.is_none() => {
                let tmp = TempDir::create("cmd_hash_object");
                GitRepository::create(tmp.test_dir()).expect("Create repo");

                for (file, content) in CONTENT {
                    fs::write(file, content).expect("Should write");
                }

                *inner = Some(tmp);
            }
            Ok(..) => {}
            Err(..) => panic!("Mutex failed!"),
        };
    }

    #[test]
    fn test_cmd_hash_object_readme() {
        setup();

        let args: [&[&str]; 1] = [&["readme"]];
        let namespaces = make_namespaces(&args).next().unwrap();

        let res = cmd_hash_object(&namespaces);

        assert!(res.is_ok(), "{res:?}");
        let res = res.unwrap();

        assert_eq!(res, "cdb5f04f10c21998fd7406f7e8ceafd2035d83e2");
    }

    #[test]
    fn test_cmd_hash_object_testfile() {
        setup();

        let args: [&[&str]; 1] = [&["test.file"]];
        let namespaces = make_namespaces(&args).next().unwrap();

        let res = cmd_hash_object(&namespaces);

        assert!(res.is_ok(), "{res:?}");
        let res = res.unwrap();

        assert_eq!(res, "26918572ece0bcfca23251753b32b672be31cf56");
    }

    #[test]
    fn test_cmd_hash_object_readme_write() {
        setup();

        let args: [&[&str]; 1] = [&["-w", "readme"]];
        let namespaces = make_namespaces(&args).next().unwrap();

        let res = cmd_hash_object(&namespaces);

        assert!(res.is_ok(), "{res:?}");
        let res = res.unwrap();

        let exp_sha = "cdb5f04f10c21998fd7406f7e8ceafd2035d83e2";
        assert_eq!(res, exp_sha);

        let file = OBJECT_DIR().join(&exp_sha[..2]).join(&exp_sha[2..]);
        assert!(file.exists(), "{file:?} doesn't exist");
        assert!(file.is_file(), "{file:?} is not a file");
    }

    #[test]
    fn test_cmd_hash_object_testfile_write() {
        setup();

        let args: [&[&str]; 1] = [&["test.file", "-w"]];
        let namespaces = make_namespaces(&args).next().unwrap();

        let res = cmd_hash_object(&namespaces);

        assert!(res.is_ok(), "{res:?}");
        let res = res.unwrap();

        let exp_sha = "26918572ece0bcfca23251753b32b672be31cf56";
        assert_eq!(res, exp_sha);

        let file = OBJECT_DIR().join(&exp_sha[..2]).join(&exp_sha[2..]);
        assert!(file.exists(), "{file:?} doesn't exist");
        assert!(file.is_file(), "{file:?} is not a file");
    }
}
