use crate::core::objects::traits;

#[derive(Debug)]
pub struct Blob {
    pub(crate) data: Vec<u8>,
}

impl<'a> Blob {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

impl traits::Format for Blob {
    fn format() -> &'static [u8] {
        const FORMAT: &[u8] = b"blob";
        FORMAT
    }
}

impl traits::Serialize for Blob {
    fn serialize(&self) -> Vec<u8> {
        self.data.clone()
    }
}

impl traits::Deserialize for Blob {
    fn deserialize(data: &[u8]) -> Result<Self, String> {
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

impl From<&[u8]> for Blob {
    fn from(data: &[u8]) -> Self {
        Self {
            data: data.to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use traits::*;

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
