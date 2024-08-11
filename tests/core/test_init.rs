#[cfg(test)]
mod tests {
    use crate::make_namespaces_from;

    use mini_git::core::commands::init::*;
    use mini_git::utils::test::TempDir;

    use std::path::Path;
    use std::sync::Mutex;

    static FS_MUTEX: Mutex<()> = Mutex::new(());

    make_namespaces_from!(make_parser);

    macro_rules! switch_dir {
        ($target_dir:ident, $body:block) => {
            match FS_MUTEX.lock() {
                Ok(_) => ($target_dir).run(|| $body),
                Err(..) => panic!("FS Mutex failed!"),
            }
        };
    }

    #[test]
    fn test_cmd_init_no_args() {
        let args: [&[&str]; 1] = [&[]];
        let namespaces = make_namespaces(&args).next().unwrap();

        let tmp_dir = TempDir::<()>::create("cmd_init_no_args")
            .with_mutex(&crate::TEST_MUTEX);

        let res = switch_dir!(tmp_dir, { init(&namespaces) });

        assert!(res.is_ok(), "{res:?}");
        let res = res.unwrap();

        assert!(res.contains("initialized"));

        check_expected_path(tmp_dir.tmp_dir());
    }

    #[test]
    fn test_cmd_init_explicit_dot() {
        let args: [&[&str]; 1] = [&["."]];
        let namespaces = make_namespaces(&args).next().unwrap();

        let tmp_dir = TempDir::<()>::create("cmd_init_no_args")
            .with_mutex(&crate::TEST_MUTEX);

        let res = switch_dir!(tmp_dir, { init(&namespaces) });

        assert!(res.is_ok(), "{res:?}");
        let res = res.unwrap();

        assert!(res.contains("initialized"));

        check_expected_path(tmp_dir.tmp_dir());
    }

    #[test]
    fn test_cmd_init_path() {
        let args: [&[&str]; 1] = [&["new_dir"]];
        let namespaces = make_namespaces(&args).next().unwrap();

        let tmp_dir = TempDir::<()>::create("cmd_init_no_args")
            .with_mutex(&crate::TEST_MUTEX);

        let res = switch_dir!(tmp_dir, { init(&namespaces) });

        assert!(res.is_ok(), "{res:?}");
        let res = res.unwrap();

        assert!(res.contains("initialized"));

        check_expected_path(&tmp_dir.tmp_dir().join(args[0][0]));
    }

    #[test]
    fn test_cmd_init_extra_args() {
        let args: [&[&str]; 1] = [&["new_repo", "arg1", "arg2"]];
        assert!(make_namespaces(&args).next().is_none());
    }

    fn check_expected_path(root: &Path) {
        use mini_git::utils::path;
        use std::path::PathBuf;

        // Really complicated struct for abstraction and code reduction
        // Basically contains
        // (function to get path, function to test path, items to test)
        struct TestData<'a, 'b, 'c, 'd, 'e, 'f, 'g>(
            &'a dyn Fn(
                &'b Path,
                &'c [&'d str],
                bool,
            ) -> Result<Option<PathBuf>, String>,
            &'e dyn Fn(&Path) -> bool,
            &'f [&'g [&'static str]],
        );

        assert!(root.exists(), "ROOT {root:?} does not exist");
        assert!(root.is_dir(), "ROOT {root:?} is not a directory");

        let expected_git_dir = root.join(".git");
        assert!(
            expected_git_dir.exists(),
            "GITDIR {expected_git_dir:?} does not exist"
        );
        assert!(
            expected_git_dir.is_dir(),
            "GITDIR {expected_git_dir:?} is not a directory"
        );

        let expected_gitdir_subitems = [
            TestData(
                &path::repo_dir,
                &Path::is_dir,
                &[
                    &["branches"],
                    &["objects"],
                    &["refs", "tags"],
                    &["refs", "heads"],
                ],
            ),
            TestData(
                &path::repo_file,
                &Path::is_file,
                &[&["description"], &["HEAD"], &["config"]],
            ),
        ];

        for TestData(path_fn, test_fn, subdirs) in expected_gitdir_subitems {
            for subdir in subdirs {
                let dir = path_fn(&expected_git_dir, subdir, false);
                assert!(
                    dir.as_ref()
                        .is_ok_and(|p| p.as_ref().is_some_and(|p| p.exists())),
                    "{subdir:?} expected, but doesn't exist"
                );
                assert!(dir
                    .as_ref()
                    .is_ok_and(|p| p.as_ref().is_some_and(|p| test_fn(p))));
            }
        }
    }
}
