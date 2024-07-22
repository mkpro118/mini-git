#[cfg(test)]
mod tests {
    use crate::make_namespaces_from;

    use mini_git::core::ls_tree::*;
    use mini_git::core::objects::traits::Serialize;
    use mini_git::core::objects::tree::{Leaf, Tree};
    use mini_git::core::GitRepository;

    use mini_git::utils::test::TempDir;

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
            leaf!(b"100644", b"subfile1", "a"),
            leaf!(b"100644", b"subfile2", "b"),
        ];

        let mut serialized = vec![];

        macro_rules! make_tree {
            ($tree:ident, $leaves:ident, $hash:literal) => {
                let mut $tree = Tree::new();
                $tree.set_leaves($leaves);
                serialized.push(($tree.serialize(), $hash.repeat(40)));
            };
        }

        make_tree!(root_tree, root, "f"); // This "f" is arbitrary
        make_tree!(dir1_tree, dir1, "0"); // The rest of the hash strings
        make_tree!(dir2_tree, dir2, "1"); // correspond to the hash in the
        make_tree!(dir1_tree, subdir1, "5"); // leaf vectors above
        make_tree!(dir2_tree, subdir2, "7");

        let obj_dir = repo.gitdir().join("objects");

        for (data, hash) in serialized {
            let path = obj_dir.join(&hash[..2]).join(&hash[2..]);
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

    #[test]
    fn test_root_dir_no_recursive() {
        setup();

        let args: [&[&str]; 0] = [];

        let res = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            ls_tree(&namespace)
        });

        assert!(res.is_ok());
        todo!()
    }

    #[test]
    fn test_root_dir_recursive() {
        setup();

        let args: [&[&str]; 0] = [];

        let res = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            ls_tree(&namespace)
        });

        assert!(res.is_ok());
        todo!()
    }

    #[test]
    fn test_root_subdir1_no_recursive() {
        setup();

        let args: [&[&str]; 0] = [];

        let res = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            ls_tree(&namespace)
        });

        assert!(res.is_ok());
        todo!()
    }

    #[test]
    fn test_root_subdir1_recursive() {
        setup();

        let args: [&[&str]; 0] = [];

        let res = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            ls_tree(&namespace)
        });

        assert!(res.is_ok());
        todo!()
    }

    #[test]
    fn test_root_subdir2_no_recursive() {
        setup();

        let args: [&[&str]; 0] = [];

        let res = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            ls_tree(&namespace)
        });

        assert!(res.is_ok());
        todo!()
    }

    #[test]
    fn test_root_subdir2_recursive() {
        setup();

        let args: [&[&str]; 0] = [];

        let res = switch_dir!({
            let namespace = make_namespaces(&args).next().unwrap();
            ls_tree(&namespace)
        });

        assert!(res.is_ok());
        todo!()
    }
}
