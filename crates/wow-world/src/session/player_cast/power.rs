//! Canonical resource checks and consumption for represented cast execution.

use super::*;
use num_traits::FromPrimitive;
use std::sync::OnceLock;
use wow_packet::packets::spell::SpellCastVisual;

const SPELL_POWER_TRACE_ENV_LIKE_CPP: &str = "RUSTYCORE_SPELL_POWER_TRACE";

fn spell_power_trace_enabled_like_cpp() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var(SPELL_POWER_TRACE_ENV_LIKE_CPP)
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
            .unwrap_or(false)
    })
}

impl WorldSession {
    fn spell_power_cost_snapshot_like_cpp(
        &mut self,
        spell_info: &wow_data::SpellInfo,
        cast_id: ObjectGuid,
        spell_id: i32,
        phase: &'static str,
    ) -> Option<(
        i32,
        Vec<wow_data::SpellPowerCostLikeCpp>,
        Vec<(i8, i32, i32)>,
    )> {
        let trace_spell_power = spell_power_trace_enabled_like_cpp();
        let Some((caster_create_mana, power_costs, before_power)) = self
            .mutate_canonical_player_like_cpp(|player| {
                let caster_create_mana = player.unit().get_create_mana_like_cpp();
                let power_costs = spell_info.calc_power_costs_like_cpp(caster_create_mana);
                let before_power = power_costs
                    .iter()
                    .filter_map(|cost| {
                        let power_type = PowerType::from_i8(cost.power_type)?;
                        Some((
                            cost.power_type,
                            player.get_power(power_type),
                            player.get_max_power(power_type),
                        ))
                    })
                    .collect::<Vec<_>>();
                (caster_create_mana, power_costs, before_power)
            })
        else {
            if trace_spell_power {
                info!(
                    "RUST_SPELL_POWER_COST phase={} spell_id={} spell_info_id={} cast_id={:?} result=no_canonical_player rows={:?}",
                    phase, spell_id, spell_info.spell_id, cast_id, spell_info.power_costs
                );
            }
            return None;
        };

        if trace_spell_power {
            info!(
                "RUST_SPELL_POWER_COST phase={} spell_id={} spell_info_id={} cast_id={:?} base_mana={} rows={:?} calculated_costs={:?} before_power={:?}",
                phase,
                spell_id,
                spell_info.spell_id,
                cast_id,
                caster_create_mana,
                spell_info.power_costs,
                power_costs,
                before_power
            );
        }

        Some((caster_create_mana, power_costs, before_power))
    }

    fn send_spell_power_no_power_like_cpp(
        &mut self,
        cast_id: ObjectGuid,
        spell_id: i32,
        visual: &SpellCastVisual,
    ) {
        self.send_packet(&CastFailed {
            cast_id,
            spell_id,
            visual: visual.clone(),
            reason: SpellCastResult::NoPower as i32,
            fail_arg1: 0,
            fail_arg2: 0,
        });
    }

    pub(crate) fn check_spell_power_like_cpp(
        &mut self,
        spell_info: &wow_data::SpellInfo,
        cast_id: ObjectGuid,
        spell_id: i32,
        visual: &SpellCastVisual,
    ) -> bool {
        let trace_spell_power = spell_power_trace_enabled_like_cpp();
        let Some((caster_create_mana, power_costs, before_power)) =
            self.spell_power_cost_snapshot_like_cpp(spell_info, cast_id, spell_id, "check")
        else {
            if spell_info.power_costs.is_empty() {
                return true;
            }
            self.send_spell_power_no_power_like_cpp(cast_id, spell_id, visual);
            return false;
        };

        if power_costs.is_empty() {
            if trace_spell_power {
                info!(
                    "RUST_SPELL_POWER_COST phase=check spell_id={} cast_id={:?} result=no_represented_cost base_mana={}",
                    spell_id, cast_id, caster_create_mana
                );
            }
            return true;
        }

        if !crate::session_rules::represented_spell_power_has_power_like_cpp(
            &power_costs,
            &before_power,
        ) {
            if trace_spell_power {
                info!(
                    "RUST_SPELL_POWER_COST phase=check spell_id={} cast_id={:?} result=no_power costs={:?} before_power={:?}",
                    spell_id, cast_id, power_costs, before_power
                );
            }
            self.send_spell_power_no_power_like_cpp(cast_id, spell_id, visual);
            return false;
        }

        true
    }

    pub(crate) fn take_spell_power_like_cpp(
        &mut self,
        spell_info: &wow_data::SpellInfo,
        cast_id: ObjectGuid,
        spell_id: i32,
        visual: &SpellCastVisual,
    ) -> bool {
        let trace_spell_power = spell_power_trace_enabled_like_cpp();
        let Some((caster_create_mana, power_costs, before_power)) =
            self.spell_power_cost_snapshot_like_cpp(spell_info, cast_id, spell_id, "take")
        else {
            if spell_info.power_costs.is_empty() {
                return true;
            }
            self.send_spell_power_no_power_like_cpp(cast_id, spell_id, visual);
            return false;
        };

        if power_costs.is_empty() {
            if trace_spell_power {
                info!(
                    "RUST_SPELL_POWER_COST phase=take spell_id={} cast_id={:?} result=no_represented_cost base_mana={}",
                    spell_id, cast_id, caster_create_mana
                );
            }
            return true;
        }

        if !crate::session_rules::represented_spell_power_has_power_like_cpp(
            &power_costs,
            &before_power,
        ) {
            if trace_spell_power {
                info!(
                    "RUST_SPELL_POWER_COST phase=take spell_id={} cast_id={:?} result=no_power costs={:?} before_power={:?}",
                    spell_id, cast_id, power_costs, before_power
                );
            }
            self.send_spell_power_no_power_like_cpp(cast_id, spell_id, visual);
            return false;
        }

        let update = self
            .mutate_canonical_player_like_cpp(|player| {
                // The map update can change resources after the diagnostic snapshot.
                // Recompute and admit the complete debit under the same owner guard
                // as its writes; never launch from a stale successful power check.
                let power_costs =
                    spell_info.calc_power_costs_like_cpp(player.unit().get_create_mana_like_cpp());
                let current_power: Vec<_> = power_costs
                    .iter()
                    .filter_map(|cost| {
                        let power = PowerType::from_i8(cost.power_type)?;
                        Some((
                            cost.power_type,
                            player.get_power(power),
                            player.get_max_power(power),
                        ))
                    })
                    .collect();
                if !crate::session_rules::represented_spell_power_has_power_like_cpp(
                    &power_costs,
                    &current_power,
                ) {
                    return None;
                }
                for cost in &power_costs {
                    let Some(power_type) = PowerType::from_i8(cost.power_type) else {
                        continue;
                    };
                    if matches!(
                        power_type,
                        PowerType::Health | PowerType::None | PowerType::Max
                    ) {
                        continue;
                    }
                    let current = player.get_power(power_type);
                    player
                        .unit_mut()
                        .set_power(power_type, current.saturating_sub(cost.amount));
                }
                let after_power = power_costs
                    .iter()
                    .filter_map(|cost| {
                        let power_type = PowerType::from_i8(cost.power_type)?;
                        Some((
                            cost.power_type,
                            player.unit().get_power_index(power_type),
                            player.get_power(power_type),
                            player.get_max_power(power_type),
                        ))
                    })
                    .collect::<Vec<_>>();
                Some((player.values_update(true), after_power))
            })
            .flatten();
        if let Some((update, after_power)) = update {
            #[cfg(test)]
            for (_, slot, current, max) in &after_power {
                if let Some(slot) = slot {
                    self.set_represented_player_power_slot_like_cpp(*slot, *current, Some(*max));
                }
            }
            if trace_spell_power {
                info!(
                    "RUST_SPELL_POWER_COST phase=take spell_id={} cast_id={:?} result=deducted costs={:?} before_power={:?} after_power={:?}",
                    spell_id, cast_id, power_costs, before_power, after_power
                );
            }
            self.send_player_values_update_like_cpp(&update);
        } else {
            if trace_spell_power {
                info!(
                    "RUST_SPELL_POWER_COST phase=take spell_id={} cast_id={:?} result=rejected_at_canonical_debit costs={:?}",
                    spell_id, cast_id, power_costs
                );
            }
            self.send_spell_power_no_power_like_cpp(cast_id, spell_id, visual);
            return false;
        }

        true
    }
}
