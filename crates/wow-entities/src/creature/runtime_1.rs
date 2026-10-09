//! Canonical owner of the persistent creature runtime state, part 1 of 1.
//!
//! #1263 F6-8A moves the runtime state that C++ keeps **on the object** —
//! `Unit::movespline`, the active `MovementGenerator`s and the path corridors
//! their `PathGenerator`s hold, `Unit::i_motionMaster`, the chase target, the
//! represented-active selector key, the delayed `AssistDelayEvent` payload,
//! `m_AlreadyCallAssistance`, the active `SPELL_AURA_MOD_TAUNT`s, the
//! `CombatAI::_events` spell deadline/epoch slots, the creature elapsed-time
//! clock and the creature-owned runtime RNG with its authority marker — into
//! the canonical `Creature` owner. The legacy `WorldCreature` bridge keeps only
//! the immutable packet projection (`CreatureCreateData`) and the canonical
//! entity itself.
//!
//! Behaviour is preserved: every field keeps its name, its type and its
//! initial value, and the clone semantics that the legacy bridge applied
//! (a fresh `MotionMaster` with cleared chase target and represented-active
//! key) are reproduced by [`Creature`]'s manual `Clone`.

use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::{MovementGeneratorMode, MovementGeneratorPriority, MovementGeneratorRef};
use wow_movement::{
    ChaseMovementGenerator, HomeMovementGenerator, IdleMovementGenerator, MotionMaster, MoveSpline,
    MovementGenerator as RuntimeMovementGenerator,
    MovementGeneratorFlags as RuntimeMovementGeneratorFlags,
    MovementGeneratorMode as RuntimeMovementGeneratorMode,
    MovementGeneratorPriority as RuntimeMovementGeneratorPriority,
    MovementGeneratorState as RuntimeMovementGeneratorState,
    MovementGeneratorType as RuntimeMovementGeneratorType, RandomMovementGenerator,
    WaypointMovementGenerator, WaypointRandomAtPathEnd,
};

use super::*;

/// One active `SPELL_AURA_MOD_TAUNT` in application order.
///
/// Moved verbatim out of the legacy map owner under #1263 F6-8A; the type now
/// lives with the canonical runtime state it is stored in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActiveTauntLikeCpp {
    pub caster: ObjectGuid,
    /// `None` represents C++/DB2's permanent duration sentinel `-1`.
    pub due_at_ms: Option<u64>,
    pub spell_id: u32,
    pub effect_mask: u32,
    pub slot: u8,
}

/// Runtime selector proxy for an active generator whose concrete lifecycle
/// still lives in `MotionSubsystem`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeRepresentedActiveKeyLikeCpp {
    pub kind: RuntimeMovementGeneratorType,
    mode: RuntimeMovementGeneratorMode,
    priority: RuntimeMovementGeneratorPriority,
    base_unit_state: u32,
}

/// Runtime selector proxy for an active generator whose concrete lifecycle
/// still lives in `MotionSubsystem`.
#[derive(Debug)]
pub struct RuntimeRepresentedActiveGeneratorLikeCpp {
    state: RuntimeMovementGeneratorState,
    kind: RuntimeMovementGeneratorType,
}

impl RuntimeRepresentedActiveGeneratorLikeCpp {
    pub fn from_represented(generator: MovementGeneratorRef) -> Option<Self> {
        let kind = RuntimeMovementGeneratorType::from_trinity_id(generator.kind.trinity_id())?;
        let mode = match generator.mode {
            MovementGeneratorMode::Default => RuntimeMovementGeneratorMode::Default,
            MovementGeneratorMode::Override => RuntimeMovementGeneratorMode::Override,
        };
        let priority = match generator.priority {
            MovementGeneratorPriority::None => RuntimeMovementGeneratorPriority::None,
            MovementGeneratorPriority::Normal => RuntimeMovementGeneratorPriority::Normal,
            MovementGeneratorPriority::Highest => RuntimeMovementGeneratorPriority::Highest,
        };
        Some(Self {
            state: RuntimeMovementGeneratorState {
                mode,
                priority,
                flags: RuntimeMovementGeneratorFlags::INITIALIZATION_PENDING,
                base_unit_state: generator.base_unit_state,
            },
            kind,
        })
    }

    pub const fn key(&self) -> RuntimeRepresentedActiveKeyLikeCpp {
        RuntimeRepresentedActiveKeyLikeCpp {
            kind: self.kind,
            mode: self.state.mode,
            priority: self.state.priority,
            base_unit_state: self.state.base_unit_state,
        }
    }
}

impl RuntimeMovementGenerator for RuntimeRepresentedActiveGeneratorLikeCpp {
    fn state(&self) -> &RuntimeMovementGeneratorState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut RuntimeMovementGeneratorState {
        &mut self.state
    }

    fn kind(&self) -> RuntimeMovementGeneratorType {
        self.kind
    }

    fn initialize(&mut self) {
        self.state.flags.remove(
            RuntimeMovementGeneratorFlags::INITIALIZATION_PENDING
                | RuntimeMovementGeneratorFlags::DEACTIVATED,
        );
        self.state
            .flags
            .insert(RuntimeMovementGeneratorFlags::INITIALIZED);
    }

    fn reset(&mut self) {
        self.initialize();
    }

    fn update(&mut self, _diff_ms: u32) -> bool {
        !self
            .state
            .flags
            .contains(RuntimeMovementGeneratorFlags::FINALIZED)
    }

    fn deactivate(&mut self) {
        self.state
            .flags
            .insert(RuntimeMovementGeneratorFlags::DEACTIVATED);
    }

    fn finalize(&mut self, _active: bool, _movement_inform: bool) {
        self.state
            .flags
            .insert(RuntimeMovementGeneratorFlags::FINALIZED);
    }
}

/// The persistent creature runtime state that C++ keeps on `Unit`/`Creature`.
///
/// This is the canonical owner; the legacy map bridge holds no copy of it. The
/// field set, order and every initial value are moved verbatim from the former
/// `WorldCreature` so the runtime phase order is unchanged.
#[derive(Debug, PartialEq)]
pub struct CreatureRuntimeLikeCpp {
    /// Active movement spline for the represented world tick.
    ///
    /// This is the first runtime bridge toward C++ `Unit::movespline`; the full
    /// `MoveSplineInit`/`MotionMaster` port still owns generalized launch/stop.
    pub active_move_spline: Option<MoveSpline>,
    pub active_random_generator: Option<RandomMovementGenerator>,
    /// Corridor kept by the random generator's `PathGenerator`, which C++
    /// allocates once per generator lifetime and only drops in `DoInitialize`
    /// (`RandomMovementGenerator.cpp:95,140-143`). Reusing it is what makes
    /// `BuildPolyPath`'s subpath/suffix branches reachable
    /// (`PathGenerator.cpp:291-413`).
    pub active_random_path_poly_refs: Vec<u64>,
    /// Selected home generator, kept so its C++ flags survive ticks.
    pub active_home_generator: Option<HomeMovementGenerator>,
    /// Selected chase generator, kept so its C++ state survives ticks.
    pub active_chase_generator: Option<ChaseMovementGenerator>,
    /// Corridor held by the chase generator's `PathGenerator`, which C++
    /// keeps alive across updates (`ChaseMovementGenerator.cpp:174-175`).
    pub active_chase_path_poly_refs: Vec<u64>,
    pub active_waypoint_generator: Option<WaypointMovementGenerator>,
    pub active_waypoint_random_at_path_end: Option<WaypointRandomAtPathEnd>,
    /// C++ `Unit::i_motionMaster`: the persistent priority stack that selects
    /// which concrete runtime generator may advance this frame.
    pub runtime_motion_master: MotionMaster,
    pub runtime_chase_target: Option<ObjectGuid>,
    pub runtime_represented_active: Option<RuntimeRepresentedActiveKeyLikeCpp>,
    /// Caller-owned delayed `AssistDelayEvent` payload: victim, assistant
    /// GUIDs, and map-local due time.
    pub pending_assistance_like_cpp: Vec<(ObjectGuid, Vec<ObjectGuid>, u64)>,
    /// C++ `m_AlreadyCallAssistance`, reset when combat stops.
    pub assistance_called_like_cpp: bool,
    /// Active `SPELL_AURA_MOD_TAUNT`s in application order: caster and expiry.
    pub active_taunts_like_cpp: Vec<ActiveTauntLikeCpp>,
    /// C++ `CombatAI::_events` due times for the eight template spell slots.
    /// `None` means that slot is not scheduled for the current engagement.
    pub creature_spell_due_at_ms_like_cpp: [Option<u64>; MAX_CREATURE_SPELLS],
    /// C++ initializes and resets `CombatAI::_events` once per AI lifecycle.
    /// The canonical owner keeps that lifecycle bit beside the due times so
    /// multiple player sessions cannot independently schedule the same cast.
    pub creature_spell_schedule_initialized_like_cpp: bool,
    /// Monotonic engagement token carried by deferred session commands. It
    /// invalidates a queued cast after evade/death/reset even when the same
    /// creature later attacks the same player again.
    pub creature_spell_engagement_epoch_like_cpp: u64,
    /// Owned runtime RNG for C++ `urand`/`frand`-style gameplay rolls.
    ///
    /// #1263 F6-8A completion: the reviewer required the RNG to move in A. It
    /// lived on the legacy `WorldCreature` bridge as the only copy; the
    /// canonical owner now owns it, its authority marker and the creature
    /// elapsed-time state. Draw order, the `from_entropy` seed installed by
    /// [`Self::new_like_cpp`], the `seed_from_u64` fixture seam and the
    /// clone carry-over are unchanged.
    runtime_rng_like_cpp: StdRng,
    /// False after the creature-spell slice reaches a C++ RNG decision whose
    /// exact number/order of draws is unknown. The marker prevents later spell
    /// casts from claiming exact RNG authority, but it must not disable the
    /// pre-existing best-effort melee and movement runtimes.
    runtime_rng_authority_complete_like_cpp: bool,
    /// C++ `Unit::Update(p_time)` advances every creature-local deadline from
    /// the `Map::Update(t_diff)` value. This logical clock is advanced only by
    /// the owning creature tick; scheduler delay or time spent between phases
    /// cannot independently move spline, combat, spell, assistance or corpse
    /// state.
    runtime_elapsed_ms_like_cpp: u64,
}

impl CreatureRuntimeLikeCpp {
    /// The state a creature that is not yet in the world carries.
    ///
    /// The `MotionMaster` is supplied by the caller because C++
    /// `Unit::i_motionMaster` is initialized together with the unit's default
    /// movement generator, which reads the creature's own movement type.
    pub fn new_like_cpp(runtime_motion_master: MotionMaster) -> Self {
        Self {
            active_move_spline: None,
            active_random_generator: None,
            active_random_path_poly_refs: Vec::new(),
            active_home_generator: None,
            active_chase_generator: None,
            active_chase_path_poly_refs: Vec::new(),
            active_waypoint_generator: None,
            active_waypoint_random_at_path_end: None,
            runtime_motion_master,
            runtime_chase_target: None,
            runtime_represented_active: None,
            pending_assistance_like_cpp: Vec::new(),
            assistance_called_like_cpp: false,
            active_taunts_like_cpp: Vec::new(),
            creature_spell_due_at_ms_like_cpp: [None; MAX_CREATURE_SPELLS],
            creature_spell_schedule_initialized_like_cpp: false,
            creature_spell_engagement_epoch_like_cpp: 0,
            runtime_rng_like_cpp: StdRng::from_entropy(),
            runtime_rng_authority_complete_like_cpp: true,
            runtime_elapsed_ms_like_cpp: 0,
        }
    }

    /// Clone the persistent runtime state for a new creature object.
    ///
    /// This preserves the legacy bridge's clone semantics exactly: the
    /// persistent generators, spline, corridors, assistance payload, taunts,
    /// spell deadline slots, elapsed-time clock, RNG authority marker and the
    /// RNG stream itself are carried over, while `Unit::i_motionMaster` is
    /// rebuilt for the receiving creature and the chase target and
    /// represented-active key start empty.
    pub fn cloned_for_like_cpp(creature: &Creature, source: &Self) -> Self {
        Self {
            active_move_spline: source.active_move_spline.clone(),
            active_random_generator: source.active_random_generator.clone(),
            active_random_path_poly_refs: source.active_random_path_poly_refs.clone(),
            active_home_generator: source.active_home_generator.clone(),
            active_chase_generator: source.active_chase_generator,
            active_chase_path_poly_refs: source.active_chase_path_poly_refs.clone(),
            active_waypoint_generator: source.active_waypoint_generator.clone(),
            active_waypoint_random_at_path_end: source.active_waypoint_random_at_path_end,
            runtime_motion_master: new_runtime_motion_master_like_cpp(creature),
            runtime_chase_target: None,
            runtime_represented_active: None,
            pending_assistance_like_cpp: source.pending_assistance_like_cpp.clone(),
            assistance_called_like_cpp: source.assistance_called_like_cpp,
            active_taunts_like_cpp: source.active_taunts_like_cpp.clone(),
            creature_spell_due_at_ms_like_cpp: source.creature_spell_due_at_ms_like_cpp,
            creature_spell_schedule_initialized_like_cpp: source
                .creature_spell_schedule_initialized_like_cpp,
            creature_spell_engagement_epoch_like_cpp: source
                .creature_spell_engagement_epoch_like_cpp,
            runtime_rng_like_cpp: source.runtime_rng_like_cpp.clone(),
            runtime_rng_authority_complete_like_cpp: source.runtime_rng_authority_complete_like_cpp,
            runtime_elapsed_ms_like_cpp: source.runtime_elapsed_ms_like_cpp,
        }
    }

    /// Canonical accessor for the creature-local elapsed time that C++
    /// `Unit::Update(p_time)` reads. The legacy scheduling bridges read it
    /// through here; no bridge holds a copy.
    pub const fn runtime_elapsed_ms_like_cpp(&self) -> u64 {
        self.runtime_elapsed_ms_like_cpp
    }

    /// Advance the creature-local clock by one owning tick's `Map::Update`
    /// difference. Saturating addition is the pre-existing behaviour.
    pub fn advance_runtime_clock_like_cpp(&mut self, diff_ms: u32) {
        self.runtime_elapsed_ms_like_cpp = self
            .runtime_elapsed_ms_like_cpp
            .saturating_add(u64::from(diff_ms));
    }

    /// Install an absolute creature-local elapsed time.
    ///
    /// Test seam for the fixtures that backdate `Unit::Update`; production
    /// ticks only ever advance the clock.
    pub fn set_runtime_elapsed_ms_like_cpp(&mut self, elapsed_ms: u64) {
        self.runtime_elapsed_ms_like_cpp = elapsed_ms;
    }

    /// Canonical accessor for the exact-RNG-authority marker.
    pub const fn runtime_rng_authority_complete_like_cpp(&self) -> bool {
        self.runtime_rng_authority_complete_like_cpp
    }

    /// Permanently tombstone exact creature-spell RNG authority for this loaded
    /// creature. C++ keeps the same generator across combat resets, so neither
    /// a new target nor a new engagement epoch can restore a provable draw
    /// position.
    pub fn invalidate_runtime_rng_authority_like_cpp(&mut self) {
        self.runtime_rng_authority_complete_like_cpp = false;
    }

    /// Replace the creature-owned stream with a deterministic seed.
    ///
    /// Test seam used by the translated fixtures; interaction never re-seeds a
    /// loaded creature.
    pub fn seed_runtime_rng_like_cpp(&mut self, seed: u64) {
        self.runtime_rng_like_cpp = StdRng::seed_from_u64(seed);
    }

    /// Draw from the creature-owned stream.
    ///
    /// Callers must keep the C++ draw order: this accessor exists so the state
    /// has one owner, not so a caller can re-seed or re-order it.
    pub const fn runtime_rng_like_cpp_mut(&mut self) -> &mut StdRng {
        &mut self.runtime_rng_like_cpp
    }
}

/// C++ `Creature::GetDefaultMovementType()` selects the `MotionMaster` default
/// generator (`MotionMaster::Initialize`).
pub fn runtime_default_generator_like_cpp(
    creature: &Creature,
) -> Box<dyn RuntimeMovementGenerator> {
    match creature.default_movement_type() {
        MovementGeneratorType::Idle => Box::new(IdleMovementGenerator::new()),
        MovementGeneratorType::Random => Box::new(RandomMovementGenerator::new(
            creature.ai_ownership().wander_radius,
            None,
        )),
        MovementGeneratorType::Waypoint => Box::new(WaypointMovementGenerator::from_db_path_id(
            creature.waypoint_path_id_like_cpp(),
            true,
        )),
    }
}

/// Build the persistent `Unit::i_motionMaster` for one creature object.
pub fn new_runtime_motion_master_like_cpp(creature: &Creature) -> MotionMaster {
    let mut motion_master = MotionMaster::new(runtime_default_generator_like_cpp(creature));
    if creature.ai_state() == CreatureAiState::InCombat
        && let Some(target) = creature.ai_ownership().combat_target
    {
        motion_master.add(
            Box::new(ChaseMovementGenerator::new(target, None, None)),
            wow_movement::MovementSlot::Active,
        );
    }
    motion_master
}

/// Manual `Clone` for the canonical creature.
///
/// `Creature` owns `Unit::i_motionMaster`, whose boxed generators and delayed
/// closures are not cloneable. The clone therefore rebuilds the motion master
/// for the receiving creature — the exact semantics the legacy `WorldCreature`
/// bridge applied before #1263 F6-8A — and carries every other field over
/// unchanged.
impl Clone for Creature {
    fn clone(&self) -> Self {
        let mut clone = Self {
            unit: self.unit.clone(),
            player_damage_req: self.player_damage_req,
            dont_clear_tap_list_on_evade: self.dont_clear_tap_list_on_evade,
            pickpocket_loot_restore: self.pickpocket_loot_restore,
            corpse_remove_time: self.corpse_remove_time,
            respawn_time: self.respawn_time,
            respawn_delay: self.respawn_delay,
            corpse_delay: self.corpse_delay,
            ignore_corpse_decay_ratio: self.ignore_corpse_decay_ratio,
            wander_distance: self.wander_distance,
            boundary_check_time: self.boundary_check_time,
            combat_pulse_time: self.combat_pulse_time,
            combat_pulse_delay: self.combat_pulse_delay,
            react_state: self.react_state,
            default_movement_type: self.default_movement_type,
            waypoint_path_id: self.waypoint_path_id,
            spawn_id: self.spawn_id,
            equipment_id: self.equipment_id,
            original_equipment_id: self.original_equipment_id,
            already_call_assistance: self.already_call_assistance,
            already_searched_assistance: self.already_searched_assistance,
            cannot_reach_target: self.cannot_reach_target,
            cannot_reach_timer: self.cannot_reach_timer,
            melee_damage_school_mask: self.melee_damage_school_mask,
            original_entry: self.original_entry,
            trigger_just_appeared: self.trigger_just_appeared,
            respawn_compatibility_mode: self.respawn_compatibility_mode,
            last_damaged_time: self.last_damaged_time,
            regenerate_health: self.regenerate_health,
            is_missing_can_swim_flag_out_of_combat: self.is_missing_can_swim_flag_out_of_combat,
            unit_type_mask: self.unit_type_mask,
            gossip_menu_id: self.gossip_menu_id,
            sparring_health_pct: self.sparring_health_pct,
            regen_timer: self.regen_timer,
            spells: self.spells,
            disable_reputation_gain: self.disable_reputation_gain,
            sight_distance: self.sight_distance,
            combat_distance: self.combat_distance,
            loot_mode: self.loot_mode,
            is_temp_world_object: self.is_temp_world_object,
            grid_unload_cleanup_before_delete_count: self.grid_unload_cleanup_before_delete_count,
            grid_unload_delete_requested: self.grid_unload_delete_requested,
            grid_unload_respawn_relocation_requested: self.grid_unload_respawn_relocation_requested,
            owned_dynamic_objects: self.owned_dynamic_objects.clone(),
            removed_dynamic_objects_from_grid_unload: self
                .removed_dynamic_objects_from_grid_unload
                .clone(),
            owned_area_triggers: self.owned_area_triggers.clone(),
            removed_area_triggers_from_grid_unload: self
                .removed_area_triggers_from_grid_unload
                .clone(),
            lifecycle_metadata: self.lifecycle_metadata.clone(),
            runtime_state: self.runtime_state.clone(),
            ai_ownership: self.ai_ownership.clone(),
            tap_list: self.tap_list.clone(),
            attack_reputation_faction_id: self.attack_reputation_faction_id,
            is_contested_guard_faction: self.is_contested_guard_faction,
            spell_focus: self.spell_focus.clone(),
            combat_log_stats: self.combat_log_stats.clone(),
            avoidance_like_cpp: self.avoidance_like_cpp.clone(),
            loot_lifecycle_revision: self.loot_lifecycle_revision,
            loot_authority: self.loot_authority.clone(),
            shared_loot: self.shared_loot.clone(),
            personal_loot: self.personal_loot.clone(),
            runtime_like_cpp: CreatureRuntimeLikeCpp::new_like_cpp(MotionMaster::new_pending()),
        };
        let runtime = CreatureRuntimeLikeCpp::cloned_for_like_cpp(&clone, &self.runtime_like_cpp);
        clone.runtime_like_cpp = runtime;
        clone
    }
}
