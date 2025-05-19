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

#[derive(Default, Clone, Debug)]
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
