#![allow(dead_code, clippy::missing_errors_doc, clippy::missing_panics_doc)]

use std::fs;

use crate::core::GitRepository;
use crate::utils::path::repo_file;
use crate::zlib;

static OBJECTS_DIR: &str = "objects";
static SPACE_BYTE: u8 = b' ';

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub enum GitObject {
    Blob,
    Commit,
    Tag,
    Tree,
}

impl GitObject {
    #[must_use]
    pub fn deserialize(_data: &[u8]) -> Self {
        todo!()
    }

    #[must_use]
    pub fn serialize(&self) -> Vec<u8> {
        todo!()
    }

    pub fn blob_from(_iter: impl Iterator<Item = u8>) -> GitObject {
        todo!()
    }

    pub fn commit_from(_iter: impl Iterator<Item = u8>) -> GitObject {
        todo!()
    }

    pub fn tag_from(_iter: impl Iterator<Item = u8>) -> GitObject {
        todo!()
    }

    pub fn tree_from(_iter: impl Iterator<Item = u8>) -> GitObject {
        todo!()
    }

    pub fn from_iter(
        format: &[u8],
        data_iter: impl Iterator<Item = u8>,
    ) -> Result<GitObject, String> {
        match format {
            b"blob" => Ok(Self::blob_from(data_iter)),
            b"commit" => Ok(Self::commit_from(data_iter)),
            b"tag" => Ok(Self::tag_from(data_iter)),
            b"tree" => Ok(Self::tree_from(data_iter)),
            _ => Err(format!("Unknown format {format:?}")),
        }
    }
}

#[allow(clippy::module_name_repetitions)]
pub fn read_object(
    repo: &GitRepository,
    sha: &str,
) -> Result<GitObject, String> {
    // Calculate the path to the object
    let path =
        repo_file(repo.gitdir(), &[OBJECTS_DIR, &sha[..2], &sha[2..]], false)?;

    // Ensure the path is a valid file
    let path = match path {
        Some(path) if path.is_file() => path,
        _ => return Err(format!("failed to find object with digest {sha}")),
    };

    // Read and decompress the file
    let Ok(raw) = fs::read(path) else {
        return Err(format!("failed to read object with digest {sha}"));
    };
    let raw = zlib::decompress(&raw)?;
    let mut raw_iter = raw.iter();

    // Read the object format
    let Some(space_idx) = raw_iter.position(|&byte| byte == SPACE_BYTE) else {
        return Err(format!(
            "malformed object with digest {sha}, format not specified"
        ));
    };
    let format = raw[..space_idx].to_vec();

    // Read the object size
    let Some(null_idx) = raw_iter.position(|&byte| byte == 0) else {
        return Err(format!(
            "malformed object with digest {sha}, size not specified"
        ));
    };
    let Ok(size) = String::from_utf8(raw[space_idx..null_idx].to_vec()) else {
        return Err(format!(
            "malformed object with digest {sha}, invalid size"
        ));
    };
    let Ok(size) = size.trim().parse::<usize>() else {
        return Err(format!(
            "failed to read size from object with digest {sha}"
        ));
    };

    // Ensure size matches contents
    if size != (raw.len() - null_idx - 1) {
        return Err(format!(
            "malformed object with digest {sha}, size mismatch!"
        ));
    }

    // Create object from data
    GitObject::from_iter(&format, raw.into_iter().skip(null_idx + 1))
}
