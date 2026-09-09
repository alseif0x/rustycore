//! Represented melee attacks and swing timers.
//!
//! Moved out of the Session root under #617. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) fn canonical_player_attack_state_like_cpp(
        &self,
    ) -> Option<Option<ObjectGuid>> {
        let guid = self.player_guid?;
        let map_id = u32::from(self.player_map_id_like_cpp());
        let manager = Arc::clone(self.canonical_map_manager.as_ref()?);
        let manager = manager.lock().ok()?;
        let mut result = None;
        manager.do_for_all_maps_with_map_id(map_id, |managed| {
            if result.is_none()
                && let Some(player) = managed.map().get_typed_player(guid)
            {
                result = Some(player.unit().attacking());
            }
        });
        result
    }
    pub(crate) fn set_player_attack_swing_error_like_cpp(&mut self, error: Option<u8>) {
        use wow_packet::ServerPacket;
        use wow_packet::packets::combat::AttackSwingError;

        if let Some(reason) = error {
            if self.player_swing_error_msg_like_cpp != Some(reason) {
                let _ = self.send_tx().send(AttackSwingError { reason }.to_bytes());
            }
        }
        self.player_swing_error_msg_like_cpp = error;
    }
    pub(in crate::session) fn take_canonical_player_attack_swings_like_cpp(
        &mut self,
        diff_ms: u32,
        in_melee_range: bool,
        facing_target: bool,
        within_los: bool,
    ) -> Option<(Vec<u32>, Option<Option<u8>>)> {
        self.mutate_canonical_player_like_cpp(|player| {
            take_canonical_player_attack_swings_like_cpp(
                player,
                diff_ms,
                in_melee_range,
                facing_target,
                within_los,
            )
        })
        .flatten()
    }
    fn canonical_unit_attack_target_state_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> (bool, bool, wow_entities::UnitAttackContextLikeCpp) {
        // Resolve the Player-owned control state before taking the map lock;
        // visibility checks below run while that same manager is borrowed.
        let moved_unit_guid = self.player_moved_unit_guid_like_cpp();
        let current_group_guid = self.resolved_group_guid_like_cpp();
        let player_phase_shift = self.represented_player_phase_shift_like_cpp();
        let Some(manager) = self.canonical_map_manager.as_ref() else {
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
        let Some(map) = manager.find_map(u32::from(self.player_map_id_like_cpp()), 0) else {
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
            if let Some(reputation_snapshot) =
                self.attack_reputation_faction_snapshot_like_cpp(creature)
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
    fn player_vehicle_seat_allows_attack_like_cpp(&self) -> bool {
        let Some((seat_flags, _)) = self.player_vehicle_seat_state_like_cpp() else {
            return false;
        };
        match seat_flags {
            Some(flags) => flags & VEHICLE_SEAT_FLAG_CAN_ATTACK != 0,
            None => true,
        }
    }
    fn add_canonical_attacker_like_cpp(&mut self, victim: ObjectGuid, attacker: ObjectGuid) {
        if self
            .mutate_canonical_player_by_guid_like_cpp(victim, |victim| {
                victim.unit_mut().add_attacker_like_cpp(attacker)
            })
            .is_some()
        {
            return;
        }
        let _ = self.mutate_canonical_creature_by_guid_like_cpp(victim, |victim| {
            victim.unit_mut().add_attacker_like_cpp(attacker)
        });
    }
    pub(crate) fn start_player_attack_like_cpp(
        &mut self,
        victim: ObjectGuid,
    ) -> PlayerAttackStartLikeCppResult {
        let _ = self.ensure_canonical_world_map_for_current_player_like_cpp();
        let player_guid = self.player_guid();
        let Some((attacker_unit_flags, _, _)) = self.player_unit_presentation_snapshot_like_cpp()
        else {
            return PlayerAttackStartLikeCppResult::Rejected;
        };
        let attacker_is_mounted_player = attacker_unit_flags.contains(UnitFlags::MOUNT);
        let (victim_alive, victim_in_world, mut attack_context) =
            self.canonical_unit_attack_target_state_like_cpp(victim);
        if !self.player_vehicle_seat_allows_attack_like_cpp() {
            self.set_combat_target_like_cpp(None);
            self.set_in_combat_like_cpp(false);
            if self.selection_guid_like_cpp() == Some(victim) {
                self.set_selection_guid_like_cpp(None);
            }
            return PlayerAttackStartLikeCppResult::Rejected;
        }
        attack_context.attacker_is_mounted_player = attacker_is_mounted_player;
        attack_context.attacker_unit_flags = attacker_unit_flags.bits();
        attack_context.attacker_has_affecting_player = true;
        #[cfg_attr(not(test), allow(unused_mut))]
        let mut attacker_pvp_flags =
            self.with_owned_player_like_cpp(|player| player.unit().pvp_flags_like_cpp());
        #[cfg(test)]
        if attacker_pvp_flags.is_none() && self.player_handle_like_cpp.is_none() {
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
                .and_then(|guid| self.canonical_player_duel_in_progress_like_cpp(guid, victim))
                .unwrap_or(false);
        }
        attack_context.attacker_is_player_uber = player_guid
            .and_then(|guid| {
                self.canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_UBER_LIKE_CPP)
            })
            .unwrap_or(false);
        let combat_relation_represented = attack_context.relation_represented;
        let combat_attacker_is_friendly_to_victim = attack_context.attacker_is_friendly_to_victim;
        let combat_victim_is_friendly_to_attacker = attack_context.victim_is_friendly_to_attacker;
        self.set_selection_guid_like_cpp(Some(victim));
        let outcome = self.mutate_canonical_player_like_cpp(|player| {
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
                self.set_combat_target_like_cpp(None);
                self.set_in_combat_like_cpp(false);
                if self.selection_guid_like_cpp() == Some(victim) {
                    self.set_selection_guid_like_cpp(None);
                }
                return PlayerAttackStartLikeCppResult::Rejected;
            }
        };
        if let Some(player_guid) = player_guid {
            if let Some(previous) = previous {
                self.remove_canonical_attacker_like_cpp(previous, player_guid);
            }
            self.add_canonical_attacker_like_cpp(victim, player_guid);
            let _ = self.mutate_world_creature(victim, |victim| {
                victim
                    .creature
                    .unit_mut()
                    .add_attacker_like_cpp(player_guid);
            });
            let _ = self.begin_canonical_player_combat_ref_like_cpp(
                player_guid,
                victim,
                combat_relation_represented,
                combat_attacker_is_friendly_to_victim,
                combat_victim_is_friendly_to_attacker,
            );
        }
        self.set_in_combat_like_cpp(true);
        let send_attack_start = matches!(
            outcome,
            Some(
                wow_entities::UnitAttackStartOutcome::NewTarget { .. }
                    | wow_entities::UnitAttackStartOutcome::MeleeStartedSameTarget
            )
        );
        PlayerAttackStartLikeCppResult::Accepted { send_attack_start }
    }
    pub(crate) fn stop_player_attack_like_cpp(&mut self) -> Option<ObjectGuid> {
        let player_guid = self.player_guid()?;
        let target = match self.mutate_canonical_player_like_cpp(|player| {
            match player.unit_mut().attack_stop_like_cpp() {
                wow_entities::UnitAttackStopOutcome::Stopped { victim } => Some(victim),
                wow_entities::UnitAttackStopOutcome::NoVictim => None,
            }
        }) {
            Some(Some(victim)) => victim,
            // C++ Unit::AttackStop returns false when m_attacking is null; a
            // stale session mirror must not invent a victim when canonical
            // player state exists and says there is none.
            Some(None) => {
                self.set_combat_target_like_cpp(None);
                self.set_in_combat_like_cpp(false);
                return None;
            }
            None => self.resolved_combat_target_like_cpp().flatten()?,
        };
        self.set_combat_target_like_cpp(None);
        self.set_in_combat_like_cpp(false);
        if self.selection_guid_like_cpp() == Some(target) {
            self.set_selection_guid_like_cpp(None);
        }
        self.remove_canonical_attacker_like_cpp(target, player_guid);
        let _ = self.mutate_world_creature(target, |victim| {
            victim
                .creature
                .unit_mut()
                .remove_attacker_like_cpp(player_guid);
        });
        Some(target)
    }
    pub(crate) fn player_class_attack_power_coefficients_like_cpp(
        &self,
        class: u8,
    ) -> Option<(u8, u8, u8)> {
        self.chr_classes_store
            .as_ref()?
            .get(u32::from(class))
            .map(|entry| {
                (
                    entry.attack_power_per_strength,
                    entry.attack_power_per_agility,
                    entry.ranged_attack_power_per_agility,
                )
            })
    }
}
