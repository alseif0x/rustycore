// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Owner release orchestration. Kept private until its selected dependencies close.

use wow_core::ObjectGuid;
use wow_packet::packets::loot::SLootRelease;
use wow_packet::packets::loot::{LOOT_TYPE_PROSPECTING_LIKE_CPP, LOOT_TYPE_MILLING_LIKE_CPP};
use wow_packet::packets::loot::LootList;
use wow_entities::ItemObjectUpdateLikeCpp;
use wow_loot::{
    CreatureLoot, LOOT_METHOD_MASTER_LIKE_CPP, OwnedLootAuthority,
    loot_has_over_threshold_item_like_cpp, loot_is_looted_like_cpp,
};
use wow_entities::{GAMEOBJECT_TYPE_CHEST, GAMEOBJECT_TYPE_FISHING_NODE, GAMEOBJECT_TYPE_FISHING_HOLE, GAMEOBJECT_TYPE_GATHERING_NODE};

#[derive(Clone)]
pub struct AuthoritativeLootReleaseLikeCpp {
    pub authority: OwnedLootAuthority,
    pub selected_generation: u64,
    pub loot: CreatureLoot,
    pub whole_object_fully_looted: bool,
    pub whole_object_fully_skinned: bool,
    pub object_generation: u64,
    pub lifecycle_revision: u64,
    pub require_no_viewers: bool,
}

mod item;
mod gameobject;
mod creature;
mod publication;
mod authority;
mod registry;
#[cfg(any(test, feature = "test-fixtures"))]
pub use registry::LootReleaseRegistryFixtureRefsLikeCpp;

pub struct LootReleaseCxLikeCpp<'a> {
    owner: wow_world_core::session::LootReleaseOwnerAccessLikeCpp<'a>,
    loot: &'a mut wow_world_loot::LootState,
    world_entities: &'a mut wow_world_entities::WorldEntitiesState,
    inventory: &'a mut wow_world_inventory::InventoryState,
    lifecycle: &'a mut wow_world_lifecycle::SessionLifecycleState,
    consumer_test: bool,
    instances: &'a wow_world_instances::InstanceState,
    stats_inputs: wow_world_core::session::LootReleaseStatsInputsLikeCpp<'a>,
    item_store: Option<&'a std::sync::Arc<wow_data::ItemStore>>,
    item_stats_store: Option<&'a std::sync::Arc<wow_data::ItemStatsStore>>,
    #[cfg(any(test, feature = "test-fixtures"))]
    registry_fixtures: LootReleaseRegistryFixtureRefsLikeCpp<'a>,
}

impl<'a> LootReleaseCxLikeCpp<'a> {
    pub fn new(
        owner: wow_world_core::session::LootReleaseOwnerAccessLikeCpp<'a>,
        loot: &'a mut wow_world_loot::LootState,
        world_entities: &'a mut wow_world_entities::WorldEntitiesState,
        inventory: &'a mut wow_world_inventory::InventoryState,
        lifecycle: &'a mut wow_world_lifecycle::SessionLifecycleState,
        consumer_test: bool,
        instances: &'a wow_world_instances::InstanceState,
        stats_inputs: wow_world_core::session::LootReleaseStatsInputsLikeCpp<'a>,
        item_store: Option<&'a std::sync::Arc<wow_data::ItemStore>>,
        item_stats_store: Option<&'a std::sync::Arc<wow_data::ItemStatsStore>>,
        #[cfg(any(test, feature = "test-fixtures"))]
        registry_fixtures: LootReleaseRegistryFixtureRefsLikeCpp<'a>,
    ) -> Self {
        Self { owner, loot, world_entities, inventory, lifecycle, consumer_test, instances, stats_inputs, item_store, item_stats_store,
            #[cfg(any(test, feature = "test-fixtures"))] registry_fixtures }
    }

    fn player_guid(&self) -> Option<ObjectGuid> {
        self.owner.player_guid_like_cpp()
    }

    fn send_packet<P: wow_packet::ServerPacket>(&self, packet: &P) -> bool {
        self.owner.publication_like_cpp().send_packet(packet)
    }

    fn resolved_inventory_items_like_cpp(&self) -> Option<std::collections::HashMap<u8, wow_entities::PlayerInventoryItem>> {
        self.inventory.resolved_inventory_items_with_access_like_cpp(&self.owner.inventory_like_cpp())
    }

    fn item_template_flags(&self, item_id: u32) -> Option<wow_constants::ItemFlags> {
        self.item_stats_store.and_then(|store| store.item_flags(item_id))
    }

    fn resolved_inventory_item_object_like_cpp(&self, guid: ObjectGuid) -> Option<wow_entities::Item> {
        self.inventory.resolved_player_inventory_item_object_with_access_like_cpp(&self.owner.inventory_like_cpp(), guid)
    }

    fn get_inventory_item_by_pos(&self, bag: u8, slot: u8) -> Option<wow_entities::PlayerInventoryItem> {
        self.inventory.get_inventory_item_by_pos_with_access_like_cpp(
            &self.owner.inventory_like_cpp(), self.item_store, self.item_stats_store, bag, slot,
        )
    }

    fn apply_inventory_item_object_updates_like_cpp(&mut self, guid: ObjectGuid, updates: &[ItemObjectUpdateLikeCpp]) -> bool {
        self.inventory.apply_inventory_item_object_updates_with_access_like_cpp(
            &self.owner.inventory_like_cpp(), guid, updates,
        )
    }

    fn send_stat_update(&mut self) {
        let mut stats = crate::CharacterStatsApplicationCxLikeCpp::new(
            self.owner.stats_like_cpp(&mut self.stats_inputs), self.inventory,
            self.owner.publication_like_cpp(),
        );
        stats.send_stat_update_like_cpp();
    }

    fn send_player_values_update_from_entity_bridge(
        &self,
        inv_slot_changes: &[(u8, ObjectGuid)],
        visible_item_changes: &[(u8, i32, u16, u16)],
        virtual_item_changes: &[(u8, i32, u16, u16)],
        buyback_changes: &[(u8, u32, i64)],
        coinage: Option<u64>,
    ) {
        self.inventory.send_player_values_update_from_entity_bridge_with_access_like_cpp(
            &self.owner.inventory_like_cpp(), &self.owner.publication_like_cpp(),
            self.item_store, self.item_stats_store, inv_slot_changes, visible_item_changes,
            virtual_item_changes, buyback_changes, coinage,
        );
    }

    fn close_stale_active_loot_view_like_cpp(
        &mut self, owner_guid: ObjectGuid, player_guid: ObjectGuid,
    ) {
        self.loot.discard_represented_personal_loot_cache_for_player_like_cpp(owner_guid, player_guid);
        self.owner.publication_like_cpp().send_packet(&SLootRelease {
            loot_obj: owner_guid,
            owner: player_guid,
        });
        self.loot.clear_active_loot_guid_if(owner_guid);
    }

    fn send_creature_loot_release_dynamic_flags_update_like_cpp(
        &self, guid: ObjectGuid, update: &wow_entities::UnitValuesUpdate,
        authority: Option<&OwnedLootAuthority>,
    ) -> usize {
        publication::LootReleasePublicationCxLikeCpp::new(
            self.owner.transitions_like_cpp(), self.owner.publication_like_cpp(), self.instances,
        ).send_creature_loot_release_dynamic_flags_update_like_cpp(guid, update, authority)
    }

    /// C++ `Loot::NotifyLootList`: only grouped owners notify, the owner's own
    /// session publishes its typed packet and the registry delivers the same
    /// bytes to every other allowed looter on this map.
    fn represented_notify_loot_list_like_cpp(&self, owner_guid: ObjectGuid) {
        if self.owner.resolved_group_guid_like_cpp().is_none() {
            return;
        }
        let Some(loot) = self.loot.cached_loot_for_owner_like_cpp(owner_guid) else {
            return;
        };
        let master = if loot.loot_method == LOOT_METHOD_MASTER_LIKE_CPP
            && loot_has_over_threshold_item_like_cpp(loot)
        {
            (!loot.loot_master.is_empty()).then_some(loot.loot_master)
        } else {
            None
        };
        let packet = LootList {
            owner: owner_guid,
            loot_obj: loot.loot_guid,
            master,
            round_robin_winner: (!loot.round_robin_player.is_empty())
                .then_some(loot.round_robin_player),
        };
        let bytes = packet.to_bytes();
        if self.owner.player_guid_like_cpp() == Some(owner_guid) {
            self.send_packet(&packet);
        }
        let _ = self.owner.send_loot_list_to_other_allowed_looters_like_cpp(
            owner_guid,
            &loot.allowed_looters,
            &bytes,
        );
    }
}

impl LootReleaseCxLikeCpp<'_> {
    pub async fn release_all_like_cpp(&mut self, player_guid: ObjectGuid) {
        let mut owners = self.loot.active_loot_view_owners_snapshot_like_cpp();
        if owners.is_empty() && !self.loot.active_loot_guid_like_cpp().is_empty() {
            owners.push(self.loot.active_loot_guid_like_cpp());
        }
        owners.sort_by_key(|guid| (guid.high_value(), guid.low_value()));
        for owner in owners {
            self.release_owner_like_cpp(owner, player_guid).await;
        }
    }

    pub async fn release_owner_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> bool {
        if !self.loot.has_active_loot_view_owner_like_cpp(owner_guid)
            && !self.loot.is_active_loot_guid(owner_guid)
        {
            return false;
        }

        let authoritative_release = if let Some(authority) =
            self.prepare_owned_loot_authority_for_active_request_like_cpp(owner_guid, player_guid)
        {
            if !self
                .loot
                .active_loot_view_authority_like_cpp(owner_guid)
                .is_some_and(|opened| opened.shares_storage_like_cpp(&authority))
            {
                self.close_stale_active_loot_view_like_cpp(owner_guid, player_guid);
                return true;
            }
            let Some(active_generation) = self
                .loot
                .active_loot_view_generation_like_cpp(owner_guid)
                .copied()
            else {
                self.close_stale_active_loot_view_like_cpp(owner_guid, player_guid);
                return true;
            };
            let Some(close) =
                authority.close_viewer_if_generation_like_cpp(active_generation, player_guid)
            else {
                self.close_stale_active_loot_view_like_cpp(owner_guid, player_guid);
                return true;
            };
            Some(AuthoritativeLootReleaseLikeCpp {
                authority,
                selected_generation: active_generation,
                loot: close.snapshot.loot,
                whole_object_fully_looted: close.whole_object_fully_looted,
                whole_object_fully_skinned: close.whole_object_fully_skinned,
                object_generation: close.object_generation,
                lifecycle_revision: close.lifecycle_revision,
                require_no_viewers: false,
            })
        } else {
            None
        };

        if authoritative_release.is_none()
            && (owner_guid.is_creature_or_vehicle() || owner_guid.is_game_object())
            && !self.consumer_test
        {
            self.close_stale_active_loot_view_like_cpp(owner_guid, player_guid);
            return true;
        }

        // C++ `Loot::isLooted()` requires both zero gold and zero remaining
        // player-visible item count.
        let Some(loot) = authoritative_release
            .as_ref()
            .map(|release| &release.loot)
            .or_else(|| self.loot.cached_loot_for_owner_like_cpp(owner_guid))
        else {
            return false;
        };
        let selected_pool_looted = loot_is_looted_like_cpp(loot);
        let represented_loot_type = loot.loot_type;
        let whole_object_fully_looted = if let Some(release) = authoritative_release.as_ref() {
            release.whole_object_fully_looted
        } else if owner_guid.is_game_object() {
            self.canonical_gameobject_fully_looted_after_represented_sync_like_cpp(
                owner_guid,
                player_guid,
                selected_pool_looted,
            )
        } else if owner_guid.is_creature_or_vehicle() {
            self.canonical_creature_fully_looted_after_represented_sync_like_cpp(
                owner_guid,
                player_guid,
                selected_pool_looted,
            )
        } else {
            selected_pool_looted
        };

        if let Some(loot) = self.loot.cached_loot_for_owner_mut_like_cpp(owner_guid) {
            loot.players_looting.retain(|looter| *looter != player_guid);
        }

        // Acknowledge the release to the client.
        let release = SLootRelease {
            loot_obj: owner_guid,
            owner: player_guid,
        };
        self.send_packet(&release);

        if owner_guid.is_game_object() {
            self.loot.clear_active_loot_guid_if(owner_guid);
            if !self
                .represented_gameobject_can_autostore_loot_item_like_cpp(owner_guid, player_guid)
            {
                if authoritative_release.is_some() {
                    self.loot
                        .discard_represented_personal_loot_cache_for_player_like_cpp(
                            owner_guid,
                            player_guid,
                        );
                }
                return true;
            }
            self.apply_represented_gameobject_loot_release_like_cpp(
                owner_guid,
                player_guid,
                selected_pool_looted,
                whole_object_fully_looted,
                authoritative_release.as_ref(),
            );
            let _ = self.queue_chest_gameobject_state_refresh_for_same_map_like_cpp(owner_guid);
            let go_type = self
                .world_entities
                .represented_gameobject_use_state_like_cpp(owner_guid)
                .and_then(|state| state.go_type)
                .map(u32::from);
            let selected_release_branch = selected_pool_looted
                || matches!(
                    go_type,
                    Some(GAMEOBJECT_TYPE_FISHING_NODE) | Some(GAMEOBJECT_TYPE_FISHING_HOLE)
                );
            if !selected_release_branch {
                if authoritative_release.is_some() {
                    self.loot
                        .discard_represented_personal_loot_cache_for_player_like_cpp(
                            owner_guid,
                            player_guid,
                        );
                }
                return true;
            }

            self.hide_represented_gameobject_for_player_after_loot_release_like_cpp(owner_guid);
            if go_type == Some(GAMEOBJECT_TYPE_GATHERING_NODE) {
                self.send_gathering_node_loot_release_dynamic_flags_update_like_cpp(owner_guid);
            }
            if authoritative_release.is_some() {
                self.loot
                    .discard_represented_personal_loot_cache_for_player_like_cpp(
                        owner_guid,
                        player_guid,
                    );
            } else {
                self.loot.remove_cached_loot_for_owner_like_cpp(owner_guid);
            }
            return true;
        }

        if owner_guid.is_item()
            && matches!(
                represented_loot_type,
                LOOT_TYPE_PROSPECTING_LIKE_CPP | LOOT_TYPE_MILLING_LIKE_CPP
            )
        {
            // C++ always clears the generated Loot and consumes at most five
            // source items for prospecting/milling, even if the window closes
            // before every generated entry was taken.
            self.loot.clear_active_loot_guid_if(owner_guid);
            self.loot.remove_cached_loot_for_owner_like_cpp(owner_guid);
            let _ = self.apply_inventory_item_object_updates_like_cpp(
                owner_guid,
                &[ItemObjectUpdateLikeCpp::SetLootGenerated(false)],
            );
            self.destroy_direct_item_count_after_loot_release_like_cpp(owner_guid, Some(5))
                .await;
            return true;
        }

        if owner_guid.is_item() && !selected_pool_looted {
            self.loot.clear_active_loot_guid_if(owner_guid);
            let item_has_loot_flag = self
                .resolved_inventory_items_like_cpp()
                .and_then(|items| items.values().find(|item| item.guid == owner_guid).cloned())
                .and_then(|item| self.item_template_flags(item.entry_id))
                .map(|flags| flags.contains(wow_constants::ItemFlags::HAS_LOOT));
            if item_has_loot_flag == Some(false) {
                self.destroy_fully_looted_direct_item(owner_guid).await;
            }
            return true;
        }

        self.loot.clear_active_loot_guid_if(owner_guid);

        if !selected_pool_looted {
            let round_robin_released = if let Some(release) = authoritative_release.as_ref() {
                release
                    .authority
                    .clear_round_robin_if_generation_like_cpp(
                        release.selected_generation,
                        player_guid,
                    )
                    .is_some_and(|outcome| {
                        self.loot
                            .insert_cached_loot_for_owner_like_cpp(
                                owner_guid,
                                outcome.snapshot.loot,
                            );
                        outcome.cleared
                    })
            } else {
                self.loot
                    .cached_loot_for_owner_mut_like_cpp(owner_guid)
                    .is_some_and(|loot| {
                        if loot.round_robin_player == player_guid {
                            loot.round_robin_player = ObjectGuid::EMPTY;
                            true
                        } else {
                            false
                        }
                    })
            };
            if round_robin_released {
                self.represented_notify_loot_list_like_cpp(owner_guid);
            }
            if owner_guid.is_creature_or_vehicle() {
                let values_update = self.owner.force_creature_loot_release_dynamic_flags_like_cpp(owner_guid);
                if let Some(values_update) = values_update.as_ref() {
                    self.send_creature_loot_release_dynamic_flags_update_like_cpp(
                        owner_guid,
                        values_update,
                        authoritative_release
                            .as_ref()
                            .map(|release| &release.authority),
                    );
                }
            }
            if authoritative_release.is_some() {
                self.loot
                    .discard_represented_personal_loot_cache_for_player_like_cpp(
                        owner_guid,
                        player_guid,
                    );
            }
            return true;
        }

        // Remove loot entry from memory once the represented loot is consumed.
        self.loot.remove_cached_loot_for_owner_like_cpp(owner_guid);

        if owner_guid.is_item() && selected_pool_looted {
            self.destroy_fully_looted_direct_item(owner_guid).await;
            return true;
        }

        if owner_guid.is_corpse() {
            self.owner.transitions_like_cpp()
                .remove_canonical_corpse_lootable_dynamic_flag_like_cpp(owner_guid);
            return true;
        }

        self.release_looted_creature_like_cpp(
            owner_guid,
            player_guid,
            whole_object_fully_looted,
            represented_loot_type,
            authoritative_release.as_ref(),
        )
    }

}
