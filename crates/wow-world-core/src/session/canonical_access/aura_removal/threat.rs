// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::PlayerAuraRemovalAccessLikeCpp;
use wow_core::ObjectGuid;

impl PlayerAuraRemovalAccessLikeCpp<'_> {
    pub fn threat_aura_snapshot_from_stores_like_cpp(
        spell_store: Option<&wow_data::SpellStore>,
        difficulty_store: Option<&wow_data::DifficultyStore>,
        spell_id: i32,
        difficulty: u8,
        effect_mask: u32,
        represented_effect_amounts: &[wow_entities::RepresentedAuraEffectAmountLikeCpp],
    ) -> wow_entities::AuraThreatSnapshotLikeCpp {
        let interrupt_flags = spell_store
            .and_then(|store| {
                store.aura_interrupt_flags_for_difficulty_like_cpp(
                    spell_id,
                    difficulty,
                    difficulty_store,
                )
            })
            .unwrap_or([0; 2]);
        let effects = spell_store
            .and_then(|store| {
                store.effects_for_difficulty_like_cpp(
                    spell_id,
                    difficulty,
                    difficulty_store,
                )
            })
            .map(|effects| {
                effects
                    .iter()
                    .filter_map(|effect| {
                        let bit = 1u32.checked_shl(effect.effect_index)?;
                        let aura_type = effect.effect_aura;
                        (effect_mask & bit != 0
                            && matches!(
                                aura_type,
                                wow_data::spell::aura_types::SPELL_AURA_MOD_THREAT
                                    | wow_data::spell::aura_types::SPELL_AURA_SCHOOL_IMMUNITY
                                    | wow_data::spell::aura_types::SPELL_AURA_DAMAGE_IMMUNITY
                                    | wow_data::spell::aura_types::SPELL_AURA_MOD_CONFUSE
                                    | wow_data::spell::aura_types::SPELL_AURA_MOD_STUN
                            ))
                        .then(|| {
                            let amount = represented_effect_amounts
                                .iter()
                                .find(|represented| {
                                    represented.effect_index == effect.effect_index as u8
                                })
                                .map_or_else(
                                    || effect.calc_value_no_caster_like_cpp(),
                                    |represented| represented.amount,
                                );
                            (bit, aura_type, amount, effect.effect_misc_value_1)
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        wow_entities::AuraThreatSnapshotLikeCpp::new(interrupt_flags, effects)
    }

    pub fn apply_player_threat_aura_for_consumer_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        slot: u8,
        effect_mask: u32,
        represented_effect_amounts: &[wow_entities::RepresentedAuraEffectAmountLikeCpp],
        spell_store: Option<&wow_data::SpellStore>,
        difficulty_store: Option<&wow_data::DifficultyStore>,
        consumer_test: bool,
    ) {
        let snapshot = self.aura_subsystem_snapshot_like_cpp()
            .and_then(|auras| auras.threat_snapshot_like_cpp(slot).cloned())
            .unwrap_or_else(|| {
                let difficulty = self.core.current_map_difficulty_id_like_cpp();
                Self::threat_aura_snapshot_from_stores_like_cpp(
                    spell_store, difficulty_store, spell_id, difficulty,
                    effect_mask, represented_effect_amounts,
                )
            });
        let Ok(spell_id) = u32::try_from(spell_id) else {
            return;
        };
        let _canonical = self.core.with_owned_player_mut_like_cpp(|player| {
            player.apply_player_threat_aura_like_cpp(
                spell_id, caster_guid, slot, snapshot.clone(),
            );
        }).is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if consumer_test && !_canonical && self.core.player_handle_like_cpp.is_none() {
            let _ = self.mutate_player_aura_subsystem_like_cpp(|auras| {
                auras.insert_threat_snapshot_like_cpp(slot, snapshot.clone());
                let interrupt_flags = snapshot.interrupt_flags();
                for &(effect_bit, aura_type, amount, misc_value) in snapshot.effects() {
                    let aura = wow_entities::AppliedAuraRef::new(
                        spell_id, caster_guid, slot, effect_bit,
                    );
                    auras.register_applied_aura(aura, None, interrupt_flags[0], interrupt_flags[1]);
                    auras.register_applied_aura_effect_like_cpp(aura, aura_type, amount, misc_value);
                }
            });
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = consumer_test;
    }

    pub fn remove_player_threat_aura_for_consumer_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        slot: u8,
        effect_mask: u32,
        consumer_test: bool,
    ) {
        let Ok(spell_id) = u32::try_from(spell_id) else {
            return;
        };
        let _canonical = self.core.with_owned_player_mut_like_cpp(|player| {
            player.remove_player_threat_aura_like_cpp(spell_id, caster_guid, slot, effect_mask);
        }).is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if consumer_test && !_canonical && self.core.player_handle_like_cpp.is_none() {
            let _ = self.mutate_player_aura_subsystem_like_cpp(|auras| {
                auras.remove_threat_snapshot_like_cpp(slot);
                for effect_index in 0..u32::BITS {
                    let effect_bit = 1u32 << effect_index;
                    if effect_mask & effect_bit != 0 {
                        auras.remove_applied(wow_entities::AppliedAuraRef::new(
                            spell_id, caster_guid, slot, effect_bit,
                        ));
                    }
                }
            });
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = consumer_test;
    }
}
