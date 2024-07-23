#[cfg(test)]
mod tests {
    use crate::make_namespaces_from;
    use crate::run_test;

    use mini_git::core::cat_file::*;
    use mini_git::core::objects::blob;
    use mini_git::core::objects::traits::Deserialize;
    use mini_git::core::objects::{write_object, GitObject};
    use mini_git::core::GitRepository;

    use mini_git::utils::test::TempDir;

    use std::path::PathBuf;
    use std::sync::Mutex;

    static FS_MUTEX: Mutex<Option<TempDir<()>>> = Mutex::new(None);
    static mut TEST_COUNTER: usize = 0;
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

                unsafe {
                    TEST_COUNTER += 1;
                }
                *inner = Some(tmp);
            }
            Ok(..) => unsafe {
                TEST_COUNTER += 1;
            },
            Err(..) => panic!("Mutex failed!"),
        };
        unsafe {
            println!("incr: {TEST_COUNTER}");
        }
    }

    fn teardown() {
        let Ok(mut guard) = FS_MUTEX.lock() else {
            panic!("Mutex failed!");
        };

        unsafe {
            TEST_COUNTER -= 1;
            println!("decr: {TEST_COUNTER}");
            if TEST_COUNTER == 0 {
                println!("Dropping MUTEX!!!!");
                // let temp = guard.clone().unwrap();
                // let temp = temp.tmp_dir().to_path_buf();
                // println!("temp 1 {:?}", temp);
                *guard = None;
                // println!("temp 2 {temp:?} exists? {:?}", temp.exists());
            }
        }
    }

    #[test]
    fn test_cmd_cat_file_readme() {
        run_test!(setup, teardown, {
            let readme_hash = "cdb5f04f10c21998fd7406f7e8ceafd2035d83e2";
            let args: [&[&str]; 1] = [&["blob", readme_hash]];

            let res = switch_dir!({
                let namespaces = make_namespaces(&args).next().unwrap();
                cat_file(&namespaces)
            });

            assert!(res.is_ok(), "{res:?}");
            let res = res.unwrap();

            assert_eq!(res, "readme.md\n");
        });
    }

    #[test]
    fn test_cmd_cat_file_testfile() {
        run_test!(setup, teardown, {
            let testfile_hash = "26918572ece0bcfca23251753b32b672be31cf56";
            let args: [&[&str]; 1] = [&["blob", testfile_hash]];

            let res = switch_dir!({
                let namespaces = make_namespaces(&args).next().unwrap();
                cat_file(&namespaces)
            });

            assert!(res.is_ok(), "{res:?}");
            let res = res.unwrap();

            assert_eq!(res, "testfile\n");
        });
    }

    #[test]
    fn test_cmd_cat_file_bad_file() {
        run_test!(setup, teardown, {
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
        });
    }
}
