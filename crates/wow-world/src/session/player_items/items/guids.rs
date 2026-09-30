//! guids for the existing items owner.

use super::*;

impl WorldSession {
    /// Install the process-wide C++
    /// `sObjectMgr->GetGenerator<HighGuid::Item>()` mirror.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_item_guid_generator_like_cpp(&mut self, generator: Arc<ObjectGuidGenerator>) {
        assert_eq!(
            generator.high_guid(),
            HighGuid::Item,
            "item GUID allocator must use HighGuid::Item"
        );
        self.item_guid_generator_like_cpp = Some(generator);
    }
    #[cfg(test)]
    pub(crate) fn item_guid_generator_like_cpp_for_bridge(
        &self,
    ) -> Option<Arc<ObjectGuidGenerator>> {
        self.item_guid_generator_like_cpp.clone()
    }
    /// Allocate item database/object GUIDs from the process-wide generator.
    ///
    /// C++ initializes this generator once from `MAX(item_instance.guid) + 1`
    /// in `ObjectMgr::SetHighestGuids`, and every `Item::CreateItem` consumes
    /// the next value.  Rust sessions execute concurrently, so the shared
    /// `ObjectGuidGenerator` uses an atomic fetch-add.  Allocations are never
    /// returned after a later persistence failure, matching C++ Item creation.
    pub(crate) fn allocate_item_instance_guids_with_generator_like_cpp(
        &self,
        generator: &ObjectGuidGenerator,
        count: usize,
    ) -> Option<Vec<(u64, ObjectGuid)>> {
        if count == 0 {
            return Some(Vec::new());
        }
        if generator.high_guid() != HighGuid::Item {
            return None;
        }

        let realm_id = self.realm_id();
        (0..count)
            .map(|_| {
                let counter = generator.generate();
                let db_guid = u64::try_from(counter).ok()?;
                Some((db_guid, ObjectGuid::create_item(realm_id, counter)))
            })
            .collect()
    }
    #[cfg(test)]
    pub(crate) fn allocate_item_instance_guids_like_cpp(
        &self,
        count: usize,
    ) -> Option<Vec<(u64, ObjectGuid)>> {
        let generator = self.item_guid_generator_like_cpp.as_deref()?;
        self.allocate_item_instance_guids_with_generator_like_cpp(generator, count)
    }
}
