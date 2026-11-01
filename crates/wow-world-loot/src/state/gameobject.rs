use super::LootState;
use std::time::Instant;
use wow_core::ObjectGuid;
use wow_world_core::session::HubRef;
use wow_world_core::session::mailbox::{SendIfVisibleLikeCppCommand, SessionCommand};

impl LootState {
    pub fn represented_gameobject_tappers_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> Option<&Vec<ObjectGuid>> {
        self.represented_gameobject_tap_lists.get(&gameobject_guid)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_player_is_locked_to_dungeon_encounter_like_cpp(
        &self,
        player_guid: ObjectGuid,
        dungeon_encounter_id: u32,
    ) -> bool {
        self.represented_locked_dungeon_encounters
            .contains(&(player_guid, dungeon_encounter_id))
    }

    pub fn queue_visible_gameobject_packet_for_same_map_like_cpp(
        &self,
        hub: HubRef<'_>,
        gameobject_guid: ObjectGuid,
        packet_bytes: Vec<u8>,
    ) -> usize {
        let Some(player_guid) = hub.core.player_guid() else {
            return 0;
        };
        let Some(registry) = hub.core.player_registry() else {
            return 0;
        };
        let current_map_id = hub.core.player_map_id_like_cpp();
        let current_instance_id = hub
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let mut queued = 0;

        for registration in
            registry.same_map_loot_recipients(player_guid, current_map_id, current_instance_id)
        {
            if registry
                .try_send_current_command(
                    registration,
                    SessionCommand::SendIfVisibleLikeCpp(SendIfVisibleLikeCppCommand {
                        queued_at: Instant::now(),
                        source_guid: gameobject_guid,
                        map_id: current_map_id,
                        instance_id: current_instance_id,
                        packet_bytes: packet_bytes.clone(),
                    }),
                )
                .is_ok()
            {
                queued += 1;
            }
        }

        queued
    }

    pub fn represented_gathering_node_xp_like_cpp(
        &self,
        hub: HubRef<'_>,
        xp_difficulty: u32,
    ) -> u32 {
        if xp_difficulty == 0 || xp_difficulty >= 10 {
            return 0;
        }

        let xp_store = hub.catalogs.quests.xp_store.as_ref();
        xp_store
            .map(|store| {
                store
                    .player_level_difficulty_xp_like_cpp(hub.player_level_like_cpp(), xp_difficulty)
            })
            .unwrap_or_default()
    }
}

impl LootState {
    pub fn canonical_gameobject_owner_for_loot_like_cpp(
        &self,
        hub: HubRef<'_>,
        guid: ObjectGuid,
    ) -> Option<ObjectGuid> {
        let map_key = hub
            .core
            .canonical_object_lookup_map_key_like_cpp(u32::from(
                hub.core.player_map_id_like_cpp(),
            ))?;
        let manager = hub.core.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        let map = manager.find_map(map_key.map_id, map_key.instance_id)?.map();
        let owner_guid = map.get_typed_game_object(guid)?.owner_guid();
        (!owner_guid.is_empty()).then_some(owner_guid)
    }
}
