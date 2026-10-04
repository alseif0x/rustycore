// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Selected synchronous map/loot transitions; no guard escapes the operation.

use std::sync::Arc;
use wow_core::ObjectGuid;
use wow_loot::OwnedLootAuthority;
use crate::session::SessionCore;

mod authority;
mod creature;
pub use creature::looted_corpse_decay_secs_like_cpp;
mod corpse;
mod stats;
mod publication;
pub use stats::LootReleaseStatsInputsLikeCpp;

/// Mutable execution owner; selected reborrows end before later release phases.
pub struct LootReleaseOwnerAccessLikeCpp<'a> {
    core: &'a mut SessionCore,
}

impl SessionCore {
    pub fn loot_release_owner_access_like_cpp(&mut self) -> LootReleaseOwnerAccessLikeCpp<'_> {
        LootReleaseOwnerAccessLikeCpp { core: self }
    }
}

impl LootReleaseOwnerAccessLikeCpp<'_> {
    pub fn registry_control_binding_like_cpp(
        &self,
    ) -> Option<crate::session::PlayerRegistryControlBindingLikeCpp<'_>> {
        let guid = self.core.player_guid()?;
        let registry = self.core.player_registry()?;
        Some(self.core.player_registry_control_binding_like_cpp(guid, registry))
    }

    pub fn registry_sync_like_cpp<'a>(
        &'a self,
        #[cfg(any(test, feature = "test-fixtures"))] position: &'a Option<wow_core::Position>,
        #[cfg(any(test, feature = "test-fixtures"))] level: &'a u8,
        #[cfg(any(test, feature = "test-fixtures"))]
        transport: &'a Option<Box<crate::session::PlayerTransportLoginStateLikeCpp>>,
    ) -> crate::session::PlayerRegistrySyncAccessLikeCpp<'a> {
        self.core.player_registry_sync_access_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))] position,
            #[cfg(any(test, feature = "test-fixtures"))] level,
            #[cfg(any(test, feature = "test-fixtures"))] transport,
        )
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn registry_hydration_like_cpp(&self) -> crate::session::PlayerRegistryHydrationAccessLikeCpp<'_> {
        self.core.player_registry_hydration_access_like_cpp()
    }

    pub fn inventory_like_cpp(&self) -> super::OwnedInventoryAccessLikeCpp<'_> {
        self.core.owned_inventory_access_like_cpp()
    }

    pub fn publication_like_cpp(&self) -> crate::session::PacketPublicationAccessLikeCpp<'_> {
        self.core.packet_publication_access_like_cpp()
    }

    pub fn transitions_like_cpp(&self) -> LootReleaseAccessLikeCpp<'_> {
        self.core.loot_release_access_like_cpp()
    }

    pub fn player_guid_like_cpp(&self) -> Option<ObjectGuid> {
        self.core.player_guid()
    }

    pub fn player_map_id_like_cpp(&self) -> u16 {
        self.core.player_map_id_like_cpp()
    }

    /// Resolve the current represented group guid at its original read point.
    pub fn resolved_group_guid_like_cpp(&self) -> Option<u64> {
        self.core
            .player_group_owner_access_like_cpp()
            .canonical_group_guid_like_cpp()
            .flatten()
    }

    /// Deliver one already-built loot-list packet to this owner's other allowed
    /// looters on the current map, mirroring C++ `Loot::NotifyLootList`. The
    /// owner's own delivery stays with the caller's typed publication path.
    pub fn send_loot_list_to_other_allowed_looters_like_cpp(
        &self,
        owner_guid: ObjectGuid,
        allowed_looters: &[ObjectGuid],
        bytes: &[u8],
    ) -> usize {
        let map_id = self.core.player_map_id_like_cpp();
        let instance_id = self
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let mut delivered = 0;
        for &allowed_looter in allowed_looters {
            if allowed_looter == owner_guid {
                continue;
            }
            let Some(registry) = self.core.player_registry() else {
                continue;
            };
            let Some(registration) =
                registry.loot_delivery_recipient(allowed_looter, map_id, instance_id)
            else {
                continue;
            };
            if registry
                .send_current_packet(registration, bytes.to_vec())
                .is_ok()
            {
                delivered += 1;
            }
        }
        delivered
    }

    pub fn retire_client_visible_guid_like_cpp(&mut self, guid: ObjectGuid) -> bool {
        self.core.client_visible_guids_like_cpp.remove(&guid)
    }

    pub fn refresh_owned_loot_summary_like_cpp(&mut self, owner_guid: ObjectGuid) {
        if owner_guid.is_creature_or_vehicle() {
            if let Some(authority) = self.represented_owned_loot_authority_like_cpp(owner_guid) {
                let _ = self.core.rebind_legacy_creature_loot_authority_like_cpp(
                    owner_guid, &authority, authority.stamp_like_cpp(), authority.clone(),
                );
                let authority_stamp = authority.stamp_like_cpp();
                let _ = self.transitions_like_cpp().rebind_canonical_creature_loot_authority_like_cpp(
                    owner_guid, &authority, authority_stamp, authority.clone(),
                );
            }
        } else if owner_guid.is_game_object() {
            if let Some(authority) = self.represented_owned_loot_authority_like_cpp(owner_guid) {
                let _ = self.transitions_like_cpp().rebind_canonical_gameobject_loot_authority_like_cpp(
                    owner_guid, &authority, authority.stamp_like_cpp(), authority.clone(),
                );
            }
        }
    }

    pub fn force_creature_loot_release_dynamic_flags_like_cpp(
        &mut self, guid: ObjectGuid,
    ) -> Option<wow_entities::UnitValuesUpdate> {
        self.core.mutate_world_creature(guid, |creature| {
            creature.force_dynamic_flags_update_like_cpp();
            creature.creature.unit().values_update()
        })
    }
}

pub struct LootReleaseAccessLikeCpp<'a> {
    core: &'a SessionCore,
}

impl SessionCore {
    pub fn loot_release_access_like_cpp(&self) -> LootReleaseAccessLikeCpp<'_> {
        LootReleaseAccessLikeCpp { core: self }
    }
}

impl LootReleaseAccessLikeCpp<'_> {
    pub fn canonical_gameobject_is_fully_looted_like_cpp(&self, guid: ObjectGuid) -> Option<bool> {
        let map_key = self.core.canonical_object_lookup_map_key_like_cpp(
            u32::from(self.core.player_map_id_like_cpp()),
        )?;
        let manager = Arc::clone(self.core.canonical_map_manager.as_ref()?);
        let mut manager = manager.lock().ok()?;
        let managed = manager.find_map_mut(map_key.map_id, map_key.instance_id)?;
        let gameobject = managed.map_mut().get_typed_game_object_mut(guid)?;
        Some(gameobject.is_fully_looted_like_cpp())
    }

    /// AddUse, MaxOpens and SetLootState retain one canonical map critical section.
    pub fn release_canonical_fishing_hole_like_cpp(
        &self,
        guid: ObjectGuid,
        max_opens: Option<u32>,
    ) -> Option<(
        u32,
        wow_entities::LootState,
        wow_map::map::GameObjectSetLootStateOutcomeLikeCpp,
    )> {
        let map_key = self.core
            .canonical_object_lookup_map_key_like_cpp(u32::from(
                self.core.player_map_id_like_cpp(),
            ))?;
        let game_time_secs = i64::try_from(wow_core::GameTime::now().as_secs()).unwrap_or(i64::MAX);
        let manager = Arc::clone(self.core.canonical_map_manager.as_ref()?);
        let mut manager = manager.lock().ok()?;
        let managed = manager.find_map_mut(map_key.map_id, map_key.instance_id)?;
        let map = managed.map_mut();
        let use_count = {
            let gameobject = map.get_typed_game_object_mut(guid)?;
            gameobject.add_use_like_cpp();
            gameobject.use_times()
        };
        let loot_state = if max_opens.is_some_and(|max_opens| use_count >= max_opens) {
            wow_entities::LootState::JustDeactivated
        } else {
            wow_entities::LootState::Ready
        };
        let outcome = map.set_gameobject_loot_state_like_cpp(
            guid,
            loot_state,
            None,
            game_time_secs,
            0,
            false,
        );
        Some((use_count, loot_state, outcome))
    }

    pub fn set_canonical_gameobject_loot_state_like_cpp(
        &self,
        guid: ObjectGuid,
        state: wow_entities::LootState,
        unit_guid: Option<ObjectGuid>,
        chest_restock_time_secs: u32,
        shared_loot_is_changed_like_cpp: bool,
    ) -> Option<wow_map::map::GameObjectSetLootStateOutcomeLikeCpp> {
        let map_key = self.core
            .canonical_object_lookup_map_key_like_cpp(u32::from(
                self.core.player_map_id_like_cpp(),
            ))?;
        let game_time_secs = i64::try_from(wow_core::GameTime::now().as_secs()).unwrap_or(i64::MAX);
        let manager = Arc::clone(self.core.canonical_map_manager.as_ref()?);
        let mut manager = manager.lock().ok()?;
        let managed = manager.find_map_mut(map_key.map_id, map_key.instance_id)?;
        Some(managed.map_mut().set_gameobject_loot_state_like_cpp(
            guid,
            state,
            unit_guid,
            game_time_secs,
            chest_restock_time_secs,
            shared_loot_is_changed_like_cpp,
        ))
    }

    /// Applies the global fully-looted transition only if the exact authority
    /// generation and pool topology observed by `DoLootRelease` are still
    /// current. The canonical map lock is acquired before the authority lock,
    /// matching personal-loot upsert order and making check+state mutation one
    /// C++-serialized operation.
    pub fn set_canonical_gameobject_loot_state_if_fully_looted_observation_like_cpp(
        &self,
        guid: ObjectGuid,
        authority: &OwnedLootAuthority,
        object_generation: u64,
        lifecycle_revision: u64,
        state: wow_entities::LootState,
        unit_guid: Option<ObjectGuid>,
        chest_restock_time_secs: u32,
        shared_loot_is_changed_like_cpp: bool,
    ) -> Option<wow_map::map::GameObjectSetLootStateOutcomeLikeCpp> {
        let map_key = self.core
            .canonical_object_lookup_map_key_like_cpp(u32::from(
                self.core.player_map_id_like_cpp(),
            ))?;
        let game_time_secs = i64::try_from(wow_core::GameTime::now().as_secs()).unwrap_or(i64::MAX);
        let manager = Arc::clone(self.core.canonical_map_manager.as_ref()?);
        let mut manager = manager.lock().ok()?;
        let managed = manager.find_map_mut(map_key.map_id, map_key.instance_id)?;
        let object_authority = managed
            .map()
            .get_typed_game_object(guid)?
            .loot_authority_like_cpp()
            .clone();
        if !object_authority.shares_storage_like_cpp(authority) {
            return None;
        }

        authority.with_fully_looted_lifecycle_observation_like_cpp(
            object_generation,
            lifecycle_revision,
            || {
                managed.map_mut().set_gameobject_loot_state_like_cpp(
                    guid,
                    state,
                    unit_guid,
                    game_time_secs,
                    chest_restock_time_secs,
                    shared_loot_is_changed_like_cpp,
                )
            },
        )
    }

    /// Detached durable-claim completion may transition the object only when
    /// no client still has any shared or personal loot pool open. The final
    /// viewer check and map mutation are serialized under the authority lock.
    pub fn set_canonical_gameobject_loot_state_if_unviewed_fully_looted_observation_like_cpp(
        &self,
        guid: ObjectGuid,
        authority: &OwnedLootAuthority,
        object_generation: u64,
        lifecycle_revision: u64,
        state: wow_entities::LootState,
        unit_guid: Option<ObjectGuid>,
        chest_restock_time_secs: u32,
        shared_loot_is_changed_like_cpp: bool,
    ) -> Option<wow_map::map::GameObjectSetLootStateOutcomeLikeCpp> {
        let map_key = self.core
            .canonical_object_lookup_map_key_like_cpp(u32::from(
                self.core.player_map_id_like_cpp(),
            ))?;
        let game_time_secs = i64::try_from(wow_core::GameTime::now().as_secs()).unwrap_or(i64::MAX);
        let manager = Arc::clone(self.core.canonical_map_manager.as_ref()?);
        let mut manager = manager.lock().ok()?;
        let managed = manager.find_map_mut(map_key.map_id, map_key.instance_id)?;
        let object_authority = managed
            .map()
            .get_typed_game_object(guid)?
            .loot_authority_like_cpp()
            .clone();
        if !object_authority.shares_storage_like_cpp(authority) {
            return None;
        }

        authority.with_unviewed_fully_looted_lifecycle_observation_like_cpp(
            object_generation,
            lifecycle_revision,
            || {
                managed.map_mut().set_gameobject_loot_state_like_cpp(
                    guid,
                    state,
                    unit_guid,
                    game_time_secs,
                    chest_restock_time_secs,
                    shared_loot_is_changed_like_cpp,
                )
            },
        )
    }
}
