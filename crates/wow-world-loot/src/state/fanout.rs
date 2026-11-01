use super::LootState;
use wow_core::ObjectGuid;
use wow_entities::INVENTORY_SLOT_BAG_0;
use wow_loot::{OwnedLootAuthority, OwnedLootScope, OwnedLootSnapshot};
use wow_packet::ServerPacket;
use wow_packet::packets::item::{
    ItemInstance, ItemModList, ItemPushResult, ItemPushResultDisplayType,
};
use wow_packet::packets::loot::{
    CreatureLoot, LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP, LootEntry, LootRemoved, LootResponse,
};
use wow_world_core::session::{HubMut, HubRef};

impl LootState {
    pub fn represented_notify_loot_item_removed_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        owner_guid: ObjectGuid,
        loot_list_id: u8,
    ) {
        let Some(loot) = self.loot_table.get(&owner_guid).cloned() else {
            return;
        };
        let snapshot = OwnedLootSnapshot {
            generation: self
                .represented_loot_cache_generations_like_cpp
                .get(&owner_guid)
                .copied()
                .unwrap_or(0),
            scope: OwnedLootScope::Shared,
            loot,
        };
        self.represented_notify_loot_item_removed_from_snapshot_like_cpp(
            hub,
            owner_guid,
            None,
            &snapshot,
            loot_list_id,
        );
    }

    pub fn represented_notify_loot_item_removed_from_snapshot_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        owner_guid: ObjectGuid,
        authority: Option<&OwnedLootAuthority>,
        snapshot: &OwnedLootSnapshot,
        loot_list_id: u8,
    ) {
        let loot = &snapshot.loot;
        let Some(entry) = loot
            .items
            .iter()
            .find(|entry| entry.loot_list_id == loot_list_id)
        else {
            return;
        };

        let packet = LootRemoved {
            owner: owner_guid,
            loot_obj: loot.loot_guid,
            loot_list_id,
        };
        let bytes = packet.to_bytes();
        let players_looting = loot.players_looting.clone();
        let allowed_looters = entry.allowed_looters.clone();
        let current_player = hub.core.player_guid();
        let current_map = hub.core.player_map_id_like_cpp();
        let current_instance = hub
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let registry = hub.core.player_registry().cloned();
        let mut stale_looters = Vec::new();

        for looter in &players_looting {
            if !allowed_looters.contains(looter) {
                continue;
            }

            if Some(*looter) == current_player {
                hub.core.send_packet(&packet);
                continue;
            }

            let Some(registry) = registry.as_ref() else {
                stale_looters.push(*looter);
                continue;
            };
            let Some(registration) =
                registry.loot_delivery_recipient(*looter, current_map, current_instance)
            else {
                stale_looters.push(*looter);
                continue;
            };
            if registry
                .send_current_packet(registration, bytes.clone())
                .is_err()
            {
                stale_looters.push(*looter);
            }
        }

        if !stale_looters.is_empty()
            && let Some(loot) = self.loot_table.get_mut(&owner_guid)
        {
            loot.players_looting
                .retain(|looter| !stale_looters.contains(looter));
        }
        if !stale_looters.is_empty() {
            if let Some(authority) = authority {
                for looter in stale_looters {
                    let _ =
                        authority.remove_viewer_if_generation_like_cpp(snapshot.generation, looter);
                }
            }
        }
    }

    pub fn send_loot_error_like_cpp(
        &self,
        hub: HubRef<'_>,
        loot_obj: ObjectGuid,
        owner: ObjectGuid,
        error: u8,
    ) {
        hub.core.send_packet(&LootResponse {
            owner,
            loot_obj,
            failure_reason: error,
            acquire_reason: 0,
            loot_method: 0,
            threshold: LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
            coins: 0,
            items: vec![],
            currencies: vec![],
            acquired: false,
            ae_looting: false,
        });
    }

    pub fn send_loot_item_push_result(
        &self,
        hub: HubRef<'_>,
        player_guid: ObjectGuid,
        item_guid: ObjectGuid,
        loot_entry: &LootEntry,
        random_properties_id: i32,
        random_properties_seed: i32,
        slot: u8,
        quantity: u32,
        quantity_in_inventory: u32,
        created: bool,
        dungeon_encounter_id: u32,
    ) {
        let is_encounter_loot = dungeon_encounter_id != 0;
        hub.core.send_packet_realm(&ItemPushResult {
            player_guid,
            slot: u8::from(INVENTORY_SLOT_BAG_0),
            slot_in_bag: i32::from(slot),
            item: ItemInstance {
                item_id: loot_entry.item_id as i32,
                random_properties_seed,
                random_properties_id,
                item_bonus: None,
                modifications: ItemModList { values: Vec::new() },
            },
            quest_log_item_id: 0,
            quantity: quantity as i32,
            quantity_in_inventory: quantity_in_inventory as i32,
            dungeon_encounter_id: dungeon_encounter_id as i32,
            battle_pet_species_id: 0,
            battle_pet_breed_id: 0,
            battle_pet_breed_quality: 0,
            battle_pet_level: 0,
            item_guid,
            pushed: false,
            display_text: if is_encounter_loot {
                ItemPushResultDisplayType::EncounterLoot
            } else {
                ItemPushResultDisplayType::Normal
            },
            created,
            is_bonus_roll: false,
            is_encounter_loot,
        });
    }
}
