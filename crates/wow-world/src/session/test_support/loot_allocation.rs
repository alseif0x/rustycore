//! Optional installed allocator access for durable loot allocation fixtures.

use super::*;

impl WorldSession {
    pub(crate) fn allocate_installed_loot_item_guids_for_test(
        &self,
        count: usize,
    ) -> Option<Vec<(u64, ObjectGuid)>> {
        let generator = self.item_guid_generator_like_cpp.as_deref()?;
        self.allocate_item_instance_guids_with_generator_like_cpp(generator, count)
    }
}
