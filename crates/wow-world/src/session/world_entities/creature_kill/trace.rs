//! Opt-in observations of the existing kill phases, not production AI/hooks.
use super::*;

impl WorldSession {
    pub(in crate::session) fn record_represented_creature_kill_hooks_like_cpp(
        &mut self,
        attacker_guid: ObjectGuid,
        creature_guid: ObjectGuid,
    ) {
        if !self.character_lifecycle_fixture_mode() {
            return;
        }
        let reward_source = self
            .mutate_world_creature(creature_guid, |creature| {
                (creature.map_id() as u16, creature.position())
            })
            .or_else(|| {
                self.player_position_like_cpp()
                    .map(|position| (self.player_map_id_like_cpp(), position))
            });
        let Some(reward_source) = reward_source else {
            return;
        };
        let mut tappers = self
            .mutate_world_creature(creature_guid, |creature| {
                creature.creature.tap_list().to_vec()
            })
            .unwrap_or_default();
        if tappers.is_empty() {
            tappers.push(attacker_guid);
        }
        let mut unique_tappers = Vec::with_capacity(tappers.len());
        for tapper in tappers {
            if !unique_tappers.contains(&tapper) {
                unique_tappers.push(tapper);
            }
        }

        self.represented_creature_kill_events_like_cpp.push(
            RepresentedCreatureKillEventLikeCpp::KillerProc {
                attacker_guid,
                victim_guid: creature_guid,
            },
        );

        for tapper_guid in unique_tappers {
            if !self.represented_player_at_group_reward_distance_like_cpp(
                tapper_guid,
                reward_source.0,
                reward_source.1,
            ) {
                continue;
            }
            self.represented_creature_kill_events_like_cpp.push(
                RepresentedCreatureKillEventLikeCpp::TapperTargetDiesProc {
                    tapper_guid,
                    victim_guid: creature_guid,
                },
            );
        }

        self.represented_creature_kill_events_like_cpp.push(
            RepresentedCreatureKillEventLikeCpp::VictimDeathProc {
                victim_guid: creature_guid,
            },
        );
        self.represented_creature_kill_events_like_cpp.push(
            RepresentedCreatureKillEventLikeCpp::DeliveredKillingBlowCriteria {
                player_guid: attacker_guid,
                victim_guid: creature_guid,
                quantity: 1,
            },
        );
    }
    pub(in crate::session) fn record_creature_kill_post_state(
        &mut self,
        attacker_guid: ObjectGuid,
        creature_guid: ObjectGuid,
        lootable: bool,
        can_skin: bool,
    ) {
        if !self.character_lifecycle_fixture_mode() {
            return;
        }
        self.represented_creature_kill_events_like_cpp.push(
            RepresentedCreatureKillEventLikeCpp::DeathStateJustDied {
                victim_guid: creature_guid,
            },
        );
        self.represented_creature_kill_events_like_cpp.push(
            RepresentedCreatureKillEventLikeCpp::ZoneScriptUnitDeath {
                unit_guid: creature_guid,
            },
        );
        self.record_represented_tapper_pet_killed_unit_hooks_like_cpp(creature_guid);
        self.represented_creature_kill_events_like_cpp.push(
            RepresentedCreatureKillEventLikeCpp::LootFlagsApplied {
                creature_guid,
                lootable,
                can_skin,
                skinnable: can_skin,
            },
        );
        self.represented_creature_kill_events_like_cpp.push(
            RepresentedCreatureKillEventLikeCpp::CreatureOnHealthDepletedAi {
                creature_guid,
                attacker_guid,
                is_kill: true,
            },
        );
        self.represented_creature_kill_events_like_cpp.push(
            RepresentedCreatureKillEventLikeCpp::CreatureJustDiedAi {
                creature_guid,
                killer_guid: attacker_guid,
            },
        );
        if attacker_guid.is_player() {
            self.represented_creature_kill_events_like_cpp.push(
                RepresentedCreatureKillEventLikeCpp::ScriptMgrOnCreatureKill {
                    killer_guid: attacker_guid,
                    creature_guid,
                },
            );
        }
    }
}
