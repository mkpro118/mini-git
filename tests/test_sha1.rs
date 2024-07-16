use mini_git::utils::sha1::SHA1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_string() {
        let mut sha1 = SHA1::new();
        let sha1 = sha1.update(b"");
        assert_eq!(
            &sha1.hex_digest(),
            "da39a3ee5e6b4b0d3255bfef95601890afd80709"
        );
    }

    #[test]
    fn test_fox() {
        let mut sha1 = SHA1::new();
        let sha1 = sha1.update(b"The quick brown fox jumps over the lazy dog");
        assert_eq!(
            sha1.hex_digest(),
            "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12"
        );
    }

    #[test]
    fn test_rust() {
        let mut sha1 = SHA1::new();
        let sha1 = sha1.update(b"Rust");
        assert_eq!(
            sha1.hex_digest(),
            "e2ae20d9ae7fcacb605c03c198e0a1c51d446f50"
        );
    }

    #[test]
    fn test_many() {
        let data = [
            (
                "A stitch in time saves nine",
                "7db97211c6b112ae3fc4c923d92c9724edb0db3d",
            ),
            (
                "Beneath the surface",
                "96d02cf04003442728df510b20772bf39bee3c2d",
            ),
            (
                "Chasing shadows in the dark",
                "1246fe83810166a690a4c5a836ba75cb71a5333f",
            ),
            (
                "Dream big, aim high",
                "ea60645034e77f647a6964c97f91274575c9815a",
            ),
            (
                "Eternal sunshine",
                "f9aafd8ea56f097a0cf5da758e7c399a3a82fcf1",
            ),
            (
                "Far beyond the horizon",
                "0b100cfb2dc2453e9307603ec99f3ac78e21f87f",
            ),
            (
                "Glimpse of the unknown",
                "2c5228be3b17a6194723f7aedbd882224dd740f1",
            ),
            (
                "Hope springs eternal",
                "c5ba5960e926a0a8c5fdc55e4153f8ff0df7905f",
            ),
            (
                "In the blink of an eye",
                "482a1719f79486d792fb861ae7cb31fe8ce6e7ee",
            ),
            (
                "Journey to the stars",
                "9997a959c9d4a79b9eced2e8b6316496d47ed945",
            ),
            (
                "Keep it simple, silly",
                "01cac2d0668b1091ed14249500802216e7fff6be",
            ),
            (
                "Lost in the moment",
                "d15fde3d753f834194d1655f1f1379e9fff9b0ce",
            ),
            (
                "Make waves, not ripples",
                "a686dcac3dfcd8d476b51b38c82be96a4946d9c8",
            ),
            (
                "Never give up on dreams",
                "edb547c25d3e060ce9ebcf1e1410fd0a01e85bf6",
            ),
            (
                "Open the door to possibilities",
                "2e982c2ea25990206a60e0564dd4b1db6c5f38eb",
            ),
        ];

        for x in data {
            let mut sha1 = SHA1::new();
            let sha1 = sha1.update(x.0.as_bytes());
            assert_eq!(sha1.hex_digest(), x.1);
        }
    }
}
