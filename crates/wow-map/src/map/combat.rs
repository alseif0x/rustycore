use super::{
    CombatBeginContextLikeCpp, CombatSubsystem, Creature, GridLifecycle, Map, ObjectGuid, Player,
    TerrainGridLoader, Unit,
};

mod cleanup;

#[derive(Clone, Copy)]
struct CombatUnitSnapshotLikeCpp<'a> {
    guid: ObjectGuid,
    unit: &'a Unit,
    game_master_player: bool,
}

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    /// Begin the represented reciprocal combat reference for a Player attacker.
    ///
    /// Source: target C++ `a5f8da2e`, `CombatManager.cpp:24-62` and `187-231`.
    /// The caller already owns the mutable Map borrow and any enclosing lock.
    /// Admission precedes all writes; the attacker is written before the victim,
    /// with no rollback if the victim-side write cannot complete.
    ///
    /// Preserved represented boundaries: only a typed Player can attack here;
    /// a creature victim uses PvE and has no resolved owner/charmer GM check.
    /// Admission still precedes existing-reference refresh, and this operation
    /// does not add the C++ AI notifications or shared-reference representation.
    pub fn begin_player_combat_ref(
        &mut self,
        attacker_guid: ObjectGuid,
        victim_guid: ObjectGuid,
        relation_represented: bool,
        attacker_is_friendly_to_victim: bool,
        victim_is_friendly_to_attacker: bool,
    ) -> bool {
        let Some(attacker) = self.get_typed_player(attacker_guid) else {
            return false;
        };
        let attacker_unit = attacker.unit();
        let attacker_world = attacker_unit.world();
        let attacker_combat = &attacker_unit.subsystems().combat;

        let (context, both_player_controlled) =
            if let Some(victim) = self.get_typed_player(victim_guid) {
                let victim_unit = victim.unit();
                let victim_world = victim_unit.world();
                let victim_combat = &victim_unit.subsystems().combat;
                (
                    wow_entities::CombatBeginContextLikeCpp {
                        same_unit: attacker_guid == victim_guid,
                        attacker_in_world: attacker_world.object().is_in_world(),
                        victim_in_world: victim_world.object().is_in_world(),
                        attacker_alive: attacker_unit.is_alive(),
                        victim_alive: victim_unit.is_alive(),
                        same_map: attacker_world.is_in_map(victim_world),
                        same_phase: attacker_world.in_same_phase(victim_world),
                        attacker_unit_state: attacker_unit.unit_state(),
                        victim_unit_state: victim_unit.unit_state(),
                        attacker_combat_disallowed: attacker_combat.combat_disallowed,
                        victim_combat_disallowed: victim_combat.combat_disallowed,
                        relation_represented,
                        attacker_is_friendly_to_victim,
                        victim_is_friendly_to_attacker,
                        attacker_or_owner_player_is_game_master: attacker.is_game_master_like_cpp(),
                        victim_or_owner_player_is_game_master: victim.is_game_master_like_cpp(),
                    },
                    true,
                )
            } else if let Some(result) = self.with_creature_like_cpp(victim_guid, |victim| {
                let victim_unit = victim.unit();
                let victim_world = victim_unit.world();
                let victim_combat = &victim_unit.subsystems().combat;
                (
                    wow_entities::CombatBeginContextLikeCpp {
                        same_unit: false,
                        attacker_in_world: attacker_world.object().is_in_world(),
                        victim_in_world: victim_world.object().is_in_world(),
                        attacker_alive: attacker_unit.is_alive(),
                        victim_alive: victim_unit.is_alive(),
                        same_map: attacker_world.is_in_map(victim_world),
                        same_phase: attacker_world.in_same_phase(victim_world),
                        attacker_unit_state: attacker_unit.unit_state(),
                        victim_unit_state: victim_unit.unit_state(),
                        attacker_combat_disallowed: attacker_combat.combat_disallowed,
                        victim_combat_disallowed: victim_combat.combat_disallowed,
                        relation_represented,
                        attacker_is_friendly_to_victim,
                        victim_is_friendly_to_attacker,
                        attacker_or_owner_player_is_game_master: attacker.is_game_master_like_cpp(),
                        victim_or_owner_player_is_game_master: false,
                    },
                    false,
                )
            }) {
                result
            } else {
                return false;
            };

        if !wow_entities::CombatSubsystem::can_begin_combat_like_cpp(context) {
            return false;
        }

        let Some(attacker) = self.get_typed_player_mut(attacker_guid) else {
            return false;
        };
        let attacker_started = attacker
            .unit_mut()
            .subsystems_mut()
            .combat
            .set_in_combat_with(victim_guid, both_player_controlled, false);

        let victim_started = if let Some(victim) = self.get_typed_player_mut(victim_guid) {
            victim
                .unit_mut()
                .subsystems_mut()
                .combat
                .set_in_combat_with(attacker_guid, both_player_controlled, false)
        } else if let Some(victim) = self.get_typed_creature_mut(victim_guid) {
            victim
                .unit_mut()
                .subsystems_mut()
                .combat
                .set_in_combat_with(attacker_guid, both_player_controlled, false)
        } else {
            false
        };

        attacker_started && victim_started
    }

    fn combat_unit_snapshot_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<CombatUnitSnapshotLikeCpp<'_>> {
        if let Some(player) = self.get_typed_player(guid) {
            return Some(CombatUnitSnapshotLikeCpp {
                guid,
                unit: player.unit(),
                game_master_player: player.is_game_master_like_cpp(),
            });
        }
        self.get_typed_creature(guid)
            .map(|creature| CombatUnitSnapshotLikeCpp {
                guid,
                unit: creature.unit(),
                game_master_player: false,
            })
    }

    fn combat_begin_context_like_cpp(
        &self,
        owner: CombatUnitSnapshotLikeCpp<'_>,
        target: CombatUnitSnapshotLikeCpp<'_>,
    ) -> CombatBeginContextLikeCpp {
        let owner_world = owner.unit.world();
        let target_world = target.unit.world();
        CombatBeginContextLikeCpp {
            same_unit: owner.guid == target.guid,
            attacker_in_world: owner_world.object().is_in_world(),
            victim_in_world: target_world.object().is_in_world(),
            attacker_alive: owner.unit.is_alive(),
            victim_alive: target.unit.is_alive(),
            same_map: owner_world.is_in_map(target_world),
            same_phase: owner_world.in_same_phase(target_world),
            attacker_unit_state: owner.unit.unit_state(),
            victim_unit_state: target.unit.unit_state(),
            attacker_combat_disallowed: owner.unit.subsystems().combat.combat_disallowed,
            victim_combat_disallowed: target.unit.subsystems().combat.combat_disallowed,
            relation_represented: false,
            attacker_is_friendly_to_victim: false,
            victim_is_friendly_to_attacker: false,
            attacker_or_owner_player_is_game_master: owner.game_master_player,
            victim_or_owner_player_is_game_master: target.game_master_player,
        }
    }

    pub fn revalidate_all_combat_refs_like_cpp(&mut self) -> Vec<(ObjectGuid, ObjectGuid)> {
        let owner_guids = self.typed_combat_unit_guids_like_cpp();
        let mut invalid = Vec::new();

        for owner_guid in owner_guids {
            let Some(owner) = self.combat_unit_snapshot_like_cpp(owner_guid) else {
                continue;
            };
            let refs: Vec<_> = owner
                .unit
                .subsystems()
                .combat
                .pve_refs
                .keys()
                .chain(owner.unit.subsystems().combat.pvp_refs.keys())
                .copied()
                .collect();

            for target_guid in refs {
                let Some(target) = self.combat_unit_snapshot_like_cpp(target_guid) else {
                    invalid.push((owner_guid, target_guid));
                    continue;
                };
                if !CombatSubsystem::can_begin_combat_like_cpp(
                    self.combat_begin_context_like_cpp(owner, target),
                ) {
                    invalid.push((owner_guid, target_guid));
                }
            }
        }

        for (owner_guid, target_guid) in &invalid {
            if let Some(owner) = self.get_typed_player_mut(*owner_guid) {
                owner
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .purge_combat_ref_like_cpp(*target_guid);
            } else if let Some(owner) = self.get_typed_creature_mut(*owner_guid) {
                owner
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .purge_combat_ref_like_cpp(*target_guid);
            }

            if let Some(target) = self.get_typed_player_mut(*target_guid) {
                target
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .purge_combat_ref_like_cpp(*owner_guid);
            } else if let Some(target) = self.get_typed_creature_mut(*target_guid) {
                target
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .purge_combat_ref_like_cpp(*owner_guid);
            }
        }

        invalid
    }
}

#[cfg(test)]
mod creation_tests;
