// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World adapters for the pet bodies moved to `wow_world_lifecycle::pet_session`
//! (#1363 slice 7a) and the battle-pet bodies moved to
//! `wow_world_inventory::battle_pet_session` (#1363 slice 7b). Every method is a
//! pure forward to a session split, a field or the same-named inherent method;
//! the full-stack destroy forwards to the same-named free function its body
//! moved to (`wow_world_inventory::item_session`, #1363 slice 8c).

use std::future::Future;

use wow_entities::Item;
use wow_world_core::session::{HubMut, HubRef};
use wow_world_inventory::InventoryState;
use wow_world_lifecycle::SessionLifecycleState;

use crate::session::{InventoryItem, WorldSession};

impl wow_world_lifecycle::PetSessionHostLikeCpp for WorldSession {
    fn split_pets_mut(&mut self) -> (&mut SessionLifecycleState, HubMut<'_>) {
        crate::session::split_lifecycle_mut(self)
    }
    fn resolved_represented_at_login_flags_like_cpp(&self) -> Option<u16> {
        WorldSession::resolved_represented_at_login_flags_like_cpp(self)
    }
}

impl wow_world_inventory::BattlePetSessionHostLikeCpp for WorldSession {
    fn lifecycle(&self) -> &SessionLifecycleState {
        &self.lifecycle
    }
    fn split_lifecycle_mut(&mut self) -> (&mut SessionLifecycleState, HubMut<'_>) {
        crate::session::split_lifecycle_mut(self)
    }
    fn split_inventory_ref(&self) -> (&InventoryState, HubRef<'_>) {
        crate::session::split_inventory_ref(self)
    }
    fn uncage_item_state_like_cpp(
        &self,
        player_db_guid: u64,
        item_db_guid: u64,
    ) -> impl Future<Output = wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp> + Send {
        WorldSession::uncage_item_state_like_cpp(self, player_db_guid, item_db_guid)
    }
    fn destroy_inventory_full_stack_by_pos_with_expected_owner_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item: InventoryItem,
        runtime_item: Option<Item>,
        expected_owner_db_guid: Option<u64>,
        context: &str,
    ) -> impl Future<Output = bool> + Send {
        wow_world_inventory::destroy_inventory_full_stack_by_pos_with_expected_owner_like_cpp(
            self,
            bag,
            slot,
            item,
            runtime_item,
            expected_owner_db_guid,
            context,
        )
    }
    #[cfg(test)]
    fn battle_pet_add_pet_fixture_like_cpp(
        &mut self,
        species: u32,
        display_id: u32,
        breed: u16,
        quality: u8,
        level: u16,
    ) -> Option<Option<wow_core::ObjectGuid>> {
        Some(WorldSession::battle_pet_add_pet_represented_like_cpp(
            self, species, display_id, breed, quality, level,
        ))
    }
    #[cfg(test)]
    fn battle_pet_grant_experience_fixture_like_cpp(
        &mut self,
        pet_guid: wow_core::ObjectGuid,
        xp: u16,
        xp_source: wow_world_inventory::RepresentedBattlePetXpSourceLikeCpp,
        pet_battle_xp_multiplier: f32,
    ) -> Option<wow_world_inventory::RepresentedBattlePetGrantExperienceOutcomeLikeCpp> {
        Some(
            WorldSession::battle_pet_grant_battle_pet_experience_represented_like_cpp(
                self,
                pet_guid,
                xp,
                xp_source,
                pet_battle_xp_multiplier,
            ),
        )
    }
    #[cfg(test)]
    fn battle_pet_grant_level_fixture_like_cpp(
        &mut self,
        pet_guid: wow_core::ObjectGuid,
        granted_levels: u16,
    ) -> Option<wow_world_inventory::RepresentedBattlePetGrantLevelOutcomeLikeCpp> {
        Some(
            WorldSession::battle_pet_grant_battle_pet_level_represented_like_cpp(
                self,
                pet_guid,
                granted_levels,
            ),
        )
    }
}
