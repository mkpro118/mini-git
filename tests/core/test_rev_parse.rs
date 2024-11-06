#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Mutex;

    use crate::make_namespaces_from;

    use mini_git::core::commands::rev_parse::*;
    use mini_git::core::objects::traits::KVLM;
    use mini_git::core::objects::{commit, tag, write_object, GitObject};
    use mini_git::core::GitRepository;

    use mini_git::utils::collections::kvlm;
    use mini_git::utils::path;
    use mini_git::utils::test::TempDir;

    static FS_MUTEX: Mutex<Option<TempDir<()>>> = Mutex::new(None);
    static REFS_DIR: fn() -> PathBuf =
        || std::env::current_dir().unwrap().join(".git").join("refs");

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
        let guard = FS_MUTEX.lock();
        match guard {
            Ok(mut inner) if inner.is_none() => {
                *inner = Some(create_mock_repo());
            }
            Ok(..) => {}
            Err(..) => panic!("Mutex failed!"),
        };
    }

    fn create_mock_repo() -> TempDir<'static, ()> {
        let tmp =
            TempDir::create("cmd_rev_parse").with_mutex(&crate::TEST_MUTEX);
        let _ = GitRepository::create(tmp.tmp_dir()).expect("Create repo");
        tmp.switch();
        tmp.run(|| {
            create_heads();
            create_tags();
            create_objects();
        });
        tmp
    }

    fn create_dir_if_not_exists(path: &std::path::Path) {
        if !path.exists() {
            fs::create_dir_all(path)
                .unwrap_or_else(|_| panic!("create dir {path:?}"));
            return;
        }
        assert!(path.is_dir(), "Invalid test state!");
    }

    fn create_heads() {
        let top = REFS_DIR().join("heads");
        create_dir_if_not_exists(&top);

        for res in [
            fs::write(top.join("main"), format!("{}\n", "a0".repeat(20))),
            fs::write(top.join("feature"), format!("{}\n", "b0".repeat(20))),
        ] {
            res.expect("Should write to refs/heads");
        }
    }

    fn create_tags() {
        let tag_content = b"object d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0\n\
                           type commit\n\
                           tag v1.0\n\
                           tagger Test User <test@example.com> 1234567890 +0000\n\
                           \n\
                           Initial release\n";

        let kvlm = kvlm::KVLM::parse(tag_content).expect("Should parse");
        let tag = tag::Tag::with_kvlm(kvlm);
        let obj = GitObject::Tag(tag);

        let repo_path = path::repo_find(".").expect("Should find repo");
        let repo = GitRepository::new(&repo_path).expect("Should find repo");

        let hash = write_object(&obj, &repo).expect("Should write to database");
        create_dir_if_not_exists(&OBJECT_DIR().join("b0"));
        fs::rename(
            OBJECT_DIR().join(&hash[..2]).join(&hash[2..]),
            OBJECT_DIR().join("b0").join("b0".repeat(19)),
        )
        .expect("REASON");
    }

    fn create_objects() {
        let commit_content = b"tree e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0\n\
                              author Test User <test@example.com> 1234567890 +0000\n\
                              committer Test User <test@example.com> 1234567890 +0000\n\
                              \n\
                              Initial commit\n";

        let kvlm = kvlm::KVLM::parse(commit_content).expect("Should parse");
        let commit = commit::Commit::with_kvlm(kvlm);
        let obj = GitObject::Commit(commit);

        let repo_path = path::repo_find(".").expect("Should find repo");
        let repo = GitRepository::new(&repo_path).expect("Should find repo");

        let hash = write_object(&obj, &repo).expect("Should write to database");
        create_dir_if_not_exists(&OBJECT_DIR().join("a0"));
        fs::rename(
            OBJECT_DIR().join(&hash[..2]).join(&hash[2..]),
            OBJECT_DIR().join("a0").join("a0".repeat(19)),
        )
        .expect("REASON");
    }

    #[test]
    fn test_rev_parse_all() {
        setup();
        let args: [&[&str]; 1] = [&["--all"]];
        let result = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            rev_parse(&namespace)
        });
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains(&"a0".repeat(20)));
        assert!(output.contains(&"b0".repeat(20)));
    }

    #[test]
    fn test_rev_parse_git_dir() {
        setup();
        let args: [&[&str]; 1] = [&["--git-dir"]];
        let (result, expected) = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            let path = path::repo_find(".").expect("Should be in repo");
            let path = GitRepository::new(&path)
                .expect("Should create git repository")
                .gitdir()
                .canonicalize()
                .unwrap();
            (rev_parse(&namespace), path)
        });
        assert!(result.is_ok());
        let output = result.unwrap();
        let expected = expected.to_str().unwrap();
        assert_eq!(output.trim(), expected);
    }

    #[test]
    #[cfg_attr(
        target_family = "windows",
        ignore(
            reason = "Fails on Windows, will debug in the future. Tracked by issue #66"
        )
    )]
    fn test_rev_parse_show_toplevel() {
        setup();
        let args: [&[&str]; 1] = [&["--show-toplevel"]];
        let (result, expected) = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            let path = path::repo_find(".").expect("Should be in repo");
            let path = path.canonicalize().unwrap();
            (rev_parse(&namespace), path)
        });
        assert!(result.is_ok());
        let output = result.unwrap();

        let expected = expected.to_str().unwrap();
        assert_eq!(output.trim(), expected);
    }

    #[test]
    fn test_rev_parse_is_inside_git_dir() {
        setup();
        let args: [&[&str]; 1] = [&["--is-inside-git-dir"]];
        let result = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            rev_parse(&namespace)
        });
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.trim(), "false");
    }

    #[test]
    fn test_rev_parse_is_inside_work_tree() {
        setup();
        let args: [&[&str]; 1] = [&["--is-inside-work-tree"]];
        let result = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            rev_parse(&namespace)
        });
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.trim(), "true");
    }

    #[test]
    fn test_rev_parse_revision() {
        setup();
        let args: [&[&str]; 1] = [&["main"]];
        let result = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            rev_parse(&namespace)
        });
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.trim(), &"a0".repeat(20));
    }

    #[test]
    fn test_rev_parse_revision_with_type() {
        setup();
        let args: [&[&str]; 1] = [&["--type", "commit", "main"]];
        let result = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            rev_parse(&namespace)
        });
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.trim(), &"a0".repeat(20));
    }

    #[test]
    fn test_rev_parse_invalid_revision() {
        setup();
        let args: [&[&str]; 1] = [&["nonexistent"]];
        let result = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            rev_parse(&namespace)
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_rev_parse_revision_with_wrong_type() {
        setup();
        let args: [&[&str]; 1] = [&["--type", "tree", "main"]];
        let result = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            rev_parse(&namespace)
        });
        assert!(result.is_err());
    }
}
