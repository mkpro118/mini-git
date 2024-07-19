use crate::core::objects::traits;
use crate::utils::collections::kvlm::KVLM;

#[derive(Debug)]
pub struct Tag {
    pub(crate) kvlm: KVLM,
}

impl Tag {
    #[must_use]
    pub fn new() -> Self {
        Self { kvlm: KVLM::new() }
    }
}

impl traits::Format for Tag {
    fn format() -> &'static [u8] {
        const FORMAT: &[u8] = b"tag";
        FORMAT
    }
}

impl traits::KVLM for Tag {
    fn with_kvlm(kvlm: crate::utils::collections::kvlm::KVLM) -> Self {
        Self { kvlm }
    }

    fn kvlm(&self) -> &crate::utils::collections::kvlm::KVLM {
        &self.kvlm
    }
}

impl Default for Tag {
    fn default() -> Self {
        Self::new()
    }
}
