#[cfg(test)]
mod tests {
    use mini_git::utils::test::walkdir;
    use mini_git::utils::zlib::{compress, compress::Strategy, decompress};
    use std::fs;
    use std::path::Path;

    struct Rng {
        seed: u64,
        multiplier: u64,
        increment: u64,
    }

    impl Rng {
        #[expect(clippy::cast_possible_truncation)]
        pub fn new() -> Self {
            let dur = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap();
            let seed = dur.as_secs();
            let multiplier = (dur.as_millis() & u128::from(u32::MAX)) as u64;
            let increment = (dur.as_nanos() & u128::from(u32::MAX)) as u64;

            Self {
                seed,
                multiplier,
                increment,
            }
        }

        pub fn randint(&mut self) -> u64 {
            self.seed = self
                .seed
                .wrapping_mul(self.multiplier)
                .wrapping_add(self.increment);
            self.seed
        }

        pub fn randbelow(&mut self, limit: u64) -> u64 {
            self.randint() % limit
        }
    }

    fn sources() -> Vec<std::path::PathBuf> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let src = root.join("src").join("utils").join("zlib");
        let mut rng = Rng::new();
        let sources = walkdir(&src);
        sources
            .iter()
            .filter(|_| rng.randbelow(10) >= 8)
            .take(3)
            .cloned()
            .collect()
    }

    #[test]
    fn test_fixed_on_license() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let license = root.join("LICENSE");
        let bytes = fs::read(license).expect("Read file!");

        let compressed = compress(&bytes, &Strategy::Fixed);
        let decompressed =
            decompress(&compressed).expect("Correct decompression");

        assert_eq!(bytes, decompressed);
    }

    #[test]
    fn test_fixed_on_source_files() {
        for file in sources() {
            let bytes = fs::read(file).expect("Read file!");

            let compressed = compress(&bytes, &Strategy::Fixed);
            let decompressed =
                decompress(&compressed).expect("Correct decompression");

            assert_eq!(bytes, decompressed);
        }
    }

    #[test]
    fn test_dynamic_on_license() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let license = root.join("LICENSE");
        let bytes = fs::read(license).expect("Read file!");

        let compressed = compress(&bytes, &Strategy::Dynamic);
        let decompressed =
            decompress(&compressed).expect("Correct decompression");

        assert_eq!(bytes, decompressed);
    }

    #[test]
    fn test_dynamic_on_source_files() {
        for file in sources() {
            let bytes = fs::read(file).expect("Read file!");

            let compressed = compress(&bytes, &Strategy::Dynamic);
            let decompressed =
                decompress(&compressed).expect("Correct decompression");

            assert_eq!(bytes, decompressed);
        }
    }

    #[test]
    fn test_auto_on_license() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let license = root.join("LICENSE");
        let bytes = fs::read(license).expect("Read file!");

        let compressed = compress(&bytes, &Strategy::Auto);
        let decompressed =
            decompress(&compressed).expect("Correct decompression");

        assert_eq!(bytes, decompressed);
    }

    #[test]
    fn test_auto_on_source_files() {
        for file in sources() {
            let bytes = fs::read(file).expect("Read file!");

            let compressed = compress(&bytes, &Strategy::Auto);
            let decompressed =
                decompress(&compressed).expect("Correct decompression");

            assert_eq!(bytes, decompressed);
        }
    }
}
