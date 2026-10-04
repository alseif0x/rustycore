use std::collections::HashSet;
use wow_data::{SkillRangeTypeLikeCpp, SpellLearnSkillLookupLikeCpp};
impl crate::SessionSpellState {
    pub fn known_spell_ids_for_aura_with_access_like_cpp(
        &self, player: &wow_world_core::session::OwnedSpellAcquisitionAccessLikeCpp<'_>,
        consumer_test: bool,
    ) -> Vec<i32> {
        #[cfg(any(test, feature = "test-fixtures"))]
        if consumer_test && player.player_handle_absent_like_cpp() {
            let snapshot = player.with_player_spell_runtime_like_cpp(crate::represented_player_spell_runtime_like_cpp)
                .unwrap_or_else(|| self.represented_spell_runtime_fixture_like_cpp());
            let runtime = crate::canonical_player_spell_runtime_like_cpp(snapshot);
            return runtime.known_spells_like_cpp().to_vec();
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = consumer_test;
        player.with_player_spell_runtime_like_cpp(|runtime| runtime.known_spells_like_cpp().to_vec()).unwrap_or_default()
    }
    pub fn represented_spell_valid_for_learning_like_cpp(
        &self,
        hub: wow_world_core::session::HubRef<'_>,
        spell_id: i32,
    ) -> bool {
        let Some(spell_store) = hub.catalogs.spell_store() else {
            return false;
        };
        wow_data::represented_spell_valid_with_seen_like_cpp(
            spell_store,
            spell_id,
            &mut HashSet::new(),
        )
    }
    pub fn represented_direct_learn_spell_triggers_like_cpp(
        &self,
        hub: wow_world_core::session::HubRef<'_>,
        spell_id: i32,
    ) -> Vec<i32> {
        hub.catalogs
            .spell_store()
            .and_then(|store| store.get(spell_id))
            .map(|spell_info| {
                spell_info
                    .effects()
                    .iter()
                    .filter(|effect| {
                        effect.effect
                            == wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_SPELL
                            && effect.effect_trigger_spell > 0
                    })
                    .map(|effect| effect.effect_trigger_spell)
                    .collect()
            })
            .unwrap_or_default()
    }
    pub fn apply_loaded_spell_learn_skills_like_cpp(
        &mut self,
        hub: &mut wow_world_core::session::HubMut<'_>,
        roots: &[i32],
    ) -> bool {
        for &spell_id in roots {
            let Ok(spell_id) = u32::try_from(spell_id) else {
                return false;
            };
            let learned_skill = match hub.catalogs.spell_learn_skill_lookup_like_cpp(spell_id) {
                SpellLearnSkillLookupLikeCpp::Present(node) => *node,
                SpellLearnSkillLookupLikeCpp::CoveredWithoutNode => continue,
                SpellLearnSkillLookupLikeCpp::Indeterminate(_)
                | SpellLearnSkillLookupLikeCpp::MissingCoverage => return false,
            };
            let Some(mut value) = hub
                .shared()
                .resolved_player_skill_value_like_cpp(learned_skill.skill)
            else {
                return false;
            };
            value = value.max(learned_skill.value);
            let Some(current_max) = hub
                .shared()
                .resolved_player_skill_max_value_like_cpp(learned_skill.skill)
            else {
                return false;
            };
            let mut new_max = learned_skill.maxvalue;
            if new_max == 0 {
                let (Some(skills), Some(lines), Some(tiers)) = (
                    hub.catalogs.skill_store(),
                    hub.catalogs.skill_line_store(),
                    hub.catalogs.skill_tiers_store(),
                ) else {
                    return false;
                };
                let Some(rc_info) = skills.skill_race_class_info_like_cpp(
                    learned_skill.skill,
                    hub.shared().player_race_like_cpp(),
                    hub.shared().player_class_like_cpp(),
                ) else {
                    return false;
                };
                match skills.skill_range_type_like_cpp(rc_info, lines, tiers) {
                    SkillRangeTypeLikeCpp::Language => {
                        value = 300;
                        new_max = 300;
                    }
                    SkillRangeTypeLikeCpp::Level => {
                        new_max = hub.shared().max_skill_value_for_level_like_cpp();
                    }
                    SkillRangeTypeLikeCpp::Mono => new_max = 1,
                    SkillRangeTypeLikeCpp::Rank => {
                        let Some(tier) = u32::try_from(rc_info.skill_tier_id)
                            .ok()
                            .and_then(|id| tiers.get_skill_tier_like_cpp(id))
                        else {
                            return false;
                        };
                        new_max = tier
                            .get_value_for_tier_index_like_cpp(u32::from(
                                learned_skill.step.saturating_sub(1),
                            ))
                            .try_into()
                            .unwrap_or(u16::MAX);
                    }
                    SkillRangeTypeLikeCpp::None => return false,
                }
                if rc_info.flags & wow_data::SKILL_FLAG_ALWAYS_MAX_VALUE_LIKE_CPP != 0 {
                    value = new_max;
                }
            }
            hub.set_represented_player_skill_like_cpp(
                learned_skill.skill,
                learned_skill.step,
                value,
                current_max.max(new_max),
            );
        }
        true
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn known_spells_fixture_like_cpp(&self) -> Vec<i32> {
        self.player_spell_test_fixture_like_cpp.known_spells.clone()
    }
}
