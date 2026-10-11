//! Represented melee attacks and swing timers.
//!
//! Moved out of the Session root under #617. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

/// One resolved white swing: the damage after every C++
/// `Unit::CalculateMeleeDamage` stage the represented model implements, plus the
/// hit outcome `SMSG_ATTACKERSTATEUPDATE` publishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::session) struct RepresentedMeleeSwingLikeCpp {
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

impl RepresentedMeleeSwingLikeCpp {
    /// A plain landed hit, for the paths that resolve their own damage without
    /// the represented attack table (the creature-owned swing).
    pub(in crate::session) fn hit_like_cpp(damage: u32) -> Self {
        let (hit_info, victim_state) = crate::session_rules::melee_outcome_presentation_like_cpp(
            crate::session_rules::RepresentedMeleeOutcomeLikeCpp::Hit,
            false,
        );
        Self {
            damage,
            original_damage: damage,
            blocked: 0,
            hit_info,
            victim_state,
        }
    }
}

/// C++ `Unit::CalcArmorReducedDamage` inputs the swing owner resolves once per
/// victim. `NONE` means "no represented armour": every field is zero, so the
/// reduction is zero and the swing damage is unchanged.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(in crate::session) struct RepresentedArmorMitigationLikeCpp {
    pub attacker_level: u8,
    pub victim_level: u8,
    pub victim_armor: i32,
    pub armor_penetration_pct: f32,
    pub target_resistance_normal_aura: i32,
    /// The attacker's `SPELL_AURA_MOD_IGNORE_TARGET_RESIST` (269) sum for
    /// `SPELL_SCHOOL_MASK_NORMAL`.
    pub ignore_target_resist_normal_pct: f32,
    /// The victim's `SPELL_AURA_BYPASS_ARMOR_FOR_CASTER` (345) sum for effects
    /// this attacker cast.
    pub bypass_armor_pct_by_caster: f32,
}

impl RepresentedArmorMitigationLikeCpp {
    pub(in crate::session) const NONE: Self = Self {
        attacker_level: 0,
        victim_level: 0,
        victim_armor: 0,
        armor_penetration_pct: 0.0,
        target_resistance_normal_aura: 0,
        ignore_target_resist_normal_pct: 0.0,
        bypass_armor_pct_by_caster: 0.0,
    };
}

impl WorldSession {
    fn canonical_unit_attack_target_state_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> (bool, bool, wow_entities::UnitAttackContextLikeCpp) {
        // Resolve the Player-owned control state before taking the map lock;
        // visibility checks below run while that same manager is borrowed.
        let moved_unit_guid = crate::session::hub_ref(self).player_moved_unit_guid_like_cpp();
        let current_group_guid = self.resolved_group_guid_like_cpp();
        let player_phase_shift = self.represented_player_phase_shift_like_cpp();
        let Some(manager) = self.core.canonical_map_manager.as_ref() else {
            return (
                true,
                true,
                wow_entities::UnitAttackContextLikeCpp::default(),
            );
        };
        let Ok(manager) = manager.lock() else {
            return (
                true,
                true,
                wow_entities::UnitAttackContextLikeCpp::default(),
            );
        };
        let Some(map) = manager.find_map(u32::from(self.core.player_map_id_like_cpp()), 0) else {
            return (
                true,
                true,
                wow_entities::UnitAttackContextLikeCpp::default(),
            );
        };
        if let Some(player) = map.map().get_typed_player(guid) {
            let pvp_flags = player.unit().pvp_flags_like_cpp();
            let attacker_can_see_or_detect_target = player_phase_shift
                .as_ref()
                .is_some_and(|phase| phase.can_see(player.unit().world().phase_shift()))
                && self
                    .player_guid()
                    .and_then(|attacker_guid| map.map().get_typed_player(attacker_guid))
                    .map(|attacker| {
                        let mut target_unit = player.unit().clone();
                        let mut seer_unit = attacker.unit().clone();
                        self.apply_target_visibility_context_for_current_player_like_cpp(
                            &mut target_unit,
                            &mut seer_unit,
                            moved_unit_guid,
                            current_group_guid,
                        );
                        seer_unit.can_see_or_detect_unit_like_cpp(&target_unit, false, true, false)
                    })
                    .unwrap_or(true);
            return (
                player.unit().is_alive(),
                player.unit().world().object().is_in_world(),
                wow_entities::UnitAttackContextLikeCpp {
                    victim_is_game_master_player: player.is_game_master_like_cpp(),
                    victim_unit_state: player.unit().unit_state(),
                    victim_unit_flags: player.unit().unit_flags_like_cpp().bits(),
                    victim_has_affecting_player: true,
                    visibility_represented: true,
                    attacker_can_see_or_detect_target,
                    victim_in_sanctuary: pvp_flags.contains(UnitPvpFlags::SANCTUARY),
                    victim_is_pvp: pvp_flags.contains(UnitPvpFlags::PVP),
                    victim_is_ffa_pvp: pvp_flags.contains(UnitPvpFlags::FFA_PVP),
                    victim_has_pvp_unk1_flag: pvp_flags.contains(UnitPvpFlags::UNK1),
                    ..Default::default()
                },
            );
        }
        if let Some(result) = map.map().with_creature_like_cpp(guid, |creature| {
            let attacker_can_see_or_detect_target = player_phase_shift
                .as_ref()
                .is_some_and(|phase| phase.can_see(creature.unit().world().phase_shift()))
                && self
                    .player_guid()
                    .and_then(|attacker_guid| map.map().get_typed_player(attacker_guid))
                    .map(|attacker| {
                        let mut target_unit = creature.unit().clone();
                        let mut seer_unit = attacker.unit().clone();
                        self.apply_target_visibility_context_for_current_player_like_cpp(
                            &mut target_unit,
                            &mut seer_unit,
                            moved_unit_guid,
                            current_group_guid,
                        );
                        seer_unit.can_see_or_detect_unit_like_cpp(&target_unit, false, true, false)
                    })
                    .unwrap_or(true);
            let mut context = wow_entities::UnitAttackContextLikeCpp {
                victim_is_evading_creature: creature.is_evading_attacks_like_cpp(),
                victim_unit_state: creature.unit().unit_state(),
                victim_unit_flags: creature.unit().unit_flags_like_cpp().bits(),
                visibility_represented: true,
                attacker_can_see_or_detect_target,
                ..Default::default()
            };
            if let Some(reputation_snapshot) = self
                .catalogs
                .attack_reputation_faction_snapshot_like_cpp(creature)
                && let Some(attacker_guid) = self.player_guid()
                && let Some(attacker) = map.map().get_typed_player(attacker_guid)
            {
                let player_has_contested_pvp_flag =
                    attacker.has_player_flag(PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP);
                let creature_has_forced_reputation_rank =
                    attacker.has_forced_reputation_rank_like_cpp(reputation_snapshot.faction_id);
                let player_has_reputation_state = match reputation_snapshot.can_have_reputation {
                    Some(false) => false,
                    Some(true) => {
                        attacker.has_reputation_state_like_cpp(reputation_snapshot.faction_id)
                    }
                    None => true,
                };
                if (reputation_snapshot.contested_guard && player_has_contested_pvp_flag)
                    || creature_has_forced_reputation_rank
                    || player_has_reputation_state
                {
                    context.player_creature_reputation_represented = true;
                    context.creature_is_contested_guard = reputation_snapshot.contested_guard;
                    context.player_has_contested_pvp_flag = player_has_contested_pvp_flag;
                    context.creature_has_forced_reputation_rank =
                        creature_has_forced_reputation_rank;
                    context.player_at_war_with_creature_faction = player_has_reputation_state
                        && attacker.is_at_war_with_faction_like_cpp(reputation_snapshot.faction_id);
                }
            }
            (
                creature.is_alive(),
                creature.unit().world().object().is_in_world(),
                context,
            )
        }) {
            return result;
        }
        (
            true,
            true,
            wow_entities::UnitAttackContextLikeCpp::default(),
        )
    }
    pub(crate) fn start_player_attack_like_cpp(
        &mut self,
        victim: ObjectGuid,
    ) -> PlayerAttackStartLikeCppResult {
        let _ = self.ensure_canonical_world_map_for_current_player_like_cpp();
        let player_guid = self.player_guid();
        let Some((attacker_unit_flags, _, _)) =
            crate::session::hub_ref(self).player_unit_presentation_snapshot_like_cpp()
        else {
            return PlayerAttackStartLikeCppResult::Rejected;
        };
        let attacker_is_mounted_player = attacker_unit_flags.contains(UnitFlags::MOUNT);
        let (victim_alive, victim_in_world, mut attack_context) =
            self.canonical_unit_attack_target_state_like_cpp(victim);
        if !crate::session::hub_ref(self).player_vehicle_seat_allows_attack_like_cpp() {
            crate::session::hub_mut(self).set_combat_target_like_cpp(None);
            crate::session::hub_mut(self).set_in_combat_like_cpp(false);
            if crate::session::hub_ref(self).selection_guid_like_cpp() == Some(victim) {
                crate::session::hub_mut(self).set_selection_guid_like_cpp(None);
            }
            return PlayerAttackStartLikeCppResult::Rejected;
        }
        attack_context.attacker_is_mounted_player = attacker_is_mounted_player;
        attack_context.attacker_unit_flags = attacker_unit_flags.bits();
        attack_context.attacker_has_affecting_player = true;
        #[cfg_attr(not(test), allow(unused_mut))]
        let mut attacker_pvp_flags = self
            .core
            .with_owned_player_like_cpp(|player| player.unit().pvp_flags_like_cpp());
        #[cfg(test)]
        if attacker_pvp_flags.is_none() && self.core.player_handle_like_cpp.is_none() {
            attacker_pvp_flags =
                player_guid.and_then(|guid| self.canonical_player_pvp_flags_like_cpp(guid));
        }
        let attacker_pvp_flags = attacker_pvp_flags.unwrap_or_default();
        attack_context.attacker_in_sanctuary = attacker_pvp_flags.contains(UnitPvpFlags::SANCTUARY);
        attack_context.attacker_is_ffa_pvp = attacker_pvp_flags.contains(UnitPvpFlags::FFA_PVP);
        attack_context.attacker_has_pvp_unk1_flag = attacker_pvp_flags.contains(UnitPvpFlags::UNK1);
        if attack_context.victim_has_affecting_player {
            attack_context.sanctuary_represented = true;
            attack_context.pvp_represented = true;
            attack_context.player_player_duel_in_progress = player_guid
                .and_then(|guid| {
                    let (s, h) = crate::session::split_social_ref(self);
                    s.canonical_player_duel_in_progress_like_cpp(h, guid, victim)
                })
                .unwrap_or(false);
        }
        attack_context.attacker_is_player_uber = player_guid
            .and_then(|guid| {
                self.core
                    .canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_UBER_LIKE_CPP)
            })
            .unwrap_or(false);
        let combat_relation_represented = attack_context.relation_represented;
        let combat_attacker_is_friendly_to_victim = attack_context.attacker_is_friendly_to_victim;
        let combat_victim_is_friendly_to_attacker = attack_context.victim_is_friendly_to_attacker;
        crate::session::hub_mut(self).set_selection_guid_like_cpp(Some(victim));
        let outcome = self.core.mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().attack_with_context_like_cpp(
                victim,
                victim_alive,
                victim_in_world,
                true,
                attack_context,
            )
        });
        let previous = match outcome {
            Some(wow_entities::UnitAttackStartOutcome::NewTarget { previous }) => previous,
            Some(
                wow_entities::UnitAttackStartOutcome::MeleeStartedSameTarget
                | wow_entities::UnitAttackStartOutcome::MeleeStoppedSameTarget
                | wow_entities::UnitAttackStartOutcome::NoChangeSameTarget,
            ) => None,
            Some(
                wow_entities::UnitAttackStartOutcome::InvalidSelfTarget
                | wow_entities::UnitAttackStartOutcome::InvalidDeadAttacker
                | wow_entities::UnitAttackStartOutcome::InvalidDeadVictim
                | wow_entities::UnitAttackStartOutcome::InvalidVictimNotInWorld
                | wow_entities::UnitAttackStartOutcome::InvalidMountedAttacker
                | wow_entities::UnitAttackStartOutcome::InvalidAttackerEvading
                | wow_entities::UnitAttackStartOutcome::InvalidVictimGameMaster
                | wow_entities::UnitAttackStartOutcome::InvalidVictimEvading
                | wow_entities::UnitAttackStartOutcome::InvalidAttackTarget,
            )
            | None => {
                crate::session::hub_mut(self).set_combat_target_like_cpp(None);
                crate::session::hub_mut(self).set_in_combat_like_cpp(false);
                if crate::session::hub_ref(self).selection_guid_like_cpp() == Some(victim) {
                    crate::session::hub_mut(self).set_selection_guid_like_cpp(None);
                }
                return PlayerAttackStartLikeCppResult::Rejected;
            }
        };
        if let Some(player_guid) = player_guid {
            if let Some(previous) = previous {
                crate::session::hub_mut(self)
                    .remove_canonical_attacker_like_cpp(previous, player_guid);
            }
            crate::session::hub_mut(self).add_canonical_attacker_like_cpp(victim, player_guid);
            let _ = self.core.mutate_world_creature(victim, |victim| {
                victim.unit_mut().add_attacker_like_cpp(player_guid);
            });
            let _ = crate::session::hub_mut(self).begin_canonical_player_combat_ref_like_cpp(
                player_guid,
                victim,
                combat_relation_represented,
                combat_attacker_is_friendly_to_victim,
                combat_victim_is_friendly_to_attacker,
            );
        }
        crate::session::hub_mut(self).set_in_combat_like_cpp(true);
        let send_attack_start = matches!(
            outcome,
            Some(
                wow_entities::UnitAttackStartOutcome::NewTarget { .. }
                    | wow_entities::UnitAttackStartOutcome::MeleeStartedSameTarget
            )
        );
        PlayerAttackStartLikeCppResult::Accepted { send_attack_start }
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/combat/melee/test_only.rs"]
mod test_only;
