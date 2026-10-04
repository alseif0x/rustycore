// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Owned loot claims and their leases.

// Explicit database imports: this module reaches its parent through
// `use super::*`, and the persistence inventory cannot resolve a glob, so
// without these every database access in the file is invisible to the
// ratchet (see #277).
use super::*;
use wow_entities::Item;
use wow_entities::ItemObjectUpdateLikeCpp;

mod release;

impl WorldSession {
    pub(super) fn creature_loot_release_values_for_viewer_like_cpp(
        &self,
        creature_guid: ObjectGuid,
        viewer_guid: ObjectGuid,
        viewer_has_pending_bind: bool,
        authority: Option<&OwnedLootAuthority>,
        update: wow_packet::packets::update::UnitDataValuesDeltaUpdate,
    ) -> wow_packet::packets::update::UnitDataValuesDeltaUpdate {
        let (state, hub) = crate::session::split_loot_ref(self);
        state.creature_loot_release_values_for_viewer_like_cpp(
            hub,
            creature_guid,
            viewer_guid,
            viewer_has_pending_bind,
            authority,
            update,
        )
    }

    /// Clone the object-owned authority while the map/entity lock is held,
    /// then release that lock before any reservation can await.
    pub(super) fn represented_owned_loot_authority_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
    ) -> Option<OwnedLootAuthority> {
        self.core.loot_release_owner_access_like_cpp()
            .represented_owned_loot_authority_like_cpp(owner_guid)
    }

    /// Bridge pre-authority represented fixtures (and the equivalent first
    /// live generation) into the object-owned source of truth exactly once.
    /// A retired non-zero generation is never reinstalled from session cache.
    pub(super) fn prepare_owned_loot_authority_for_active_request_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        scope_player: ObjectGuid,
    ) -> Option<OwnedLootAuthority> {
        let authority = self.represented_owned_loot_authority_like_cpp(owner_guid)?;
        let can_install_first_generation = represented_local_loot_fixture_allowed_like_cpp()
            && authority.is_retired_like_cpp()
            && authority.generation_like_cpp() == 0
            && self.loot.cached_loot_contains_owner_like_cpp(owner_guid)
            && (self.loot.has_active_loot_view_owner_like_cpp(owner_guid)
                || self.loot.is_active_loot_guid(owner_guid));
        if !can_install_first_generation {
            return Some(authority);
        }

        if owner_guid.is_game_object() {
            let _ = self
                .sync_represented_gameobject_loot_to_canonical_like_cpp(owner_guid, scope_player);
        } else if owner_guid.is_creature_or_vehicle() {
            let _ =
                self.sync_represented_creature_loot_to_canonical_like_cpp(owner_guid, scope_player);
        }

        let authority = self.represented_owned_loot_authority_like_cpp(owner_guid)?;
        if let Some(snapshot) = authority.snapshot_for_player_like_cpp(scope_player) {
            self.loot
                .ensure_active_loot_view_generation_like_cpp(owner_guid, snapshot.generation);
            self.loot
                .insert_active_loot_view_authority_if_absent_like_cpp(owner_guid, &authority);
        }
        Some(authority)
    }

    pub(super) fn refresh_owned_loot_summary_like_cpp(&mut self, owner_guid: ObjectGuid) {
        self.core.loot_release_owner_access_like_cpp()
            .refresh_owned_loot_summary_like_cpp(owner_guid);
    }

    #[cfg(test)]
    pub(super) async fn store_claimed_direct_loot_item_from_owner_like_cpp(
        &mut self,
        loot_entry: &LootEntry,
        dungeon_encounter_id: u32,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        claim: &LootClaimLease,
    ) -> bool {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return false;
        };
        self.store_claimed_direct_loot_item_from_owner_with_generator_like_cpp(
            generator.as_ref(),
            loot_entry,
            dungeon_encounter_id,
            owner_guid,
            loot_obj,
            claim,
        )
        .await
    }

    pub(super) async fn store_claimed_direct_loot_item_from_owner_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        loot_entry: &LootEntry,
        dungeon_encounter_id: u32,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        claim: &LootClaimLease,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        self.store_direct_loot_item_with_source_and_generator_like_cpp(
            item_guid_generator,
            loot_entry,
            dungeon_encounter_id,
            owner_guid.is_item().then_some(owner_guid),
            Some(claim),
            Some(LootItemClaimCommitContextLikeCpp {
                owner_guid,
                loot_obj,
                loot_list_id: loot_entry.loot_list_id,
                player_guid,
                free_for_all: loot_entry.flags.freeforall,
            }),
        )
        .await
    }

    pub(super) async fn destroy_direct_item_count_after_loot_release_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        maximum_destroy_count: Option<u32>,
    ) {
        let player_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };

        let runtime_item = self.resolved_inventory_item_object_like_cpp(item_guid);
        let (bag, slot) = match runtime_item.as_ref() {
            Some(item) => (item.bag_slot(), item.slot()),
            None => return,
        };

        let Some(item) = self.get_inventory_item_by_pos(bag, slot) else {
            return;
        };

        let port = match self.lifecycle.stored_item_persistence_port_like_cpp() {
            Some(port) => port,
            None => return,
        };

        let current_count = runtime_item.as_ref().map_or(1, Item::count);
        let new_count =
            direct_item_count_after_loot_release_like_cpp(current_count, maximum_destroy_count);
        if new_count != 0 {
            match port
                .update_inventory_item_count_like_cpp(
                    wow_persistence::InventoryItemCountPersistenceRequestLikeCpp {
                        item_guid: item.db_guid,
                        count: new_count,
                    },
                )
                .await
            {
                wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
                wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
                | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                    warn!(error = %reason, "LootRelease: update partially consumed item failed");
                    return;
                }
            }
            let _ = self.apply_inventory_item_object_updates_like_cpp(
                item_guid,
                &[
                    ItemObjectUpdateLikeCpp::SetCount(new_count),
                    ItemObjectUpdateLikeCpp::SetLootGenerated(false),
                ],
            );
            self.send_packet(&UpdateObject::item_stack_count_update(
                item_guid,
                self.core.player_map_id_like_cpp(),
                new_count,
            ));
            return;
        }

        let should_expire_refund = runtime_item
            .as_ref()
            .is_some_and(|item_object| item_object.is_refundable());
        match port
            .destroy_inventory_item_like_cpp(
                wow_persistence::InventoryItemDestroyPersistenceRequestLikeCpp {
                    owner_guid: player_guid.counter() as u64,
                    item_guid: item.db_guid,
                    expire_refund: should_expire_refund,
                },
            )
            .await
        {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!(error = %reason, "LootRelease: delete fully looted item failed");
                return;
            }
        }

        self.remove_fully_looted_runtime_item(bag, slot, item.guid);

        if should_expire_refund {
            self.send_packet(&ItemExpirePurchaseRefund {
                item_guid: item.guid,
            });
        }

        // Player-values update and stat refresh only apply to top-level slots.
        if bag == INVENTORY_SLOT_BAG_0 {
            let mut visible_item_changes = Vec::new();
            let mut virtual_item_changes = Vec::new();
            if (slot as usize) < 19 {
                visible_item_changes.push((slot, 0i32, 0u16, 0u16));
            }
            if slot >= 15 && slot <= 17 {
                virtual_item_changes.push((slot - 15, 0i32, 0u16, 0u16));
            }

            self.send_player_values_update_from_entity_bridge(
                &[(slot, ObjectGuid::EMPTY)],
                &visible_item_changes,
                &virtual_item_changes,
                &[],
                None,
            );

            if slot < 19 {
                self.send_stat_update();
            }
        }
    }
}

impl crate::session::LootCx<'_> {
    pub(super) fn record_represented_gameobject_chest_release_metadata_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        source: GameObjectLootSource,
    ) {
        let state = self
            .world_entities
            .ensure_represented_gameobject_use_state_like_cpp(gameobject_guid);
        state.go_type = Some(GAMEOBJECT_TYPE_CHEST as u8);
        state.chest_restock_time_secs = Some(source.chest_restock_time_secs);
        state.chest_consumable = Some(source.chest_consumable);
        state.despawn_at_action = source.chest_consumable;
        state.chest_loot_source = Some(source);
        state.chest_personal_loot_id = Some(source.personal_loot_id);
        state.linked_trap_entry =
            (source.linked_trap_entry != 0).then_some(source.linked_trap_entry);
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/handlers/loot/claims/f3_shims.rs"]
mod f3_shims;
