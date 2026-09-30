use wow_constants::{SheathState, UnitPvpFlags, UnitStandStateType};

/// TrinityCore visibility category stored by the creature template/addon records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VisibilityDistanceTypeLikeCpp {
    Normal = 0,
    Tiny = 1,
    Small = 2,
    Large = 3,
    Gigantic = 4,
    Infinite = 5,
}

impl VisibilityDistanceTypeLikeCpp {
    pub const MAX_LIKE_CPP: u8 = 6;

    pub const fn from_u8_like_cpp(value: u8) -> Self {
        match value {
            1 => Self::Tiny,
            2 => Self::Small,
            3 => Self::Large,
            4 => Self::Gigantic,
            5 => Self::Infinite,
            _ => Self::Normal,
        }
    }

    pub const fn distance_like_cpp(self) -> f32 {
        match self {
            Self::Normal => DEFAULT_VISIBILITY_DISTANCE,
            Self::Tiny => VISIBILITY_DISTANCE_TINY,
            Self::Small => VISIBILITY_DISTANCE_SMALL,
            Self::Large => VISIBILITY_DISTANCE_LARGE,
            Self::Gigantic => VISIBILITY_DISTANCE_GIGANTIC,
            Self::Infinite => MAX_VISIBILITY_DISTANCE,
        }
    }
}

/// TrinityCore `MAX_VISIBILITY_DISTANCE` (`SIZE_OF_GRIDS`).
pub const MAX_VISIBILITY_DISTANCE: f32 = 533.3333;
/// TrinityCore normal visibility distance.
pub const DEFAULT_VISIBILITY_DISTANCE: f32 = 100.0;
pub const VISIBILITY_DISTANCE_TINY: f32 = 25.0;
pub const VISIBILITY_DISTANCE_SMALL: f32 = 50.0;
pub const VISIBILITY_DISTANCE_LARGE: f32 = 200.0;
pub const VISIBILITY_DISTANCE_GIGANTIC: f32 = 400.0;

/// Immutable, resolved addon input consumed by Creature creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureAddonLifecycleRecordLikeCpp {
    pub path_id: u32,
    pub mount_display_id: u32,
    pub stand_state: UnitStandStateType,
    pub vis_flags: u8,
    pub anim_tier: u8,
    pub sheath_state: SheathState,
    pub pvp_flags: UnitPvpFlags,
    pub emote: u32,
    pub ai_anim_kit_id: u16,
    pub movement_anim_kit_id: u16,
    pub melee_anim_kit_id: u16,
    pub visibility_distance_type: VisibilityDistanceTypeLikeCpp,
    pub auras: Vec<u32>,
    pub aura_applications: Vec<CreatureAddonAuraApplicationLikeCpp>,
}

impl Default for CreatureAddonLifecycleRecordLikeCpp {
    fn default() -> Self {
        Self {
            path_id: 0,
            mount_display_id: 0,
            stand_state: UnitStandStateType::Stand,
            vis_flags: 0,
            anim_tier: 0,
            sheath_state: SheathState::Unarmed,
            pvp_flags: UnitPvpFlags::empty(),
            emote: 0,
            ai_anim_kit_id: 0,
            movement_anim_kit_id: 0,
            melee_anim_kit_id: 0,
            visibility_distance_type: VisibilityDistanceTypeLikeCpp::Normal,
            auras: Vec::new(),
            aura_applications: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureAddonAuraApplicationLikeCpp {
    pub spell_id: u32,
    /// Resolved visual ID; runtime CastGUID allocation remains Map-owned.
    pub spell_visual_id: i32,
    pub effect_mask: u32,
    pub flags: u32,
    /// Resolved effect values for each surviving aura effect slot.
    pub effects: Vec<CreatureAddonAuraEffectLikeCpp>,
}

/// One resolved `AuraEffect` of a spawn-addon aura (`Unit::AddAura`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureAddonAuraEffectLikeCpp {
    pub aura_type: i32,
    pub amount: i32,
    pub misc_value: i32,
    pub effect_index: u8,
}
