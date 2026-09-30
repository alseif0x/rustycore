//! Two private storage adapters execute one algorithm; no public storage callback.
use super::*;
use crate::map::{CreatureActorWitness, MapCommandLikeCpp, MapRuntime};

pub(crate) enum AggroMap<'a> {
    Legacy { manager: &'a mut LegacyMapManager, map_id: u16, instance_id: u32 },
    Canonical { runtime: &'a mut MapRuntime, witnesses: &'a HashMap<ObjectGuid, CreatureActorWitness>,
        order: &'a [ObjectGuid] },
}

impl AggroMap<'_> {
    pub(super) fn guids(&self, primaries: &[ObjectGuid]) -> Vec<ObjectGuid> {
        match self {
            Self::Legacy { .. } => primaries.to_vec(),
            Self::Canonical { order, .. } => order.to_vec(),
        }
    }
    pub(super) fn actor(&self, guid: ObjectGuid) -> Option<&WorldCreature> {
        match self {
            Self::Legacy { manager, map_id, instance_id } => manager.find_creature(*map_id, *instance_id, guid),
            Self::Canonical { runtime, witnesses, .. } => {
                let expected = witnesses.get(&guid)?;
                let current = runtime.map.creature_actor_witness(guid)?;
                expected.same_actor(&current).then(|| runtime.map.creature_actor(guid)).flatten()
            }
        }
    }
    pub(super) fn actor_mut(&mut self, guid: ObjectGuid) -> Option<&mut WorldCreature> {
        match self {
            Self::Legacy { manager, map_id, instance_id } => manager.find_creature_mut(*map_id, *instance_id, guid),
            Self::Canonical { runtime, witnesses, .. } => {
                let expected = witnesses.get(&guid)?;
                let current = runtime.map.creature_actor_witness(guid)?;
                if !expected.same_actor(&current) { return None; }
                runtime.map.creature_actor_mut(guid)
            }
        }
    }
    pub(super) fn player_present(&self, guid: ObjectGuid) -> bool {
        match self {
            Self::Legacy { .. } => true,
            Self::Canonical { runtime, .. } => runtime.map.get_typed_player(guid).is_some(),
        }
    }
    pub(super) fn victim_present(&self, guid: ObjectGuid) -> bool {
        match self {
            Self::Legacy { .. } => true,
            Self::Canonical { .. } => self.actor(guid).is_some() || self.player_present(guid),
        }
    }
    pub(super) fn commit_combat(&mut self, outcome: &mut AggroOutcome) {
        let Self::Canonical { runtime, .. } = self else { return; };
        let starts: Vec<_> = outcome.commands.iter().map(|command| runtime.execute(
            MapCommandLikeCpp::CreatureAttackStart {
                attacker_guid: command.attacker_guid, victim_guid: command.victim_guid,
                previous_victim_guid: command.previous_victim_guid,
            }).is_applied()).collect();
        let stops: Vec<_> = outcome.stop_commands.iter().map(|command| runtime.execute(
            MapCommandLikeCpp::CreatureCombatStop {
                attacker_guid: command.attacker_guid, victim_guid: command.victim_guid,
            }).is_applied()).collect();
        let commands = &outcome.commands;
        let stop_commands = &outcome.stop_commands;
        outcome.effects.retain(|effect| match &effect.kind {
            AggroEffectKind::AttackStart { victim } => commands.iter().zip(&starts)
                .filter(|(command, _)| command.attacker_guid == effect.source_guid && command.victim_guid == *victim)
                .any(|(_, applied)| *applied),
            AggroEffectKind::AttackStop { victim } => {
                let mut matching = stop_commands.iter().zip(&stops)
                    .filter(|(command, _)| command.attacker_guid == effect.source_guid && command.victim_guid == *victim)
                    .peekable();
                matching.peek().is_none() || matching.any(|(_, applied)| *applied)
            }
            _ => true,
        });
        let mut index = 0;
        outcome.commands.retain(|_| { let applied = starts[index]; index += 1; applied });
        let mut index = 0;
        outcome.stop_commands.retain(|_| { let applied = stops[index]; index += 1; applied });
    }
}
