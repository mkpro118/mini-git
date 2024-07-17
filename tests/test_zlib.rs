#![cfg(feature = "test-utils")]

#[cfg(test)]
mod tests {
    use mini_git::utils::test_utils::walkdir;
    use mini_git::zlib::{compress, compress::Strategy, decompress};
    use std::{
        fs,
        path::{Path, PathBuf},
    };

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
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let src = root.join("src");

        for file in walkdir(&src) {
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
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let src = root.join("src");

        for file in walkdir(&src) {
            println!("{file:?}");
            let bytes = fs::read(file).expect("Read file!");

            // Skip files that are too short,
            // dynamic compression doesn't work well with them anyway
            if bytes.len() < 100 {
                continue;
            }

            let compressed = compress(&bytes, &Strategy::Dynamic);
            let decompressed =
                decompress(&compressed).expect("Correct decompression");

            assert_eq!(bytes, decompressed);
        }
    }
}
