use std::sync::Arc;

use crate::session::state::SessionCore;
use crate::session_policy::LootDropRatesLikeCpp;
use wow_core::ObjectGuid;
use wow_loot::{LootStoreKind, LootStores};
use wow_loot::{OwnedLootAuthority, OwnedLootAuthorityStamp};

impl crate::session::state::SessionWorldConfig {
    pub fn enable_ae_loot_like_cpp(&self) -> bool {
        self.enable_ae_loot_like_cpp
    }

    pub fn loot_drop_rates_like_cpp(&self) -> LootDropRatesLikeCpp {
        self.loot_drop_rates
    }
}

impl crate::session::state::SessionCatalogs {
    pub fn represented_gameobject_loot_ids_have_quest_loot_like_cpp(
        &self,
        loot_ids: impl IntoIterator<Item = u32>,
    ) -> bool {
        let Some(stores) = self.loot_stores.as_ref() else {
            return false;
        };
        let Some(store) = stores.get(&LootStoreKind::Gameobject) else {
            return false;
        };
        loot_ids
            .into_iter()
            .filter(|id| *id != 0)
            .any(|loot_id| store.have_quest_loot_for_like_cpp(loot_id, stores.as_ref()))
    }

    /// Get the C++ LootTemplates_* foundation stores.
    pub fn loot_stores(&self) -> Option<&Arc<LootStores>> {
        self.loot_stores.as_ref()
    }
}

impl SessionCore {
    pub fn rebind_legacy_creature_loot_authority_like_cpp(
        &self,
        guid: ObjectGuid,
        expected: &OwnedLootAuthority,
        expected_stamp: OwnedLootAuthorityStamp,
        authority: OwnedLootAuthority,
    ) -> Option<bool> {
        let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
        self.rebind_legacy_creature_loot_authority_on_map_like_cpp(
            guid,
            wow_map::MapKey::new(u32::from(map_id), instance_id),
            expected,
            expected_stamp,
            authority,
        )
    }

    pub fn rebind_legacy_creature_loot_authority_on_map_like_cpp(
        &self,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
        expected: &OwnedLootAuthority,
        expected_stamp: OwnedLootAuthorityStamp,
        authority: OwnedLootAuthority,
    ) -> Option<bool> {
        let map_id = u16::try_from(map_key.map_id).ok()?;
        let manager = self.map_manager.as_ref()?;
        manager
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .find_creature_mut(map_id, map_key.instance_id, guid)
            .and_then(|world_creature| {
                world_creature
                    .creature
                    .rebind_loot_authority_if_current_like_cpp(expected, expected_stamp, authority)
            })
    }
}
