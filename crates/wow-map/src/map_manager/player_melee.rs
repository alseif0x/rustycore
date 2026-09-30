//! Player melee applied to the existing live creature owner.
//!
//! This moves the represented damage transition without changing readiness,
//! timers, RNG, deferred kill hooks or the legacy/canonical ownership bridge.

use super::{ObjectGuid, Position, WorldCreature};

/// One resolved white swing, including the original combat-log presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerMeleeSwing {
    pub damage: u32,
    /// C++ `CalcDamageInfo::OriginalDamage`.
    pub original_damage: u32,
    /// C++ `CalcDamageInfo::Blocked`.
    pub blocked: u32,
    /// C++ `CalcDamageInfo::HitInfo`.
    pub hit_info: u32,
    /// C++ `CalcDamageInfo::TargetState`.
    pub victim_state: u8,
}

impl PlayerMeleeSwing {
    /// Existing plain-hit constructor retained for the legacy adapter caller.
    pub fn hit_like_cpp(damage: u32) -> Self {
        Self {
            damage,
            original_damage: damage,
            blocked: 0,
            // a5f8da2e Unit.cpp:1434-1436 / Unit.h:48: plain landed hit.
            hit_info: 0x0000_0002,
            victim_state: 1,
        }
    }
}

/// Owned results; packet construction and deferred kill work stay with APP.
#[derive(Clone, Debug)]
pub struct PlayerMeleeCreatureHit {
    /// `(damage, killed, over_damage)` per swing, in swing order.
    pub swings: Vec<(u32, bool, i32)>,
    /// `(hit_info, victim_state, blocked, original_damage)` per swing,
    /// index-aligned with `swings`.
    pub swing_presentations: Vec<(u32, u8, u32, u32)>,
    pub entry: u32,
    pub level: u8,
    pub died: bool,
    pub move_stop: Option<(Position, u32)>,
    pub values_update: wow_entities::UnitValuesUpdate,
}

impl WorldCreature {
    /// Apply the original ordered player swing batch to this live creature.
    ///
    /// `Some`, including an empty slice, bypasses the creature swing timer and
    /// RNG. `None` retains the represented creature-owned fallback. This does
    /// not advance the runtime clock or complete deferred death/loot hooks.
    pub fn apply_player_melee(
        &mut self,
        player_guid: ObjectGuid,
        tap_group_guids: &[ObjectGuid],
        canonical_swings: Option<&[PlayerMeleeSwing]>,
    ) -> Option<PlayerMeleeCreatureHit> {
        if !self.is_alive() {
            return None;
        }
        if self.state() != wow_entities::CreatureAiState::InCombat {
            self.enter_combat(player_guid);
        }
        let damages: Vec<PlayerMeleeSwing> = match canonical_swings {
            Some(swings) => swings.to_vec(),
            None => {
                if !self.can_swing() {
                    return None;
                }
                vec![PlayerMeleeSwing::hit_like_cpp(self.roll_damage()?.max(1))]
            }
        };
        let entry = self.entry();
        let level = self.level();
        let mut swings = Vec::new();
        let mut swing_presentations = Vec::new();
        let mut died = false;
        let mut move_stop = None;
        for swing in damages {
            if !self.is_alive() {
                break;
            }
            let damage = swing.damage;
            // C++ `DealMeleeDamage` applies nothing for a missed or avoided swing:
            // no damage, no tap and no threat.
            if damage == 0 {
                swings.push((0, false, -1));
                swing_presentations.push((
                    swing.hit_info,
                    swing.victim_state,
                    swing.blocked,
                    swing.original_damage,
                ));
                continue;
            }
            let health_before = self.current_hp();
            self.creature
                .set_tapped_by_player(player_guid, tap_group_guids);
            died = self.take_damage_before_death_state_like_cpp(damage);
            let over_damage = if died {
                damage.saturating_sub(health_before) as i32
            } else {
                -1
            };
            self.creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .add_threat(player_guid, damage as f32);
            swings.push((damage, died, over_damage));
            swing_presentations.push((
                swing.hit_info,
                swing.victim_state,
                swing.blocked,
                swing.original_damage,
            ));
            if died {
                let combat = &mut self.creature.unit_mut().subsystems_mut().combat;
                combat.clear_threat();
                combat.clear_attackers();
                move_stop = self
                    .stop_move_spline_like_cpp()
                    .map(|stop| (stop.position, stop.spline_id));
                break;
            }
        }
        if canonical_swings.is_none() {
            self.record_swing();
        }
        let values_update = self.creature.unit().values_update();
        Some(PlayerMeleeCreatureHit {
            swings,
            swing_presentations,
            entry,
            level,
            died,
            move_stop,
            values_update,
        })
    }
}

#[cfg(test)]
mod tests;
