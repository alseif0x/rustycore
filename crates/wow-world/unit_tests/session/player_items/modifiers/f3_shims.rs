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
