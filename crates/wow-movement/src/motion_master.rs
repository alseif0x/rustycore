use std::collections::VecDeque;
use std::fmt;

use bitflags::bitflags;

use crate::{
    MovementGenerator, MovementGeneratorFlags, MovementGeneratorMode, MovementGeneratorPriority,
    MovementGeneratorType, MovementSlot,
};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct MotionMasterFlags: u8 {
        const NONE = 0x0;
        const UPDATE = 0x1;
        const STATIC_INITIALIZATION_PENDING = 0x2;
        const INITIALIZATION_PENDING = 0x4;
        const INITIALIZING = 0x8;
        const DELAYED = Self::UPDATE.bits() | Self::INITIALIZATION_PENDING.bits();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MotionMasterDelayedActionType {
    Clear = 0,
    ClearSlot = 1,
    ClearMode = 2,
    ClearPriority = 3,
    Add = 4,
    Remove = 5,
    RemoveType = 6,
    Initialize = 7,
}

impl MotionMasterDelayedActionType {
    #[must_use]
    pub const fn trinity_id(self) -> u8 {
        self as u8
    }

    #[must_use]
    pub const fn from_trinity_id(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Clear),
            1 => Some(Self::ClearSlot),
            2 => Some(Self::ClearMode),
            3 => Some(Self::ClearPriority),
            4 => Some(Self::Add),
            5 => Some(Self::Remove),
            6 => Some(Self::RemoveType),
            7 => Some(Self::Initialize),
            _ => None,
        }
    }
}

pub struct DelayedAction<M> {
    pub action_type: MotionMasterDelayedActionType,
    action: Box<dyn FnOnce(&mut M) + Send + Sync>,
    validator: Box<dyn Fn() -> bool + Send + Sync>,
}

impl<M> DelayedAction<M> {
    #[must_use]
    pub fn new(
        action_type: MotionMasterDelayedActionType,
        action: impl FnOnce(&mut M) + Send + Sync + 'static,
    ) -> Self {
        Self {
            action_type,
            action: Box::new(action),
            validator: Box::new(|| true),
        }
    }

    #[must_use]
    pub fn with_validator(
        action_type: MotionMasterDelayedActionType,
        action: impl FnOnce(&mut M) + Send + Sync + 'static,
        validator: impl Fn() -> bool + Send + Sync + 'static,
    ) -> Self {
        Self {
            action_type,
            action: Box::new(action),
            validator: Box::new(validator),
        }
    }

    pub fn resolve(self, motion_master: &mut M) -> bool {
        if !(self.validator)() {
            return false;
        }
        (self.action)(motion_master);
        true
    }
}

pub struct MotionMaster {
    default_generator: Option<Box<dyn MovementGenerator>>,
    active_generators: Vec<Box<dyn MovementGenerator>>,
    delayed_actions: DelayedActionQueue<MotionMaster>,
    flags: MotionMasterFlags,
    base_unit_state_refs: Vec<u32>,
    pub last_resolved_delayed_actions: Vec<ResolvedDelayedAction>,
}

impl fmt::Debug for MotionMaster {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MotionMaster")
            .field(
                "default_generator",
                &self.current_kind_for_slot(MovementSlot::Default),
            )
            .field(
                "active_generators",
                &self
                    .active_generators
                    .iter()
                    .map(|generator| generator.kind())
                    .collect::<Vec<_>>(),
            )
            .field("flags", &self.flags)
            .field("base_unit_state_refs", &self.base_unit_state_refs)
            .field(
                "last_resolved_delayed_actions",
                &self.last_resolved_delayed_actions,
            )
            .finish()
    }
}

impl MotionMaster {
    #[must_use]
    pub fn new(default_generator: Box<dyn MovementGenerator>) -> Self {
        Self {
            default_generator: Some(default_generator),
            active_generators: Vec::new(),
            delayed_actions: DelayedActionQueue::default(),
            flags: MotionMasterFlags::NONE,
            base_unit_state_refs: Vec::new(),
            last_resolved_delayed_actions: Vec::new(),
        }
    }

    #[must_use]
    pub fn new_pending() -> Self {
        Self {
            default_generator: None,
            active_generators: Vec::new(),
            delayed_actions: DelayedActionQueue::default(),
            flags: MotionMasterFlags::INITIALIZATION_PENDING,
            base_unit_state_refs: Vec::new(),
            last_resolved_delayed_actions: Vec::new(),
        }
    }

    #[must_use]
    pub const fn flags(&self) -> MotionMasterFlags {
        self.flags
    }

    #[must_use]
    pub fn delayed_action_count(&self) -> usize {
        self.delayed_actions.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.default_generator.is_none() && self.active_generators.is_empty()
    }

    #[must_use]
    pub fn size(&self) -> usize {
        usize::from(self.default_generator.is_some()) + self.active_generators.len()
    }

    #[must_use]
    pub fn current_slot(&self) -> Option<MovementSlot> {
        if !self.active_generators.is_empty() {
            Some(MovementSlot::Active)
        } else if self.default_generator.is_some() {
            Some(MovementSlot::Default)
        } else {
            None
        }
    }

    #[must_use]
    pub fn current_kind(&self) -> Option<MovementGeneratorType> {
        self.active_generators
            .first()
            .map(|generator| generator.kind())
            .or_else(|| {
                self.default_generator
                    .as_ref()
                    .map(|generator| generator.kind())
            })
    }

    #[must_use]
    pub fn current_kind_for_slot(&self, slot: MovementSlot) -> Option<MovementGeneratorType> {
        match slot {
            MovementSlot::Default => self
                .default_generator
                .as_ref()
                .map(|generator| generator.kind()),
            MovementSlot::Active => self
                .active_generators
                .first()
                .map(|generator| generator.kind()),
        }
    }

    #[must_use]
    pub fn has_generator_kind(&self, kind: MovementGeneratorType, slot: MovementSlot) -> bool {
        match slot {
            MovementSlot::Default => self
                .default_generator
                .as_ref()
                .is_some_and(|generator| generator.kind() == kind),
            MovementSlot::Active => self
                .active_generators
                .iter()
                .any(|generator| generator.kind() == kind),
        }
    }

    #[must_use]
    pub fn base_unit_state_ref_count(&self, base_unit_state: u32) -> usize {
        self.base_unit_state_refs
            .iter()
            .filter(|state| **state == base_unit_state)
            .count()
    }

    pub fn add(&mut self, movement: Box<dyn MovementGenerator>, slot: MovementSlot) {
        if self.flags.intersects(MotionMasterFlags::DELAYED) {
            self.delayed_actions.push(DelayedAction::new(
                MotionMasterDelayedActionType::Add,
                move |motion_master: &mut MotionMaster| motion_master.add(movement, slot),
            ));
            return;
        }

        self.direct_add(movement, slot);
    }

    pub fn remove_kind(&mut self, kind: MovementGeneratorType, slot: MovementSlot) {
        if self.flags.intersects(MotionMasterFlags::DELAYED) {
            self.delayed_actions.push(DelayedAction::new(
                MotionMasterDelayedActionType::RemoveType,
                move |motion_master: &mut MotionMaster| motion_master.remove_kind(kind, slot),
            ));
            return;
        }

        match slot {
            MovementSlot::Default => {
                if self
                    .default_generator
                    .as_ref()
                    .is_some_and(|generator| generator.kind() == kind)
                {
                    self.delete_default(false, false);
                }
            }
            MovementSlot::Active => {
                if let Some(index) = self
                    .active_generators
                    .iter()
                    .position(|generator| generator.kind() == kind)
                {
                    self.remove_active_index(index, index == 0, false);
                }
            }
        }
    }

    pub fn clear(&mut self) {
        if self.flags.intersects(MotionMasterFlags::DELAYED) {
            self.delayed_actions.push(DelayedAction::new(
                MotionMasterDelayedActionType::Clear,
                |motion_master: &mut MotionMaster| motion_master.clear(),
            ));
            return;
        }

        self.direct_clear();
    }

    pub fn clear_slot(&mut self, slot: MovementSlot) {
        if self.flags.intersects(MotionMasterFlags::DELAYED) {
            self.delayed_actions.push(DelayedAction::new(
                MotionMasterDelayedActionType::ClearSlot,
                move |motion_master: &mut MotionMaster| motion_master.clear_slot(slot),
            ));
            return;
        }

        match slot {
            MovementSlot::Default => self.delete_default(self.active_generators.is_empty(), false),
            MovementSlot::Active => self.direct_clear_active(),
        }
    }

    pub fn clear_mode(&mut self, mode: MovementGeneratorMode) {
        if self.flags.intersects(MotionMasterFlags::DELAYED) {
            self.delayed_actions.push(DelayedAction::new(
                MotionMasterDelayedActionType::ClearMode,
                move |motion_master: &mut MotionMaster| motion_master.clear_mode(mode),
            ));
            return;
        }

        let mut index = 0;
        while index < self.active_generators.len() {
            if self.active_generators[index].state().mode == mode {
                self.remove_active_index(index, false, false);
            } else {
                index += 1;
            }
        }
    }

    pub fn clear_priority(&mut self, priority: MovementGeneratorPriority) {
        if self.flags.intersects(MotionMasterFlags::DELAYED) {
            self.delayed_actions.push(DelayedAction::new(
                MotionMasterDelayedActionType::ClearPriority,
                move |motion_master: &mut MotionMaster| motion_master.clear_priority(priority),
            ));
            return;
        }

        let mut index = 0;
        while index < self.active_generators.len() {
            if self.active_generators[index].state().priority == priority {
                self.remove_active_index(index, false, false);
            } else {
                index += 1;
            }
        }
    }

    pub fn update(&mut self, diff_ms: u32) {
        if self
            .flags
            .intersects(MotionMasterFlags::INITIALIZATION_PENDING | MotionMasterFlags::INITIALIZING)
            || self.is_empty()
        {
            return;
        }

        self.flags.insert(MotionMasterFlags::UPDATE);
        let keep_running = {
            let top = self
                .current_generator_mut()
                .expect("non-empty motion master");
            if top.has_flag(MovementGeneratorFlags::INITIALIZATION_PENDING) {
                top.initialize();
            }
            if top.has_flag(MovementGeneratorFlags::DEACTIVATED) {
                top.reset();
            }
            top.update(diff_ms)
        };

        if !keep_running && !self.active_generators.is_empty() {
            self.remove_active_index(0, true, true);
        }

        self.flags.remove(MotionMasterFlags::UPDATE);
        self.resolve_delayed_actions();
    }

    pub fn propagate_speed_change(&mut self) {
        if let Some(generator) = self.current_generator_mut() {
            generator.unit_speed_changed();
        }
    }

    pub fn stop_on_death(
        &mut self,
        owner_in_world: bool,
        idle_factory: impl FnOnce() -> Box<dyn MovementGenerator>,
    ) -> StopOnDeathAction {
        if self
            .current_generator()
            .is_some_and(|generator| generator.has_flag(MovementGeneratorFlags::PERSIST_ON_DEATH))
        {
            return StopOnDeathAction {
                persisted: true,
                cleared_motion_master: false,
                moved_idle: false,
                stop_moving: false,
            };
        }

        if owner_in_world {
            self.clear();
            self.add(idle_factory(), MovementSlot::Default);
        }

        StopOnDeathAction {
            persisted: false,
            cleared_motion_master: owner_in_world,
            moved_idle: owner_in_world,
            stop_moving: true,
        }
    }

    fn current_generator(&self) -> Option<&dyn MovementGenerator> {
        if !self.active_generators.is_empty() {
            self.active_generators
                .first()
                .map(|generator| generator.as_ref())
        } else {
            self.default_generator
                .as_ref()
                .map(|generator| generator.as_ref())
        }
    }

    fn current_generator_mut(&mut self) -> Option<&mut Box<dyn MovementGenerator>> {
        if !self.active_generators.is_empty() {
            self.active_generators.first_mut()
        } else {
            self.default_generator.as_mut()
        }
    }

    fn direct_add(&mut self, movement: Box<dyn MovementGenerator>, slot: MovementSlot) {
        match slot {
            MovementSlot::Default => {
                self.delete_default(self.active_generators.is_empty(), false);
                if movement.has_flag(MovementGeneratorFlags::INITIALIZATION_PENDING) {
                    self.flags
                        .insert(MotionMasterFlags::STATIC_INITIALIZATION_PENDING);
                }
                self.default_generator = Some(movement);
            }
            MovementSlot::Active => {
                if self.active_generators.is_empty() {
                    if let Some(default_generator) = self.default_generator.as_mut() {
                        default_generator.deactivate();
                    }
                } else {
                    let top_priority = self.active_generators[0].state().priority;
                    let priority = movement.state().priority;
                    if priority >= top_priority {
                        if priority == top_priority {
                            self.remove_active_index(0, true, false);
                        } else {
                            self.active_generators[0].deactivate();
                        }
                    } else if let Some(index) = self
                        .active_generators
                        .iter()
                        .position(|generator| generator.state().priority == priority)
                    {
                        self.remove_active_index(index, false, false);
                    }
                }

                self.add_base_unit_state(movement.state().base_unit_state);
                let priority = movement.state().priority;
                let index = self
                    .active_generators
                    .iter()
                    .position(|generator| generator.state().priority < priority)
                    .unwrap_or(self.active_generators.len());
                self.active_generators.insert(index, movement);
            }
        }
    }

    fn direct_clear(&mut self) {
        self.delete_default(self.active_generators.is_empty(), false);
        self.direct_clear_active();
        self.base_unit_state_refs.clear();
    }

    fn direct_clear_active(&mut self) {
        if !self.active_generators.is_empty() {
            self.remove_active_index(0, true, false);
        }
        while !self.active_generators.is_empty() {
            self.remove_active_index(0, false, false);
        }
    }

    fn remove_active_index(&mut self, index: usize, active: bool, movement_inform: bool) {
        let mut movement = self.active_generators.remove(index);
        movement.finalize(active, movement_inform);
        self.clear_base_unit_state(movement.state().base_unit_state);
    }

    fn delete_default(&mut self, active: bool, movement_inform: bool) {
        if let Some(mut default_generator) = self.default_generator.take() {
            default_generator.finalize(active, movement_inform);
        }
    }

    fn add_base_unit_state(&mut self, base_unit_state: u32) {
        if base_unit_state != 0 {
            self.base_unit_state_refs.push(base_unit_state);
        }
    }

    fn clear_base_unit_state(&mut self, base_unit_state: u32) {
        if base_unit_state == 0 {
            return;
        }
        if let Some(index) = self
            .base_unit_state_refs
            .iter()
            .position(|state| *state == base_unit_state)
        {
            self.base_unit_state_refs.remove(index);
        }
    }

    fn resolve_delayed_actions(&mut self) {
        self.last_resolved_delayed_actions.clear();
        while !self.delayed_actions.is_empty() {
            let mut delayed_actions = std::mem::take(&mut self.delayed_actions);
            let resolved = delayed_actions.resolve_all(self);
            self.last_resolved_delayed_actions.extend(resolved);
            self.delayed_actions = delayed_actions;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StopOnDeathAction {
    pub persisted: bool,
    pub cleared_motion_master: bool,
    pub moved_idle: bool,
    pub stop_moving: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedDelayedAction {
    pub action_type: MotionMasterDelayedActionType,
    pub executed: bool,
}

pub struct DelayedActionQueue<M> {
    actions: VecDeque<DelayedAction<M>>,
}

impl<M> Default for DelayedActionQueue<M> {
    fn default() -> Self {
        Self {
            actions: VecDeque::new(),
        }
    }
}

impl<M> DelayedActionQueue<M> {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.actions.len()
    }

    pub fn push(&mut self, action: DelayedAction<M>) {
        self.actions.push_back(action);
    }

    pub fn resolve_all(&mut self, motion_master: &mut M) -> Vec<ResolvedDelayedAction> {
        let mut resolved = Vec::new();
        while let Some(action) = self.actions.pop_front() {
            let action_type = action.action_type;
            let executed = action.resolve(motion_master);
            resolved.push(ResolvedDelayedAction {
                action_type,
                executed,
            });
        }
        resolved
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AssistanceDistractMovementGenerator, DistractMovementGenerator, IdleMovementGenerator,
        MovementGeneratorState, RotateDirection, RotateMovementGenerator,
        UNIT_STATE_DISTRACTED_LIKE_CPP, UNIT_STATE_ROTATING_LIKE_CPP,
    };
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    struct TrackingMovementGenerator {
        state: MovementGeneratorState,
        kind: MovementGeneratorType,
        speed_changes: Arc<AtomicUsize>,
        finalized: Arc<AtomicUsize>,
    }

    impl TrackingMovementGenerator {
        fn new(
            kind: MovementGeneratorType,
            priority: MovementGeneratorPriority,
            flags: MovementGeneratorFlags,
            speed_changes: Arc<AtomicUsize>,
            finalized: Arc<AtomicUsize>,
        ) -> Self {
            Self {
                state: MovementGeneratorState {
                    mode: MovementGeneratorMode::Default,
                    priority,
                    flags,
                    base_unit_state: 0,
                },
                kind,
                speed_changes,
                finalized,
            }
        }
    }

    impl MovementGenerator for TrackingMovementGenerator {
        fn state(&self) -> &MovementGeneratorState {
            &self.state
        }

        fn state_mut(&mut self) -> &mut MovementGeneratorState {
            &mut self.state
        }

        fn kind(&self) -> MovementGeneratorType {
            self.kind
        }

        fn initialize(&mut self) {}

        fn reset(&mut self) {}

        fn update(&mut self, _diff_ms: u32) -> bool {
            true
        }

        fn deactivate(&mut self) {}

        fn finalize(&mut self, _active: bool, _movement_inform: bool) {
            self.finalized.fetch_add(1, Ordering::SeqCst);
        }

        fn unit_speed_changed(&mut self) {
            self.speed_changes.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn motion_master_flags_and_delayed_action_values_match_cpp() {
        assert_eq!(MotionMasterFlags::NONE.bits(), 0x0);
        assert_eq!(MotionMasterFlags::UPDATE.bits(), 0x1);
        assert_eq!(MotionMasterFlags::STATIC_INITIALIZATION_PENDING.bits(), 0x2);
        assert_eq!(MotionMasterFlags::INITIALIZATION_PENDING.bits(), 0x4);
        assert_eq!(MotionMasterFlags::INITIALIZING.bits(), 0x8);
        assert_eq!(
            MotionMasterFlags::DELAYED,
            MotionMasterFlags::UPDATE | MotionMasterFlags::INITIALIZATION_PENDING
        );

        assert_eq!(MotionMasterDelayedActionType::Clear.trinity_id(), 0);
        assert_eq!(MotionMasterDelayedActionType::ClearSlot.trinity_id(), 1);
        assert_eq!(MotionMasterDelayedActionType::ClearMode.trinity_id(), 2);
        assert_eq!(MotionMasterDelayedActionType::ClearPriority.trinity_id(), 3);
        assert_eq!(MotionMasterDelayedActionType::Add.trinity_id(), 4);
        assert_eq!(MotionMasterDelayedActionType::Remove.trinity_id(), 5);
        assert_eq!(MotionMasterDelayedActionType::RemoveType.trinity_id(), 6);
        assert_eq!(MotionMasterDelayedActionType::Initialize.trinity_id(), 7);
        assert_eq!(MotionMasterDelayedActionType::from_trinity_id(8), None);
    }

    #[test]
    fn delayed_action_queue_resolves_fifo_and_honors_validator() {
        let mut queue = DelayedActionQueue::default();
        queue.push(DelayedAction::new(
            MotionMasterDelayedActionType::Add,
            |value: &mut Vec<u8>| {
                value.push(1);
            },
        ));
        queue.push(DelayedAction::with_validator(
            MotionMasterDelayedActionType::Remove,
            |value: &mut Vec<u8>| value.push(2),
            || false,
        ));
        queue.push(DelayedAction::new(
            MotionMasterDelayedActionType::Initialize,
            |value: &mut Vec<u8>| value.push(3),
        ));

        let mut value = Vec::new();
        let resolved = queue.resolve_all(&mut value);
        assert_eq!(value, vec![1, 3]);
        assert!(queue.is_empty());
        assert_eq!(
            resolved,
            vec![
                ResolvedDelayedAction {
                    action_type: MotionMasterDelayedActionType::Add,
                    executed: true,
                },
                ResolvedDelayedAction {
                    action_type: MotionMasterDelayedActionType::Remove,
                    executed: false,
                },
                ResolvedDelayedAction {
                    action_type: MotionMasterDelayedActionType::Initialize,
                    executed: true,
                },
            ]
        );
    }

    #[test]
    fn motion_master_add_orders_active_generators_like_cpp_priorities() {
        let mut motion = MotionMaster::new(Box::new(IdleMovementGenerator::new()));
        assert_eq!(motion.current_slot(), Some(MovementSlot::Default));
        assert_eq!(motion.current_kind(), Some(MovementGeneratorType::Idle));

        motion.add(
            Box::new(RotateMovementGenerator::new(
                7,
                1_000,
                RotateDirection::Left,
            )),
            MovementSlot::Active,
        );
        assert_eq!(motion.current_slot(), Some(MovementSlot::Active));
        assert_eq!(motion.current_kind(), Some(MovementGeneratorType::Rotate));
        assert_eq!(
            motion.base_unit_state_ref_count(UNIT_STATE_ROTATING_LIKE_CPP),
            1
        );

        motion.add(
            Box::new(DistractMovementGenerator::new(500, 1.0)),
            MovementSlot::Active,
        );
        assert_eq!(motion.current_kind(), Some(MovementGeneratorType::Distract));
        assert_eq!(
            motion.base_unit_state_ref_count(UNIT_STATE_DISTRACTED_LIKE_CPP),
            1
        );
        assert_eq!(
            motion.base_unit_state_ref_count(UNIT_STATE_ROTATING_LIKE_CPP),
            1
        );
    }

    #[test]
    fn motion_master_update_initializes_pops_and_resolves_delayed_like_cpp() {
        let mut motion = MotionMaster::new(Box::new(IdleMovementGenerator::new()));
        motion.add(
            Box::new(RotateMovementGenerator::new(
                7,
                1_000,
                RotateDirection::Left,
            )),
            MovementSlot::Active,
        );

        motion.flags.insert(MotionMasterFlags::UPDATE);
        motion.add(
            Box::new(AssistanceDistractMovementGenerator::new(500, 1.0)),
            MovementSlot::Active,
        );
        assert_eq!(motion.delayed_action_count(), 1);
        motion.flags.remove(MotionMasterFlags::UPDATE);

        motion.update(1_000);
        assert_eq!(
            motion.last_resolved_delayed_actions,
            vec![ResolvedDelayedAction {
                action_type: MotionMasterDelayedActionType::Add,
                executed: true,
            }]
        );
        assert_eq!(
            motion.current_kind(),
            Some(MovementGeneratorType::AssistanceDistract)
        );
        assert_eq!(
            motion.base_unit_state_ref_count(UNIT_STATE_ROTATING_LIKE_CPP),
            0
        );
        assert_eq!(
            motion.base_unit_state_ref_count(UNIT_STATE_DISTRACTED_LIKE_CPP),
            1
        );
    }

    #[test]
    fn motion_master_delays_clear_remove_and_filters_like_cpp() {
        let mut motion = MotionMaster::new(Box::new(IdleMovementGenerator::new()));
        motion.add(
            Box::new(RotateMovementGenerator::new(
                7,
                1_000,
                RotateDirection::Right,
            )),
            MovementSlot::Active,
        );
        motion.add(
            Box::new(AssistanceDistractMovementGenerator::new(500, 1.0)),
            MovementSlot::Active,
        );

        motion
            .flags
            .insert(MotionMasterFlags::INITIALIZATION_PENDING);
        motion.remove_kind(MovementGeneratorType::Rotate, MovementSlot::Active);
        motion.clear_priority(MovementGeneratorPriority::Normal);
        assert_eq!(motion.delayed_action_count(), 2);
        motion
            .flags
            .remove(MotionMasterFlags::INITIALIZATION_PENDING);
        motion.update(1);
        assert_eq!(motion.last_resolved_delayed_actions.len(), 2);
        assert!(!motion.has_generator_kind(MovementGeneratorType::Rotate, MovementSlot::Active));
        assert!(!motion.has_generator_kind(
            MovementGeneratorType::AssistanceDistract,
            MovementSlot::Active
        ));
        assert_eq!(motion.current_kind(), Some(MovementGeneratorType::Idle));
    }

    #[test]
    fn motion_master_propagates_speed_change_to_current_generator_only_like_cpp() {
        let default_speed_changes = Arc::new(AtomicUsize::new(0));
        let active_speed_changes = Arc::new(AtomicUsize::new(0));
        let finalized = Arc::new(AtomicUsize::new(0));
        let mut motion = MotionMaster::new(Box::new(TrackingMovementGenerator::new(
            MovementGeneratorType::Idle,
            MovementGeneratorPriority::None,
            MovementGeneratorFlags::NONE,
            Arc::clone(&default_speed_changes),
            Arc::clone(&finalized),
        )));
        motion.add(
            Box::new(TrackingMovementGenerator::new(
                MovementGeneratorType::Rotate,
                MovementGeneratorPriority::Normal,
                MovementGeneratorFlags::NONE,
                Arc::clone(&active_speed_changes),
                Arc::clone(&finalized),
            )),
            MovementSlot::Active,
        );

        motion.propagate_speed_change();
        assert_eq!(active_speed_changes.load(Ordering::SeqCst), 1);
        assert_eq!(default_speed_changes.load(Ordering::SeqCst), 0);

        motion.clear_slot(MovementSlot::Active);
        motion.propagate_speed_change();
        assert_eq!(active_speed_changes.load(Ordering::SeqCst), 1);
        assert_eq!(default_speed_changes.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn motion_master_stop_on_death_preserves_persisting_current_generator_like_cpp() {
        let speed_changes = Arc::new(AtomicUsize::new(0));
        let finalized = Arc::new(AtomicUsize::new(0));
        let mut motion = MotionMaster::new(Box::new(IdleMovementGenerator::new()));
        motion.add(
            Box::new(TrackingMovementGenerator::new(
                MovementGeneratorType::Effect,
                MovementGeneratorPriority::Highest,
                MovementGeneratorFlags::PERSIST_ON_DEATH,
                Arc::clone(&speed_changes),
                Arc::clone(&finalized),
            )),
            MovementSlot::Active,
        );

        let action = motion.stop_on_death(true, || Box::new(IdleMovementGenerator::new()));

        assert_eq!(
            action,
            StopOnDeathAction {
                persisted: true,
                cleared_motion_master: false,
                moved_idle: false,
                stop_moving: false,
            }
        );
        assert_eq!(motion.current_kind(), Some(MovementGeneratorType::Effect));
        assert_eq!(motion.size(), 2);
        assert_eq!(finalized.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn motion_master_stop_on_death_clears_and_moves_idle_when_owner_is_in_world_like_cpp() {
        let default_finalized = Arc::new(AtomicUsize::new(0));
        let active_finalized = Arc::new(AtomicUsize::new(0));
        let speed_changes = Arc::new(AtomicUsize::new(0));
        let mut motion = MotionMaster::new(Box::new(TrackingMovementGenerator::new(
            MovementGeneratorType::Idle,
            MovementGeneratorPriority::None,
            MovementGeneratorFlags::NONE,
            Arc::clone(&speed_changes),
            Arc::clone(&default_finalized),
        )));
        motion.add(
            Box::new(TrackingMovementGenerator::new(
                MovementGeneratorType::Rotate,
                MovementGeneratorPriority::Normal,
                MovementGeneratorFlags::NONE,
                Arc::clone(&speed_changes),
                Arc::clone(&active_finalized),
            )),
            MovementSlot::Active,
        );

        let action = motion.stop_on_death(true, || Box::new(IdleMovementGenerator::new()));

        assert_eq!(
            action,
            StopOnDeathAction {
                persisted: false,
                cleared_motion_master: true,
                moved_idle: true,
                stop_moving: true,
            }
        );
        assert_eq!(motion.current_kind(), Some(MovementGeneratorType::Idle));
        assert_eq!(motion.size(), 1);
        assert_eq!(default_finalized.load(Ordering::SeqCst), 1);
        assert_eq!(active_finalized.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn motion_master_stop_on_death_stops_without_clearing_when_owner_is_not_in_world_like_cpp() {
        let speed_changes = Arc::new(AtomicUsize::new(0));
        let finalized = Arc::new(AtomicUsize::new(0));
        let mut motion = MotionMaster::new(Box::new(TrackingMovementGenerator::new(
            MovementGeneratorType::Idle,
            MovementGeneratorPriority::None,
            MovementGeneratorFlags::NONE,
            Arc::clone(&speed_changes),
            Arc::clone(&finalized),
        )));

        let action = motion.stop_on_death(false, || Box::new(IdleMovementGenerator::new()));

        assert_eq!(
            action,
            StopOnDeathAction {
                persisted: false,
                cleared_motion_master: false,
                moved_idle: false,
                stop_moving: true,
            }
        );
        assert_eq!(motion.current_kind(), Some(MovementGeneratorType::Idle));
        assert_eq!(motion.size(), 1);
        assert_eq!(finalized.load(Ordering::SeqCst), 0);
    }
}
