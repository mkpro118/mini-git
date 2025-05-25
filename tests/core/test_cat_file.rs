#[cfg(test)]
mod tests {
    use crate::make_namespaces_from;

    use mini_git::core::commands::cat_file::*;
    use mini_git::core::objects::blob;
    use mini_git::core::objects::traits::Deserialize;
    use mini_git::core::objects::{write_object, GitObject};
    use mini_git::core::GitRepository;

    use mini_git::utils::test::TempDir;

    use std::path::PathBuf;
    use std::sync::Mutex;

    static FS_MUTEX: Mutex<Option<TempDir<()>>> = Mutex::new(None);
    static OBJECT_DIR: fn() -> PathBuf = || {
        std::env::current_dir()
            .unwrap()
            .join(".git")
            .join("objects")
    };

    make_namespaces_from!(make_parser);

    macro_rules! switch_dir {
        ($body:block) => {
            match FS_MUTEX.lock() {
                Ok(inner) if inner.is_some() => {
                    (inner.as_ref().unwrap()).run(|| $body)
                }
                Ok(_) => unreachable!(),
                Err(..) => panic!("FS Mutex failed!"),
            }
        };
    }

    fn setup() {
        let content: &[(GitObject, &str)] = &[
            (
                GitObject::Blob(
                    blob::Blob::deserialize(b"readme.md\n").unwrap(),
                ),
                "cdb5f04f10c21998fd7406f7e8ceafd2035d83e2",
            ),
            (
                GitObject::Blob(
                    blob::Blob::deserialize(b"testfile\n").unwrap(),
                ),
                "26918572ece0bcfca23251753b32b672be31cf56",
            ),
        ];
        let guard = FS_MUTEX.lock();
        match guard {
            Ok(mut inner) if inner.is_none() => {
                let tmp = TempDir::create("cmd_cat_file")
                    .with_mutex(&crate::TEST_MUTEX);
                let repo =
                    GitRepository::create(tmp.tmp_dir()).expect("Create repo");
                tmp.switch();

                tmp.run(|| {
                    for (obj, exp_hash) in content {
                        let Ok(hash) = write_object(obj, &repo) else {
                            panic!("Setup failed to write objects!");
                        };
                        assert_eq!(
                            *exp_hash, hash,
                            "hashes did not match in setup"
                        );

                        let file =
                            OBJECT_DIR().join(&hash[..2]).join(&hash[2..]);
                        assert!(file.is_file());
                    }
                });

                *inner = Some(tmp);
            }
            Ok(..) => {}
            Err(..) => panic!("Mutex failed!"),
        }
    }

    #[test]
    fn test_cmd_cat_file_readme() {
        setup();

        let readme_hash = "cdb5f04f10c21998fd7406f7e8ceafd2035d83e2";
        let args: [&[&str]; 1] = [&["blob", readme_hash]];

        let res = switch_dir!({
            let namespaces = make_namespaces(&args).next().unwrap();
            cat_file(&namespaces)
        });

        assert!(res.is_ok(), "{res:?}");
        let res = res.unwrap();

        assert_eq!(res, "readme.md\n");
    }

    #[test]
    fn test_cmd_cat_file_testfile() {
        setup();

        let testfile_hash = "26918572ece0bcfca23251753b32b672be31cf56";
        let args: [&[&str]; 1] = [&["blob", testfile_hash]];

        let res = switch_dir!({
            let namespaces = make_namespaces(&args).next().unwrap();
            cat_file(&namespaces)
        });

        assert!(res.is_ok(), "{res:?}");
        let res = res.unwrap();

        assert_eq!(res, "testfile\n");
    }

    #[test]
    fn test_cmd_cat_file_bad_file() {
        setup();

        let args: [&[&str]; 4] = [
            &["blob", "blob"],
            &["commit", "commit"],
            &["tag", "tag"],
            &["tree", "tree"],
        ];

        let res = switch_dir!({
            let namespaces = make_namespaces(&args).next().unwrap();
            cat_file(&namespaces)
        });

        assert!(res.is_err(), "{res:?}");
    }
}
