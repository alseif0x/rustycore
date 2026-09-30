//! Immutable phase facts and the owned tail cursor; no Actor/generator/RNG clone.
use super::*;
const LIQUID_MAP_IN_WATER_LIKE_CPP: u32 = 4;
const LIQUID_MAP_UNDER_WATER_LIKE_CPP: u32 = 8;

pub(crate) struct AggroFrame {
    pub(super) map_id: u16, pub(super) instance_id: u32,
    pub(super) primary_guids: Vec<ObjectGuid>, pub(super) secondary_guids: Vec<ObjectGuid>,
    pub(super) candidates: Vec<AggroCandidate>, pub(super) settings: AggroSettings,
    pub(super) owners: HashMap<ObjectGuid, AggroOwnerSnapshot>,
    pub(super) factions: HashMap<ObjectGuid, i32>,
    pub(super) calls: Vec<(ObjectGuid, ObjectGuid, WorldObject, i32)>,
    pub(super) call_index: usize, pub(super) assistant_index: usize,
    pub(super) assistants: Vec<ObjectGuid>,
    pub(super) outcome: AggroOutcome,
}

impl AggroFrame {
    pub(crate) fn new(map_id: u16, instance_id: u32, primary_guids: Vec<ObjectGuid>,
        candidates: Vec<AggroCandidate>, settings: AggroSettings, backend: &AggroMap<'_>) -> Self {
        let mut owners: HashMap<_, _> = candidates
        .iter()
        .map(|candidate| {
            (
                candidate.player_guid,
                AggroOwnerSnapshot {
                    map_id: candidate.map_id,
                    instance_id: candidate.instance_id,
                    position: candidate.position,
                    phase_shift: candidate.player_phase_shift.clone(),
                    combat_reach: candidate.player_combat_reach,
                    alive: !UnitState::from_bits_truncate(candidate.player_unit_state)
                        .contains(UnitState::DIED),
                    in_water: candidate.player_liquid_status
                        & (LIQUID_MAP_IN_WATER_LIKE_CPP | LIQUID_MAP_UNDER_WATER_LIKE_CPP)
                        != 0,
                    in_evade_mode: false,
                    unit_flags: UnitFlags::from_bits_truncate(candidate.player_unit_flags),
                    faction_template_id: Some(candidate.player_faction_template_id),
                    school_immunity_mask: candidate.player_school_immunity_mask,
                    damage_immunity_mask: candidate.player_damage_immunity_mask,
                    has_confuse_aura: candidate.player_has_confuse_aura,
                    has_breakable_stun_aura: candidate.player_has_breakable_stun_aura,
                },
            )
        })
        .collect();

        let guids = backend.guids(&primary_guids);
        // Canonical counterpart admission precedes the first actor mutation;
        // registry facts cannot manufacture a missing canonical victim.
        owners.retain(|guid, owner| !guid.is_player() || owner.map_id != map_id
            || owner.instance_id != instance_id || backend.player_present(*guid));
        let mut creature_factions = HashMap::new();
        for owner_guid in &guids {
            if let Some(owner) = backend.actor(*owner_guid) {
                owners.insert(
                    *owner_guid,
                    AggroOwnerSnapshot {
                        map_id,
                        instance_id,
                        position: owner.position(),
                        phase_shift: owner.phase_shift().clone(),
                        combat_reach: owner.creature.unit().world().combat_reach(),
                        alive: owner.is_alive(),
                        in_water: owner
                            .creature
                            .movement_flags_like_cpp()
                            .contains(wow_constants::movement::MovementFlag::SWIMMING),
                        in_evade_mode: owner.creature.is_in_evade_mode_like_cpp(),
                        unit_flags: owner.creature.unit().unit_flags_like_cpp(),
                        faction_template_id: u32::try_from(
                            owner.creature.unit().data().faction_template,
                        )
                        .ok(),
                        school_immunity_mask: owner
                            .creature
                            .unit()
                            .subsystems()
                            .auras
                            .aura_school_mask_like_cpp(
                                wow_constants::spell::aura_types::SPELL_AURA_SCHOOL_IMMUNITY,
                            ),
                        damage_immunity_mask: owner
                            .creature
                            .unit()
                            .subsystems()
                            .auras
                            .aura_school_mask_like_cpp(
                                wow_constants::spell::aura_types::SPELL_AURA_DAMAGE_IMMUNITY,
                            ),
                        has_confuse_aura: owner
                            .creature
                            .unit()
                            .subsystems()
                            .auras
                            .has_aura_type_like_cpp(
                                wow_constants::spell::aura_types::SPELL_AURA_MOD_CONFUSE,
                            ),
                        has_breakable_stun_aura: owner
                            .creature
                            .unit()
                            .subsystems()
                            .auras
                            .has_breakable_by_damage_aura_type_like_cpp(
                                wow_constants::spell::aura_types::SPELL_AURA_MOD_STUN,
                            ),
                    },
                );
                creature_factions
                    .insert(*owner_guid, owner.creature.unit().data().faction_template);
            }
        }

        let candidates_seen = candidates.iter().filter(|candidate|
            candidate.map_id == map_id && candidate.instance_id == instance_id).count();
        Self { map_id, instance_id, primary_guids, secondary_guids: guids,
            candidates, settings, owners, factions: creature_factions,
            calls: Vec::new(), call_index: 0, assistant_index: 0, assistants: Vec::new(),
            outcome: AggroOutcome { maps_seen: 1, candidates_seen, ..AggroOutcome::default() },
        }
    }
}

pub(crate) enum AggroTailProgress { Complete(AggroOutcome), Pending(assistance::AggroLosPending) }
