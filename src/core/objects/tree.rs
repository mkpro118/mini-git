use crate::core::objects::traits;

const SPACE_BYTE: u8 = b' ';
const NULL_BYTE: u8 = b'\0';
const MODE_SIZE: usize = 6;

#[derive(Debug)]
struct Leaf {
    mode: [u8; MODE_SIZE],
    path: Vec<u8>,
    sha: String,
    len: usize,
}

#[derive(Debug)]
pub struct Tree {
    leaves: Vec<Leaf>,
}

impl Leaf {
    pub fn len(&self) -> usize {
        self.len
    }
}

impl traits::Deserialize for Leaf {
    fn deserialize(data: &[u8]) -> Result<Self, String> {
        let err = |x| Err(format!("invalid tree leaf: {x}"));
        let Some(space_idx) = data.iter().position(|x| *x == SPACE_BYTE) else {
            return err("mode not found");
        };

        if space_idx < 5 {
            return err("mode is too short");
        } else if space_idx > 6 {
            return err("mode is too long");
        }

        let mode = data[..space_idx].iter().enumerate().fold(
            [SPACE_BYTE; 6],
            |mut acc, (i, byte)| {
                acc[MODE_SIZE - i - 1] = *byte;
                acc
            },
        );

        let path_start_idx = space_idx + 1;

        let Some(null_idx) = data
            .iter()
            .skip(path_start_idx)
            .position(|x| *x == NULL_BYTE)
        else {
            return err("path not found");
        };

        let null_idx = null_idx + path_start_idx;

        let path = data[path_start_idx..null_idx].to_vec();

        if data.len() < null_idx + 21 {
            return err("sha not found");
        }

        let sha = data[(null_idx + 1)..(null_idx + 21)].to_vec();
        let Ok(sha) = String::from_utf8(sha) else {
            return err("could not parse sha");
        };

        Ok(Self {
            mode,
            path,
            sha,
            len: null_idx + 21,
        })
    }
}

impl Tree {
    pub fn new() -> Self {
        Self { leaves: Vec::new() }
    }
}

impl traits::Format for Tree {
    fn format() -> &'static [u8] {
        const FORMAT: &[u8] = b"tree";
        FORMAT
    }
}

impl traits::Serialize for Tree {
    fn serialize(&self) -> Vec<u8> {
        todo!()
    }
}

impl traits::Deserialize for Tree {
    fn deserialize(data: &[u8]) -> Result<Self, String> {
        let mut pos = 0;
        let mut leaves = vec![];
        while pos < data.len() {
            let leaf = Leaf::deserialize(&data[pos..])?;
            pos += leaf.len();
            leaves.push(leaf);
        }

        Ok(Self { leaves })
    }
}

impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
}
