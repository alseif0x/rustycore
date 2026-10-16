// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the bodies moved in #1363 slice 12b2b
//! (the nearby-entry destination search to `wow-world-spell`; the loaded
//! item-mod, enchantment and item-set-aura family and the creature-damage
//! kill cascade to `wow-world-application`): one-call forwards to the moved
//! functions the scenarios call, so no scenario call site changes.
#![cfg(test)]

use wow_core::{ObjectGuid, Position};
use wow_entities::{
    ApplyEnchantmentEffectAction, ApplyEnchantmentEffectRef, ApplyEnchantmentRandomSuffixRef,
};
use wow_world_inventory::{
    InitialLoadedItemModsOutcomeLikeCpp, LoadedEquippedItemEnchantmentsOutcomeLikeCpp,
};

use crate::session::WorldSession;

impl WorldSession {
    pub(crate) fn represented_nearby_entry_destination_like_cpp(
        &self,
        effect: &wow_data::SpellEffectInfo,
        caster_position: &Position,
        range: f32,
        implicit_conditions: Option<&[wow_data::Condition]>,
    ) -> Option<Position> {
        wow_world_spell::represented_nearby_entry_destination_like_cpp(
            self,
            effect,
            caster_position,
            range,
            implicit_conditions,
        )
    }
    pub(crate) fn apply_initial_loaded_item_mods_like_cpp(
        &mut self,
        loaded_equipped_item_guids: &[ObjectGuid],
    ) -> InitialLoadedItemModsOutcomeLikeCpp {
        wow_world_application::apply_initial_loaded_item_mods_like_cpp(
            self,
            loaded_equipped_item_guids,
        )
    }
    pub(crate) fn apply_loaded_equipped_item_enchantments_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
    ) -> LoadedEquippedItemEnchantmentsOutcomeLikeCpp {
        wow_world_application::apply_loaded_equipped_item_enchantments_like_cpp(self, item_guid)
    }
    pub(crate) fn apply_represented_item_bonus_action_state_like_cpp(
        &mut self,
        action: ApplyEnchantmentEffectAction,
    ) -> bool {
        wow_world_application::apply_represented_item_bonus_action_state_like_cpp(self, action)
    }
    /// Moved verbatim from `session/player_items/equipment.rs`: its last
    /// production caller moved to `wow-world-application`.
    pub(crate) fn apply_initial_item_equip_auras_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
    ) -> usize {
        self.player_aura_application_cx_like_cpp()
            .apply_initial_item_equip_auras_like_cpp(item_guid)
    }
    /// Moved from `session/spell_state/aura_application.rs` (its last
    /// production caller moved); the private provenance-and-update wrapper
    /// it called is inlined, the same call the moved body makes.
    pub(crate) fn apply_aura_with_effect_mask_without_update_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        duration_ms: u32,
        aura_flags: u32,
        effect_mask: u32,
    ) -> Result<(), &'static str> {
        self.player_aura_application_cx_like_cpp()
            .apply_aura_with_effect_mask_provenance_and_update_like_cpp(
                spell_id,
                caster_guid,
                duration_ms,
                aura_flags,
                effect_mask,
                wow_entities::AuraCastProvenanceLikeCpp::default(),
                false,
            )
    }
    /// Moved verbatim from `session/player_items/enchantment.rs`, like the
    /// next one: the moved enchantment-effects body reads the catalog
    /// through the host, so only the scenarios call these.
    pub(crate) fn apply_enchantment_random_suffix_ref(
        &self,
        random_properties_id: i32,
    ) -> Option<ApplyEnchantmentRandomSuffixRef> {
        self.catalogs
            .apply_enchantment_random_suffix_ref(random_properties_id)
    }
    pub(crate) fn apply_enchantment_effect_refs(
        &self,
        enchantment_id: u32,
    ) -> Option<[ApplyEnchantmentEffectRef; 3]> {
        self.catalogs.apply_enchantment_effect_refs(enchantment_id)
    }
}
