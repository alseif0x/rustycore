// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World adapter for the battle-pet purchase saga moved to
//! `wow_world_inventory::battle_pet_purchase` (#1363 slice 7a). Every method
//! is a pure forward to the same-named inherent `WorldSession` method. The
//! journal reads, the lease and the purchase criteria are the
//! `wow_world_inventory::battle_pet_session` free functions over the
//! [`wow_world_inventory::BattlePetSessionHostLikeCpp`] adapter (#1363 slice 7b).

use std::future::Future;

use wow_core::ObjectGuid;
use wow_world_inventory::BattlePetPurchaseHostLikeCpp;

use crate::session::{ExclusivePlayerMoneyPersistenceLikeCpp, WorldSession};

impl BattlePetPurchaseHostLikeCpp for WorldSession {
    fn resolved_player_money_like_cpp(&self) -> Option<u64> {
        WorldSession::resolved_player_money_like_cpp(self)
    }
    fn stage_player_money_change_like_cpp(&mut self, old_money: u64, new_money: u64) -> bool {
        wow_world_inventory::stage_player_money_change_like_cpp(self, old_money, new_money)
    }
    fn send_player_values_update_from_entity_bridge(
        &self,
        inv_slot_changes: &[(u8, ObjectGuid)],
        visible_item_changes: &[(u8, i32, u16, u16)],
        virtual_item_changes: &[(u8, i32, u16, u16)],
        buyback_changes: &[(u8, u32, i64)],
        coinage: Option<u64>,
    ) -> bool {
        WorldSession::send_player_values_update_from_entity_bridge(
            self,
            inv_slot_changes,
            visible_item_changes,
            virtual_item_changes,
            buyback_changes,
            coinage,
        )
    }
    fn drain_represented_quest_objective_progress_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) -> impl Future<Output = ()> + Send {
        WorldSession::drain_represented_quest_objective_progress_with_generator_like_cpp(
            self,
            item_guid_generator,
        )
    }
    fn learn_dependent_known_spell_like_cpp(&mut self, spell_id: i32) {
        wow_world_spell::learn_dependent_known_spell_like_cpp(self, spell_id)
    }
    fn begin_exclusive_player_money_persistence_like_cpp(
        &mut self,
    ) -> impl Future<Output = Option<ExclusivePlayerMoneyPersistenceLikeCpp>> + Send {
        WorldSession::begin_exclusive_player_money_persistence_like_cpp(self)
    }
}
