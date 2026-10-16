// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn represented_avg_equipped_item_level_updates_like_cpp(&self) -> &[f32] {
        self.inventory
            .represented_avg_equipped_item_level_updates_like_cpp()
    }
}

impl WorldSession {
    pub(crate) fn apply_initial_equipped_item_equip_auras_like_cpp(&mut self) -> Option<usize> {
        let mut equipped: Vec<_> = self
            .resolved_inventory_item_objects_like_cpp()?
            .values()
            .filter(|item| item.container_guid().is_empty() && item.slot() < INVENTORY_SLOT_BAG_END)
            .map(|item| (item.slot(), item.object().guid()))
            .collect();
        equipped.sort_by_key(|(slot, guid)| (*slot, guid.counter()));
        Some(
            equipped
                .into_iter()
                .map(|(_slot, item_guid)| self.apply_initial_item_equip_auras_like_cpp(item_guid))
                .sum(),
        )
    }
    pub(crate) fn represented_avg_equipped_item_level_like_cpp(&self) -> Option<f32> {
        let inventory_access = self.core.owned_inventory_access_like_cpp();
        let valuation_access = self.core.inventory_valuation_access_like_cpp();
        let modifier_access = self.core.owned_item_modifiers_access_like_cpp();
        let catalogs = self.catalogs.inventory_valuation_catalog_view_like_cpp();
        self.inventory
            .represented_avg_equipped_item_level_with_access_like_cpp(
                &inventory_access,
                &valuation_access,
                &modifier_access,
                &catalogs,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.identity.player_level,
                Self::MIN_ITEM_LEVEL_LIKE_CPP,
                Self::MAX_ITEM_LEVEL_LIKE_CPP,
            )
    }
}
