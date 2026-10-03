//! Target implicit-target metadata and exact pure queries, not execution.
//! 02245dcd SpellInfo.h:41-105 / SpellInfo.cpp:44-221,246-401.
//! No 3.4.3 enum imports or unknown-value fallback.
mod table;
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TargetObject {
    None = 0,
    Source = 1,
    Destination = 2,
    Unit = 3,
    UnitAndDestination = 4,
    GameObject = 5,
    GameObjectItem = 6,
    Item = 7,
    Corpse = 8,
    CorpseEnemy = 9,
    CorpseAlly = 10,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TargetReference {
    None = 0,
    Caster = 1,
    Target = 2,
    Last = 3,
    Source = 4,
    Destination = 5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TargetSelection {
    NotImplemented = 0,
    Default = 1,
    Channel = 2,
    Nearby = 3,
    Cone = 4,
    Area = 5,
    Trajectory = 6,
    Line = 7,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TargetCheck {
    Default = 0,
    Entry = 1,
    Enemy = 2,
    Ally = 3,
    Party = 4,
    Raid = 5,
    RaidClass = 6,
    Passenger = 7,
    Summoned = 8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TargetDirection {
    None = 0,
    Front = 1,
    Back = 2,
    Right = 3,
    Left = 4,
    FrontRight = 5,
    BackRight = 6,
    BackLeft = 7,
    FrontLeft = 8,
    Random = 9,
    Entry = 10,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImplicitTargetInfo {
    target: u32,
}
impl ImplicitTargetInfo {
    /// Source callers must already bound the ID. Rust rejects undefined indexing.
    pub fn from_id(target: u32) -> Option<Self> {
        table::TARGETS.get(target as usize).map(|_| Self { target })
    }
    fn metadata(self) -> table::Metadata {
        table::TARGETS[self.target as usize]
    }
    pub fn id(self) -> u32 {
        self.target
    }
    pub fn object_type(self) -> TargetObject {
        self.metadata().0
    }
    pub fn reference_type(self) -> TargetReference {
        self.metadata().1
    }
    pub fn selection_category(self) -> TargetSelection {
        self.metadata().2
    }
    pub fn check_type(self) -> TargetCheck {
        self.metadata().3
    }
    pub fn direction_type(self) -> TargetDirection {
        self.metadata().4
    }
    pub fn is_area(self) -> bool {
        matches!(
            self.selection_category(),
            TargetSelection::Area | TargetSelection::Cone
        )
    }
    /// Source consumes a random float only for RANDOM. The caller retains RNG
    /// authority; this metadata owner introduces no engine or implicit draw.
    pub fn direction_angle(self, normalized_random: impl FnOnce() -> f32) -> f32 {
        use std::f64::consts::PI;
        match self.direction_type() {
            TargetDirection::Front => 0.0,
            TargetDirection::Back => PI as f32,
            TargetDirection::Right => (-PI / 2.0) as f32,
            TargetDirection::Left => (PI / 2.0) as f32,
            TargetDirection::FrontRight => (-PI / 4.0) as f32,
            TargetDirection::BackRight => (-3.0 * PI / 4.0) as f32,
            TargetDirection::BackLeft => (3.0 * PI / 4.0) as f32,
            TargetDirection::FrontLeft => (PI / 4.0) as f32,
            TargetDirection::Random => normalized_random() * (2.0 * PI) as f32,
            _ => 0.0,
        }
    }
    /// SpellInfo.cpp:140-221. Read the old source/destination state before
    /// recording this object's provided location. A then B ordering is visible.
    pub fn explicit_target_mask(self, source_set: &mut bool, destination_set: &mut bool) -> u32 {
        let mut mask = 0;
        if self.target == 89 {
            if !*source_set {
                mask = 0x20;
            }
            if !*destination_set {
                mask |= 0x40;
            }
        } else {
            match self.reference_type() {
                TargetReference::Source if !*source_set => mask = 0x20,
                TargetReference::Destination if !*destination_set => mask = 0x40,
                TargetReference::Target => match self.object_type() {
                    TargetObject::GameObject => mask = 0x800,
                    TargetObject::GameObjectItem => mask = 0x4000,
                    TargetObject::UnitAndDestination
                    | TargetObject::Unit
                    | TargetObject::Destination => {
                        mask = match self.check_type() {
                            TargetCheck::Enemy => 0x80,
                            TargetCheck::Ally => 0x100,
                            TargetCheck::Party => 0x8,
                            TargetCheck::Raid => 0x4,
                            TargetCheck::Passenger => 0x10_0000,
                            _ => 0x2,
                        };
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        match self.object_type() {
            TargetObject::Source => *source_set = true,
            TargetObject::Destination | TargetObject::UnitAndDestination => *destination_set = true,
            _ => {}
        }
        mask
    }
}
impl TargetObject {
    /// SpellInfo.cpp:44-70::GetTargetFlagMask, including effect-only object kinds.
    pub fn flag_mask(self) -> u32 {
        match self {
            Self::Destination => 0x40,
            Self::UnitAndDestination => 0x40 | 0x2,
            Self::CorpseAlly => 0x8000,
            Self::CorpseEnemy => 0x200,
            Self::Corpse => 0x8000 | 0x200,
            Self::Unit => 0x2,
            Self::GameObject => 0x800,
            Self::GameObjectItem => 0x4000,
            Self::Item => 0x10,
            Self::Source => 0x20,
            _ => 0,
        }
    }
}
