// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Native pet combat stop on the canonical map-owned unit records.
//!
//! Target reference: Unit.cpp:5746–5780,5802–5821 and
//! CombatManager.cpp:60–86,307–314,345–349 at a5f8da2ebf5424bf0450ca4e08843ecbf72577bd.

use std::collections::{HashMap, HashSet};

use crate::map_rules::{map_record_unit_like_cpp, map_record_unit_mut_like_cpp};
use wow_constants::UnitFlags;
use wow_core::ObjectGuid;
use wow_entities::{Unit, UnitAttackStopOutcome};

use super::Map;

/// State effects produced by C++-shaped `Unit::CombatStop` for a canonical Pet.
///
/// AI exit identities are returned to the owning application. This map
/// participant updates touched units' native `IN_COMBAT` flags from their
/// remaining references, but does not run native `AtExitCombat` or
/// `AtDisengage`, update the master's pet-combat flag, or invoke AI callbacks.
/// Those effects are caller continuations represented by `exited_combat`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MapPetCombatStopOutcomeLikeCpp {
    pub attacks_stopped: Vec<(ObjectGuid, ObjectGuid)>,
    pub exited_combat: Vec<ObjectGuid>,
}

#[derive(Debug)]
struct UnitCombatSnapshot {
    attacking: Option<ObjectGuid>,
    attackers: HashSet<ObjectGuid>,
    pve_refs: HashSet<ObjectGuid>,
    pvp_refs: HashSet<ObjectGuid>,
    threat_refs: HashSet<ObjectGuid>,
    threatened_by_me: HashSet<ObjectGuid>,
    in_combat_flag: bool,
}

impl UnitCombatSnapshot {
    fn from_unit(unit: &Unit) -> Self {
        let combat = &unit.subsystems().combat;
        Self {
            attacking: unit.attacking(),
            attackers: combat.attackers.clone(),
            pve_refs: combat.pve_refs.keys().copied().collect(),
            pvp_refs: combat.pvp_refs.keys().copied().collect(),
            threat_refs: combat.threat_refs.keys().copied().collect(),
            threatened_by_me: combat.threatened_by_me.keys().copied().collect(),
            in_combat_flag: unit.unit_flags_like_cpp().contains(UnitFlags::IN_COMBAT),
        }
    }
}

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: super::TerrainGridLoader,
    Lifecycle: super::GridLifecycle,
{
    /// Stop the canonical typed pet's combat state like C++ `Unit::CombatStop()`.
    ///
    /// The method reads the pet and its explicitly indexed combat neighbors,
    /// mutates only those map-owned units, and returns owned effects for its
    /// caller. It does not send packets, perform I/O, await, or invoke AI
    /// callbacks.
    pub fn stop_pet_combat_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
    ) -> Result<MapPetCombatStopOutcomeLikeCpp, &'static str> {
        let pet = self
            .get_typed_pet(pet_guid)
            .ok_or("pet is not a canonical typed Pet")?;
        let pet_snapshot = UnitCombatSnapshot::from_unit(pet.creature().unit());

        let mut combat_guids = HashSet::from([pet_guid]);
        combat_guids.extend(pet_snapshot.pve_refs.iter().copied());
        combat_guids.extend(pet_snapshot.pvp_refs.iter().copied());
        combat_guids.extend(pet_snapshot.threat_refs.iter().copied());
        combat_guids.extend(pet_snapshot.threatened_by_me.iter().copied());

        let mut referenced_guids = combat_guids.clone();
        referenced_guids.extend(pet_snapshot.attackers.iter().copied());
        if let Some(victim_guid) = pet_snapshot.attacking {
            referenced_guids.insert(victim_guid);
        }
        referenced_guids.remove(&pet_guid);

        // Snapshot only the neighbors named by the pet's own state. Every
        // explicit participant is validated before the first destructive edit.
        let mut neighbors = HashMap::new();
        let mut neighbor_guids: Vec<_> = referenced_guids.into_iter().collect();
        sort_guids(&mut neighbor_guids);
        for guid in neighbor_guids {
            let snapshot = snapshot_map_unit(self, guid)
                .ok_or("pet combat state references a missing or non-unit participant")?;
            neighbors.insert(guid, snapshot);
        }

        let mut attacker_guids: Vec<_> = pet_snapshot.attackers.iter().copied().collect();
        sort_guids(&mut attacker_guids);
        for attacker_guid in &attacker_guids {
            if neighbors
                .get(attacker_guid)
                .is_none_or(|attacker| attacker.attacking != Some(pet_guid))
            {
                return Err("pet attacker index contains a unit targeting another victim");
            }
        }

        let mut outcome = MapPetCombatStopOutcomeLikeCpp::default();

        // C++ CombatStop calls AttackStop on the pet before RemoveAllAttackers.
        if let UnitAttackStopOutcome::Stopped { victim } =
            map_unit_mut_prevalidated(self, pet_guid).attack_stop_like_cpp()
        {
            outcome.attacks_stopped.push((pet_guid, victim));
            map_unit_mut_prevalidated(self, victim)
                .subsystems_mut()
                .combat
                .remove_attacker(pet_guid);
        }

        for attacker_guid in attacker_guids {
            if let UnitAttackStopOutcome::Stopped { victim } =
                map_unit_mut_prevalidated(self, attacker_guid).attack_stop_like_cpp()
            {
                outcome.attacks_stopped.push((attacker_guid, victim));
                map_unit_mut_prevalidated(self, victim)
                    .subsystems_mut()
                    .combat
                    .remove_attacker(attacker_guid);
            }
            map_unit_mut_prevalidated(self, pet_guid)
                .subsystems_mut()
                .combat
                .remove_attacker(attacker_guid);
        }
        map_unit_mut_prevalidated(self, pet_guid)
            .subsystems_mut()
            .combat
            .clear_attackers();

        let mut combat_guids: Vec<_> = combat_guids.into_iter().collect();
        sort_guids(&mut combat_guids);
        if let Some(pet_index) = combat_guids.iter().position(|guid| *guid == pet_guid) {
            combat_guids.swap(0, pet_index);
        }
        for guid in combat_guids {
            let unit = map_unit_mut_prevalidated(self, guid);
            let combat = &mut unit.subsystems_mut().combat;
            if guid == pet_guid {
                // C++ ClearInCombat() ends PvE before PvP and clears the pet's
                // complete threat-manager state.
                combat.end_all_pve_combat();
                combat.end_all_pvp_combat();
            } else {
                combat.pve_refs.remove(&pet_guid);
                combat.pvp_refs.remove(&pet_guid);
                combat.remove_threat(pet_guid);
                combat.purge_threatened_by_me_ref(pet_guid);
            }

            let remains_in_combat = combat.has_combat();
            let mut flags = unit.unit_flags_like_cpp();
            flags.set(UnitFlags::IN_COMBAT, remains_in_combat);
            unit.set_unit_flags_like_cpp(flags);

            let was_in_combat = if guid == pet_guid {
                pet_snapshot.in_combat_flag
            } else {
                neighbors[&guid].in_combat_flag
            };
            if was_in_combat && !remains_in_combat {
                outcome.exited_combat.push(guid);
            }
        }

        Ok(outcome)
    }
}

fn snapshot_map_unit<Terrain, Lifecycle>(
    map: &Map<Terrain, Lifecycle>,
    guid: ObjectGuid,
) -> Option<UnitCombatSnapshot> {
    let record = map.entity_world.get(&guid)?;
    map_record_unit_like_cpp(record).map(UnitCombatSnapshot::from_unit)
}

fn map_unit_mut_prevalidated<Terrain, Lifecycle>(
    map: &mut Map<Terrain, Lifecycle>,
    guid: ObjectGuid,
) -> &mut Unit {
    let record = map
        .entity_world
        .get_mut(&guid)
        .expect("pet combat participant was prevalidated");
    map_record_unit_mut_like_cpp(record).expect("pet combat participant has a native Unit")
}

fn guid_key(guid: ObjectGuid) -> (i64, i64) {
    (guid.high_value(), guid.low_value())
}

fn sort_guids(guids: &mut [ObjectGuid]) {
    guids.sort_unstable_by_key(|guid| guid_key(*guid));
}
