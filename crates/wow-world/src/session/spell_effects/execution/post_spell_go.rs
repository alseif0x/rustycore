//! Ordered target-dependent effect phases that follow spell-go publication.

use super::*;

impl WorldSession {
    pub(super) async fn apply_post_spell_go_target_effects_like_cpp(
        &mut self,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        spell_info: &wow_data::SpellInfo,
        target_data: &SpellTargetData,
        effect_target_data_like_cpp: &[(u32, SpellTargetData)],
        spell_id: i32,
        caster_guid: ObjectGuid,
        target_guid: ObjectGuid,
        spell_visual_id: u32,
        effect_type: u32,
        effect_base_points: i32,
    ) {
        let mut force_visibility_after_add_farsight = false;
        for effect in spell_info.effects() {
            if effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_ADD_FARSIGHT {
                let effect_target_data = effect_target_data_like_cpp
                    .iter()
                    .find(|(effect_index, _)| *effect_index == effect.effect_index)
                    .map(|(_, target_data)| target_data)
                    .unwrap_or(target_data);
                if let Some(outcome) = self.apply_effect_add_farsight_like_cpp(
                    spell_id,
                    effect,
                    effect_target_data,
                    spell_visual_id,
                    spell_info.cast_time_ms,
                ) {
                    force_visibility_after_add_farsight |=
                        self.consume_add_farsight_set_seer_outcome_like_cpp(&outcome);
                }
            }
        }
        if force_visibility_after_add_farsight {
            self.force_update_visibility_with_catalogs_like_cpp(creature_spawn_catalogs)
                .await;
        }

        for effect in spell_info.effects() {
            let effect_target_data = effect_target_data_like_cpp
                .iter()
                .find(|(effect_index, _)| *effect_index == effect.effect_index)
                .map(|(_, target_data)| target_data)
                .unwrap_or(target_data);
            self.apply_effect_teleport_units_like_cpp(effect, target_guid, effect_target_data)
                .await;
            self.apply_effect_bind_like_cpp(effect, caster_guid, target_guid, effect_target_data)
                .await;
        }

        if spell_info.effects().is_empty() {
            let primary_effect_like_cpp = wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: effect_type,
                effect_base_points,
                ..Default::default()
            };
            self.apply_effect_teleport_units_like_cpp(
                &primary_effect_like_cpp,
                target_guid,
                target_data,
            )
            .await;
            self.apply_effect_bind_like_cpp(
                &primary_effect_like_cpp,
                caster_guid,
                target_guid,
                target_data,
            )
            .await;
        }
    }

    pub(super) fn apply_spell_gameobject_summon_effects_like_cpp(
        &mut self,
        spell_id: i32,
        spell_info: &wow_data::SpellInfo,
        target_data: &SpellTargetData,
        effect_target_data_like_cpp: &[(u32, SpellTargetData)],
        represented_focus_object: Option<RepresentedSpellFocusObjectLikeCpp>,
    ) -> bool {
        let mut force_visibility_after_gameobject_summon = false;
        for effect in spell_info.effects() {
            let effect_target_data = effect_target_data_like_cpp
                .iter()
                .find(|(effect_index, _)| *effect_index == effect.effect_index)
                .map(|(_, target_data)| target_data)
                .unwrap_or(target_data);
            if let Some(outcome) = self.apply_effect_summon_object_wild_with_focus_like_cpp(
                spell_id,
                effect,
                effect_target_data,
                represented_focus_object,
            ) {
                if outcome.map_outcome.as_ref().is_some_and(|map_outcome| {
                    map_outcome.status
                        == wow_map::map::SpellEffectSummonObjectWildStatusLikeCpp::CreatedAddedToMap
                }) {
                    force_visibility_after_gameobject_summon = true;
                }
                continue;
            }

            if spell_effect_is_represented_summon_object_slot_like_cpp(effect.effect) {
                if let Some(outcome) = self.apply_effect_summon_object_slot_like_cpp(
                    spell_id,
                    effect,
                    effect_target_data,
                ) {
                    if outcome.map_outcome.as_ref().is_some_and(|map_outcome| {
                        map_outcome.status
                            == wow_map::map::GameObjectSummonObjectForOwnerSlotStatusLikeCpp::CreatedAddedAndSlotted
                    }) {
                        force_visibility_after_gameobject_summon = true;
                    }
                }
            }
        }
        force_visibility_after_gameobject_summon
    }
}
