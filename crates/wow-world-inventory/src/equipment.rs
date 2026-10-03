// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_constants::{InventoryResult, SpellCastResult};
use wow_entities::{
    is_bag_pos, is_equipment_packed_pos, make_item_pos, CanUnequipItemArgs, Item,
    ItemStorageTemplate,
};
use wow_world_core::session::HubRef;

impl crate::InventoryState {
    /// C++ `Player::CanUnequipItem` for any represented top-level or bag position.
    pub fn can_unequip_inventory_item_at_like_cpp(
        &self,
        hub: HubRef<'_>,
        bag: u8,
        slot: u8,
        swap: bool,
        source_item: Option<&Item>,
        proto: Option<&ItemStorageTemplate>,
        source_is_not_empty_bag: bool,
    ) -> InventoryResult {
        let pos = make_item_pos(bag, slot);
        if !is_equipment_packed_pos(pos) && !is_bag_pos(pos) {
            return InventoryResult::Ok;
        }

        let Some(player) = self.direct_inventory_player_snapshot(hub) else {
            return InventoryResult::Ok;
        };
        let is_charmed = hub
            .core
            .canonical_player_snapshot_like_cpp(|player| {
                player.unit().subsystems().control.is_charmed()
            })
            .unwrap_or(false);
        let is_in_progress_arena = hub
            .player_battleground_state_snapshot_like_cpp()
            .is_some_and(|state| state.battleground_status_like_cpp() == Some(3))
            && hub
                .catalogs
                .map_store()
                .and_then(|store| store.get(u32::from(hub.core.player_map_id_like_cpp())))
                .is_some_and(|entry| entry.instance_type == wow_data::map::MAP_ARENA);

        let Some(is_in_combat) = hub.resolved_in_combat_like_cpp() else {
            return InventoryResult::CantDoThatRightNow;
        };

        player.can_unequip_item_like_cpp(CanUnequipItemArgs {
            pos,
            source_item,
            proto,
            swap,
            source_is_not_empty_bag,
            is_charmed,
            is_in_combat,
            is_in_progress_arena,
        })
    }
}

impl crate::InventoryState {
    pub fn represented_equip_spell_fits_shapeshift_like_cpp(
        &self,
        hub: HubRef<'_>,
        spell_id: u32,
    ) -> bool {
        let Some(spell_store) = hub.catalogs.spell_catalogs.spell_store.as_ref() else {
            return true;
        };
        let Ok(spell_id) = i32::try_from(spell_id) else {
            return false;
        };

        let Some(form_id) = hub.represented_shapeshift_form_like_cpp() else {
            return false;
        };
        spell_store
            .check_shapeshift_like_cpp(spell_id, form_id, |form| {
                hub.catalogs
                    .spell_catalogs
                    .spell_shapeshift_form_store
                    .as_ref()
                    .and_then(|store| store.get(form))
            })
            .unwrap_or(SpellCastResult::Success)
            == SpellCastResult::Success
    }

    pub fn inventory_equip_capabilities_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<(bool, bool)> {
        if let Some(capabilities) = hub.core.with_owned_player_like_cpp(|player| {
            (
                player.unit().can_dual_wield_like_cpp(),
                player.can_titan_grip(),
            )
        }) {
            return Some(capabilities);
        }

        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            return Some((false, false));
        }

        None
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_avg_equipped_item_level_updates_like_cpp(&self) -> &[f32] {
        &self
            .player_item_test_fixture_like_cpp
            .represented_avg_equipped_item_level_updates_like_cpp
    }

}
