use crate::core::objects::traits;

const SPACE_BYTE: u8 = b' ';
const NULL_BYTE: u8 = b'\0';
const MODE_SIZE: usize = 6;

#[cfg_attr(test, derive(PartialEq, Eq))]
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

#[cfg(test)]
mod tests {
    use self::traits::Deserialize;

    use super::*;

    fn concat_leaf(leaf: &Leaf) -> Vec<u8> {
        vec![
            match leaf.mode[0] {
                SPACE_BYTE => leaf.mode[1..].to_vec(),
                _ => leaf.mode.to_vec(),
            },
            vec![SPACE_BYTE],
            leaf.path.clone(),
            vec![NULL_BYTE],
            leaf.sha.as_bytes().to_vec(),
        ]
        .concat()
    }

    #[test]
    fn test_leaf_deserializer_good() {
        let mut leaves = [
            Leaf {
                mode: *b"100644",
                path: b"test0".to_vec(),
                sha: "1".repeat(20),
                len: 0,
            },
            Leaf {
                mode: *b" 10644",
                path: b"test1".to_vec(),
                sha: "2".repeat(20),
                len: 0,
            },
            Leaf {
                mode: *b"100644",
                path: b"test2".to_vec(),
                sha: "3".repeat(20),
                len: 0,
            },
        ];

        for test_leaf in &mut leaves {
            let data = concat_leaf(test_leaf);
            test_leaf.len = data.len();

            let leaf = Leaf::deserialize(&data).expect("Should deserialize");

            assert_eq!(leaf, *test_leaf);
        }
    }

    #[test]
    fn test_leaf_deserializer_no_space() {
        let data = [0; 32];
        let res = Leaf::deserialize(&data);
        assert!(res.is_err());
    }

    #[test]
    fn test_leaf_deserializer_no_null() {
        let data = [1, 2, 3, 4, 5, SPACE_BYTE, 10, 20];
        let res = Leaf::deserialize(&data);
        assert!(res.is_err());
    }
}
