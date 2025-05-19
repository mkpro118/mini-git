//! Key Value List with Messages
//!
//! This module provides functionality for parsing and serializing
//! Key Value List with Messages (KVLM) data format.

use crate::utils::collections::ordered_map::OrderedMap;

#[macro_export]
#[expect(clippy::module_name_repetitions)]
macro_rules! kvlm_val_to_string {
    ($kvlm_val:expr) => {
        String::from_utf8($kvlm_val[0].to_vec()).map_err(|e| e.to_string())?
    };
}

#[macro_export]
#[expect(clippy::module_name_repetitions)]
macro_rules! kvlm_msg_to_string {
    ($kvlm_msg:expr) => {
        String::from_utf8($kvlm_msg.to_vec()).map_err(|e| e.to_string())?
    };
}

/// Represents a space byte
pub const SPACE_BYTE: u8 = b' ';
/// Represents a newline byte
pub const NEWLINE_BYTE: u8 = b'\n';

/// Represents a Key Value List with Messages structure
#[derive(Debug)]
pub struct KVLM {
    store: OrderedMap<Keys, Values>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
enum Keys {
    Key(Vec<u8>),
    Message,
}

#[derive(Debug)]
enum Values {
    Value(Vec<Vec<u8>>),
    Message(Vec<u8>),
}

impl Default for KVLM {
    fn default() -> Self {
        Self::new()
    }
}

impl KVLM {
    /// Creates a new, empty KVLM instance
    ///
    /// # Examples
    ///
    /// ```
    /// use mini_git::utils::collections::kvlm::KVLM;
    /// let kvlm = KVLM::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            store: OrderedMap::new(),
        }
    }

    /// Parses the given byte slice into a KVLM instance
    ///
    /// # Arguments
    ///
    /// - `data` - A byte slice containing KVLM formatted data
    ///
    /// # Returns
    ///
    /// - `Ok(KVLM)` if parsing is successful
    /// - `Err(String)` if parsing fails
    ///
    /// # Errors
    /// If the input is not a valid KVLM data.
    ///
    /// # Examples
    ///
    /// ```
    /// use mini_git::utils::collections::kvlm::KVLM;
    /// let data = b"key1 value1\nkey2 value2\n\nMessage content";
    /// let kvlm = KVLM::parse(data).expect("Failed to parse KVLM data");
    /// ```
    pub fn parse(data: &[u8]) -> Result<Self, String> {
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
                if newline_idx != 0 {
                    return Err("malformed KVLM data".to_owned());
                }

                kvlm.store.insert(
                    Keys::Message,
                    Values::Message(data[(start + 1)..].to_vec()),
                );
                return Ok(kvlm);
            }

            let space_idx = space_idx + start;

            let key = Keys::Key(data[start..space_idx].to_vec());
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

            if let Some(v) = kvlm.store.get_mut(&key) {
                let Values::Value(ref mut list) = v else {
                    unreachable!();
                };
                list.push(value);
            } else {
                kvlm.store.insert(key, Values::Value(vec![value]));
            }

            start = end + 1;
        }
    }

    /// Serializes the KVLM instance into a byte vector
    ///
    /// # Returns
    ///
    /// A `Vec<u8>` containing the serialized KVLM data
    ///
    /// # Examples
    ///
    /// ```
    /// use mini_git::utils::collections::kvlm::KVLM;
    /// let kvlm = KVLM::parse(b"key1 value1\nkey2 value2\n\nMessage content").unwrap();
    /// let serialized = kvlm.serialize();
    /// ```
    #[must_use]
    pub fn serialize(&self) -> Vec<u8> {
        let mut res = vec![];

        let items = self.store.iter().filter_map(|(k, v)| match (&k, v) {
            (Keys::Key(key), Values::Value(values)) => Some((key, values)),
            _ => None,
        });

        // Fields
        for (key, values) in items {
            let values = values
                .iter()
                .flat_map(|vec: &Vec<u8>| String::from_utf8(vec.clone()))
                .map(|s| s.replace('\n', "\n "))
                .map(String::into_bytes);
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

    /// Retrieves the values associated with the given key
    ///
    /// # Arguments
    ///
    /// - `key` - A byte slice representing the key to look up
    ///
    /// # Returns
    ///
    /// An `Option` containing a reference to a vector of byte vectors if the key exists,
    /// or `None` if the key doesn't exist
    ///
    /// # Examples
    ///
    /// ```
    /// use mini_git::utils::collections::kvlm::KVLM;
    /// let kvlm = KVLM::parse(b"key1 value1\nkey2 value2\n\nMessage content").unwrap();
    /// if let Some(values) = kvlm.get_key(b"key1") {
    ///     println!("Values for key1: {:?}", values);
    /// }
    /// ```
    #[must_use]
    pub fn get_key(&self, key: &[u8]) -> Option<&Vec<Vec<u8>>> {
        match self.store.get(&Keys::Key(key.to_vec())) {
            Some(Values::Value(ref msg)) => Some(msg),
            _ => None,
        }
    }

    /// Retrieves the message content of the KVLM instance
    ///
    /// # Returns
    ///
    /// An `Option` containing a reference to the message as a byte vector if it exists,
    /// or `None` if there's no message
    ///
    /// # Examples
    ///
    /// ```
    /// use mini_git::utils::collections::kvlm::KVLM;
    /// let kvlm = KVLM::parse(b"key1 value1\nkey2 value2\n\nMessage content").unwrap();
    /// if let Some(message) = kvlm.get_msg() {
    ///     println!("Message: {:?}", String::from_utf8_lossy(message));
    /// }
    /// ```
    #[must_use]
    pub fn get_msg(&self) -> Option<&Vec<u8>> {
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

    enum TestDataKeys {
        Tree,
        Parent,
        Author,
        Committer,
        GPGSig,
        Message,
    }

    use TestDataKeys::*;

    fn test_data_get(key: &TestDataKeys) -> Vec<Vec<u8>> {
        match key {
            Tree => vec![TEST_DATA[1].to_vec()],
            Parent => vec![TEST_DATA[3].to_vec()],
            Author => vec![TEST_DATA[5].to_vec()],
            Committer => vec![TEST_DATA[7].to_vec()],
            Message => vec![TEST_DATA[11].to_vec()],
            GPGSig => vec![
                TEST_DATA[9]
                    .iter()
                    .fold((0, vec![]), |(prev, mut acc), &byte| {
                        if !(prev == NEWLINE_BYTE && byte == SPACE_BYTE) {
                            acc.push(byte);
                        }
                        (byte, acc)
                    })
                    .1,
            ],
        }
    }

    #[test]
    fn test_kvlm_parse() {
        let data = TEST_DATA.concat();
        let kvlm = KVLM::parse(&data).expect("Should parse");

        let exp_tree = test_data_get(&Tree);
        assert_eq!(kvlm.get_key(b"tree"), Some(exp_tree).as_ref());

        let exp_parent = test_data_get(&Parent);
        assert_eq!(kvlm.get_key(b"parent"), Some(exp_parent).as_ref());

        let exp_author = test_data_get(&Author);
        assert_eq!(kvlm.get_key(b"author"), Some(exp_author).as_ref());

        let exp_committer = test_data_get(&Committer);
        assert_eq!(kvlm.get_key(b"committer"), Some(exp_committer).as_ref());

        let exp_gpgsig = test_data_get(&GPGSig);
        assert_eq!(kvlm.get_key(b"gpgsig"), Some(exp_gpgsig).as_ref());

        let exp_msg = &test_data_get(&Message)[0];
        assert_eq!(kvlm.get_msg(), Some(exp_msg));
    }

    #[test]
    fn test_kvlm_serialize() {
        let mut kvlm = KVLM::new();

        // Manually create the test data
        kvlm.store.insert(
            Keys::Key(b"tree".to_vec()),
            Values::Value(test_data_get(&Tree)),
        );

        kvlm.store.insert(
            Keys::Key(b"parent".to_vec()),
            Values::Value(test_data_get(&Parent)),
        );
        kvlm.store.insert(
            Keys::Key(b"author".to_vec()),
            Values::Value(test_data_get(&Author)),
        );
        kvlm.store.insert(
            Keys::Key(b"committer".to_vec()),
            Values::Value(test_data_get(&Committer)),
        );
        kvlm.store.insert(
            Keys::Key(b"gpgsig".to_vec()),
            Values::Value(test_data_get(&GPGSig)),
        );

        let msg = test_data_get(&Message).into_iter().next().unwrap();
        kvlm.store.insert(Keys::Message, Values::Message(msg));

        let serialized = kvlm.serialize();
        let len = serialized.len();

        let combined = TEST_DATA.concat();

        assert_eq!(combined[..len], serialized[..len]);
    }
}
