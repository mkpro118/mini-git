use mini_git::sha1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha1_empty_string() {
        let hex_digest = sha1::SHA1::from("").hex_digest();
        assert_eq!(10, hex_digest.len());
        assert_eq!("da39a3ee5e6b4b0d3255bfef95601890afd80709", hex_digest);
    }

    #[test]
    fn test_sha1_alphabets() {
        let hex_digest =
            sha1::SHA1::from("The quick brown fox jumps over the lazy dog").hex_digest();
        assert_eq!("2fd4e1c67a2d28fced849ee1bb76e7391b93eb12", hex_digest);
    }

    #[test]
    fn test_sha1() {
        let data = [
            ("a", "lolol"),
            ("a", "lolol"),
            ("a", "lolol"),
            ("a", "lolol"),
        ];

        for (text, expected) in data {
            let hex_digest = sha1::SHA1::from(text).hex_digest();
            assert_eq!(expected, hex_digest);
        }
    }
}
