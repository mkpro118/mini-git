#[cfg(test)]
mod tests {
    use mini_git::core::hash_object::*;
    use mini_git::core::GitRepository;
    use mini_git::utils::argparse::Namespace;
    use mini_git::utils::test::TempDir;
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
        static CONTENT: &[(&str, &[u8])] =
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
