// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Single-writer command driver for one canonical map instance.
//!
//! C++ applies these transitions while the live Units belong to one `Map`:
//! `Unit::Attack` links the outgoing attacker and victim (`Unit.cpp:5645-5745`),
//! `CombatManager::SetInCombatWith` installs the reciprocal combat reference
//! (`CombatManager.cpp:187-228`), and `Unit::CombatStop` removes the combat and
//! attacker relations (`Unit.cpp:5802-5821`). The Rust driver keeps that
//! multi-entity mutation inside the map owner and returns only owned evidence.

use wow_core::ObjectGuid;
use wow_entities::AccessorObjectKind;

use super::{Map, NoopGridLifecycle, NoopTerrainGridLoader};
use crate::MapKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapCommandLikeCpp {
    CreatureAttackStart {
        attacker_guid: ObjectGuid,
        victim_guid: ObjectGuid,
        previous_victim_guid: Option<ObjectGuid>,
    },
    CreatureCombatStop {
        attacker_guid: ObjectGuid,
        victim_guid: ObjectGuid,
    },
}

impl MapCommandLikeCpp {
    const fn identity(self) -> (MapCommandKindLikeCpp, ObjectGuid, ObjectGuid) {
        match self {
            Self::CreatureAttackStart {
                attacker_guid,
                victim_guid,
                ..
            } => (
                MapCommandKindLikeCpp::CreatureAttackStart,
                attacker_guid,
                victim_guid,
            ),
            Self::CreatureCombatStop {
                attacker_guid,
                victim_guid,
            } => (
                MapCommandKindLikeCpp::CreatureCombatStop,
                attacker_guid,
                victim_guid,
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapCommandKindLikeCpp {
    CreatureAttackStart,
    CreatureCombatStop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapCommandStatusLikeCpp {
    Applied,
    MissingMap,
    SameUnit,
    MissingAttacker,
    AttackerNotCreature,
    MissingVictim,
    VictimNotPlayerOrCreature,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapCommandOutcomeLikeCpp {
    pub map_key: MapKey,
    pub kind: MapCommandKindLikeCpp,
    pub attacker_guid: ObjectGuid,
    pub victim_guid: ObjectGuid,
    pub status: MapCommandStatusLikeCpp,
    pub previous_victim_unlinked: bool,
    pub victim_attack_stopped: bool,
    pub victim_still_in_combat: Option<bool>,
}

impl MapCommandOutcomeLikeCpp {
    pub const fn is_applied(self) -> bool {
        matches!(self.status, MapCommandStatusLikeCpp::Applied)
    }

    pub(crate) fn missing_map(map_key: MapKey, command: MapCommandLikeCpp) -> Self {
        let (kind, attacker_guid, victim_guid) = command.identity();
        Self {
            map_key,
            kind,
            attacker_guid,
            victim_guid,
            status: MapCommandStatusLikeCpp::MissingMap,
            previous_victim_unlinked: false,
            victim_attack_stopped: false,
            victim_still_in_combat: None,
        }
    }
}

/// One driver exists inside each [`crate::ManagedMap`]. Its `Map` is not
/// exposed through this type; the legacy `ManagedMap::map[_mut]` accessors stay
/// temporarily available while callers move to commands and owned queries.
#[derive(Debug)]
pub(crate) struct MapRuntime {
    pub(crate) map: Map<NoopTerrainGridLoader, NoopGridLifecycle>,
}

impl MapRuntime {
    pub(crate) fn new(map: Map<NoopTerrainGridLoader, NoopGridLifecycle>) -> Self {
        Self { map }
    }

    pub(crate) fn execute(&mut self, command: MapCommandLikeCpp) -> MapCommandOutcomeLikeCpp {
        match command {
            MapCommandLikeCpp::CreatureAttackStart {
                attacker_guid,
                victim_guid,
                previous_victim_guid,
            } => self.apply_creature_attack_start(attacker_guid, victim_guid, previous_victim_guid),
            MapCommandLikeCpp::CreatureCombatStop {
                attacker_guid,
                victim_guid,
            } => self.apply_creature_combat_stop(attacker_guid, victim_guid),
        }
    }

    fn base_outcome(
        &self,
        kind: MapCommandKindLikeCpp,
        attacker_guid: ObjectGuid,
        victim_guid: ObjectGuid,
        status: MapCommandStatusLikeCpp,
    ) -> MapCommandOutcomeLikeCpp {
        MapCommandOutcomeLikeCpp {
            map_key: MapKey::new(self.map.map_id(), self.map.instance_id()),
            kind,
            attacker_guid,
            victim_guid,
            status,
            previous_victim_unlinked: false,
            victim_attack_stopped: false,
            victim_still_in_combat: None,
        }
    }

    fn validate_creature_unit_pair(
        &self,
        kind: MapCommandKindLikeCpp,
        attacker_guid: ObjectGuid,
        victim_guid: ObjectGuid,
    ) -> Result<AccessorObjectKind, MapCommandOutcomeLikeCpp> {
        if attacker_guid == victim_guid {
            return Err(self.base_outcome(
                kind,
                attacker_guid,
                victim_guid,
                MapCommandStatusLikeCpp::SameUnit,
            ));
        }
        match self.map.entity_world.kind(attacker_guid) {
            None => {
                return Err(self.base_outcome(
                    kind,
                    attacker_guid,
                    victim_guid,
                    MapCommandStatusLikeCpp::MissingAttacker,
                ));
            }
            Some(AccessorObjectKind::Creature) => {}
            Some(_) => {
                return Err(self.base_outcome(
                    kind,
                    attacker_guid,
                    victim_guid,
                    MapCommandStatusLikeCpp::AttackerNotCreature,
                ));
            }
        }
        match self.map.entity_world.kind(victim_guid) {
            None => Err(self.base_outcome(
                kind,
                attacker_guid,
                victim_guid,
                MapCommandStatusLikeCpp::MissingVictim,
            )),
            Some(kind @ (AccessorObjectKind::Player | AccessorObjectKind::Creature)) => Ok(kind),
            Some(_) => Err(self.base_outcome(
                kind,
                attacker_guid,
                victim_guid,
                MapCommandStatusLikeCpp::VictimNotPlayerOrCreature,
            )),
        }
    }

    fn unlink_previous_victim(
        &mut self,
        previous_victim_guid: Option<ObjectGuid>,
        attacker_guid: ObjectGuid,
    ) -> bool {
        let Some(previous_victim_guid) = previous_victim_guid else {
            return false;
        };
        match self.map.entity_world.kind(previous_victim_guid) {
            Some(AccessorObjectKind::Player) => self
                .map
                .get_typed_player_mut(previous_victim_guid)
                .is_some_and(|previous| {
                    previous.unit_mut().remove_attacker_like_cpp(attacker_guid)
                }),
            Some(AccessorObjectKind::Creature) => self
                .map
                .with_creature_mut_like_cpp(previous_victim_guid, |previous| {
                    previous.unit_mut().remove_attacker_like_cpp(attacker_guid)
                })
                .unwrap_or(false),
            _ => false,
        }
    }

    fn apply_creature_attack_start(
        &mut self,
        attacker_guid: ObjectGuid,
        victim_guid: ObjectGuid,
        previous_victim_guid: Option<ObjectGuid>,
    ) -> MapCommandOutcomeLikeCpp {
        let kind = MapCommandKindLikeCpp::CreatureAttackStart;
        let victim_kind = match self.validate_creature_unit_pair(kind, attacker_guid, victim_guid) {
            Ok(victim_kind) => victim_kind,
            Err(outcome) => return outcome,
        };

        // Prevalidation above and exclusive `&mut self` make both types stable
        // for the full transition. No delivery or external callback can observe
        // the intermediate half-linked state.
        let threat_ref = self
            .map
            .with_creature_mut_like_cpp(attacker_guid, |attacker| {
                let combat = &mut attacker.unit_mut().subsystems_mut().combat;
                combat.set_in_combat_with(victim_guid, false, false);
                combat.add_threat(victim_guid, 0.0);
                combat.threat_ref(victim_guid).copied()
            })
            .expect("validated Creature must remain in the single-writer entity world");
        let previous_victim_unlinked =
            self.unlink_previous_victim(previous_victim_guid, attacker_guid);
        let victim_still_in_combat = match victim_kind {
            AccessorObjectKind::Player => {
                let victim = self
                    .map
                    .get_typed_player_mut(victim_guid)
                    .expect("validated Player must remain in the single-writer entity world");
                let combat = &mut victim.unit_mut().subsystems_mut().combat;
                combat.set_in_combat_with(attacker_guid, false, false);
                if let Some(threat_ref) = threat_ref {
                    combat.put_threatened_by_me_ref(attacker_guid, threat_ref);
                }
                victim.unit_mut().add_attacker_like_cpp(attacker_guid);
                victim.unit().subsystems().combat.has_combat()
            }
            AccessorObjectKind::Creature => self
                .map
                .with_creature_mut_like_cpp(victim_guid, |victim| {
                    let combat = &mut victim.unit_mut().subsystems_mut().combat;
                    combat.set_in_combat_with(attacker_guid, false, false);
                    if let Some(threat_ref) = threat_ref {
                        combat.put_threatened_by_me_ref(attacker_guid, threat_ref);
                    }
                    victim.unit_mut().add_attacker_like_cpp(attacker_guid);
                    victim.unit().subsystems().combat.has_combat()
                })
                .expect("validated Creature must remain in the single-writer entity world"),
            _ => unreachable!("pair validation accepts only Player or Creature victims"),
        };

        MapCommandOutcomeLikeCpp {
            previous_victim_unlinked,
            victim_still_in_combat: Some(victim_still_in_combat),
            ..self.base_outcome(
                kind,
                attacker_guid,
                victim_guid,
                MapCommandStatusLikeCpp::Applied,
            )
        }
    }

    fn apply_creature_combat_stop(
        &mut self,
        attacker_guid: ObjectGuid,
        victim_guid: ObjectGuid,
    ) -> MapCommandOutcomeLikeCpp {
        let kind = MapCommandKindLikeCpp::CreatureCombatStop;
        let victim_kind = match self.validate_creature_unit_pair(kind, attacker_guid, victim_guid) {
            Ok(victim_kind) => victim_kind,
            Err(outcome) => return outcome,
        };

        self.map
            .with_creature_mut_like_cpp(attacker_guid, |attacker| {
                attacker
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .purge_combat_ref_like_cpp(victim_guid);
                // The player-victim bridge already applied this C++
                // `RemoveAllAttackers` edge. Preserve the existing
                // creature-victim path until its broader evade vertical moves.
                if victim_kind == AccessorObjectKind::Player {
                    attacker.unit_mut().remove_attacker_like_cpp(victim_guid);
                }
            })
            .expect("validated Creature must remain in the single-writer entity world");
        let (victim_still_in_combat, victim_attack_stopped) = match victim_kind {
            AccessorObjectKind::Player => {
                let victim = self
                    .map
                    .get_typed_player_mut(victim_guid)
                    .expect("validated Player must remain in the single-writer entity world");
                // C++ `Unit::CombatStop` calls `RemoveAllAttackers`; an
                // attacker that currently targets this Creature therefore
                // executes `AttackStop` and clears its own `m_attacking`.
                let attack_stopped = victim.unit().attacking() == Some(attacker_guid);
                if attack_stopped {
                    victim.unit_mut().set_attacking(None);
                }
                victim
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .purge_combat_ref_like_cpp(attacker_guid);
                victim
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .purge_threatened_by_me_ref(attacker_guid);
                victim.unit_mut().remove_attacker_like_cpp(attacker_guid);
                (
                    victim.unit().subsystems().combat.has_combat(),
                    attack_stopped,
                )
            }
            AccessorObjectKind::Creature => (
                self.map
                    .with_creature_mut_like_cpp(victim_guid, |victim| {
                        victim
                            .unit_mut()
                            .subsystems_mut()
                            .combat
                            .purge_combat_ref_like_cpp(attacker_guid);
                        victim.unit_mut().remove_attacker_like_cpp(attacker_guid);
                        victim.unit().subsystems().combat.has_combat()
                    })
                    .expect("validated Creature must remain in the single-writer entity world"),
                false,
            ),
            _ => unreachable!("pair validation accepts only Player or Creature victims"),
        };

        MapCommandOutcomeLikeCpp {
            victim_attack_stopped,
            victim_still_in_combat: Some(victim_still_in_combat),
            ..self.base_outcome(
                kind,
                attacker_guid,
                victim_guid,
                MapCommandStatusLikeCpp::Applied,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spawn::Difficulty;
    use wow_core::guid::HighGuid;
    use wow_entities::{Creature, GameObject, MapObjectRecord, Player};

    fn guid(high: HighGuid, counter: i64) -> ObjectGuid {
        if high == HighGuid::Player {
            ObjectGuid::create_player(1, counter)
        } else {
            ObjectGuid::create_world_object(high, 0, 1, 571, 7, 100, counter)
        }
    }

    fn runtime() -> MapRuntime {
        MapRuntime::new(Map::new(571, 7, Difficulty::default(), 60_000))
    }

    fn insert_creature(runtime: &mut MapRuntime, creature_guid: ObjectGuid) {
        let mut creature = Creature::new(false);
        creature
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(creature_guid);
        creature.unit_mut().world_mut().set_map(571, 7).unwrap();
        creature.unit_mut().world_mut().object_mut().add_to_world();
        runtime
            .map
            .insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
            .unwrap();
    }

    fn insert_player(runtime: &mut MapRuntime, player_guid: ObjectGuid) {
        let mut player = Player::new(None, false);
        player
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(player_guid);
        player.unit_mut().world_mut().set_map(571, 7).unwrap();
        player.unit_mut().world_mut().object_mut().add_to_world();
        runtime
            .map
            .insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
            .unwrap();
    }

    #[test]
    fn creature_player_start_and_stop_are_one_owned_command_transition_like_cpp() {
        let mut runtime = runtime();
        let attacker_guid = guid(HighGuid::Creature, 41);
        let victim_guid = guid(HighGuid::Player, 42);
        let previous_victim_guid = guid(HighGuid::Player, 40);
        insert_creature(&mut runtime, attacker_guid);
        insert_player(&mut runtime, victim_guid);
        insert_player(&mut runtime, previous_victim_guid);
        runtime
            .map
            .get_typed_player_mut(previous_victim_guid)
            .unwrap()
            .unit_mut()
            .add_attacker_like_cpp(attacker_guid);

        let started = runtime.execute(MapCommandLikeCpp::CreatureAttackStart {
            attacker_guid,
            victim_guid,
            previous_victim_guid: Some(previous_victim_guid),
        });
        assert_eq!(started.status, MapCommandStatusLikeCpp::Applied);
        assert!(started.previous_victim_unlinked);
        assert_eq!(started.victim_still_in_combat, Some(true));
        assert!(
            !runtime
                .map
                .get_typed_player(previous_victim_guid)
                .unwrap()
                .unit()
                .has_attacker_like_cpp(attacker_guid)
        );
        assert!(
            runtime
                .map
                .with_creature_like_cpp(attacker_guid, |attacker| {
                    attacker
                        .unit()
                        .subsystems()
                        .combat
                        .is_in_combat_with(victim_guid)
                        && attacker
                            .unit()
                            .subsystems()
                            .combat
                            .threat_value(victim_guid)
                            == Some(0.0)
                })
                .unwrap()
        );
        let victim = runtime.map.get_typed_player(victim_guid).unwrap();
        assert!(
            victim
                .unit()
                .subsystems()
                .combat
                .is_in_combat_with(attacker_guid)
        );
        assert!(victim.unit().has_attacker_like_cpp(attacker_guid));

        // Model the reciprocal `m_attackers` edge created when this Player is
        // also actively attacking the Creature. C++ `RemoveAllAttackers`
        // forces that Player through `AttackStop` during evade.
        runtime
            .map
            .get_typed_player_mut(victim_guid)
            .unwrap()
            .unit_mut()
            .set_attacking(Some(attacker_guid));
        runtime
            .map
            .with_creature_mut_like_cpp(attacker_guid, |attacker| {
                attacker.unit_mut().add_attacker_like_cpp(victim_guid);
            })
            .unwrap();

        let stopped = runtime.execute(MapCommandLikeCpp::CreatureCombatStop {
            attacker_guid,
            victim_guid,
        });
        assert_eq!(stopped.status, MapCommandStatusLikeCpp::Applied);
        assert!(stopped.victim_attack_stopped);
        assert_eq!(stopped.victim_still_in_combat, Some(false));
        let victim = runtime.map.get_typed_player(victim_guid).unwrap();
        assert!(
            !victim
                .unit()
                .subsystems()
                .combat
                .is_in_combat_with(attacker_guid)
        );
        assert!(!victim.unit().has_attacker_like_cpp(attacker_guid));
        assert_eq!(victim.unit().attacking(), None);
        assert!(
            !runtime
                .map
                .with_creature_like_cpp(attacker_guid, |attacker| {
                    attacker.unit().has_attacker_like_cpp(victim_guid)
                })
                .unwrap()
        );
    }

    #[test]
    fn creature_target_start_and_stop_keep_reciprocal_threat_owned_like_cpp() {
        let mut runtime = runtime();
        let attacker_guid = guid(HighGuid::Creature, 43);
        let victim_guid = guid(HighGuid::Creature, 44);
        insert_creature(&mut runtime, attacker_guid);
        insert_creature(&mut runtime, victim_guid);

        assert!(
            runtime
                .execute(MapCommandLikeCpp::CreatureAttackStart {
                    attacker_guid,
                    victim_guid,
                    previous_victim_guid: None,
                })
                .is_applied()
        );
        assert!(
            runtime
                .map
                .with_creature_like_cpp(victim_guid, |victim| {
                    victim
                        .unit()
                        .subsystems()
                        .combat
                        .threatened_by_me_owner_guids()
                        .contains(&attacker_guid)
                        && victim.unit().has_attacker_like_cpp(attacker_guid)
                })
                .unwrap()
        );

        assert!(
            runtime
                .execute(MapCommandLikeCpp::CreatureCombatStop {
                    attacker_guid,
                    victim_guid,
                })
                .is_applied()
        );
        assert!(
            runtime
                .map
                .with_creature_like_cpp(victim_guid, |victim| {
                    !victim
                        .unit()
                        .subsystems()
                        .combat
                        .is_in_combat_with(attacker_guid)
                        && !victim.unit().has_attacker_like_cpp(attacker_guid)
                })
                .unwrap()
        );
    }

    #[test]
    fn rejected_pair_fails_before_mutating_the_attacker_like_cpp() {
        let mut runtime = runtime();
        let attacker_guid = guid(HighGuid::Creature, 45);
        let missing_victim = guid(HighGuid::Player, 46);
        insert_creature(&mut runtime, attacker_guid);

        let outcome = runtime.execute(MapCommandLikeCpp::CreatureAttackStart {
            attacker_guid,
            victim_guid: missing_victim,
            previous_victim_guid: None,
        });
        assert_eq!(outcome.status, MapCommandStatusLikeCpp::MissingVictim);
        assert!(
            runtime
                .map
                .with_creature_like_cpp(attacker_guid, |attacker| {
                    !attacker.unit().subsystems().combat.has_combat()
                        && attacker
                            .unit()
                            .subsystems()
                            .combat
                            .threat_value(missing_victim)
                            .is_none()
                })
                .unwrap()
        );
    }

    #[test]
    fn wrong_kind_and_same_unit_are_explicit_without_defaults_like_cpp() {
        let mut runtime = runtime();
        let attacker_guid = guid(HighGuid::Creature, 47);
        let gameobject_guid = guid(HighGuid::GameObject, 48);
        insert_creature(&mut runtime, attacker_guid);
        let mut gameobject = GameObject::new();
        gameobject.world_mut().object_mut().create(gameobject_guid);
        gameobject.world_mut().set_map(571, 7).unwrap();
        runtime
            .map
            .insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
            .unwrap();

        assert_eq!(
            runtime
                .execute(MapCommandLikeCpp::CreatureAttackStart {
                    attacker_guid,
                    victim_guid: gameobject_guid,
                    previous_victim_guid: None,
                })
                .status,
            MapCommandStatusLikeCpp::VictimNotPlayerOrCreature
        );
        assert_eq!(
            runtime
                .execute(MapCommandLikeCpp::CreatureCombatStop {
                    attacker_guid,
                    victim_guid: attacker_guid,
                })
                .status,
            MapCommandStatusLikeCpp::SameUnit
        );
    }

    #[test]
    fn map_manager_missing_map_returns_owned_rejection_like_cpp() {
        let mut manager = crate::MapManager::default();
        let attacker_guid = guid(HighGuid::Creature, 49);
        let victim_guid = guid(HighGuid::Player, 50);
        let outcome = manager.execute_map_command_like_cpp(
            571,
            7,
            MapCommandLikeCpp::CreatureAttackStart {
                attacker_guid,
                victim_guid,
                previous_victim_guid: None,
            },
        );

        assert_eq!(outcome.map_key, MapKey::new(571, 7));
        assert_eq!(outcome.status, MapCommandStatusLikeCpp::MissingMap);
        assert_eq!(outcome.victim_still_in_combat, None);
    }
}
