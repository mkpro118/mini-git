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
                assert_eq!(newline_idx, 0);

                kvlm.store.insert(
                    Keys::Message,
                    Values::Message(data[(start + 1)..].to_vec()),
                );
                return Ok(kvlm);
            }

            let space_idx = space_idx + start;

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

    pub fn serialize(&self) -> Vec<u8> {
        let mut res = vec![];

        let items = self.store.iter().filter_map(|(ref k, v)| match (k, v) {
            (Keys::Key(key), Values::Value(values)) => Some((*key, values)),
            _ => None,
        });

        // Fields
        for (key, values) in items {
            let values = values
                .into_iter()
                .map(|vec: &Vec<u8>| String::from_utf8(vec.to_vec()))
                .flatten() // Straight up ignore non-utf stuff
                .map(|s| s.replace("\n", "\n "))
                .map(|s| s.into_bytes());
            for value in values {
                res.extend_from_slice(key);
                res.push(SPACE_BYTE);
                res.extend_from_slice(&value);
                res.push(NEWLINE_BYTE);
            }
        }

        // Message
        let Some(Values::Message(msg)) = self.store.get(&Keys::Message) else {
            unreachable!();
        };

        res.push(NEWLINE_BYTE);
        res.extend_from_slice(msg);
        // res.push(NEWLINE_BYTE);

        res
    }

    pub fn get_key<'b>(&'a self, key: &'b [u8]) -> Option<&'a Vec<Vec<u8>>>
    where
        'b: 'a,
    {
        match self.store.get(&Keys::Key(key)) {
            Some(Values::Value(ref msg)) => Some(msg),
            _ => None,
        }
    }

    pub fn get_msg<'b>(&'a self) -> Option<&'a Vec<u8>>
    where
        'b: 'a,
    {
        match self.store.get(&Keys::Message) {
            Some(Values::Message(ref msg)) => Some(msg),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_DATA: &[&[u8]] = &[
        b"tree ",
        b"c14e2652b22f8b78b29ff16c9b08a5a07930c147",
        b"\nparent ",
        b"aea388a7ae24d49a0206941306e8a8af65b66eaa",
        b"\nauthor ",
        b"John Doe <john@doe.com> 1845675023 +0200",
        b"\ncommitter ",
        b"John Doe <john@doe.com> 1845675044 +0200",
        b"\ngpgsig ",
        b"-----BEGIN PGP SIGNATURE-----
 iQIzBAABCAAdFiEExwXquOM8bWb4Q2zVGxM2FxoLkGQFAlsEjZQACgkQGxM2FxoL
 /obspcvace4wy8uO0bdVhc4nJ+Rla4InVSJaUaBeiHTW8kReSFYyMmDCzLjGIu1q
 kGQdcBAAqPP+ln4nGDd2gETXjvOpOxLzIMEw4A9gU6CzWzm+oB8mEIKyaH0UFIPh
 V75R/7FjSuPLS8NaZF4wfi52btXMSxO/u7GuoJkzJscP3p4qtwe6Rl9dc1XC8P7k
 NIbGZ5Yg5cEPcfmhgXFOhQZkD0yxcJqBUcoFpnp2vu5XJl2E5I/quIyVxUXi6O6c
 3eYgTUKz34cB6tAq9YwHnZpyPx8UJCZGkshpJmgtZ3mCbtQaO17LoihnqPn4UOMr
 Q52UWybBzpaP9HEd4XnR+HuQ4k2K0ns2KgNImsNvIyFwbpMUyUWLMPimaV1DWUXo
 rNUZ1j7/ZGFNeBDtT55LPdPIQw4KKlcf6kC8MPWP3qSu3xHqx12C5zyai2duFZUU
 doU61OM3Zv1ptsLu3gUE6GU27iWYj2RWN3e3HE4Sbd89IFwLXNdSuM0ifDLZk7AQ
 wqOt9iCFCscFQYqKs3xsHI+ncQb+PGjVZA8+jPw7nrPIkeSXQV2aZb1E68wa2YIL
 WBhRhipCCgZhkj9g2NEk7jRVslti1NdN5zoQLaJNqSwO1MtxTmJ15Ksk3QP6kfLB
 5SBjDB/V/W2JBFR+XKHFJeFwYhj7DD/ocsGr4ZMx/lgc8rjIBkI=
 =lgTX
 -----END PGP SIGNATURE-----",
        b"

",
        b"Test data for KVLM
",
    ];

    #[test]
    fn test_kvlm_parse() {
        let data = TEST_DATA.concat();
        let kvlm = KVLM::parse(&data).expect("Should parse");

        let exp_tree = vec![TEST_DATA[1].to_vec()];
        assert_eq!(kvlm.get_key(b"tree"), Some(exp_tree).as_ref());

        let exp_parent = vec![TEST_DATA[3].to_vec()];
        assert_eq!(kvlm.get_key(b"parent"), Some(exp_parent).as_ref());

        let exp_author = vec![TEST_DATA[5].to_vec()];
        assert_eq!(kvlm.get_key(b"author"), Some(exp_author).as_ref());

        let exp_committer = vec![TEST_DATA[7].to_vec()];
        assert_eq!(kvlm.get_key(b"committer"), Some(exp_committer).as_ref());

        let exp_gpgsig = vec![
            TEST_DATA[9]
                .iter()
                .fold((0, vec![]), |(prev, mut acc), &byte| {
                    if !(prev == NEWLINE_BYTE && byte == SPACE_BYTE) {
                        acc.push(byte);
                    }
                    (byte, acc)
                })
                .1,
        ];
        assert_eq!(kvlm.get_key(b"gpgsig"), Some(exp_gpgsig).as_ref());

        let exp_msg = TEST_DATA[11].to_vec();
        assert_eq!(kvlm.get_msg(), Some(exp_msg).as_ref());
    }
}
