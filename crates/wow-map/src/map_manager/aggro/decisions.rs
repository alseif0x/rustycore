//! Catalog-independent actor facts and the existing alert mutation.
use super::*;

pub fn candidate_hostile(actor: &WorldCreature, candidate: &AggroCandidate,
    policies: &mut AggroPolicies<'_>) -> Option<bool> {
    (policies.hostility)(actor.creature.unit().data().faction_template,
        AggroFactionTarget::Player(candidate))
}

pub fn snapshot_hostile(actor: &WorldCreature, snapshot: &AggroOwnerSnapshot,
    policies: &mut AggroPolicies<'_>) -> Option<bool> {
    (policies.hostility)(actor.creature.unit().data().faction_template,
        AggroFactionTarget::Unit(snapshot.faction_template_id))
}

pub fn select_ai(actor: &WorldCreature, policies: &mut AggroPolicies<'_>) -> AggroAiSelection {
    let metadata = actor.creature.lifecycle_metadata();
    let is_pet = actor.guid().is_pet();
    if !is_pet && !metadata.script_name.is_empty() { return AggroAiSelection::Unrepresented; }
    (policies.select_ai)(actor.aggro_ai_facts())
}

impl WorldCreature {
    /// Narrow immutable AI selection facts, also used by legacy spell callers.
    pub fn aggro_ai_facts(&self) -> AggroAiFacts<'_> {
        let actor = self;
        let metadata = actor.creature.lifecycle_metadata();
        let is_pet = actor.guid().is_pet();
        AggroAiFacts {
        ai_name: &metadata.ai_name, script_name: &metadata.script_name,
        is_pet, is_vehicle: actor.creature.is_vehicle_unit_type_like_cpp(),
        is_totem: actor.creature.is_totem_unit_type_like_cpp(), flags_extra: metadata.flags_extra,
        first_spell_id: actor.creature.spells()[0], creature_type: metadata.creature_type,
        is_guardian: actor.creature.is_guardian_unit_type_like_cpp(),
        is_civilian: actor.creature.is_civilian_like_cpp(),
        faction_template: actor.creature.unit().data().faction_template,
        npc_flags: actor.npc_flags(),
        is_controllable_guardian: actor.creature.is_controlable_guardian_unit_type_like_cpp(),
        owner_is_player: actor.creature.unit().subsystems().control
            .charmer_or_owner_guid().is_some_and(|guid| guid.is_player()),
        }
    }
}

pub fn can_attack(kind: &AggroAiKind, actor: &WorldCreature, candidate: &AggroCandidate,
    policies: &mut AggroPolicies<'_>) -> AggroAttackDecision {
    if !matches!(kind, AggroAiKind::Turret) { return AggroAttackDecision::Allowed; }
    (policies.can_attack)(AggroTurretFacts {
        kind: *kind, first_spell_id: actor.creature.spells()[0],
        difficulty: candidate.map_difficulty_id,
        position: actor.position(), combat_reach: actor.creature.unit().world().combat_reach(),
        target_position: candidate.position, target_combat_reach: candidate.player_combat_reach,
    })
}

pub fn trigger_alert(actor: &mut WorldCreature, candidate: &AggroCandidate,
    policies: &mut AggroPolicies<'_>) -> bool {
    // C++ `CreatureAI::TriggerAlert` after `CreatureUnitRelocationWorker`:
    // only hostile stealthed players can distract an alive, non-engaged,
    // non-controlled aggressive creature. The APP retains `SMSG_AI_REACTION`
    // publication; this motor retains the original MoveDistract(5s, angle).
    if !candidate_has_stealth(candidate) { return false; }
    if actor.creature.ai_ownership().combat_target.is_some() { return false; }
    if actor.creature.is_civilian_like_cpp() || actor.creature.has_react_state(wow_entities::ReactState::Passive) {
        return false;
    }
    if actor.creature.unit().has_unit_state((UnitState::CONFUSED | UnitState::STUNNED
        | UnitState::FLEEING | UnitState::DISTRACTED).bits()) { return false; }
    if !candidate_targetable(candidate) || !candidate_hostile(actor, candidate, policies).unwrap_or(false) {
        return false;
    }
    let orientation = actor.position().angle_to(&candidate.position);
    actor.begin_distract_movement_like_cpp(5_000, orientation).is_some()
}
