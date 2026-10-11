// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn represented_item_bonus_actions_like_cpp(
        &self,
    ) -> &[RepresentedItemBonusActionLikeCpp] {
        self.inventory.represented_item_bonus_actions_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_item_set_spell_events_like_cpp(
        &self,
    ) -> &[RepresentedItemSetSpellEventLikeCpp] {
        self.inventory.represented_item_set_spell_events_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_item_set_aura_refresh_events_like_cpp(
        &self,
    ) -> &[RepresentedItemSetAuraRefreshEventLikeCpp] {
        self.inventory
            .represented_item_set_aura_refresh_events_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_item_set_effect_like_cpp(
        &self,
        item_set_id: u32,
    ) -> Option<RepresentedItemSetEffectLikeCpp> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_item_set_effect_like_cpp(hub, item_set_id)
    }
    #[cfg(test)]
    pub(crate) fn represented_item_bonus_state_like_cpp(&self) -> RepresentedItemBonusStateLikeCpp {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_item_bonus_state_like_cpp(hub)
    }
}

impl WorldSession {
    /// C++ `sItemLimitCategoryStore.LookupEntry(limitCategory)`.
    pub(crate) fn item_limit_category_template_like_cpp(
        &self,
        limit_category_id: u32,
    ) -> Option<ItemLimitCategoryTemplate> {
        self.player_condition_projection_cx_like_cpp()
            .item_limit_category_template_like_cpp(limit_category_id)
    }
    pub(in crate::session) fn player_item_modifier_runtime_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerItemModifierRuntimeStateLikeCpp> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.player_item_modifier_runtime_snapshot_like_cpp(hub)
    }
    pub(in crate::session) fn add_player_item_set_item_like_cpp(
        &mut self,
        item_set_id: u32,
        item_guid: ObjectGuid,
    ) -> Option<usize> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.add_player_item_set_item_like_cpp(&mut hub, item_set_id, item_guid)
    }
    pub(in crate::session) fn add_player_item_set_bonus_like_cpp(
        &mut self,
        item_set_id: u32,
        spell_entry_id: u32,
    ) -> Option<bool> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.add_player_item_set_bonus_like_cpp(&mut hub, item_set_id, spell_entry_id)
    }
    pub(in crate::session) fn set_player_item_level_caps_like_cpp(
        &mut self,
        caps: wow_entities::PlayerItemLevelCapsLikeCpp,
    ) -> bool {
        self.inventory
            .set_player_item_level_caps_with_access_like_cpp(
                &self.core.owned_item_modifiers_access_like_cpp(),
                caps,
                cfg!(test),
            )
    }
    pub(crate) fn record_represented_update_item_set_auras_like_cpp(
        &mut self,
        form_change: bool,
    ) -> usize {
        let events = self.plan_represented_update_item_set_auras_like_cpp(form_change);
        #[cfg(test)]
        self.inventory
            .record_represented_item_set_aura_refresh_events_for_test_like_cpp(&events);
        events.len()
    }
    pub(crate) fn plan_represented_update_item_set_auras_like_cpp(
        &self,
        form_change: bool,
    ) -> Vec<RepresentedItemSetAuraRefreshEventLikeCpp> {
        wow_world_application::plan_item_set_aura_refresh_with_access_like_cpp(
            &self.inventory,
            &self.core.owned_item_modifiers_access_like_cpp(),
            &crate::session::hub_ref(self).owned_item_set_access_like_cpp(),
            self.catalogs.spell_catalogs.spell_store.as_deref(),
            self.catalogs
                .spell_catalogs
                .spell_shapeshift_form_store
                .as_deref(),
            form_change,
            cfg!(test),
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.auras.represented_shapeshift_form_like_cpp,
        )
    }
    pub(crate) fn apply_represented_item_set_aura_refresh_events_like_cpp(
        &mut self,
        form_change: bool,
    ) -> usize {
        self.player_aura_application_cx_like_cpp()
            .apply_item_set_aura_refresh_events_like_cpp(form_change)
    }
}
