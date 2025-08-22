use std::fs;

use crate::{core::GitRepository, utils::path::repo_path};

macro_rules! u32_from_be_bytes {
    ($var:ident, $from:expr, $to:expr) => {
        u32::from_be_bytes({
            $var[($from)..($to)]
                .try_into()
                .expect("Should contain 4 bytes")
        })
    };
}

macro_rules! u16_from_be_bytes {
    ($var:ident, $from:expr, $to:expr) => {
        u16::from_be_bytes({
            $var[($from)..($to)]
                .try_into()
                .expect("Should contain 2 bytes")
        })
    };
}

type SHA1 = [u8; 20];

#[derive(Default, Clone, Debug, PartialEq)]
pub struct GitIndexEntry {
    pub ctime: (u32, u32),
    pub mtime: (u32, u32),
    pub device_id: u32,
    pub inode: u32,
    pub mode_type: u8,
    pub mode_perms: u16,
    pub uid: u32,
    pub gid: u32,
    pub size: u32,
    pub sha1: SHA1,
    pub flag_assume_valid: bool,
    pub flag_stage: u16,
    pub name: String,
}

#[derive(Debug)]
#[expect(unused)]
pub struct GitIndex {
    version: u32,
    entries: Vec<GitIndexEntry>,
}

impl Default for GitIndex {
    fn default() -> Self {
        Self {
            version: 2,
            entries: Vec::default(),
        }
    }
}

impl GitIndex {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn set_version(self, version: u32) -> Self {
        Self { version, ..self }
    }

    #[must_use]
    pub fn set_entries(self, entries: &[GitIndexEntry]) -> Self {
        Self {
            entries: entries.to_vec(),
            ..self
        }
    }

    #[must_use]
    pub fn entries(&self) -> &[GitIndexEntry] {
        &self.entries
    }

    /// Read the index file from disk
    ///
    /// # Errors
    ///   - If the index file is invalid or corrupted
    ///
    // Allow too many lines because most of the lines just read/write metadata
    #[expect(clippy::too_many_lines)]
    pub fn read_index(repo: &GitRepository) -> Result<Self, String> {
        const HEADER_SIZE: usize = 12;
        const SIGNATURE_SIZE: usize = 4;
        const METADATA_SIZE: usize = 62;
        const REGULAR_FILE_MODE: u8 = 0b1000;
        const SYMLINK_FILE_MODE: u8 = 0b1010;
        const GITLINK_FILE_MODE: u8 = 0b1110;

        let index_file = repo_path(repo.gitdir(), &["index"]);
        if !index_file.exists() {
            return Ok(Self::new());
        }

        let raw = fs::read(&index_file).map_err(|e| e.to_string())?;
        if raw.len() < HEADER_SIZE {
            return Err("Invalid index file".to_string());
        }
        let header = &raw[..HEADER_SIZE];
        let signature = &header[..SIGNATURE_SIZE];

        if signature != b"DIRC" {
            return Err("Invalid index file signature".to_string());
        }

        let version = u32_from_be_bytes!(header, 4, 8);
        if version != 2 {
            return Err("Unsupported index file version".to_string());
        }

        let count = u32_from_be_bytes!(header, 8, 12);

        let mut entries = Vec::new();

        let contents = &raw[HEADER_SIZE..];
        let contents_size = contents.len();

        let mut idx = 0;

        for _ in 0..count {
            // +1 for at least one byte of name
            if idx + METADATA_SIZE + 1 > contents_size {
                let remaining_bytes = contents_size - idx;
                return Err(
                    format!("Index file is corrupted, expected at least {contents_size} more bytes, got {remaining_bytes}")
                );
            }
            let ctime_seconds = u32_from_be_bytes!(contents, idx, idx + 4);
            let ctime_nanoseconds =
                u32_from_be_bytes!(contents, idx + 4, idx + 8);
            let mtime_seconds = u32_from_be_bytes!(contents, idx + 8, idx + 12);
            let mtime_nanoseconds =
                u32_from_be_bytes!(contents, idx + 12, idx + 16);
            let device_id = u32_from_be_bytes!(contents, idx + 16, idx + 20);
            let inode = u32_from_be_bytes!(contents, idx + 20, idx + 24);

            let unused = u16_from_be_bytes!(contents, idx + 24, idx + 26);
            if unused != 0 {
                return Err(format!(
                    "Expected unused bits to be 0, got {unused}"
                ));
            }

            let mode = u16_from_be_bytes!(contents, idx + 26, idx + 28);

            // First 4 bits are the mode bits
            let mode_type = (mode >> 12) as u8;
            match mode_type {
                REGULAR_FILE_MODE | SYMLINK_FILE_MODE | GITLINK_FILE_MODE => {}
                _ => return Err(format!("Invalid file mode {mode_type:#b}")),
            }
            // Last 9 bits are the permission bits
            let mode_perms = mode & 0x1FF;

            let uid = u32_from_be_bytes!(contents, idx + 28, idx + 32);
            let gid = u32_from_be_bytes!(contents, idx + 32, idx + 36);

            let file_size = u32_from_be_bytes!(contents, idx + 36, idx + 40);
            let sha: SHA1 = {
                let mut sha = [0; 20];
                sha.clone_from_slice(&contents[idx + 40..idx + 60]);
                sha
            };

            let flags = u16_from_be_bytes!(contents, idx + 60, idx + 62);

            let flag_assume_valid = flags & 0x8000 != 0;
            let flag_extended = flags & 0x4000 != 0;
            if flag_extended {
                return Err(
                    "Extended index files are not supported".to_string()
                );
            }
            let flag_stage = flags & 0x3000;

            let name_length = flags & 0x0FFF;

            // Metadata parsed in the first 62 bytes (METADATA_SIZE)
            idx += METADATA_SIZE;

            if (idx + (name_length as usize)) > contents_size {
                return Err(
                    format!("Index file is corrupted, expected at least {name_length} more bytes")
                );
            }

            let raw_name = if name_length < 0xFFF {
                if idx + name_length as usize + 1 > contents_size
                    || contents[idx + name_length as usize] != 0
                {
                    return Err(
                        "Index file is corrupted, name must be null-terminated"
                            .to_string(),
                    );
                }
                let name_slice = &contents[idx..(idx + name_length as usize)];
                idx += name_length as usize + 1; // +1 for null terminator
                name_slice
            } else {
                // Find NULL terminator starting from idx + 0xFFF
                let mut null_idx = idx + 0xFFF;
                while null_idx < contents_size && contents[null_idx] != 0 {
                    null_idx += 1;
                }

                if null_idx >= contents_size {
                    return Err(
                        "Index file is corrupted, name must be null-terminated"
                            .to_string(),
                    );
                }

                let name_slice = &contents[idx..null_idx];
                idx = null_idx + 1; // Move past the null terminator
                name_slice
            };

            let name = String::from_utf8_lossy(raw_name).to_string();

            // Align to 8-byte boundary for next entry
            idx = 8 * idx.div_ceil(8);

            let entry = GitIndexEntry {
                ctime: (ctime_seconds, ctime_nanoseconds),
                mtime: (mtime_seconds, mtime_nanoseconds),
                device_id,
                inode,
                mode_type,
                mode_perms,
                uid,
                gid,
                size: file_size,
                sha1: sha,
                flag_assume_valid,
                flag_stage,
                name,
            };
            entries.push(entry);
        }

        Ok(Self::new().set_version(version).set_entries(&entries))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test::TempDir;
    use std::error::Error;
    use std::fs;

    #[test]
    fn new_has_no_entries() {
        let index = GitIndex::new();
        assert!(index.entries().is_empty());
    }

    #[test]
    fn set_entries_sets_entries() {
        let entry = GitIndexEntry::default();
        let index = GitIndex::new().set_entries(std::slice::from_ref(&entry));
        assert_eq!(index.entries(), &[entry]);
    }

    #[test]
    fn read_index_missing_file() -> Result<(), Box<dyn Error>> {
        let temp = TempDir::<()>::create("test_repo");
        let repo = GitRepository::create(temp.tmp_dir())?;
        let index = GitIndex::read_index(&repo)?;
        assert!(index.entries().is_empty());
        Ok(())
    }

    #[test]
    fn read_index_invalid_short_file() -> Result<(), Box<dyn Error>> {
        let temp = TempDir::<()>::create("test_repo");
        let repo = GitRepository::create(temp.tmp_dir())?;
        let index_path = repo.gitdir().join("index");
        fs::write(&index_path, [0u8; 4])?;
        let err = GitIndex::read_index(&repo).unwrap_err();
        assert_eq!(err, "Invalid index file");
        Ok(())
    }

    #[test]
    fn read_index_bad_signature() -> Result<(), Box<dyn Error>> {
        let temp = TempDir::<()>::create("test_repo");
        let repo = GitRepository::create(temp.tmp_dir())?;
        let mut data = Vec::new();
        data.extend_from_slice(b"BAD!");
        data.extend_from_slice(&[0u8; 8]);
        fs::write(repo.gitdir().join("index"), &data)?;
        let err = GitIndex::read_index(&repo).unwrap_err();
        assert_eq!(err, "Invalid index file signature");
        Ok(())
    }

    #[test]
    fn read_index_unsupported_version() -> Result<(), Box<dyn Error>> {
        let temp = TempDir::<()>::create("test_repo");
        let repo = GitRepository::create(temp.tmp_dir())?;
        let mut data = Vec::new();
        data.extend_from_slice(b"DIRC");
        data.extend_from_slice(&1u32.to_be_bytes()); // wrong version
        data.extend_from_slice(&0u32.to_be_bytes()); // count = 0
        fs::write(repo.gitdir().join("index"), &data)?;
        let err = GitIndex::read_index(&repo).unwrap_err();
        assert_eq!(err, "Unsupported index file version");
        Ok(())
    }

    #[test]
    fn read_index_unused_nonzero() -> Result<(), Box<dyn Error>> {
        let temp = TempDir::<()>::create("test_repo");
        let repo = GitRepository::create(temp.tmp_dir())?;
        let mut raw = Vec::new();
        // Header
        raw.extend_from_slice(b"DIRC");
        raw.extend_from_slice(&2u32.to_be_bytes());
        raw.extend_from_slice(&1u32.to_be_bytes());
        // Metadata fields
        raw.extend_from_slice(&1u32.to_be_bytes());
        raw.extend_from_slice(&1u32.to_be_bytes());
        raw.extend_from_slice(&1u32.to_be_bytes());
        raw.extend_from_slice(&1u32.to_be_bytes());
        raw.extend_from_slice(&1u32.to_be_bytes());
        raw.extend_from_slice(&1u32.to_be_bytes());
        raw.extend_from_slice(&1u16.to_be_bytes()); // unused non-zero
        raw.extend_from_slice(&((0b1000u16 << 12) | 0o644).to_be_bytes()); // mode
        raw.extend_from_slice(&0u32.to_be_bytes());
        raw.extend_from_slice(&0u32.to_be_bytes());
        raw.extend_from_slice(&0u32.to_be_bytes());
        raw.extend_from_slice(&[0u8; 20]); // sha1
        raw.extend_from_slice(&1u16.to_be_bytes()); // flags/name length
        raw.extend_from_slice(b"x"); // name
        raw.push(0); // null terminator
        fs::write(repo.gitdir().join("index"), &raw)?;
        let err = GitIndex::read_index(&repo).unwrap_err();
        assert_eq!(err, "Expected unused bits to be 0, got 1");
        Ok(())
    }

    #[test]
    fn read_index_invalid_mode_type() -> Result<(), Box<dyn Error>> {
        let temp = TempDir::<()>::create("test_repo");
        let repo = GitRepository::create(temp.tmp_dir())?;
        let mut raw = Vec::new();
        // Header
        raw.extend_from_slice(b"DIRC");
        raw.extend_from_slice(&2u32.to_be_bytes());
        raw.extend_from_slice(&1u32.to_be_bytes());
        // Metadata fields
        raw.extend_from_slice(&1u32.to_be_bytes());
        raw.extend_from_slice(&1u32.to_be_bytes());
        raw.extend_from_slice(&1u32.to_be_bytes());
        raw.extend_from_slice(&1u32.to_be_bytes());
        raw.extend_from_slice(&1u32.to_be_bytes());
        raw.extend_from_slice(&1u32.to_be_bytes());
        raw.extend_from_slice(&0u16.to_be_bytes()); // unused
        raw.extend_from_slice(&(0b0001u16 << 12).to_be_bytes()); // invalid mode
        raw.extend_from_slice(&0u32.to_be_bytes());
        raw.extend_from_slice(&0u32.to_be_bytes());
        raw.extend_from_slice(&0u32.to_be_bytes());
        raw.extend_from_slice(&[0u8; 20]);
        raw.extend_from_slice(&1u16.to_be_bytes()); // flags/name length
        raw.extend_from_slice(b"x");
        raw.push(0);
        fs::write(repo.gitdir().join("index"), &raw)?;
        let err = GitIndex::read_index(&repo).unwrap_err();
        assert_eq!(err, "Invalid file mode 0b1");
        Ok(())
    }

    #[test]
    fn read_index_extended_flag() -> Result<(), Box<dyn Error>> {
        let temp = TempDir::<()>::create("test_repo");
        let repo = GitRepository::create(temp.tmp_dir())?;
        let mut raw = Vec::new();
        // Header
        raw.extend_from_slice(b"DIRC");
        raw.extend_from_slice(&2u32.to_be_bytes());
        raw.extend_from_slice(&1u32.to_be_bytes());
        // Metadata fields
        raw.extend_from_slice(&1u32.to_be_bytes());
        raw.extend_from_slice(&1u32.to_be_bytes());
        raw.extend_from_slice(&1u32.to_be_bytes());
        raw.extend_from_slice(&1u32.to_be_bytes());
        raw.extend_from_slice(&1u32.to_be_bytes());
        raw.extend_from_slice(&1u32.to_be_bytes());
        raw.extend_from_slice(&0u16.to_be_bytes()); // unused
        raw.extend_from_slice(&(0b1000u16 << 12).to_be_bytes());
        raw.extend_from_slice(&0u32.to_be_bytes());
        raw.extend_from_slice(&0u32.to_be_bytes());
        raw.extend_from_slice(&0u32.to_be_bytes());
        raw.extend_from_slice(&[0u8; 20]); // sha1
        raw.extend_from_slice(&(0x4000u16 | 1).to_be_bytes()); // flags extended + name length
        raw.extend_from_slice(b"x"); // name
        raw.push(0); // null terminator
        fs::write(repo.gitdir().join("index"), &raw)?;
        let err = GitIndex::read_index(&repo).unwrap_err();
        assert_eq!(err, "Extended index files are not supported");
        Ok(())
    }

    #[test]
    fn read_index_name_not_null_terminated() -> Result<(), Box<dyn Error>> {
        let temp = TempDir::<()>::create("test_repo");
        let repo = GitRepository::create(temp.tmp_dir())?;
        let mut raw = Vec::new();
        // Header
        raw.extend_from_slice(b"DIRC");
        raw.extend_from_slice(&2u32.to_be_bytes());
        raw.extend_from_slice(&1u32.to_be_bytes());
        // Metadata fields
        raw.extend_from_slice(&1u32.to_be_bytes());
        raw.extend_from_slice(&1u32.to_be_bytes());
        raw.extend_from_slice(&1u32.to_be_bytes());
        raw.extend_from_slice(&1u32.to_be_bytes());
        raw.extend_from_slice(&1u32.to_be_bytes());
        raw.extend_from_slice(&1u32.to_be_bytes());
        raw.extend_from_slice(&0u16.to_be_bytes()); // unused
        raw.extend_from_slice(&(0b1000u16 << 12).to_be_bytes());
        raw.extend_from_slice(&0u32.to_be_bytes());
        raw.extend_from_slice(&0u32.to_be_bytes());
        raw.extend_from_slice(&0u32.to_be_bytes());
        raw.extend_from_slice(&[0u8; 20]); // sha1
        raw.extend_from_slice(&3u16.to_be_bytes()); // flags/name length = 3
        raw.extend_from_slice(b"foo"); // name without null terminator
        fs::write(repo.gitdir().join("index"), &raw)?;
        let err = GitIndex::read_index(&repo).unwrap_err();
        assert_eq!(
            err,
            "Index file is corrupted, name must be null-terminated"
        );
        Ok(())
    }

    #[test]
    fn read_index_assume_valid_and_stage_flags() -> Result<(), Box<dyn Error>> {
        let temp = TempDir::<()>::create("test_repo");
        let repo = GitRepository::create(temp.tmp_dir())?;
        let mut raw = Vec::new();
        // Header
        raw.extend_from_slice(b"DIRC");
        raw.extend_from_slice(&2u32.to_be_bytes());
        raw.extend_from_slice(&1u32.to_be_bytes());
        // Metadata fields
        raw.extend_from_slice(&0u32.to_be_bytes());
        raw.extend_from_slice(&0u32.to_be_bytes());
        raw.extend_from_slice(&0u32.to_be_bytes());
        raw.extend_from_slice(&0u32.to_be_bytes());
        raw.extend_from_slice(&0u32.to_be_bytes());
        raw.extend_from_slice(&0u32.to_be_bytes());
        raw.extend_from_slice(&0u16.to_be_bytes()); // unused
        raw.extend_from_slice(&(0b1000u16 << 12).to_be_bytes());
        raw.extend_from_slice(&0u32.to_be_bytes());
        raw.extend_from_slice(&0u32.to_be_bytes());
        raw.extend_from_slice(&0u32.to_be_bytes());
        raw.extend_from_slice(&[0u8; 20]); // sha1
        raw.extend_from_slice(&(0x8000u16 | 0x2000 | 1).to_be_bytes()); // flags assume_valid + stage2 + name length
        raw.extend_from_slice(b"a"); // name
        raw.push(0);
        fs::write(repo.gitdir().join("index"), &raw)?;
        let index = GitIndex::read_index(&repo)?;
        let entry = &index.entries()[0];
        assert!(entry.flag_assume_valid);
        assert_eq!(entry.flag_stage, 0x2000);
        Ok(())
    }

    #[test]
    fn read_index_parses_valid_index() -> Result<(), Box<dyn Error>> {
        let temp = TempDir::<()>::create("test_repo");
        let repo = GitRepository::create(temp.tmp_dir())?;

        let mut raw = Vec::new();
        // Header
        raw.extend_from_slice(b"DIRC");
        raw.extend_from_slice(&2u32.to_be_bytes());
        raw.extend_from_slice(&1u32.to_be_bytes());
        // Metadata fields
        raw.extend_from_slice(&1u32.to_be_bytes()); // ctime seconds
        raw.extend_from_slice(&2u32.to_be_bytes()); // ctime nanoseconds
        raw.extend_from_slice(&3u32.to_be_bytes()); // mtime seconds
        raw.extend_from_slice(&4u32.to_be_bytes()); // mtime nanoseconds
        raw.extend_from_slice(&5u32.to_be_bytes()); // device_id
        raw.extend_from_slice(&6u32.to_be_bytes()); // inode
        raw.extend_from_slice(&0u16.to_be_bytes()); // unused
        let mode = (0b1000u16 << 12) | 0o755;
        raw.extend_from_slice(&mode.to_be_bytes()); // mode
        raw.extend_from_slice(&7u32.to_be_bytes()); // uid
        raw.extend_from_slice(&8u32.to_be_bytes()); // gid
        raw.extend_from_slice(&9u32.to_be_bytes()); // file size
        raw.extend_from_slice(&[1u8; 20]); // sha1
        raw.extend_from_slice(&3u16.to_be_bytes()); // flags: name length = 3
        raw.extend_from_slice(b"foo"); // name
        raw.push(0); // null terminator

        fs::write(repo.gitdir().join("index"), &raw)?;

        let index = GitIndex::read_index(&repo)?;
        let entries = index.entries();
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.ctime, (1, 2));
        assert_eq!(e.mtime, (3, 4));
        assert_eq!(e.device_id, 5);
        assert_eq!(e.inode, 6);
        assert_eq!(e.mode_type, 0b1000);
        assert_eq!(e.mode_perms, 0o755);
        assert_eq!(e.uid, 7);
        assert_eq!(e.gid, 8);
        assert_eq!(e.size, 9);
        assert_eq!(e.sha1, [1u8; 20]);
        assert!(!e.flag_assume_valid);
        assert_eq!(e.flag_stage, 0);
        assert_eq!(e.name, "foo");
        Ok(())
    }
}
