// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn auto_reply_msg_like_cpp(&self) -> Option<String> {
        self.core.auto_reply_msg_like_cpp()
    }
    #[cfg(test)]
    pub fn set_void_storage_item_id_generator_like_cpp(
        &mut self,
        generator: Arc<VoidStorageItemIdGeneratorLikeCpp>,
    ) {
        self.core
            .set_void_storage_item_id_generator_like_cpp(generator)
    }
    pub(crate) fn realm_list_secret_like_cpp(&self) -> &[u8; 32] {
        self.core.realm_list_secret_like_cpp()
    }
    #[cfg(test)]
    pub fn guid_generator(&self) -> Option<&Arc<ObjectGuidGenerator>> {
        self.core.guid_generator()
    }
}
