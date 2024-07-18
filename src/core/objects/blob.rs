#[derive(Debug)]
pub struct Blob {
    pub(crate) data: Vec<u8>,
}

impl<'a> Blob {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub const fn format() -> &'static [u8] {
        const FORMAT: &[u8] = b"blob";
        FORMAT
    }

    pub fn serialize(&self) -> Vec<u8> {
        self.data.clone()
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, String> {
        Ok(Blob {
            data: Vec::from(data),
        })
    }
}

impl Default for Blob {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blob_serialize() {
        let data = &[0; 16];
        let blob = Blob {
            data: Vec::from(data),
        };
        let serialized = blob.serialize();
        assert_eq!(&serialized, data);
    }

    #[test]
    fn test_blob_deserialize() {
        let data = &[0; 16];
        match Blob::deserialize(data) {
            Ok(Blob { data: inner }) => assert_eq!(inner, data),
            _ => panic!("Deserialize did not return a blob"),
        }
    }
}
