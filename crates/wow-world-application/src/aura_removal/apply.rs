// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::AuraRemovalCxLikeCpp;
use std::time::Instant;
use wow_core::ObjectGuid;
use wow_entities::{
    AuraApplicationLikeCpp as AuraApplication, RepresentedAuraEffectAmountLikeCpp,
    RepresentedAuraEffectLikeCpp,
};
use wow_world_core::session::SPELL_PVP_RULES_ENABLED_LIKE_CPP;

impl AuraRemovalCxLikeCpp<'_> {
    pub fn apply_aura_with_effect_mask_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        duration_ms: u32,
        aura_flags: u32,
        effect_mask: u32,
    ) -> Result<(), &'static str> {
        self.apply_aura_with_effect_mask_provenance_and_update_like_cpp(
            spell_id,
            caster_guid,
            duration_ms,
            aura_flags,
            effect_mask,
            wow_entities::AuraCastProvenanceLikeCpp::default(),
            true,
        )
    }

    pub fn apply_aura_with_effect_mask_provenance_and_update_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        duration_ms: u32,
        aura_flags: u32,
        effect_mask: u32,
        provenance: wow_entities::AuraCastProvenanceLikeCpp,
        send_update: bool,
    ) -> Result<(), &'static str> {
        // Find a free slot (0-254) on the canonical Unit owner.
        let slot =
            wow_world_spell::SessionSpellState::next_player_visible_aura_slot_with_access_like_cpp(
                &self.player,
            )
            .ok_or("No free aura slots or missing Player aura owner")?;

        // Preserve the represented StatSystem-relevant multiplier on the same
        // AuraApplication. C++ AuraEffect::HandleModTotalPercentStat uses
        // MiscValueB as a per-stat bitmask (zero means all stats), while the
        // generic AuraApplication continues to own the visible slot.
        let (is_ability, total_stat_percentage_effects) = self
            .spell_store
            .map(|store| {
                let effects = store
                    .get(spell_id)
                    .map(|spell| {
                        spell
                            .effects()
                            .iter()
                            .filter(|effect| {
                                1u32.checked_shl(effect.effect_index)
                                    .is_some_and(|bit| effect_mask & bit != 0)
                                    && effect.effect_aura
                                        == wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE
                            })
                            .map(|effect| {
                                (
                                    effect.effect_index,
                                    effect.calc_value_no_caster_like_cpp(),
                                    effect.effect_misc_value_1,
                                    effect.effect_misc_value_2,
                                )
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                (
                    store.has_attribute0_like_cpp(
                        spell_id,
                        wow_data::spell::attributes::SPELL_ATTR0_IS_ABILITY,
                    ),
                    effects,
                )
            })
            .unwrap_or_default();
        let modifies_total_stats = !total_stat_percentage_effects.is_empty();
        let preserve_health_pct = is_ability
            && total_stat_percentage_effects
                .iter()
                .any(|(_, _, _, stat_mask)| *stat_mask == 0 || *stat_mask & (1 << 2) != 0);
        let first_total_stat_percentage = total_stat_percentage_effects.first().copied();
        let (
            represented_effect,
            represented_amount,
            represented_misc_value,
            represented_multiplier,
        ) = if let Some((_, amount, _, stat_mask)) = first_total_stat_percentage {
            (
                Some(RepresentedAuraEffectLikeCpp::ModTotalStatPercentage),
                amount,
                Some(stat_mask),
                1.0 + amount as f32 / 100.0,
            )
        } else {
            (None, 0, None, 1.0)
        };
        let represented_effect_amounts: Vec<_> = total_stat_percentage_effects
            .iter()
            .filter_map(|(effect_index, amount, _, _)| {
                u8::try_from(*effect_index).ok().map(|effect_index| {
                    RepresentedAuraEffectAmountLikeCpp {
                        effect_index,
                        amount: *amount,
                    }
                })
            })
            .collect();

        // Create aura
        let aura = AuraApplication {
            spell_id,
            difficulty_id: self.player.aura_difficulty_like_cpp(),
            caster_guid,
            slot,
            duration_total: duration_ms,
            duration_remaining: duration_ms,
            stack_count: 1,
            aura_flags,
            effect_mask,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect,
            represented_amount,
            represented_effect_amounts: represented_effect_amounts.clone(),
            represented_misc_value,
            represented_multiplier,
            applied_at: Instant::now(),
        };

        if !self
            .spell
            .insert_player_visible_aura_with_access_and_provenance_like_cpp(
                &mut self.player,
                self.spell_store.map(|store| store.as_ref()),
                aura,
                provenance,
            )
        {
            return Err("Missing Player aura owner");
        }
        self.player.apply_player_threat_aura_for_consumer_like_cpp(
            spell_id,
            caster_guid,
            slot,
            effect_mask,
            &represented_effect_amounts,
            self.spell_store.map(|store| store.as_ref()),
            self.difficulty_store,
            self.consumer_test,
        );

        if send_update {
            self.spell.send_aura_update_applied_with_access_like_cpp(
                &self.player,
                spell_id,
                slot,
                caster_guid,
                duration_ms,
                aura_flags,
                effect_mask,
                #[cfg(any(test, feature = "test-fixtures"))]
                self.player_level,
            );
            // C++ applies login/load auras while Player is not yet in world,
            // then folds their modifiers into UpdateAllStats and the initial
            // CreateObject. Do not publish a VALUES delta for a GUID the
            // client has not created yet.
            if modifies_total_stats && self.player.is_logged_in_like_cpp() {
                let player = self.stats.reborrow_like_cpp(
                    &self.player,
                    #[cfg(any(test, feature = "test-fixtures"))]
                    &*self.shapeshift_form,
                );
                let mut stats = crate::CharacterStatsApplicationCxLikeCpp::new(
                    player,
                    &*self.inventory,
                    self.player.packet_publication_like_cpp(),
                );
                stats.send_total_stat_percentage_update_like_cpp(preserve_health_pct);
            }
        }
        if spell_id == SPELL_PVP_RULES_ENABLED_LIKE_CPP {
            let _ = self.item_scaling_phase_like_cpp();
        }
        // C++ `AuraEffect::HandleModAttackSpeed`/`HandleModMeleeSpeedPct`/
        // `HandleModCombatSpeedPct`/`HandleAuraModRangedHaste`
        // (`SpellAuraEffects.cpp:4353-4393`) reinstall the attack-time
        // multipliers on every apply.
        self.sync_attack_speed_phase_like_cpp(self.spell_store.map(|store| store.as_ref()));
        // C++ `AuraEffect::HandleAuraModShapeshift` -> `Player::InitDataForForm`
        // (`Player.cpp:22076-22098`) owns the form and recalcs its attack times
        // and damage.
        if let Some(mutation) = self.shapeshift_ownership_phase_like_cpp(spell_id) {
            self.sync_shapeshift_form_like_cpp(mutation);
        }
        self.display_power_phase_like_cpp(spell_id);

        Ok(())
    }
}
