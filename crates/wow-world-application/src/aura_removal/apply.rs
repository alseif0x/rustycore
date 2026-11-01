// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::AuraRemovalCxLikeCpp;
use super::stat_application::select_aura_stat_application;
use std::time::Instant;
use wow_core::ObjectGuid;
use wow_entities::{
    AuraApplicationLikeCpp as AuraApplication, RepresentedAuraEffectAmountLikeCpp,
    RepresentedAuraEffectLikeCpp, legacy_aura_duration_pair,
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
        let Some((duration_total, duration_remaining)) =
            legacy_aura_duration_pair(duration_ms, duration_ms)
        else {
            return Err("Aura duration exceeds the signed application range");
        };
        // Find a free slot (0-254) on the canonical Unit owner.
        let slot =
            wow_world_spell::SessionSpellState::next_player_visible_aura_slot_with_access_like_cpp(
                &self.player,
            )
            .ok_or("No free aura slots or missing Player aura owner")?;

        // C++ applies the active effect subset, with 80's MiscValue and 137's
        // MiscValueB selecting which stat update handlers run. The sickness
        // spell id comes from this Player's ChrRaces row, never a fixed race id.
        let required_sickness = self.is_current_race_sickness_spell(spell_id) == Some(true);
        let selected_lifetime = required_sickness || matches!(spell_id, 8_326 | 20_584);
        let spell_store = self.spell_store.map(|store| store.as_ref());
        let stat_application = if required_sickness {
            super::stat_application::select_selected_aura_stat_application(
                spell_store,
                spell_id,
                self.map_difficulty_id,
                self.difficulty_store,
                effect_mask,
            )?
        } else {
            select_aura_stat_application(spell_store, spell_id, effect_mask, false)?
        };
        let is_ability = stat_application
            .as_ref()
            .is_some_and(|selection| selection.is_ability);
        let total_stat_percentage_effects =
            stat_application.as_ref().map_or(&[][..], |selection| {
                selection.total_stat_percentage_effects.as_slice()
            });
        let stat_cause = stat_application
            .as_ref()
            .and_then(|selection| selection.cause);
        if required_sickness && self.player.is_logged_in_like_cpp() {
            self.preflight_selected_sickness_stats()?;
        }
        let preserve_health_pct = matches!(
            stat_cause,
            Some(
                wow_world_core::session::PlayerStatApplicationCause::StatAura {
                    preserve_health_pct: true,
                    ..
                }
            )
        );
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
            duration_total,
            duration_remaining,
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
        let no_reagent_mask = self.no_reagent_mask_after_aura_apply_like_cpp(&aura)?;

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
        if let Some(mask) = no_reagent_mask {
            self.player
                .set_no_reagent_cost_mask_like_cpp(mask)
                .ok_or("Missing Player owner for NoReagentCostMask")?;
        }
        self.player.apply_player_threat_aura_for_consumer_like_cpp(
            spell_id,
            caster_guid,
            slot,
            effect_mask,
            required_sickness,
            selected_lifetime,
            &represented_effect_amounts,
            spell_store,
            self.difficulty_store,
            self.consumer_test,
        );

        if selected_lifetime {
            self.player
                .set_visible_aura_update_pending(slot, true)
                .ok_or("Missing Player aura owner")?;
        } else if send_update {
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
        }
        // C++ applies login/load auras while Player is not yet in world, then
        // folds their modifiers into UpdateAllStats and the initial
        // CreateObject. Do not publish a VALUES delta for a GUID the client
        // has not created yet. Selected sickness and dead/continued Players
        // update the canonical VALUES snapshot; unrelated Alive auras preserve
        // the established immediate stat packet.
        if let Some(cause) = stat_cause.filter(|_| self.player.is_logged_in_like_cpp()) {
            let use_values_application = self.stat_application_uses_values(required_sickness);
            let player = self.stats.reborrow_like_cpp(
                &self.player,
                #[cfg(any(test, feature = "test-fixtures"))]
                &*self.shapeshift_form,
            );
            let mut stats = wow_world_inventory::CharacterStatsApplicationCxLikeCpp::new(
                player,
                &*self.inventory,
                self.player.packet_publication_like_cpp(),
            );
            if use_values_application {
                if stats.apply_stats_for_values(cause).is_none() {
                    #[cfg(any(test, feature = "test-fixtures"))]
                    if self.player.player_handle_absent_like_cpp() {
                        stats.send_total_stat_percentage_update_like_cpp(
                            preserve_health_pct,
                            cause.damage_pct_operation(),
                        );
                    } else {
                        return Err("Unable to apply calculated Player stats");
                    }
                    #[cfg(not(any(test, feature = "test-fixtures")))]
                    return Err("Unable to apply calculated Player stats");
                }
            } else {
                stats.send_total_stat_percentage_update_like_cpp(
                    preserve_health_pct,
                    cause.damage_pct_operation(),
                );
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
