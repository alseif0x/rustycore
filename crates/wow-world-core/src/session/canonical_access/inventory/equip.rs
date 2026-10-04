// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::OwnedInventoryAccessLikeCpp;


impl OwnedInventoryAccessLikeCpp<'_> {
    pub fn reborrow_like_cpp(&self) -> OwnedInventoryAccessLikeCpp<'_> {
        OwnedInventoryAccessLikeCpp { core: self.core }
    }

    pub fn send_equip_error_like_cpp(&self, result: wow_constants::InventoryResult,
        item1: Option<wow_core::ObjectGuid>, item2: Option<wow_core::ObjectGuid>,
        required_level: u32, limit_category: u32) {
        self.core.send_equip_error(result, item1, item2, required_level, limit_category);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn titan_grip_penalty_action_for_inventory_like_cpp(
        &self, using_two_handed_weapon_in_one_hand: bool,
        visible_auras: &std::collections::HashMap<u8, wow_entities::AuraApplicationLikeCpp>,
    ) -> Option<wow_entities::TitanGripPenaltyAction> {
        self.core.canonical_player_snapshot_like_cpp(|player| {
            let penalty_spell_id = player.titan_grip_penalty_spell_id();
            let has_penalty_aura = penalty_spell_id > 0
                && visible_auras.values().any(|aura| aura.spell_id == penalty_spell_id as i32);
            player.check_titan_grip_penalty_action(using_two_handed_weapon_in_one_hand, has_penalty_aura)
        })
    }
}
