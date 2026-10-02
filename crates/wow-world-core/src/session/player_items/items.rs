use crate::session::state::SessionCore;
use wow_core::{ObjectGuid, ObjectGuidGenerator, guid::HighGuid};

impl SessionCore {
    /// Allocate item database/object GUIDs from the process-wide generator.
    ///
    /// C++ initializes this generator once from `MAX(item_instance.guid) + 1`
    /// in `ObjectMgr::SetHighestGuids`, and every `Item::CreateItem` consumes
    /// the next value.  Rust sessions execute concurrently, so the shared
    /// `ObjectGuidGenerator` uses an atomic fetch-add.  Allocations are never
    /// returned after a later persistence failure, matching C++ Item creation.
    pub fn allocate_item_instance_guids_with_generator_like_cpp(
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
}
