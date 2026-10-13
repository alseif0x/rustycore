// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the spell-effect bodies moved to
//! `wow_world_spell` (#1363 slice 9c).
//!
//! The first impl block holds the `#[cfg(test)]` method of
//! `session/spell_effects/effect_combat/damage_and_combat_application.rs`,
//! verbatim except that its call to the moved damage entry is a free call: it
//! was already test-only. The second holds one-call method forwards for the
//! moved functions the scenarios call, with this session as the host, so no
//! scenario call site changes.

#![cfg(test)]

use crate::session::*;
use std::sync::Arc;
use wow_core::ObjectGuid;
use wow_packet::packets::spell::SpellTargetData;
use wow_world_spell::{
    ApplyEffectSummonObjectSlotSessionOutcomeLikeCpp,
    ApplyEffectSummonObjectWildSessionOutcomeLikeCpp,
};

impl WorldSession {
    pub(crate) async fn apply_damage(
        &mut self,
        spell_id: Option<i32>,
        target_guid: ObjectGuid,
        damage_amount: u32,
    ) -> Result<(), &'static str> {
        let generators = self.id_generators_for_test_like_cpp();
        wow_world_spell::apply_damage_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
            spell_id,
            target_guid,
            damage_amount,
        )
        .await
    }
}

impl WorldSession {
    pub(crate) fn set_represented_homebind_like_cpp(
        &mut self,
        homebind: RepresentedHomebindLikeCpp,
    ) -> bool {
        wow_world_spell::set_represented_homebind_like_cpp(self, homebind)
    }
    pub(crate) async fn apply_heal(
        &mut self,
        spell_id: Option<i32>,
        target_guid: ObjectGuid,
        heal_amount: u32,
    ) -> Result<(), &'static str> {
        wow_world_spell::apply_heal(self, spell_id, target_guid, heal_amount).await
    }
    pub(crate) fn apply_effect_summon_object_wild_like_cpp(
        &mut self,
        spell_id: i32,
        effect: &wow_data::SpellEffectInfo,
        target_data: &SpellTargetData,
    ) -> Option<ApplyEffectSummonObjectWildSessionOutcomeLikeCpp> {
        wow_world_spell::apply_effect_summon_object_wild_like_cpp(
            self,
            spell_id,
            effect,
            target_data,
        )
    }
    pub(crate) fn apply_effect_summon_object_slot_like_cpp(
        &mut self,
        spell_id: i32,
        effect: &wow_data::SpellEffectInfo,
        target_data: &SpellTargetData,
    ) -> Option<ApplyEffectSummonObjectSlotSessionOutcomeLikeCpp> {
        wow_world_spell::apply_effect_summon_object_slot_like_cpp(
            self,
            spell_id,
            effect,
            target_data,
        )
    }
    pub(crate) fn represented_homebind_like_cpp(&self) -> Option<RepresentedHomebindLikeCpp> {
        wow_world_spell::represented_homebind_like_cpp(self)
    }
    pub(crate) fn apply_effect_add_farsight_like_cpp(
        &mut self,
        spell_id: i32,
        effect: &wow_data::SpellEffectInfo,
        target_data: &SpellTargetData,
        spell_visual_id: u32,
        cast_time_ms: u32,
    ) -> Option<wow_map::map::FarsightDynamicObjectCreateOutcomeLikeCpp> {
        wow_world_spell::apply_effect_add_farsight_like_cpp(
            self,
            spell_id,
            effect,
            target_data,
            spell_visual_id,
            cast_time_ms,
        )
    }
    pub(crate) fn represented_target_mechanic_mask_like_cpp(&self, target_guid: ObjectGuid) -> u64 {
        wow_world_spell::represented_target_mechanic_mask_like_cpp(self, target_guid)
    }
    pub(crate) fn represented_target_creature_type_mask_like_cpp(
        &self,
        target_guid: ObjectGuid,
    ) -> u32 {
        wow_world_spell::represented_target_creature_type_mask_like_cpp(self, target_guid)
    }
    pub(crate) fn represented_target_aura_state_mask_like_cpp(
        &self,
        target_guid: ObjectGuid,
    ) -> u32 {
        wow_world_spell::represented_target_aura_state_mask_like_cpp(self, target_guid)
    }
    pub(crate) fn notify_other_players_visibility_changed_like_cpp(&self) {
        wow_world_spell::notify_other_players_visibility_changed_like_cpp(self)
    }
    pub(crate) fn implicit_target_conditions_like_cpp(
        &self,
        spell_id: i32,
        effect_index: u32,
    ) -> Option<Arc<wow_data::ConditionContainer>> {
        wow_world_spell::implicit_target_conditions_like_cpp(self, spell_id, effect_index)
    }
    pub(crate) async fn apply_heal_max_health_like_cpp(
        &mut self,
        spell_id: i32,
        damage: i32,
        healer_guid: ObjectGuid,
        target_guid: ObjectGuid,
    ) -> Result<(), &'static str> {
        wow_world_spell::apply_heal_max_health_like_cpp(
            self,
            spell_id,
            damage,
            healer_guid,
            target_guid,
        )
        .await
    }
}
