// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::LootReleaseAccessLikeCpp;
use wow_constants::UnitDynFlags;
use wow_core::ObjectGuid;
use wow_loot::{OwnedLootAuthority, creature_loot_is_allowed_to_player_like_cpp};
use wow_packet::packets::update::UnitDataValuesDeltaUpdate;

/// Chest routing admission borrows the registry at the original read point.
/// No map guard or channel is exposed to the caller.
pub struct ChestLootReleaseRoutingLikeCpp<'a> {
    core: &'a crate::session::SessionCore,
    registry: &'a crate::player_directory::PlayerRegistry,
    player_guid: ObjectGuid,
}

impl ChestLootReleaseRoutingLikeCpp<'_> {
    pub fn queue_like_cpp(
        &self,
        command: crate::session::mailbox::SyncChestGameobjectStateAndRefreshLikeCppCommand,
    ) -> usize {
        let current_map_id = self.core.player_map_id_like_cpp();
        let current_instance_id = self.core.current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id).unwrap_or(0);
        let mut queued = 0;
        for registration in self.registry.same_map_loot_recipients(
            self.player_guid, current_map_id, current_instance_id,
        ) {
            if self.registry.try_send_current_command(
                registration,
                crate::session::mailbox::SessionCommand::SyncChestGameobjectStateAndRefreshLikeCpp(command.clone()),
            ).is_ok() {
                queued += 1;
            }
        }
        queued
    }
}

impl LootReleaseAccessLikeCpp<'_> {
    pub fn chest_routing_like_cpp(&self) -> Option<ChestLootReleaseRoutingLikeCpp<'_>> {
        let player_guid = self.core.player_guid()?;
        let registry = self.core.player_registry()?;
        Some(ChestLootReleaseRoutingLikeCpp {
            core: self.core, registry: registry.as_ref(), player_guid,
        })
    }

    pub fn player_guid_like_cpp(&self) -> Option<ObjectGuid> {
        self.core.player_guid()
    }

    pub fn player_map_id_like_cpp(&self) -> u16 {
        self.core.player_map_id_like_cpp()
    }

    pub fn loot_instance_id_like_cpp(&self) -> u32 {
        self.core.current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id).unwrap_or(0)
    }

    pub fn creature_is_client_visible_like_cpp(&self, guid: ObjectGuid) -> bool {
        self.core.client_visible_guids_like_cpp.contains(&guid)
    }

    pub fn queue_creature_loot_release_values_like_cpp(
        &self,
        player_guid: ObjectGuid,
        creature_guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
        packet_update: &UnitDataValuesDeltaUpdate,
        authority: Option<&OwnedLootAuthority>,
    ) -> usize {
        let Some(registry) = self.core.player_registry() else {
            return 0;
        };
        let recipients = registry.same_map_loot_recipients(player_guid, map_id, instance_id);
        let mut sent = 0;
        for registration in recipients {
            if registry.queue_current_command_reliably(
                registration,
                crate::session::mailbox::SessionCommand::SendCreatureLootReleaseValuesUpdateLikeCpp(
                    crate::session::mailbox::SendCreatureLootReleaseValuesUpdateLikeCppCommand {
                        creature_guid,
                        map_id,
                        instance_id,
                        unit_values_update: packet_update.clone(),
                        authority: authority.cloned(),
                    },
                ),
            ) != crate::session::directory::PlayerDirectoryReliableSendOutcome::StaleOrDisconnected
            {
                sent += 1;
            }
        }
        sent
    }

    pub fn creature_loot_release_values_for_viewer_like_cpp(
        &self,
        creature_guid: ObjectGuid,
        viewer_guid: ObjectGuid,
        viewer_has_pending_bind: bool,
        authority: Option<&OwnedLootAuthority>,
        mut update: UnitDataValuesDeltaUpdate,
    ) -> UnitDataValuesDeltaUpdate {
        let Some(object_data) = update.object_data.as_mut() else {
            return update;
        };
        if object_data.dynamic_flags & UnitDynFlags::Lootable as u32 == 0 {
            return update;
        }
        let Some(authority) = authority else {
            // The authority-less path exists only for bounded unit fixtures.
            // Preserve the canonical flag rather than inventing per-viewer
            // ownership without `Creature::GetLootForPlayer` evidence.
            return update;
        };

        // C++ `ViewerDependentValue<ObjectData::DynamicFlags>` removes
        // UNIT_DYNFLAG_LOOTABLE when the complete `Player::isAllowedToLoot`
        // predicate is false. The object-owned authority is the Rust
        // equivalent of `Creature::GetLootForPlayer`; one exhausted personal
        // pool must not hide a different player's still-live pool.
        let creature_is_dead =
            self.represented_creature_is_dead_for_loot_visibility_like_cpp(creature_guid);
        let viewer_can_still_loot = authority
            .snapshot_for_player_like_cpp(viewer_guid)
            .is_some_and(|snapshot| {
                creature_loot_is_allowed_to_player_like_cpp(
                    creature_is_dead,
                    viewer_has_pending_bind,
                    &snapshot.loot,
                    viewer_guid,
                )
            });
        if !viewer_can_still_loot {
            object_data.dynamic_flags &= !(UnitDynFlags::Lootable as u32);
        }
        update
    }

    fn represented_creature_is_dead_for_loot_visibility_like_cpp(
        &self,
        creature_guid: ObjectGuid,
    ) -> bool {
        let (map_id, instance_id) = self.core.current_legacy_runtime_map_key_like_cpp();
        if let Some(manager) = self.core.map_manager.as_ref()
            && let Some(creature) = manager
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .find_creature(map_id, instance_id, creature_guid)
        {
            return !creature.is_alive();
        }

        let Some(map_key) = self.core
            .canonical_object_lookup_map_key_like_cpp(u32::from(self.core.player_map_id_like_cpp()))
        else {
            return false;
        };
        let Some(manager) = self.core.canonical_map_manager.as_ref() else {
            return false;
        };
        let Ok(manager) = manager.lock() else {
            return false;
        };
        manager
            .find_map(map_key.map_id, map_key.instance_id)
            .and_then(|map| {
                map.map()
                    .creature_transform_vitals_snapshot_like_cpp(creature_guid)
            })
            .is_some_and(|creature| !creature.is_alive)
    }
}
