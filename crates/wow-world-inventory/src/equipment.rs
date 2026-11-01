// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_constants::{InventoryResult, SpellCastResult};
use wow_entities::{
    CanUnequipItemArgs, Item, ItemStorageTemplate, is_bag_pos, is_equipment_packed_pos,
    make_item_pos,
};
use wow_world_core::session::{
    HubRef, InventoryValuationAccessLikeCpp, InventoryValuationCatalogViewLikeCpp,
    OwnedInventoryAccessLikeCpp,
};

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
        let valuation_access = hub.core.inventory_valuation_access_like_cpp();
        let inventory_access = hub.core.owned_inventory_access_like_cpp();
        let catalogs = hub.catalogs.inventory_valuation_catalog_view_like_cpp();
        self.can_unequip_inventory_item_at_with_access_like_cpp(
            &valuation_access,
            &inventory_access,
            &catalogs,
            bag,
            slot,
            swap,
            source_item,
            proto,
            source_is_not_empty_bag,
            #[cfg(any(test, feature = "test-fixtures"))]
            &hub.fixtures.battleground,
            #[cfg(any(test, feature = "test-fixtures"))]
            &hub.fixtures.combat.in_combat,
        )
    }

    /// The selected-owner form used by application CanEquip without rebuilding a Hub.
    pub fn can_unequip_inventory_item_at_with_access_like_cpp(
        &self,
        valuation_access: &InventoryValuationAccessLikeCpp<'_>,
        inventory_access: &OwnedInventoryAccessLikeCpp<'_>,
        catalogs: &InventoryValuationCatalogViewLikeCpp<'_>,
        bag: u8,
        slot: u8,
        swap: bool,
        source_item: Option<&Item>,
        proto: Option<&ItemStorageTemplate>,
        source_is_not_empty_bag: bool,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_battleground: &wow_world_core::session::BattlegroundState,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_in_combat: &bool,
    ) -> InventoryResult {
        let pos = make_item_pos(bag, slot);
        if !is_equipment_packed_pos(pos) && !is_bag_pos(pos) {
            return InventoryResult::Ok;
        }

        let Some(player) = self.direct_inventory_player_snapshot_with_access_like_cpp(
            inventory_access,
            catalogs.item_store_like_cpp(),
            catalogs.item_stats_store_like_cpp(),
        ) else {
            return InventoryResult::Ok;
        };
        let is_charmed = valuation_access.is_charmed_like_cpp();
        let is_in_progress_arena = valuation_access
            .battleground_state_snapshot_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                fixture_battleground,
            )
            .is_some_and(|state| state.battleground_status_like_cpp() == Some(3))
            && catalogs
                .map_store_like_cpp()
                .and_then(|store| store.get(u32::from(valuation_access.player_map_id_like_cpp())))
                .is_some_and(|entry| entry.instance_type == wow_data::map::MAP_ARENA);

        let Some(is_in_combat) = valuation_access.resolved_in_combat_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_in_combat,
        ) else {
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
        self.equip_spell_fits_shapeshift_like_cpp(
            hub.catalogs.spell_catalogs.spell_store.as_deref(),
            hub.catalogs
                .spell_catalogs
                .spell_shapeshift_form_store
                .as_deref(),
            spell_id,
            || hub.represented_shapeshift_form_like_cpp(),
        )
    }

    pub fn represented_equip_spell_fits_shapeshift_with_access_like_cpp(
        &self,
        player: &wow_world_core::session::OwnedItemModifiersAccessLikeCpp<'_>,
        spell_store: Option<&wow_data::SpellStore>,
        form_store: Option<&wow_data::SpellShapeshiftFormStore>,
        spell_id: u32,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_form: &u32,
    ) -> bool {
        self.equip_spell_fits_shapeshift_like_cpp(spell_store, form_store, spell_id, || {
            let canonical = player.shapeshift_form_snapshot_like_cpp();
            #[cfg(any(test, feature = "test-fixtures"))]
            if canonical.is_none() && player.owner_handle_absent_like_cpp() {
                return Some(*fixture_form);
            }
            canonical
        })
    }

    fn equip_spell_fits_shapeshift_like_cpp(
        &self,
        spell_store: Option<&wow_data::SpellStore>,
        form_store: Option<&wow_data::SpellShapeshiftFormStore>,
        spell_id: u32,
        form: impl FnOnce() -> Option<u32>,
    ) -> bool {
        let Some(spell_store) = spell_store else {
            return true;
        };
        let Ok(spell_id) = i32::try_from(spell_id) else {
            return false;
        };

        let Some(form_id) = form() else {
            return false;
        };
        spell_store
            .check_shapeshift_like_cpp(spell_id, form_id, |form| {
                form_store.and_then(|store| store.get(form))
            })
            .unwrap_or(SpellCastResult::Success)
            == SpellCastResult::Success
    }

    pub fn inventory_equip_capabilities_like_cpp(&self, hub: HubRef<'_>) -> Option<(bool, bool)> {
        let access = hub.core.inventory_valuation_access_like_cpp();
        self.inventory_equip_capabilities_with_access_like_cpp(&access)
    }

    pub fn inventory_equip_capabilities_with_access_like_cpp(
        &self,
        access: &InventoryValuationAccessLikeCpp<'_>,
    ) -> Option<(bool, bool)> {
        let canonical = access.equip_capabilities_like_cpp();
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if access.owner_handle_absent_like_cpp() {
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
