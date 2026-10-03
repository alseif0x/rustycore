use super::LootState;
use wow_core::ObjectGuid;

impl LootState {
    pub fn active_loot_view_owners_snapshot_like_cpp(&self) -> Vec<ObjectGuid> {
        self.active_loot_view_owners.iter().copied().collect()
    }

    pub fn active_loot_guid_like_cpp(&self) -> ObjectGuid {
        self.active_loot_guid
    }

    pub fn has_active_non_item_loot_views_like_cpp(&self) -> bool {
        (!self.active_loot_guid.is_empty() && !self.active_loot_guid.is_item())
            || self
                .active_loot_view_owners
                .iter()
                .any(|guid| !guid.is_item())
    }

    pub fn active_loot_view_owners_is_empty_like_cpp(&self) -> bool {
        self.active_loot_view_owners.is_empty()
    }

    pub fn has_active_loot_view_owner_like_cpp(&self, owner: ObjectGuid) -> bool {
        self.active_loot_view_owners.contains(&owner)
    }

    pub fn active_loot_view_authority_like_cpp(
        &self,
        owner: ObjectGuid,
    ) -> Option<&wow_loot::OwnedLootAuthority> {
        self.active_loot_view_authorities_like_cpp.get(&owner)
    }

    pub fn active_loot_view_authorities_iter_like_cpp(
        &self,
    ) -> impl Iterator<Item = &wow_loot::OwnedLootAuthority> {
        self.active_loot_view_authorities_like_cpp.values()
    }

    pub fn active_loot_view_generation_like_cpp(&self, owner: ObjectGuid) -> Option<&u64> {
        self.active_loot_view_generations_like_cpp.get(&owner)
    }

    pub fn ensure_active_loot_view_generation_like_cpp(
        &mut self,
        owner: ObjectGuid,
        generation: u64,
    ) -> &mut u64 {
        self.active_loot_view_generations_like_cpp
            .entry(owner)
            .or_insert(generation)
    }

    pub fn insert_active_loot_view_authority_if_absent_like_cpp(
        &mut self,
        owner: ObjectGuid,
        authority: &wow_loot::OwnedLootAuthority,
    ) -> &mut wow_loot::OwnedLootAuthority {
        self.active_loot_view_authorities_like_cpp
            .entry(owner)
            .or_insert_with(|| (*authority).clone())
    }

    pub fn set_active_loot_guid(&mut self, guid: ObjectGuid) {
        self.active_loot_guid = ObjectGuid::EMPTY;
        self.active_loot_view_owners.clear();
        self.active_loot_view_generations_like_cpp.clear();
        self.active_loot_view_authorities_like_cpp.clear();
        self.add_active_loot_view_owner_like_cpp(guid);
    }

    pub fn has_active_loot_views_like_cpp(&self) -> bool {
        !self.active_loot_guid.is_empty() || !self.active_loot_view_owners.is_empty()
    }

    pub fn add_active_loot_view_owner_like_cpp(&mut self, guid: ObjectGuid) {
        if guid.is_empty() {
            return;
        }

        if self.active_loot_guid.is_empty() {
            self.active_loot_guid = guid;
        }

        self.active_loot_view_owners.insert(guid);
        if let Some(generation) = self
            .represented_loot_cache_generations_like_cpp
            .get(&guid)
            .copied()
        {
            self.active_loot_view_generations_like_cpp
                .insert(guid, generation);
        }
    }

    pub fn clear_active_loot_guid_if(&mut self, guid: ObjectGuid) {
        self.active_loot_view_owners.remove(&guid);
        self.active_loot_view_generations_like_cpp.remove(&guid);
        self.active_loot_view_authorities_like_cpp.remove(&guid);
        if self.active_loot_guid == guid {
            self.active_loot_guid = ObjectGuid::EMPTY;
        }
    }

    pub fn is_active_loot_guid(&self, guid: ObjectGuid) -> bool {
        !guid.is_empty() && self.active_loot_guid == guid
    }

}
