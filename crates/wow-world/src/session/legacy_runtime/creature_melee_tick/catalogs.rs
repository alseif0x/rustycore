//! Application catalog resolution for the synchronous Map melee motor.
use super::*;
use std::collections::HashMap;
use wow_entities::{AppliedAuraRef, AuraApplicationLikeCpp, AuraSubsystem};
use wow_map::map::{CreatureMeleeCatalogsLikeCpp, MeleeThreatSpellFacts,
    ShareAuraIdentityLikeCpp, ShareAuraSnapshotLikeCpp};

pub(super) struct Catalogs<'a>(pub &'a LegacyCreatureAggroConfigLikeCpp);

impl CreatureMeleeCatalogsLikeCpp for Catalogs<'_> {
    fn represented(&self) -> bool { self.0.spell_store.is_some() }
    fn creature_effects(&self, applied: &[AppliedAuraRef], difficulty: u8)
        -> Vec<crate::session_rules::AppliedAuraEffectLikeCpp> {
        crate::session_rules::creature_aura_effects_like_cpp(applied,
            self.0.spell_store.as_deref().unwrap(), difficulty, self.0.difficulty_store.as_deref())
    }
    fn player_effects(&self, auras: &HashMap<u8, AuraApplicationLikeCpp>)
        -> Vec<crate::session_rules::AppliedAuraEffectLikeCpp> {
        crate::session_rules::player_aura_effects_all_like_cpp(auras, self.0.spell_store.as_deref().unwrap())
    }
    fn player_effects_of_type(&self, auras: &HashMap<u8, AuraApplicationLikeCpp>, aura_type: i32)
        -> Vec<crate::session_rules::AppliedAuraEffectLikeCpp> {
        crate::session_rules::player_aura_effects_full_by_spell_aura_type_like_cpp(
            auras, self.0.spell_store.as_deref().unwrap(), aura_type)
    }
    fn player_amounts_of_type(&self, auras: &HashMap<u8, AuraApplicationLikeCpp>, aura_type: i32)
        -> Vec<(i32, i32)> {
        crate::session_rules::player_aura_effects_by_spell_aura_type_like_cpp(
            auras, self.0.spell_store.as_deref().unwrap(), aura_type)
    }
    fn player_mechanic_mask(&self, auras: &HashMap<u8, AuraApplicationLikeCpp>) -> u32 {
        crate::session_rules::aura_application_mechanic_mask_like_cpp(
            auras, self.0.spell_store.as_deref().unwrap(), self.0.difficulty_store.as_deref())
    }
    fn creature_mechanic_mask(&self, applied: &[AppliedAuraRef], difficulty: u8) -> u32 {
        crate::session_rules::applied_aura_mechanic_mask_like_cpp(
            applied, self.0.spell_store.as_deref().unwrap(), difficulty, self.0.difficulty_store.as_deref())
    }
    fn race_creature_type_mask(&self, race: u8) -> u32 {
        self.0.chr_races_store.as_ref().and_then(|store| store.get(u32::from(race)))
            .and_then(|race| u32::try_from(race.creature_type).ok())
            .filter(|creature_type| *creature_type >= 1)
            .and_then(|creature_type| 1_u32.checked_shl(creature_type - 1)).unwrap_or(0)
    }
    fn template_creature_type_mask(&self, entry: u32) -> u32 {
        self.0.creature_template_lifecycle_store.as_ref().and_then(|store| store.get(entry))
            .and_then(|template| (template.creature_type >= 1)
                .then(|| 1_u32.checked_shl(template.creature_type - 1))).flatten().unwrap_or(0)
    }
    fn block_armor_constant(&self, attacker_level: u8) -> f32 {
        self.0.expected_stat_store.as_ref().map_or(1.0,
            |store| store.armor_constant_like_cpp(u32::from(attacker_level), -2))
    }
    fn player_shields(&self, auras: &HashMap<u8, AuraApplicationLikeCpp>, difficulty: u8,
        school_mask: u32) -> Vec<crate::session_rules::RepresentedAbsorbShieldLikeCpp> {
        crate::session_rules::player_absorb_shields_like_cpp(auras, self.0.spell_store.as_deref().unwrap(),
            difficulty, self.0.difficulty_store.as_deref(), school_mask)
    }
    fn player_mana_shields(&self, auras: &HashMap<u8, AuraApplicationLikeCpp>, difficulty: u8,
        school_mask: u32) -> Vec<crate::session_rules::RepresentedManaShieldLikeCpp> {
        crate::session_rules::player_mana_shields_like_cpp(auras, self.0.spell_store.as_deref().unwrap(),
            difficulty, self.0.difficulty_store.as_deref(), school_mask)
    }
    fn creature_shields(&self, auras: &AuraSubsystem, difficulty: u8,
        school_mask: u32) -> Vec<crate::session_rules::RepresentedAbsorbShieldLikeCpp> {
        crate::session_rules::creature_absorb_shields_like_cpp(auras, self.0.spell_store.as_deref().unwrap(),
            difficulty, self.0.difficulty_store.as_deref(), school_mask)
    }
    fn share_player(&self, auras: &HashMap<u8, AuraApplicationLikeCpp>) -> Vec<ShareAuraSnapshotLikeCpp> {
        let spell_store = self.0.spell_store.as_deref().unwrap();
                let mut slots = auras.keys().copied().collect::<Vec<_>>();
                slots.sort_unstable();
                let mut snapshots = Vec::new();
                for slot in slots {
                    let aura = &auras[&slot];
                    let Some(spell) = spell_store.get(aura.spell_id) else {
                        continue;
                    };
                    for effect in spell.effects().iter().filter(|effect| {
                        effect.effect_aura
                            == wow_data::spell::aura_types::SPELL_AURA_SHARE_DAMAGE_PCT
                            && 1_u32
                                .checked_shl(effect.effect_index)
                                .is_some_and(|bit| aura.effect_mask & bit != 0)
                    }) {
                        let amount = aura
                            .represented_effect_amounts
                            .iter()
                            .find(|represented| {
                                u8::try_from(effect.effect_index).ok()
                                    == Some(represented.effect_index)
                            })
                            .map(|represented| represented.amount)
                            .unwrap_or_else(|| effect.calc_value_no_caster_like_cpp());
                        snapshots.push(ShareAuraSnapshotLikeCpp {
                            identity: ShareAuraIdentityLikeCpp::Player {
                                slot,
                                spell_id: aura.spell_id,
                                caster_guid: aura.caster_guid,
                                effect_index: effect.effect_index,
                            },
                            caster_guid: aura.caster_guid,
                            school_mask: effect.effect_misc_value_1 as u32,
                            amount,
                        });
                    }
                }
        snapshots
    }
    fn share_creature(&self, applied: &[AppliedAuraRef], difficulty_id: u8) -> Vec<ShareAuraSnapshotLikeCpp> {
        let spell_store = self.0.spell_store.as_deref().unwrap();
        let difficulty_store = self.0.difficulty_store.as_deref();
                let mut snapshots = Vec::new();
                for applied in applied {
                    let spell_id = i32::try_from(applied.spell_id).unwrap_or(0);
                    let Some(effects) = spell_store.effects_for_difficulty_like_cpp(
                        spell_id,
                        difficulty_id,
                        difficulty_store,
                    ) else {
                        continue;
                    };
                    for effect in effects.iter().filter(|effect| {
                        effect.effect_aura
                            == wow_data::spell::aura_types::SPELL_AURA_SHARE_DAMAGE_PCT
                            && 1_u32
                                .checked_shl(effect.effect_index)
                                .is_some_and(|bit| applied.effect_mask & bit != 0)
                    }) {
                        snapshots.push(ShareAuraSnapshotLikeCpp {
                            identity: ShareAuraIdentityLikeCpp::Creature {
                                applied: *applied,
                                effect_index: effect.effect_index,
                            },
                            caster_guid: applied.caster_guid,
                            school_mask: effect.effect_misc_value_1 as u32,
                            amount: effect.calc_value_no_caster_like_cpp(),
                        });
                    }
                }
        snapshots
    }
    fn threat_spell(&self, spell_id: Option<i32>, difficulty_id: u8) -> MeleeThreatSpellFacts {
        let spell_store = self.0.spell_store.as_deref();
        let spell_misc_store = self.0.spell_misc_store.as_deref();
        let spell_threat_store = self.0.spell_threat_store.as_deref();
        let spell_chain_store = self.0.spell_chain_store.as_deref();
        let difficulty_store = self.0.difficulty_store.as_deref();
    let suppress = spell_id.is_some_and(|spell_id| {
        spell_store.is_some_and(|store| {
            store.has_attribute_for_difficulty_like_cpp(
                spell_id,
                difficulty_id,
                difficulty_store,
                1,
                wow_data::spell::attributes::SPELL_ATTR1_NO_THREAT,
            ) || store.has_attribute_for_difficulty_like_cpp(
                spell_id,
                difficulty_id,
                difficulty_store,
                4,
                wow_data::spell::attributes::SPELL_ATTR4_NO_HARMFUL_THREAT,
            )
        })
    });
    let no_initial_threat = spell_id.is_some_and(|spell_id| {
        spell_store.is_some_and(|store| {
            store.has_attribute_for_difficulty_like_cpp(
                spell_id,
                difficulty_id,
                difficulty_store,
                2,
                wow_data::spell::attributes::SPELL_ATTR2_NO_INITIAL_THREAT,
            )
        })
    });
    let school_mask = spell_id
        .and_then(|spell_id| u32::try_from(spell_id).ok())
        .and_then(|spell_id| {
            spell_misc_store.and_then(|store| {
                store.entry_for_spell_difficulty_with_fallback_like_cpp(
                    spell_id,
                    difficulty_id,
                    difficulty_store,
                )
            })
        })
        .map_or(0x01, |entry| u32::from(entry.school_mask));
    let spell_multiplier = spell_id
        .and_then(|spell_id| u32::try_from(spell_id).ok())
        .and_then(|spell_id| {
            spell_threat_store.and_then(|store| {
                store.get_spell_threat_entry_like_cpp(spell_id, |spell_id| {
                    spell_chain_store.map_or(spell_id, |chains| {
                        chains.first_spell_in_chain_like_cpp(spell_id)
                    })
                })
            })
        })
        .map_or(1.0, |entry| entry.pct_mod);

        MeleeThreatSpellFacts { suppress, no_initial_threat, school_mask, multiplier: spell_multiplier }
    }
    fn threat_aura(&self, applied: &[AppliedAuraRef], difficulty: u8, school_mask: u32) -> f32 {
        self.creature_effects(applied, difficulty).into_iter()
            .filter(|effect| effect.aura_type == wow_data::spell::aura_types::SPELL_AURA_MOD_THREAT
                && effect.misc_value as u32 & school_mask != 0)
            .fold(1.0_f32, |multiplier, effect| multiplier * (1.0 + effect.amount as f32 / 100.0))
    }
    fn game_time_ms(&self) -> u64 { u64::from(crate::session::game_time_ms_like_cpp()) }
}
