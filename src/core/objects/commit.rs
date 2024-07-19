use crate::core::objects::traits;
use crate::utils::collections::kvlm::KVLM;

#[derive(Debug)]
pub struct Commit {
    pub(crate) kvlm: KVLM,
}

impl Commit {
    #[must_use]
    pub fn new() -> Self {
        Self { kvlm: KVLM::new() }
    }
}

impl traits::Format for Commit {
    fn format() -> &'static [u8] {
        const FORMAT: &[u8] = b"commit";
        FORMAT
    }
}

impl traits::KVLM for Commit {
    fn with_kvlm(kvlm: crate::utils::collections::kvlm::KVLM) -> Self {
        Self { kvlm }
    }

    fn kvlm(&self) -> &crate::utils::collections::kvlm::KVLM {
        &self.kvlm
    }
}

impl Default for Commit {
    fn default() -> Self {
        Self::new()
    }
}
