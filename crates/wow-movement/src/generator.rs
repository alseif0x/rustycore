use bitflags::bitflags;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MovementGeneratorType {
    Idle = 0,
    Random = 1,
    Waypoint = 2,
    Confused = 4,
    Chase = 5,
    Home = 6,
    Flight = 7,
    Point = 8,
    Fleeing = 9,
    Distract = 10,
    Assistance = 11,
    AssistanceDistract = 12,
    TimedFleeing = 13,
    Follow = 14,
    Rotate = 15,
    Effect = 16,
    SplineChain = 17,
    Formation = 18,
}

impl MovementGeneratorType {
    #[must_use]
    pub const fn from_trinity_id(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Idle),
            1 => Some(Self::Random),
            2 => Some(Self::Waypoint),
            3 | 19..=u8::MAX => None,
            4 => Some(Self::Confused),
            5 => Some(Self::Chase),
            6 => Some(Self::Home),
            7 => Some(Self::Flight),
            8 => Some(Self::Point),
            9 => Some(Self::Fleeing),
            10 => Some(Self::Distract),
            11 => Some(Self::Assistance),
            12 => Some(Self::AssistanceDistract),
            13 => Some(Self::TimedFleeing),
            14 => Some(Self::Follow),
            15 => Some(Self::Rotate),
            16 => Some(Self::Effect),
            17 => Some(Self::SplineChain),
            18 => Some(Self::Formation),
        }
    }

    #[must_use]
    pub const fn trinity_id(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum MovementGeneratorMode {
    Default = 0,
    Override = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum MovementGeneratorPriority {
    None = 0,
    Normal = 1,
    Highest = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MovementSlot {
    Default = 0,
    Active = 1,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct MovementGeneratorFlags: u16 {
        const NONE = 0x000;
        const INITIALIZATION_PENDING = 0x001;
        const INITIALIZED = 0x002;
        const SPEED_UPDATE_PENDING = 0x004;
        const INTERRUPTED = 0x008;
        const PAUSED = 0x010;
        const TIMED_PAUSED = 0x020;
        const DEACTIVATED = 0x040;
        const INFORM_ENABLED = 0x080;
        const FINALIZED = 0x100;
        const PERSIST_ON_DEATH = 0x200;
        const TRANSITORY = Self::SPEED_UPDATE_PENDING.bits() | Self::INTERRUPTED.bits();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MovementGeneratorState {
    pub mode: MovementGeneratorMode,
    pub priority: MovementGeneratorPriority,
    pub flags: MovementGeneratorFlags,
    pub base_unit_state: u32,
}

impl Default for MovementGeneratorState {
    fn default() -> Self {
        Self {
            mode: MovementGeneratorMode::Default,
            priority: MovementGeneratorPriority::None,
            flags: MovementGeneratorFlags::NONE,
            base_unit_state: 0,
        }
    }
}

pub trait MovementGenerator: Send + Sync {
    fn state(&self) -> &MovementGeneratorState;
    fn state_mut(&mut self) -> &mut MovementGeneratorState;
    fn kind(&self) -> MovementGeneratorType;

    fn initialize(&mut self);
    fn reset(&mut self);
    fn update(&mut self, diff_ms: u32) -> bool;
    fn deactivate(&mut self);
    fn finalize(&mut self, active: bool, movement_inform: bool);

    fn unit_speed_changed(&mut self) {}
    fn pause(&mut self, _timer_ms: u32) {}
    fn resume(&mut self, _override_timer_ms: u32) {}
    fn reset_position(&self) -> Option<(f32, f32, f32)> {
        None
    }

    fn add_flag(&mut self, flag: MovementGeneratorFlags) {
        self.state_mut().flags.insert(flag);
    }

    fn has_flag(&self, flag: MovementGeneratorFlags) -> bool {
        self.state().flags.intersects(flag)
    }

    fn remove_flag(&mut self, flag: MovementGeneratorFlags) {
        self.state_mut().flags.remove(flag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movement_generator_type_values_match_cpp() {
        assert_eq!(MovementGeneratorType::Idle.trinity_id(), 0);
        assert_eq!(MovementGeneratorType::Random.trinity_id(), 1);
        assert_eq!(MovementGeneratorType::Waypoint.trinity_id(), 2);
        assert_eq!(MovementGeneratorType::from_trinity_id(3), None);
        assert_eq!(
            MovementGeneratorType::from_trinity_id(18),
            Some(MovementGeneratorType::Formation)
        );
        assert_eq!(MovementGeneratorType::from_trinity_id(19), None);
    }

    #[test]
    fn movement_generator_flags_match_cpp() {
        assert_eq!(MovementGeneratorFlags::NONE.bits(), 0x000);
        assert_eq!(MovementGeneratorFlags::INITIALIZATION_PENDING.bits(), 0x001);
        assert_eq!(MovementGeneratorFlags::INITIALIZED.bits(), 0x002);
        assert_eq!(MovementGeneratorFlags::SPEED_UPDATE_PENDING.bits(), 0x004);
        assert_eq!(MovementGeneratorFlags::INTERRUPTED.bits(), 0x008);
        assert_eq!(MovementGeneratorFlags::PAUSED.bits(), 0x010);
        assert_eq!(MovementGeneratorFlags::TIMED_PAUSED.bits(), 0x020);
        assert_eq!(MovementGeneratorFlags::DEACTIVATED.bits(), 0x040);
        assert_eq!(MovementGeneratorFlags::INFORM_ENABLED.bits(), 0x080);
        assert_eq!(MovementGeneratorFlags::FINALIZED.bits(), 0x100);
        assert_eq!(MovementGeneratorFlags::PERSIST_ON_DEATH.bits(), 0x200);
        assert_eq!(
            MovementGeneratorFlags::TRANSITORY,
            MovementGeneratorFlags::SPEED_UPDATE_PENDING | MovementGeneratorFlags::INTERRUPTED
        );
    }
}
