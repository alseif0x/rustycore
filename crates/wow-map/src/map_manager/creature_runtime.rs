// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private runtime state carried by the existing live `WorldCreature`.
//!
//! Moving the actor carries this state by value. Snapshot cloning retains the
//! existing MotionMaster reset contract. This extraction prepares a later actor
//! transfer; it does not unify the canonical and legacy creature owners.

use super::{
    ActiveTauntLikeCpp, ChaseMovementGenerator, Creature, HomeMovementGenerator,
    MotionMaster, MoveSpline, ObjectGuid, RandomMovementGenerator,
    RuntimeRepresentedActiveKeyLikeCpp, WaypointMovementGenerator, WaypointRandomAtPathEnd,
    WorldCreature,
};
use rand::{SeedableRng, rngs::StdRng};

#[derive(Debug)]
pub(in crate::map_manager) struct WorldCreatureRuntime {
    /// Active movement spline for the represented world tick.
    ///
    /// This is the first runtime bridge toward C++ `Unit::movespline`; the full
    /// `MoveSplineInit`/`MotionMaster` port still owns generalized launch/stop.
    pub(in crate::map_manager) active_move_spline: Option<MoveSpline>,
    pub(in crate::map_manager) active_random_generator: Option<RandomMovementGenerator>,
    /// Corridor kept by the random generator's `PathGenerator`, which C++
    /// allocates once per generator lifetime and only drops in `DoInitialize`
    /// (`RandomMovementGenerator.cpp:95,140-143`). Reusing it is what makes
    /// `BuildPolyPath`'s subpath/suffix branches reachable
    /// (`PathGenerator.cpp:291-413`).
    pub(in crate::map_manager) active_random_path_poly_refs: Vec<u64>,
    /// Selected home generator, kept so its C++ flags survive ticks.
    pub(in crate::map_manager) active_home_generator: Option<HomeMovementGenerator>,
    /// Selected chase generator, kept so its C++ state survives ticks.
    pub(in crate::map_manager) active_chase_generator: Option<ChaseMovementGenerator>,
    /// Corridor held by the chase generator's `PathGenerator`, which C++ keeps
    /// alive across updates (`ChaseMovementGenerator.cpp:174-175`).
    pub(in crate::map_manager) active_chase_path_poly_refs: Vec<u64>,
    pub(in crate::map_manager) active_waypoint_generator: Option<WaypointMovementGenerator>,
    pub(in crate::map_manager) active_waypoint_random_at_path_end: Option<WaypointRandomAtPathEnd>,
    /// C++ `Unit::i_motionMaster`: the persistent priority stack that selects
    /// which concrete runtime generator may advance this frame.
    pub(in crate::map_manager) runtime_motion_master: MotionMaster,
    pub(in crate::map_manager) runtime_chase_target: Option<ObjectGuid>,
    pub(in crate::map_manager) runtime_represented_active: Option<RuntimeRepresentedActiveKeyLikeCpp>,
    /// Caller-owned delayed `AssistDelayEvent` payload: victim, assistant
    /// GUIDs, and map-local due time.
    pub(in crate::map_manager) pending_assistance_like_cpp: Vec<(ObjectGuid, Vec<ObjectGuid>, u64)>,
    /// C++ `m_AlreadyCallAssistance`, reset when combat stops.
    pub(in crate::map_manager) assistance_called_like_cpp: bool,
    /// Active `SPELL_AURA_MOD_TAUNT`s in application order: caster and expiry.
    pub(in crate::map_manager) active_taunts_like_cpp: Vec<ActiveTauntLikeCpp>,
    /// C++ `CombatAI::_events` due times for the eight template spell slots.
    /// `None` means that slot is not scheduled for the current engagement.
    pub(in crate::map_manager) creature_spell_due_at_ms_like_cpp: [Option<u64>; wow_entities::MAX_CREATURE_SPELLS],
    /// C++ initializes and resets `CombatAI::_events` once per AI lifecycle.
    /// The legacy map owner keeps that lifecycle bit beside the due times so
    /// multiple player sessions cannot independently schedule the same cast.
    pub(in crate::map_manager) creature_spell_schedule_initialized_like_cpp: bool,
    /// Monotonic engagement token carried by deferred session commands. It
    /// invalidates a queued cast after evade/death/reset even when the same
    /// creature later attacks the same player again.
    pub(in crate::map_manager) creature_spell_engagement_epoch_like_cpp: u64,
    /// Set by reached-home finalization until the global movement owner
    /// publishes the restored health values update.
    pub(in crate::map_manager) home_health_restored_pending_like_cpp: bool,
    pub(in crate::map_manager) runtime_motion_master_ticks: u64,
    /// False after the creature-spell slice reaches a C++ RNG decision whose
    /// exact number/order of draws is unknown. The marker prevents later spell
    /// casts from claiming exact RNG authority, but it must not disable the
    /// pre-existing best-effort melee and movement runtimes.
    pub(in crate::map_manager) runtime_rng_authority_complete_like_cpp: bool,
    /// DB-backed aura-source proofs that may be re-accredited only after the
    /// respawn rail reapplies the captured creature/template addon source.
    /// These are provenance, not the live AuraSubsystem markers: ordinary aura
    /// mutations still revoke the live markers permanently for that lifetime.
    pub(in crate::map_manager) respawn_spell_hit_aura_source_authority_like_cpp: bool,
    pub(in crate::map_manager) respawn_spell_cast_log_aura_source_authority_like_cpp: bool,
    pub(in crate::map_manager) runtime_rng_like_cpp: StdRng,
    /// C++ `Unit::Update(p_time)` advances every creature-local deadline from
    /// the `Map::Update(t_diff)` value. This logical clock is advanced only by
    /// the owning creature tick; scheduler delay or time spent between phases
    /// cannot independently move spline, combat, spell, assistance or corpse
    /// state.
    pub(in crate::map_manager) runtime_elapsed_ms_like_cpp: u64,
}

impl WorldCreatureRuntime {
    pub(in crate::map_manager) fn new(runtime_motion_master: MotionMaster) -> Self {
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
            creature_spell_due_at_ms_like_cpp: [None; wow_entities::MAX_CREATURE_SPELLS],
            creature_spell_schedule_initialized_like_cpp: false,
            creature_spell_engagement_epoch_like_cpp: 0,
            home_health_restored_pending_like_cpp: false,
            runtime_motion_master_ticks: 0,
            runtime_rng_authority_complete_like_cpp: true,
            respawn_spell_hit_aura_source_authority_like_cpp: false,
            respawn_spell_cast_log_aura_source_authority_like_cpp: false,
            runtime_rng_like_cpp: StdRng::from_entropy(),
            runtime_elapsed_ms_like_cpp: 0,
        }
    }

    pub(in crate::map_manager) fn clone_for_creature(&self, creature: &Creature) -> Self {
        Self {
            runtime_motion_master: WorldCreature::new_runtime_motion_master_like_cpp(creature),
            runtime_chase_target: None,
            runtime_represented_active: None,
            pending_assistance_like_cpp: self.pending_assistance_like_cpp.clone(),
            assistance_called_like_cpp: self.assistance_called_like_cpp,
            active_taunts_like_cpp: self.active_taunts_like_cpp.clone(),
            creature_spell_due_at_ms_like_cpp: self.creature_spell_due_at_ms_like_cpp,
            creature_spell_schedule_initialized_like_cpp: self.creature_spell_schedule_initialized_like_cpp,
            creature_spell_engagement_epoch_like_cpp: self.creature_spell_engagement_epoch_like_cpp,
            home_health_restored_pending_like_cpp: self.home_health_restored_pending_like_cpp,
            runtime_motion_master_ticks: self.runtime_motion_master_ticks,
            runtime_rng_authority_complete_like_cpp: self.runtime_rng_authority_complete_like_cpp,
            respawn_spell_hit_aura_source_authority_like_cpp: self.respawn_spell_hit_aura_source_authority_like_cpp,
            respawn_spell_cast_log_aura_source_authority_like_cpp: self.respawn_spell_cast_log_aura_source_authority_like_cpp,
            active_move_spline: self.active_move_spline.clone(),
            active_random_generator: self.active_random_generator.clone(),
            active_random_path_poly_refs: self.active_random_path_poly_refs.clone(),
            active_home_generator: self.active_home_generator.clone(),
            active_chase_generator: self.active_chase_generator,
            active_chase_path_poly_refs: self.active_chase_path_poly_refs.clone(),
            active_waypoint_generator: self.active_waypoint_generator.clone(),
            active_waypoint_random_at_path_end: self.active_waypoint_random_at_path_end,
            runtime_rng_like_cpp: self.runtime_rng_like_cpp.clone(),
            runtime_elapsed_ms_like_cpp: self.runtime_elapsed_ms_like_cpp,
        }
    }
}

#[cfg(test)]
impl WorldCreature {
    pub(crate) fn seed_actor_storage_runtime(&mut self, point: bool) -> StdRng {
        use rand::RngCore;
        use wow_core::Position;

        let target = ObjectGuid::create_player(1, 99);
        if point {
            // Charge installs an actual Highest-priority Point generator.
            // A normal-priority Point would be replaced by the later Chase;
            // launching its spline alone would install no generator at all.
            self.creature.unit_mut().subsystems_mut().motion.move_charge(42);
            self.begin_move_spline_like_cpp(Position::xyz(20.0, 10.0, 3.0)).unwrap();
        } else {
            // Distract is also Highest-priority and survives normal Chase.
            self.begin_distract_movement_like_cpp(8_000, 1.25).unwrap();
        }
        self.enter_combat(target);
        self.tick_runtime_motion_master_like_cpp(25);
        self.seed_runtime_rng_like_cpp(0xA10E_7022);
        for _ in 0..5 {
            self.runtime.runtime_rng_like_cpp.next_u64();
        }
        self.runtime.runtime_elapsed_ms_like_cpp = 7_000;
        self.runtime.creature_spell_due_at_ms_like_cpp[1] = Some(7_600);
        self.runtime.creature_spell_schedule_initialized_like_cpp = true;
        self.runtime.creature_spell_engagement_epoch_like_cpp = 11;
        self.runtime.pending_assistance_like_cpp.push((target, vec![target], 7_250));
        self.runtime.assistance_called_like_cpp = true;
        self.runtime.active_random_path_poly_refs = vec![101, 102];
        self.runtime.active_chase_path_poly_refs = vec![201, 202];
        self.runtime.runtime_rng_authority_complete_like_cpp = false;
        self.runtime.respawn_spell_hit_aura_source_authority_like_cpp = true;
        self.runtime.respawn_spell_cast_log_aura_source_authority_like_cpp = true;
        self.runtime.runtime_rng_like_cpp.clone()
    }

    pub(crate) fn assert_actor_storage_runtime(&mut self, expected_rng: &mut StdRng, point: bool) {
        use rand::RngCore;
        use super::RuntimeMovementGeneratorType;

        let target = ObjectGuid::create_player(1, 99);
        let expected_kind = if point {
            RuntimeMovementGeneratorType::Point
        } else {
            RuntimeMovementGeneratorType::Distract
        };
        assert_eq!(self.runtime_motion_master_current_kind_like_cpp(), Some(expected_kind));
        assert_eq!(self.runtime.runtime_chase_target, Some(target));
        assert_eq!(self.runtime.runtime_represented_active.map(|key| key.kind), Some(expected_kind));
        assert_eq!(self.runtime.runtime_motion_master_ticks, 1);
        assert_eq!(self.runtime.runtime_elapsed_ms_like_cpp, 7_000);
        assert_eq!(self.runtime.creature_spell_due_at_ms_like_cpp[1], Some(7_600));
        assert!(self.runtime.creature_spell_schedule_initialized_like_cpp);
        assert_eq!(self.runtime.creature_spell_engagement_epoch_like_cpp, 11);
        assert_eq!(self.runtime.pending_assistance_like_cpp, vec![(target, vec![target], 7_250)]);
        assert!(self.runtime.assistance_called_like_cpp);
        assert_eq!(self.runtime.active_random_path_poly_refs, vec![101, 102]);
        assert_eq!(self.runtime.active_chase_path_poly_refs, vec![201, 202]);
        assert!(self.runtime.active_move_spline.is_some());
        assert!(!self.runtime.runtime_rng_authority_complete_like_cpp);
        assert!(self.runtime.respawn_spell_hit_aura_source_authority_like_cpp);
        assert!(self.runtime.respawn_spell_cast_log_aura_source_authority_like_cpp);
        for _ in 0..8 {
            assert_eq!(self.runtime.runtime_rng_like_cpp.next_u64(), expected_rng.next_u64());
        }
    }
}
