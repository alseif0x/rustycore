//! Subsystem packets.
//!
//! Separated from movement.rs under #693.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotionSubsystem {
    pub default_generator: MovementGeneratorRef,
    pub active_generators: Vec<MovementGeneratorRef>,
    pub current_generator: MovementGeneratorKind,
    pub base_unit_states: HashMap<u32, usize>,
    pub flags: u8,
    pub delayed_actions: Vec<MotionMasterDelayedAction>,
    pub paused: bool,
    pub stopped: bool,
    pub spline: MoveSplineState,
}

impl MotionSubsystem {
    pub const fn has_motion_master_flag(&self, flag: u8) -> bool {
        self.flags & flag != 0
    }

    pub const fn should_delay_motion_master_action_like_cpp(&self) -> bool {
        self.has_motion_master_flag(MOTIONMASTER_FLAG_DELAYED)
    }

    pub fn push_delayed_action_like_cpp(&mut self, action_type: MotionMasterDelayedActionType) {
        let payload = match action_type {
            MotionMasterDelayedActionType::Clear => MotionMasterDelayedActionPayload::Clear,
            MotionMasterDelayedActionType::ClearSlot => {
                MotionMasterDelayedActionPayload::ClearSlot(MovementSlot::Active)
            }
            MotionMasterDelayedActionType::ClearMode => {
                MotionMasterDelayedActionPayload::ClearMode(MovementGeneratorMode::Default)
            }
            MotionMasterDelayedActionType::ClearPriority => {
                MotionMasterDelayedActionPayload::ClearPriority(MovementGeneratorPriority::Normal)
            }
            MotionMasterDelayedActionType::Add => MotionMasterDelayedActionPayload::Add(
                MovementGeneratorRef::new(MovementGeneratorKind::Idle, MovementSlot::Active),
            ),
            MotionMasterDelayedActionType::Remove => MotionMasterDelayedActionPayload::Remove {
                kind: MovementGeneratorKind::Idle,
                slot: MovementSlot::Active,
            },
            MotionMasterDelayedActionType::RemoveType => {
                MotionMasterDelayedActionPayload::RemoveType {
                    kind: MovementGeneratorKind::Idle,
                    slot: MovementSlot::Active,
                }
            }
            MotionMasterDelayedActionType::Initialize => {
                MotionMasterDelayedActionPayload::Initialize
            }
        };
        self.push_delayed_payload_like_cpp(payload);
    }

    pub fn push_delayed_action_with_validator_like_cpp(
        &mut self,
        action_type: MotionMasterDelayedActionType,
        validator_passed: bool,
    ) {
        self.delayed_actions
            .push(MotionMasterDelayedAction::with_validator(
                match action_type {
                    MotionMasterDelayedActionType::Clear => MotionMasterDelayedActionPayload::Clear,
                    MotionMasterDelayedActionType::ClearSlot => {
                        MotionMasterDelayedActionPayload::ClearSlot(MovementSlot::Active)
                    }
                    MotionMasterDelayedActionType::ClearMode => {
                        MotionMasterDelayedActionPayload::ClearMode(MovementGeneratorMode::Default)
                    }
                    MotionMasterDelayedActionType::ClearPriority => {
                        MotionMasterDelayedActionPayload::ClearPriority(
                            MovementGeneratorPriority::Normal,
                        )
                    }
                    MotionMasterDelayedActionType::Add => {
                        MotionMasterDelayedActionPayload::Add(MovementGeneratorRef::new(
                            MovementGeneratorKind::Idle,
                            MovementSlot::Active,
                        ))
                    }
                    MotionMasterDelayedActionType::Remove => {
                        MotionMasterDelayedActionPayload::Remove {
                            kind: MovementGeneratorKind::Idle,
                            slot: MovementSlot::Active,
                        }
                    }
                    MotionMasterDelayedActionType::RemoveType => {
                        MotionMasterDelayedActionPayload::RemoveType {
                            kind: MovementGeneratorKind::Idle,
                            slot: MovementSlot::Active,
                        }
                    }
                    MotionMasterDelayedActionType::Initialize => {
                        MotionMasterDelayedActionPayload::Initialize
                    }
                },
                validator_passed,
            ));
    }

    pub fn push_delayed_payload_like_cpp(&mut self, payload: MotionMasterDelayedActionPayload) {
        self.delayed_actions
            .push(MotionMasterDelayedAction::new(payload));
    }

    pub fn push_delayed_payload_with_validator_like_cpp(
        &mut self,
        payload: MotionMasterDelayedActionPayload,
        validator_passed: bool,
    ) {
        self.delayed_actions
            .push(MotionMasterDelayedAction::with_validator(
                payload,
                validator_passed,
            ));
    }

    pub fn resolve_delayed_actions_like_cpp(&mut self) -> Vec<MotionMasterResolvedDelayedAction> {
        self.delayed_actions
            .drain(..)
            .map(|action| MotionMasterResolvedDelayedAction {
                action_type: action.action_type(),
                executed: action.validator_passed,
            })
            .collect()
    }

    pub fn resolve_delayed_action_payloads_like_cpp(
        &mut self,
    ) -> Vec<MotionMasterResolvedDelayedAction> {
        let mut resolved = Vec::new();
        while !self.delayed_actions.is_empty() {
            let action = self.delayed_actions.remove(0);
            if action.validator_passed {
                self.apply_delayed_action_payload_like_cpp(action.payload);
            }
            resolved.push(MotionMasterResolvedDelayedAction {
                action_type: action.action_type(),
                executed: action.validator_passed,
            });
        }
        resolved
    }

    pub fn update_motion_master_like_cpp(
        &mut self,
        context: MotionMasterUpdateContext,
    ) -> MotionMasterUpdateOutcome {
        if self.has_motion_master_flag(
            MOTIONMASTER_FLAG_INITIALIZATION_PENDING | MOTIONMASTER_FLAG_INITIALIZING,
        ) {
            return MotionMasterUpdateOutcome::Stalled;
        }

        if self.is_empty() {
            return MotionMasterUpdateOutcome::Empty;
        }

        self.flags |= MOTIONMASTER_FLAG_UPDATE;

        if self.has_motion_master_flag(MOTIONMASTER_FLAG_STATIC_INITIALIZATION_PENDING)
            && self.current_slot() == MovementSlot::Default
        {
            self.flags &= !MOTIONMASTER_FLAG_STATIC_INITIALIZATION_PENDING;
            self.default_generator
                .initialize_for_motion_master_update_like_cpp(context);
        }

        let keep_running = if self.active_generators.is_empty() {
            initialize_or_reset_for_motion_master_update_like_cpp(
                &mut self.default_generator,
                context,
            );
            self.default_generator
                .update_for_motion_master_like_cpp(context)
        } else {
            let top = &mut self.active_generators[0];
            initialize_or_reset_for_motion_master_update_like_cpp(top, context);
            top.update_for_motion_master_like_cpp(context)
        };

        let popped = if !keep_running && !self.active_generators.is_empty() {
            Some(self.remove_generator_at(0))
        } else {
            None
        };
        self.current_generator = self.current_movement_generator().kind;

        self.flags &= !MOTIONMASTER_FLAG_UPDATE;
        let resolved_delayed_actions = self.resolve_delayed_action_payloads_like_cpp();

        MotionMasterUpdateOutcome::Updated {
            popped,
            resolved_delayed_actions,
        }
    }

    pub fn set_current_generator(&mut self, generator: MovementGeneratorKind) {
        self.add_generator(MovementGeneratorRef::new(generator, MovementSlot::Active));
    }

    pub fn add_to_world(&mut self) {
        let _ = self.add_to_world_like_cpp();
    }

    pub fn add_to_world_like_cpp(&mut self) -> MotionMasterAddToWorldOutcomeLikeCpp {
        let flags_before = self.flags;
        let had_initialization_pending =
            self.has_motion_master_flag(MOTIONMASTER_FLAG_INITIALIZATION_PENDING);

        if !had_initialization_pending {
            return MotionMasterAddToWorldOutcomeLikeCpp {
                had_initialization_pending,
                entered_initializing: false,
                direct_initialize_represented: false,
                resolved_delayed_actions: Vec::new(),
                exited_initializing: false,
                flags_before,
                flags_after: self.flags,
                current_generator_after: self.current_generator,
            };
        }

        self.flags |= MOTIONMASTER_FLAG_INITIALIZING;
        self.flags &= !MOTIONMASTER_FLAG_INITIALIZATION_PENDING;

        self.direct_initialize_like_cpp();
        let resolved_delayed_actions = self.resolve_delayed_action_payloads_like_cpp();

        self.flags &= !MOTIONMASTER_FLAG_INITIALIZING;
        self.current_generator = self.current_movement_generator().kind;

        MotionMasterAddToWorldOutcomeLikeCpp {
            had_initialization_pending,
            entered_initializing: (flags_before & MOTIONMASTER_FLAG_INITIALIZING) == 0,
            direct_initialize_represented: true,
            resolved_delayed_actions,
            exited_initializing: !self.has_motion_master_flag(MOTIONMASTER_FLAG_INITIALIZING),
            flags_before,
            flags_after: self.flags,
            current_generator_after: self.current_generator,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.active_generators.is_empty()
            && self.default_generator.kind == MovementGeneratorKind::Custom(u32::MAX)
    }

    pub fn size(&self) -> usize {
        1 + self.active_generators.len()
    }

    pub fn current_slot(&self) -> MovementSlot {
        if self.active_generators.is_empty() {
            MovementSlot::Default
        } else {
            MovementSlot::Active
        }
    }

    pub fn current_movement_generator(&self) -> MovementGeneratorRef {
        self.active_generators
            .first()
            .copied()
            .unwrap_or(self.default_generator)
    }

    fn movement_generator_for_slot_mut(
        &mut self,
        slot: MovementSlot,
    ) -> Option<&mut MovementGeneratorRef> {
        match slot {
            MovementSlot::Default => Some(&mut self.default_generator),
            MovementSlot::Active => self.active_generators.first_mut(),
        }
    }

    pub fn add_generator(&mut self, mut generator: MovementGeneratorRef) {
        match generator.slot {
            MovementSlot::Default => {
                generator.slot = MovementSlot::Default;
                self.default_generator = generator;
                if generator.kind == MovementGeneratorKind::Idle {
                    self.flags |= MOTIONMASTER_FLAG_STATIC_INITIALIZATION_PENDING;
                }
            }
            MovementSlot::Active => {
                generator.slot = MovementSlot::Active;
                if let Some(top) = self.active_generators.first().copied() {
                    if generator.priority >= top.priority {
                        if generator.priority == top.priority {
                            self.remove_generator_at(0);
                        } else if let Some(top) = self.active_generators.first_mut() {
                            top.flags |= MOVEMENTGENERATOR_FLAG_DEACTIVATED;
                        }
                    } else if let Some(index) = self
                        .active_generators
                        .iter()
                        .position(|known| known.priority == generator.priority)
                    {
                        self.remove_generator_at(index);
                    }
                }

                self.add_base_unit_state(generator.base_unit_state);
                self.active_generators.push(generator);
                self.sort_active_generators();
            }
        }
        self.current_generator = self.current_movement_generator().kind;
        self.stopped = false;
    }

    pub fn remove_generator_kind(
        &mut self,
        kind: MovementGeneratorKind,
        slot: MovementSlot,
    ) -> Option<MovementGeneratorRef> {
        let removed = match slot {
            MovementSlot::Default if self.default_generator.kind == kind => {
                let previous = self.default_generator;
                self.move_idle();
                Some(previous)
            }
            MovementSlot::Default => None,
            MovementSlot::Active => self
                .active_generators
                .iter()
                .position(|generator| generator.kind == kind)
                .map(|index| self.remove_generator_at(index)),
        };
        self.current_generator = self.current_movement_generator().kind;
        removed
    }

    pub fn clear_active(&mut self) -> Vec<MovementGeneratorRef> {
        let removed = std::mem::take(&mut self.active_generators);
        self.base_unit_states.clear();
        self.current_generator = self.default_generator.kind;
        removed
    }

    pub fn clear_slot(&mut self, slot: MovementSlot) -> Vec<MovementGeneratorRef> {
        match slot {
            MovementSlot::Default => {
                let previous = self.default_generator;
                self.move_idle();
                vec![previous]
            }
            MovementSlot::Active => self.clear_active(),
        }
    }

    pub fn clear_by_priority(
        &mut self,
        priority: MovementGeneratorPriority,
    ) -> Vec<MovementGeneratorRef> {
        let mut removed = Vec::new();
        let mut index = 0;
        while index < self.active_generators.len() {
            if self.active_generators[index].priority == priority {
                removed.push(self.remove_generator_at(index));
            } else {
                index += 1;
            }
        }
        self.current_generator = self.current_movement_generator().kind;
        removed
    }

    pub fn clear_by_mode(&mut self, mode: MovementGeneratorMode) -> Vec<MovementGeneratorRef> {
        let mut removed = Vec::new();
        let mut index = 0;
        while index < self.active_generators.len() {
            if self.active_generators[index].mode == mode {
                removed.push(self.remove_generator_at(index));
            } else {
                index += 1;
            }
        }
        self.current_generator = self.current_movement_generator().kind;
        removed
    }

    pub fn direct_initialize_like_cpp(&mut self) {
        let selected_default = self.default_generator.kind;
        self.clear_active();
        self.initialize_default_generator_like_cpp(selected_default);
    }

    fn apply_delayed_action_payload_like_cpp(&mut self, payload: MotionMasterDelayedActionPayload) {
        match payload {
            MotionMasterDelayedActionPayload::Clear => {
                self.clear_active();
            }
            MotionMasterDelayedActionPayload::ClearSlot(slot) => {
                self.clear_slot(slot);
            }
            MotionMasterDelayedActionPayload::ClearMode(mode) => {
                self.clear_by_mode(mode);
            }
            MotionMasterDelayedActionPayload::ClearPriority(priority) => {
                self.clear_by_priority(priority);
            }
            MotionMasterDelayedActionPayload::Add(generator) => {
                self.add_generator(generator);
            }
            MotionMasterDelayedActionPayload::Remove { kind, slot }
            | MotionMasterDelayedActionPayload::RemoveType { kind, slot } => {
                self.remove_generator_kind(kind, slot);
            }
            MotionMasterDelayedActionPayload::Initialize => {
                self.direct_initialize_like_cpp();
            }
        }
    }

    pub fn move_idle(&mut self) {
        self.initialize_default_generator_like_cpp(MovementGeneratorKind::Idle);
        self.flags |= MOTIONMASTER_FLAG_STATIC_INITIALIZATION_PENDING;
        if self.active_generators.is_empty() {
            self.current_generator = MovementGeneratorKind::Idle;
        }
    }

    pub fn initialize_default_generator_like_cpp(&mut self, kind: MovementGeneratorKind) {
        self.default_generator = match kind {
            MovementGeneratorKind::Random | MovementGeneratorKind::Waypoint => {
                MovementGeneratorRef::new(kind, MovementSlot::Default)
                    .with_priority(MovementGeneratorPriority::Normal)
                    .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING)
                    .with_base_unit_state(UnitState::ROAMING.bits())
            }
            MovementGeneratorKind::Idle => {
                MovementGeneratorRef::new(MovementGeneratorKind::Idle, MovementSlot::Default)
                    .with_priority(MovementGeneratorPriority::Normal)
                    .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZED)
            }
            other => MovementGeneratorRef::new(other, MovementSlot::Default)
                .with_priority(MovementGeneratorPriority::Normal)
                .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING),
        };
        if self.active_generators.is_empty() {
            self.current_generator = self.default_generator.kind;
        }
    }

    pub fn move_point(&mut self, movement_id: u32) {
        self.add_generator(
            MovementGeneratorRef::new(MovementGeneratorKind::Point, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Normal)
                .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING)
                .with_base_unit_state(UnitState::ROAMING.bits())
                .with_movement_id(movement_id),
        );
    }

    pub fn move_seek_assistance_like_cpp(&mut self) -> SeekAssistancePlan {
        self.add_generator(
            MovementGeneratorRef::new(MovementGeneratorKind::Assistance, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Normal)
                .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING)
                .with_base_unit_state(UnitState::ROAMING.bits())
                .with_movement_id(EVENT_ASSIST_MOVE),
        );
        SeekAssistancePlan {
            attack_stop: true,
            cast_stop: true,
            do_not_reacquire_spell_focus_target: true,
            set_react_passive: true,
            generator_added: true,
        }
    }

    pub fn move_seek_assistance_distract_like_cpp(&mut self, timer_ms: u32) {
        self.add_generator(
            MovementGeneratorRef::new(
                MovementGeneratorKind::AssistanceDistract,
                MovementSlot::Active,
            )
            .with_priority(MovementGeneratorPriority::Normal)
            .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING)
            .with_base_unit_state(UnitState::DISTRACTED.bits())
            .with_duration_ms(timer_ms),
        );
    }

    pub fn move_distract_like_cpp(&mut self, timer_ms: u32) {
        self.add_generator(
            MovementGeneratorRef::new(MovementGeneratorKind::Distract, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Highest)
                .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING)
                .with_base_unit_state(UnitState::DISTRACTED.bits())
                .with_duration_ms(timer_ms),
        );
    }

    pub fn move_rotate_like_cpp(
        &mut self,
        movement_id: u32,
        time_ms: u32,
        direction: RotateDirection,
    ) -> bool {
        if time_ms == 0 {
            return false;
        }

        self.add_generator(
            MovementGeneratorRef::new(MovementGeneratorKind::Rotate, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Normal)
                .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING)
                .with_base_unit_state(UnitState::ROTATING.bits())
                .with_movement_id(movement_id)
                .with_duration_ms(time_ms)
                .with_max_duration_ms(time_ms)
                .with_rotate_direction(direction),
        );
        true
    }

    pub fn move_charge(&mut self, movement_id: u32) {
        self.add_generator(
            MovementGeneratorRef::new(MovementGeneratorKind::Point, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Highest)
                .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING)
                .with_base_unit_state(UnitState::CHARGING.bits())
                .with_movement_id(movement_id),
        );
    }

    pub fn move_follow(&mut self, target_guid: ObjectGuid, duration_ms: Option<u32>) {
        let mut generator =
            MovementGeneratorRef::new(MovementGeneratorKind::Follow, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Normal)
                .with_target_guid(target_guid);
        if let Some(duration_ms) = duration_ms {
            generator = generator.with_duration_ms(duration_ms);
        }
        self.add_generator(generator);
    }

    pub fn move_chase_like_cpp(&mut self, target_guid: ObjectGuid) {
        self.add_generator(
            MovementGeneratorRef::new(MovementGeneratorKind::Chase, MovementSlot::Active)
                .with_priority(MovementGeneratorPriority::Normal)
                .with_flags(MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING)
                .with_base_unit_state(UnitState::CHASE.bits())
                .with_target_guid(target_guid),
        );
    }

    pub fn launch_generic_movement(
        &mut self,
        kind: MovementGeneratorKind,
        movement_id: u32,
        duration_ms: u32,
        arrival_spell: Option<(u32, ObjectGuid)>,
    ) {
        self.add_generic_movement(
            kind,
            movement_id,
            duration_ms,
            MovementGeneratorPriority::Normal,
            UnitState::ROAMING.bits(),
            MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING,
            arrival_spell,
        );
    }

    pub fn launch_move_spline_like_cpp(
        &mut self,
        kind: MovementGeneratorKind,
        movement_id: u32,
        priority: MovementGeneratorPriority,
        duration_ms: u32,
    ) -> bool {
        let trinity_type = kind.trinity_id();
        if trinity_type == 3 || trinity_type >= 19 {
            return false;
        }

        self.add_generic_movement(
            kind,
            movement_id,
            duration_ms,
            priority,
            UnitState::ROAMING.bits(),
            MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING,
            None,
        );
        true
    }

    pub fn move_jump_like_cpp(
        &mut self,
        movement_id: u32,
        duration_ms: u32,
        speed_xy: f32,
        arrival_spell: Option<(u32, ObjectGuid)>,
    ) -> bool {
        if speed_xy < 0.01 {
            return false;
        }

        self.add_generic_movement(
            MovementGeneratorKind::Effect,
            movement_id,
            duration_ms,
            MovementGeneratorPriority::Highest,
            UnitState::JUMPING.bits(),
            MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING,
            arrival_spell,
        );
        true
    }

    pub fn move_jump_with_gravity_like_cpp(
        &mut self,
        movement_id: u32,
        duration_ms: u32,
        speed_xy: f32,
        arrival_spell: Option<(u32, ObjectGuid)>,
    ) -> bool {
        if speed_xy < 0.01 {
            return false;
        }

        self.add_generic_movement(
            MovementGeneratorKind::Effect,
            movement_id,
            duration_ms,
            MovementGeneratorPriority::Highest,
            UnitState::JUMPING.bits(),
            MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING | MOVEMENTGENERATOR_FLAG_PERSIST_ON_DEATH,
            arrival_spell,
        );
        true
    }

    pub fn move_knockback_from_like_cpp(
        &mut self,
        is_player: bool,
        duration_ms: u32,
        speed_xy: f32,
    ) -> bool {
        if is_player || speed_xy < 0.01 {
            return false;
        }

        self.add_generic_movement(
            MovementGeneratorKind::Effect,
            0,
            duration_ms,
            MovementGeneratorPriority::Highest,
            0,
            MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING | MOVEMENTGENERATOR_FLAG_PERSIST_ON_DEATH,
            None,
        );
        true
    }

    pub fn move_fall_like_cpp(
        &mut self,
        movement_id: u32,
        duration_ms: u32,
        has_valid_ground_height: bool,
        vertical_delta: f32,
        has_root_or_stun_state: bool,
        is_player: bool,
    ) -> MoveFallPlan {
        if !has_valid_ground_height || vertical_delta.abs() < 0.1 || has_root_or_stun_state {
            return MoveFallPlan::Noop;
        }

        if is_player {
            return MoveFallPlan::PlayerFallInfo;
        }

        self.add_generic_movement(
            MovementGeneratorKind::Effect,
            movement_id,
            duration_ms,
            MovementGeneratorPriority::Highest,
            0,
            MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING,
            None,
        );
        MoveFallPlan::SplineStarted
    }

    fn add_generic_movement(
        &mut self,
        kind: MovementGeneratorKind,
        movement_id: u32,
        duration_ms: u32,
        priority: MovementGeneratorPriority,
        base_unit_state: u32,
        flags: u16,
        arrival_spell: Option<(u32, ObjectGuid)>,
    ) {
        let mut generator = MovementGeneratorRef::new(kind, MovementSlot::Active)
            .with_priority(priority)
            .with_flags(flags)
            .with_base_unit_state(base_unit_state)
            .with_movement_id(movement_id)
            .with_duration_ms(duration_ms);
        if let Some((spell_id, target_guid)) = arrival_spell {
            generator = generator.with_arrival_spell(spell_id, target_guid);
        }
        self.add_generator(generator);
    }

    pub fn stop_on_death(&mut self) -> bool {
        if self
            .active_generators
            .first()
            .is_some_and(|generator| generator.has_flag(MOVEMENTGENERATOR_FLAG_PERSIST_ON_DEATH))
        {
            return false;
        }

        self.clear_active();
        self.move_idle();
        self.stop_moving();
        true
    }

    pub fn pause_movement(&mut self) {
        self.paused = true;
    }

    pub fn pause_current_movement_like_cpp(
        &mut self,
        timer_ms: u32,
        slot: MovementSlot,
        forced: bool,
    ) -> bool {
        let Some(generator) = self.movement_generator_for_slot_mut(slot) else {
            return false;
        };

        if timer_ms > 0 {
            generator.flags |= MOVEMENTGENERATOR_FLAG_TIMED_PAUSED;
            generator.flags &= !MOVEMENTGENERATOR_FLAG_PAUSED;
            generator.duration_ms = Some(timer_ms);
            generator.elapsed_ms = 0;
        } else {
            generator.flags |= MOVEMENTGENERATOR_FLAG_PAUSED;
            generator.flags &= !MOVEMENTGENERATOR_FLAG_TIMED_PAUSED;
        }

        self.paused = true;
        if forced && self.current_slot() == slot {
            self.stop_moving();
        }
        true
    }

    pub fn resume_movement(&mut self) {
        self.paused = false;
    }

    pub fn stop_moving(&mut self) {
        self.stopped = true;
        self.finalize_spline();
    }

    pub fn start_spline(&mut self, spline_id: u32, duration_ms: u32) {
        self.spline = MoveSplineState {
            enabled: true,
            finalized: false,
            cyclic: false,
            on_transport: false,
            spline_id,
            progress_ms: 0,
            duration_ms,
            velocity: None,
            final_destination: None,
            current_destination: None,
        };
        self.stopped = false;
    }

    pub fn launch_spline(
        &mut self,
        spline_id: u32,
        duration_ms: u32,
        destination: (i32, i32, i32),
        cyclic: bool,
        on_transport: bool,
        velocity: Option<u32>,
    ) {
        self.spline = MoveSplineState {
            enabled: true,
            finalized: false,
            cyclic,
            on_transport,
            spline_id,
            progress_ms: 0,
            duration_ms,
            velocity,
            final_destination: Some(destination),
            current_destination: Some(destination),
        };
        self.stopped = false;
    }

    pub fn set_spline_progress(&mut self, progress_ms: u32) {
        self.spline.progress_ms = progress_ms.min(self.spline.duration_ms);
        if self.spline.progress_ms >= self.spline.duration_ms && !self.spline.cyclic {
            self.finalize_spline();
        }
    }

    pub fn update_spline(&mut self, diff_ms: u32) -> bool {
        if !self.spline.enabled || self.spline.finalized {
            return false;
        }
        let next_progress = self.spline.progress_ms.saturating_add(diff_ms);
        if self.spline.cyclic && self.spline.duration_ms > 0 {
            self.spline.progress_ms = next_progress % self.spline.duration_ms;
            return false;
        }
        self.set_spline_progress(next_progress);
        self.spline.finalized
    }

    pub fn finalize_spline(&mut self) {
        self.spline.enabled = false;
        self.spline.finalized = true;
        self.spline.progress_ms = self.spline.duration_ms;
    }

    pub fn interrupt_spline(&mut self) {
        self.finalize_spline();
        self.spline.current_destination = None;
    }

    fn sort_active_generators(&mut self) {
        self.active_generators.sort_by(|left, right| {
            right
                .mode
                .cmp(&left.mode)
                .then_with(|| right.priority.cmp(&left.priority))
        });
    }

    fn remove_generator_at(&mut self, index: usize) -> MovementGeneratorRef {
        let removed = self.active_generators.remove(index);
        self.clear_base_unit_state(removed.base_unit_state);
        removed
    }

    fn add_base_unit_state(&mut self, base_unit_state: u32) {
        if base_unit_state != 0 {
            *self.base_unit_states.entry(base_unit_state).or_insert(0) += 1;
        }
    }

    fn clear_base_unit_state(&mut self, base_unit_state: u32) {
        if base_unit_state == 0 {
            return;
        }
        if let Some(count) = self.base_unit_states.get_mut(&base_unit_state) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.base_unit_states.remove(&base_unit_state);
            }
        }
    }
}
