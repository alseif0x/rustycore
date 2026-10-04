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
