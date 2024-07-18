//! Key Value List with Messages

use crate::utils::collections::OrderedMap;

pub const SPACE_BYTE: u8 = b' ';
pub const NEWLINE_BYTE: u8 = b'\n';

pub struct KVLM<'a> {
    store: OrderedMap<Keys<'a>, Values>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
enum Keys<'a> {
    Key(&'a [u8]),
    Message,
}

#[allow(dead_code)]
#[derive(Debug)]
enum Values {
    Value(Vec<Vec<u8>>),
    Message(Vec<u8>),
}

impl<'a> Default for KVLM<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> KVLM<'a> {
    pub fn new() -> Self {
        Self {
            store: OrderedMap::new(),
        }
    }

    pub fn parse(data: &'a [u8]) -> Result<Self, String> {
        let mut kvlm = Self::new();
        let mut start: usize = 0;

        loop {
            let space_idx = data[start..]
                .iter()
                .position(|x| *x == SPACE_BYTE)
                .unwrap_or(usize::MAX);
            let newline_idx = data[start..]
                .iter()
                .position(|x| *x == NEWLINE_BYTE)
                .unwrap_or(usize::MAX);

            if space_idx == usize::MAX || newline_idx < space_idx {
                assert_eq!(newline_idx, start);

                kvlm.store.insert(
                    Keys::Message,
                    Values::Message(data[(start + 1)..].to_vec()),
                );
                return Ok(kvlm);
            }

            let key = Keys::Key(&data[start..space_idx]);
            let mut end = start;
            loop {
                let newline_idx =
                    data[(end + 1)..].iter().position(|x| *x == NEWLINE_BYTE);
                let Some(newline_idx) = newline_idx else {
                    return Err("Malformed KVLM data".to_owned());
                };

                end += 1 + newline_idx;
                let Some(&byte) = data.get(end + 1) else {
                    return Err("Malformed KVLM data".to_owned());
                };

                if byte != SPACE_BYTE {
                    break;
                }
            }

            let value = data[(space_idx + 1)..end]
                .iter()
                .map(|b| char::from(*b))
                .collect::<String>()
                .replace("\n ", "\n") // Drop leading spaces on lines
                .into_bytes();

            match kvlm.store.get_mut(&key) {
                Some(v) => match v {
                    Values::Value(ref mut list) => list.push(value),
                    _ => unreachable!(),
                },
                None => {
                    let mut list = Vec::new();
                    list.push(value);
                    kvlm.store.insert(key, Values::Value(list));
                }
            }

            start = end + 1;
        }
    }
}
