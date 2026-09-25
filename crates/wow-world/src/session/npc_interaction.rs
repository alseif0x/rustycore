// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Npc interaction: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{CreatureTypeFlags, ObjectGuid, PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP};
use super::{PlayerInteractionDataLikeCpp, Position, RepresentedCreatureAccessLikeCpp};
use super::{RepresentedGetReactionInputLikeCpp, UnitFlags2, WorldSession, canonical_access};

impl WorldSession {
    pub(crate) fn represented_npc_can_interact_with_like_cpp(
        &self,
        guid: ObjectGuid,
        npc_flags: u32,
        npc_flags2: u32,
    ) -> Option<RepresentedCreatureAccessLikeCpp> {
        if guid.is_empty() || !guid.is_any_type_creature() {
            return None;
        }
        let player_guid = self.player_guid()?;
        let player_position = self.player_position_like_cpp()?;
        let target_player_contested_pvp = self
            .canonical_player_has_player_flag_like_cpp(
                player_guid,
                PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP,
            )
            .unwrap_or(false);
        let player_faction_template_id = self.player_faction_template_id_like_cpp();
        let player_interaction_combat_reach = self.player_interaction_combat_reach_like_cpp();
        if self.resolved_is_in_taxi_flight_like_cpp() != Some(false) {
            return None;
        }

        let player_map_key = self
            .current_canonical_player_map_key_like_cpp()
            .unwrap_or_else(|| wow_map::MapKey::new(u32::from(self.player_map_id_like_cpp()), 0));
        let mut canonical_record_found_like_cpp = false;
        let mut canonical_fail_closed_like_cpp = false;
        let mut canonical_reaction_input_like_cpp = None;
        let canonical_access = (|| {
            let manager = self.canonical_map_manager.as_ref()?;
            let Ok(manager) = manager.lock() else {
                return None;
            };
            let map = manager.find_map(player_map_key.map_id, player_map_key.instance_id)?;
            let target_player_faction_template_id = player_faction_template_id.or_else(|| {
                map.map()
                    .get_typed_player(player_guid)
                    .and_then(|player| u32::try_from(player.unit().data().faction_template).ok())
                    .filter(|faction| *faction != 0)
            });
            let player_is_alive = if let Some(canonical_player) =
                map.map().get_typed_player(player_guid)
            {
                if !canonical_player.unit().world().object().is_in_world() {
                    canonical_fail_closed_like_cpp = true;
                    return None;
                }
                #[cfg(test)]
                let is_alive = if canonical_player.unit().data().max_health == 0
                    && self.player_handle_like_cpp.is_none()
                {
                    self.player_alive_like_cpp && self.player_health_like_cpp > 0
                } else {
                    canonical_player.unit().is_alive() && canonical_player.unit().data().health > 0
                };
                #[cfg(not(test))]
                let is_alive =
                    canonical_player.unit().is_alive() && canonical_player.unit().data().health > 0;
                is_alive
            } else {
                #[cfg(test)]
                {
                    if self.player_handle_like_cpp.is_some() {
                        canonical_fail_closed_like_cpp = true;
                        return None;
                    }
                    self.player_alive_like_cpp && self.player_health_like_cpp > 0
                }
                #[cfg(not(test))]
                {
                    canonical_fail_closed_like_cpp = true;
                    return None;
                }
            };
            canonical_record_found_like_cpp = map.map().contains_map_object_like_cpp(guid);
            map.map()
                .with_creature_or_pet_like_cpp(guid, |creature, _| {
                    let type_flags = CreatureTypeFlags::from_bits_retain(
                        creature.lifecycle_metadata().type_flags,
                    );
                    if !player_is_alive
                        && !type_flags.contains(CreatureTypeFlags::VISIBLE_TO_GHOSTS)
                    {
                        return None;
                    }
                    if !creature.is_alive()
                        && !type_flags.contains(CreatureTypeFlags::INTERACT_WHILE_DEAD)
                    {
                        return None;
                    }
                    if (npc_flags != 0 || npc_flags2 != 0)
                        && (creature.ai_ownership().npc_flags & npc_flags) == 0
                        && (creature.ai_ownership().npc_flags2 & npc_flags2) == 0
                    {
                        return None;
                    }
                    if creature.unit().subsystems().control.charmer_guid.is_some() {
                        return None;
                    }
                    let unit_flags2 = creature.unit().unit_flags2_like_cpp();
                    if !unit_flags2.contains(UnitFlags2::INTERACT_WHILE_HOSTILE) {
                        canonical_reaction_input_like_cpp =
                            Some(RepresentedGetReactionInputLikeCpp {
                                self_faction_template_id: creature
                                    .unit()
                                    .data()
                                    .faction_template
                                    .max(0)
                                    as u32,
                                target_faction_template_id: target_player_faction_template_id?,
                                same_object: false,
                                attackable_by_summoner: false,
                                same_charmer_or_owner_or_self: false,
                                self_has_player_owner: false,
                                target_has_player_owner: true,
                                target_player_owner_is_current_session: true,
                                target_owner_forced_rank_for_self: None,
                                same_player_owner: false,
                                duel_in_progress: false,
                                same_raid: false,
                                self_unit_player_controlled: false,
                                target_unit_player_controlled: true,
                                self_ffa_pvp: false,
                                target_ffa_pvp: false,
                                self_ignores_reputation: false,
                                target_ignores_reputation: false,
                                target_is_unit: true,
                                target_player_contested_pvp,
                            });
                    }

                    let interaction_distance = creature.unit().world().combat_reach() + 4.0;
                    let in_range = if let Some(player) = map.map().get_typed_player(player_guid) {
                        creature.unit().world().is_within_dist_in_map(
                            player.unit().world(),
                            interaction_distance,
                            true,
                        )
                    } else {
                        let fallback_distance = interaction_distance
                            + creature.unit().world().combat_reach()
                            + player_interaction_combat_reach;
                        creature
                            .unit()
                            .world()
                            .position()
                            .is_within_dist(&player_position, fallback_distance)
                    };
                    if !in_range {
                        return None;
                    }

                    Some(RepresentedCreatureAccessLikeCpp {
                        entry: creature.entry(),
                        position: creature.unit().world().position(),
                        npc_flags: creature.ai_ownership().npc_flags,
                        npc_flags2: creature.ai_ownership().npc_flags2,
                        trainer_class: creature.trainer_class_like_cpp(),
                        faction_template_id: creature.unit().data().faction_template.max(0) as u32,
                    })
                })
                .flatten()
        })();
        if let Some(canonical_access) = canonical_access {
            // Reputation is Player-owned and resolves through the same
            // canonical manager. Evaluate it only after releasing the map
            // guard held by the lookup closure above.
            if canonical_reaction_input_like_cpp.is_some_and(|input| {
                self.represented_get_reaction_to_like_cpp(input)
                    <= wow_data::reputation::ReputationRankLikeCpp::Unfriendly
            }) {
                return None;
            }
            return Some(canonical_access);
        }
        if canonical_record_found_like_cpp || canonical_fail_closed_like_cpp {
            return None;
        }

        self.represented_legacy_npc_can_interact_with_like_cpp(
            guid,
            npc_flags,
            npc_flags2,
            player_position,
            player_map_key.instance_id,
            player_interaction_combat_reach,
            target_player_contested_pvp,
        )
    }

    pub(in crate::session) fn represented_legacy_npc_can_interact_with_like_cpp(
        &self,
        guid: ObjectGuid,
        npc_flags: u32,
        npc_flags2: u32,
        player_position: Position,
        player_instance_id: u32,
        player_interaction_combat_reach: f32,
        target_player_contested_pvp: bool,
    ) -> Option<RepresentedCreatureAccessLikeCpp> {
        let manager = self.map_manager.as_ref()?;
        let manager = manager
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let creature =
            manager.find_creature(self.player_map_id_like_cpp(), player_instance_id, guid)?;
        let type_flags =
            CreatureTypeFlags::from_bits_retain(creature.creature.lifecycle_metadata().type_flags);
        if self.resolved_player_is_alive_like_cpp() != Some(true)
            && !type_flags.contains(CreatureTypeFlags::VISIBLE_TO_GHOSTS)
        {
            return None;
        }
        if !creature.is_alive() && !type_flags.contains(CreatureTypeFlags::INTERACT_WHILE_DEAD) {
            return None;
        }
        if (npc_flags != 0 || npc_flags2 != 0)
            && (creature.npc_flags() & npc_flags) == 0
            && (creature.npc_flags2() & npc_flags2) == 0
        {
            return None;
        }
        if creature
            .creature
            .unit()
            .subsystems()
            .control
            .charmer_guid
            .is_some()
        {
            return None;
        }
        if !creature
            .unit_flags2_like_cpp()
            .contains(UnitFlags2::INTERACT_WHILE_HOSTILE)
        {
            let reaction =
                self.represented_get_reaction_to_like_cpp(RepresentedGetReactionInputLikeCpp {
                    self_faction_template_id: creature.faction(),
                    target_faction_template_id: self
                        .player_faction_template_id_like_cpp()
                        .unwrap_or(0),
                    same_object: false,
                    attackable_by_summoner: false,
                    same_charmer_or_owner_or_self: false,
                    self_has_player_owner: false,
                    target_has_player_owner: true,
                    target_player_owner_is_current_session: true,
                    target_owner_forced_rank_for_self: None,
                    same_player_owner: false,
                    duel_in_progress: false,
                    same_raid: false,
                    self_unit_player_controlled: false,
                    target_unit_player_controlled: true,
                    self_ffa_pvp: false,
                    target_ffa_pvp: false,
                    self_ignores_reputation: false,
                    target_ignores_reputation: false,
                    target_is_unit: true,
                    target_player_contested_pvp,
                });
            if reaction <= wow_data::reputation::ReputationRankLikeCpp::Unfriendly {
                return None;
            }
        }
        let interaction_distance = creature.creature.unit().world().combat_reach() + 4.0;
        let interaction_distance_with_radii = interaction_distance
            + creature.creature.unit().world().combat_reach()
            + player_interaction_combat_reach;
        if !creature
            .position()
            .is_within_dist(&player_position, interaction_distance_with_radii)
        {
            return None;
        }

        Some(RepresentedCreatureAccessLikeCpp {
            entry: creature.entry(),
            position: creature.position(),
            npc_flags: creature.npc_flags(),
            npc_flags2: creature.npc_flags2(),
            trainer_class: creature.trainer_class_like_cpp(),
            faction_template_id: creature.faction(),
        })
    }

    pub(in crate::session) fn resolved_player_interaction_data_like_cpp(
        &self,
    ) -> Option<PlayerInteractionDataLikeCpp> {
        let canonical =
            self.with_owned_player_like_cpp(|player| *player.interaction_data_like_cpp());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.player_interaction_data_like_cpp);
        }
        canonical
    }

    pub(crate) fn reset_player_interaction_data_like_cpp(&mut self) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| player.reset_interaction_data_like_cpp())
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_interaction_data_like_cpp.reset();
        }
        canonical || cfg!(test) && self.player_handle_like_cpp.is_none()
    }

    pub(crate) fn set_player_interaction_source_like_cpp(
        &mut self,
        source_guid: ObjectGuid,
    ) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_interaction_source_like_cpp(source_guid);
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_interaction_data_like_cpp
                .set_source(source_guid);
        }
        canonical || cfg!(test) && self.player_handle_like_cpp.is_none()
    }

    pub(crate) fn set_player_trainer_interaction_like_cpp(
        &mut self,
        source_guid: ObjectGuid,
        trainer_id: u32,
    ) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_trainer_interaction_like_cpp(source_guid, trainer_id);
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_interaction_data_like_cpp
                .set_trainer(source_guid, trainer_id);
        }
        canonical || cfg!(test) && self.player_handle_like_cpp.is_none()
    }

    pub(crate) fn reset_player_interaction_if_source_like_cpp(
        &mut self,
        source_guid: ObjectGuid,
    ) -> bool {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            player.reset_interaction_if_source_like_cpp(source_guid)
        });
        #[cfg(test)]
        if canonical.is_some() || self.player_handle_like_cpp.is_none() {
            let fixture = self
                .player_interaction_data_like_cpp
                .reset_if_source(source_guid);
            return canonical.unwrap_or(fixture);
        }
        canonical.unwrap_or(false)
    }

    pub(crate) fn player_interaction_source_guid_like_cpp(&self) -> Option<ObjectGuid> {
        let interaction = self.resolved_player_interaction_data_like_cpp()?;
        (!interaction.source_guid.is_empty()).then_some(interaction.source_guid)
    }

    pub(crate) fn resolved_player_interaction_trainer_id_like_cpp(&self) -> Option<u32> {
        self.resolved_player_interaction_data_like_cpp()
            .map(|interaction| interaction.trainer_id)
    }

    #[cfg(test)]
    pub(crate) fn player_interaction_trainer_id_like_cpp(&self) -> u32 {
        self.resolved_player_interaction_trainer_id_like_cpp()
            .unwrap_or(0)
    }

    pub(crate) fn player_trainer_interaction_matches_like_cpp(
        &self,
        source_guid: ObjectGuid,
        trainer_id: i32,
    ) -> bool {
        self.resolved_player_interaction_data_like_cpp()
            .is_some_and(|interaction| interaction.trainer_matches(source_guid, trainer_id))
    }
}
