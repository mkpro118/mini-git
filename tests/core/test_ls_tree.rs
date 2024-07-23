#[cfg(test)]
mod tests {
    use crate::make_namespaces_from;

    use mini_git::core::ls_tree::*;
    use mini_git::core::objects::traits::Serialize;
    use mini_git::core::objects::tree::{Leaf, Tree};
    use mini_git::core::GitRepository;

    use mini_git::utils::test::TempDir;
    use mini_git::utils::zlib;

    use std::sync::Mutex;

    static FS_MUTEX: Mutex<Option<TempDir<()>>> = Mutex::new(None);

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

    macro_rules! exp_tree {
        ($sha:literal, $name:literal) => {
            ("040000", &$sha.repeat(40), "tree", $name.to_string())
        };
        ($sha:literal, $name:expr) => {{
            ("040000", &$sha.repeat(40), "tree", $name)
        }};
    }

    macro_rules! exp_blob {
        ($sha:literal, $name:literal) => {
            ("100644", &$sha.repeat(40), "blob", $name.to_string())
        };
        ($sha:literal, $name:expr) => {{
            ("100644", &$sha.repeat(40), "blob", $name)
        }};
    }

    fn create_temp_repo<'a>() -> TempDir<'a, ()> {
        let tmp = TempDir::create("cmd_ls_tree").with_mutex(&crate::TEST_MUTEX);
        let repo = GitRepository::create(tmp.tmp_dir()).expect("Create repo");

        macro_rules! leaf {
            ($mode:literal, $path:literal, $hash:literal) => {
                Leaf::new($mode, $path, &$hash.repeat(40))
            };
        }

        let root = vec![
            leaf!(b"040000", b"dir1", "0"),
            leaf!(b"040000", b"dir2", "1"),
            leaf!(b"100644", b"readme.md", "3"),
            leaf!(b"100644", b"test.file", "4"),
        ];

        let dir1 = vec![
            leaf!(b"040000", b"subdir1", "5"),
            leaf!(b"100644", b"file1", "6"),
            leaf!(b"100644", b"file2", "7"),
        ];

        let dir2 = vec![
            leaf!(b"040000", b"subdir2", "7"),
            leaf!(b"100644", b"file1", "8"),
            leaf!(b"100644", b"file2", "9"),
        ];

        let subdir1 = vec![
            leaf!(b"100644", b"subfile1", "a"),
            leaf!(b"100644", b"subfile2", "b"),
        ];

        let subdir2 = vec![
            leaf!(b"100644", b"subfile1", "c"),
            leaf!(b"100644", b"subfile2", "d"),
        ];

        let mut serialized = vec![];

        macro_rules! make_tree {
            ($tree:ident, $leaves:ident, $hash:literal) => {
                let mut $tree = Tree::new();
                $tree.set_leaves($leaves);
                let serialized_tree = &$tree.serialize();
                let len = serialized_tree.len();
                let mut data = format!("tree {len}\0").as_bytes().to_vec();
                data.extend_from_slice(&serialized_tree);
                let compressed = zlib::compress(&data, &zlib::Strategy::Auto);
                serialized.push((compressed, $hash.repeat(40)));
            };
        }

        make_tree!(root_tree, root, "f"); // This "f" is arbitrary
        make_tree!(dir1_tree, dir1, "0"); // The rest of the hash strings
        make_tree!(dir2_tree, dir2, "1"); // correspond to the hash in the
        make_tree!(dir1_tree, subdir1, "5"); // leaf vectors above
        make_tree!(dir2_tree, subdir2, "7");

        let obj_dir = repo.gitdir().join("objects");

        for (data, hash) in serialized {
            let dir = obj_dir.join(&hash[..2]);
            std::fs::create_dir(&dir).expect("Should create dir");
            let path = dir.join(&hash[2..]);
            assert!(!path.is_file(), "Setup failed! File already exists");
            std::fs::write(&path, data).expect("Should write");
            assert!(path.is_file(), "Setup failed! File write failed");
        }

        tmp
    }

    fn setup() {
        let guard = FS_MUTEX.lock();
        match guard {
            Ok(mut inner) if inner.is_none() => {
                let tmp = create_temp_repo();
                *inner = Some(tmp);
            }
            Ok(..) => {}
            Err(..) => panic!("Mutex failed!"),
        };
    }

    fn check_output(expected: &[(&str, &String, &str, String)], res: &str) {
        let res: Vec<&str> = res.trim().lines().map(str::trim).collect();
        assert_eq!(res.len(), expected.len());
        for (line, (mode, sha, type_, path)) in res.iter().zip(expected.iter())
        {
            let parts: Vec<&str> = line.split_whitespace().collect();
            assert_eq!(parts.len(), 4);
            assert_eq!(parts[0], *mode);
            assert_eq!(parts[1], *type_);
            assert_eq!(parts[2], *sha);
            assert_eq!(parts[3], *path);
        }
    }

    fn join_path(x: &str, y: &str) -> String {
        std::path::Path::new(x)
            .join(y)
            .as_os_str()
            .to_str()
            .expect("path")
            .to_owned()
    }

    #[test]
    fn test_root_dir_no_recursive() {
        setup();

        let args: [&[&str]; 1] = [&[&"f".repeat(40)]];

        let res = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            ls_tree(&namespace)
        });

        assert!(res.is_ok());
        let expected = [
            exp_tree!("0", "dir1"),
            exp_tree!("1", "dir2"),
            exp_blob!("3", "readme.md"),
            exp_blob!("4", "test.file"),
        ];

        check_output(&expected, &res.unwrap());
    }

    #[test]
    fn test_root_dir_recursive() {
        setup();

        let args: [&[&str]; 1] = [&["-r", &"f".repeat(40)]];

        let res = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            ls_tree(&namespace)
        });

        assert!(res.is_ok());
        let res = res.unwrap();
        let expected = [
            exp_blob!("6", join_path("dir1", "file1")),
            exp_blob!("7", join_path("dir1", "file2")),
            exp_blob!(
                "a",
                join_path(&join_path("dir1", "subdir1"), "subfile1")
            ),
            exp_blob!(
                "b",
                join_path(&join_path("dir1", "subdir1"), "subfile2")
            ),
            exp_blob!("8", join_path("dir2", "file1")),
            exp_blob!("9", join_path("dir2", "file2")),
            exp_blob!(
                "c",
                join_path(&join_path("dir2", "subdir2"), "subfile1")
            ),
            exp_blob!(
                "d",
                join_path(&join_path("dir2", "subdir2"), "subfile2")
            ),
            exp_blob!("3", "readme.md"),
            exp_blob!("4", "test.file"),
        ];
        check_output(&expected, &res);
    }

    #[test]
    fn test_dir1_no_recursive() {
        setup();

        let args: [&[&str]; 1] = [&[&"0".repeat(40)]];

        let res = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            ls_tree(&namespace)
        });

        assert!(res.is_ok());
        let res = res.unwrap();
        let expected = [
            exp_blob!("6", "file1"),
            exp_blob!("7", "file2"),
            exp_tree!("5", "subdir1"),
        ];
        check_output(&expected, &res);
    }

    #[test]
    fn test_dir1_recursive() {
        setup();

        let args: [&[&str]; 1] = [&["-r", &"0".repeat(40)]];

        let res = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            ls_tree(&namespace)
        });

        assert!(res.is_ok());
        let res = res.unwrap();
        let expected = [
            exp_blob!("6", "file1"),
            exp_blob!("7", "file2"),
            exp_blob!("a", join_path("subdir1", "subfile1")),
            exp_blob!("b", join_path("subdir1", "subfile2")),
        ];
        check_output(&expected, &res);
    }

    #[test]
    fn test_dir2_no_recursive() {
        setup();

        let args: [&[&str]; 1] = [&[&"1".repeat(40)]];

        let res = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            ls_tree(&namespace)
        });

        assert!(res.is_ok());
        let res = res.unwrap();
        let expected = [
            exp_blob!("8", "file1"),
            exp_blob!("9", "file2"),
            exp_tree!("7", "subdir2"),
        ];
        check_output(&expected, &res);
    }

    #[test]
    fn test_dir2_recursive() {
        setup();

        let args: [&[&str]; 1] = [&["-r", &"1".repeat(40)]];

        let res = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            ls_tree(&namespace)
        });

        assert!(res.is_ok());
        let res = res.unwrap();
        let expected = [
            exp_blob!("8", "file1"),
            exp_blob!("9", "file2"),
            exp_blob!("c", join_path("subdir2", "subfile1")),
            exp_blob!("d", join_path("subdir2", "subfile2")),
        ];
        check_output(&expected, &res);
    }

    #[test]
    fn test_root_dir_show_trees() {
        setup();

        let args: [&[&str]; 1] = [&["-r", "-t", &"f".repeat(40)]];

        let res = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            ls_tree(&namespace)
        });

        assert!(res.is_ok());
        let res = res.unwrap();
        let expected = [
            exp_tree!("0", "dir1"),
            exp_blob!("6", join_path("dir1", "file1")),
            exp_blob!("7", join_path("dir1", "file2")),
            exp_tree!("5", join_path("dir1", "subdir1")),
            exp_blob!(
                "a",
                join_path(&join_path("dir1", "subdir1"), "subfile1")
            ),
            exp_blob!(
                "b",
                join_path(&join_path("dir1", "subdir1"), "subfile2")
            ),
            exp_tree!("1", "dir2"),
            exp_blob!("8", join_path("dir2", "file1")),
            exp_blob!("9", join_path("dir2", "file2")),
            exp_tree!("7", join_path("dir2", "subdir2")),
            exp_blob!(
                "c",
                join_path(&join_path("dir2", "subdir2"), "subfile1")
            ),
            exp_blob!(
                "d",
                join_path(&join_path("dir2", "subdir2"), "subfile2")
            ),
            exp_blob!("3", "readme.md"),
            exp_blob!("4", "test.file"),
        ];
        check_output(&expected, &res);
    }

    #[test]
    fn test_root_dir_only_trees() {
        setup();

        let args: [&[&str]; 1] = [&["-r", "-t", "-d", &"f".repeat(40)]];

        let res = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            ls_tree(&namespace)
        });

        assert!(res.is_ok());
        let res = res.unwrap();
        let expected = [
            exp_tree!("0", "dir1"),
            exp_tree!("5", join_path("dir1", "subdir1")),
            exp_tree!("1", "dir2"),
            exp_tree!("7", join_path("dir2", "subdir2")),
        ];
        check_output(&expected, &res);
    }
}
