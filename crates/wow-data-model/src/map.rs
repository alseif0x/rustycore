//! Pure Map.db2 and MapDifficulty.db2 row schemas and row-local flags.

pub const MAP_FLAG_FLEXIBLE_RAID_LOCKING: u32 = 0x0000_8000;
pub const MAP_FLAG_GARRISON: u32 = 0x0400_0000;
pub const MAP_FLAG2_ACTIVATES_PVP_ITEM_LEVELS_LIKE_CPP: u32 = 0x0000_0040;
pub const MAP_FLAG2_IGNORE_INSTANCE_FARM_LIMIT: u32 = 0x0000_0080;
pub const MAP_DIFFICULTY_FLAG_USE_LOOT_BASED_LOCK: u8 = 0x02;

pub const MAP_COMMON: i8 = 0;
pub const MAP_INSTANCE: i8 = 1;
pub const MAP_RAID: i8 = 2;
pub const MAP_BATTLEGROUND: i8 = 3;
pub const MAP_ARENA: i8 = 4;
pub const MAP_SCENARIO: i8 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapEntry {
    pub id: u32,
    pub instance_type: i8,
    pub expansion_id: u8,
    pub parent_map_id: i16,
    pub cosmetic_parent_map_id: i16,
    pub flags1: u32,
    pub flags2: u32,
}

impl MapEntry {
    pub const fn is_flex_locking(self) -> bool {
        self.flags1 & MAP_FLAG_FLEXIBLE_RAID_LOCKING != 0
    }

    pub const fn is_garrison(self) -> bool {
        self.flags1 & MAP_FLAG_GARRISON != 0
    }

    pub const fn ignores_instance_farm_limit_like_cpp(self) -> bool {
        self.flags2 & MAP_FLAG2_IGNORE_INSTANCE_FARM_LIMIT != 0
    }

    pub const fn activates_pvp_item_levels_like_cpp(self) -> bool {
        self.flags2 & MAP_FLAG2_ACTIVATES_PVP_ITEM_LEVELS_LIKE_CPP != 0
    }

    pub const fn is_dungeon(self) -> bool {
        matches!(self.instance_type, MAP_INSTANCE | MAP_RAID | MAP_SCENARIO) && !self.is_garrison()
    }

    /// C++ `MapEntry::Instanceable()`; garrison flags do not exempt instance/scenario maps.
    pub const fn is_instanceable_like_cpp(self) -> bool {
        matches!(
            self.instance_type,
            MAP_INSTANCE | MAP_RAID | MAP_BATTLEGROUND | MAP_ARENA | MAP_SCENARIO
        )
    }

    pub const fn expansion_like_cpp(self) -> u8 {
        self.expansion_id
    }

    pub const fn is_non_raid_dungeon_like_cpp(self) -> bool {
        self.instance_type == MAP_INSTANCE
    }

    pub const fn is_battleground_or_arena(self) -> bool {
        matches!(self.instance_type, MAP_BATTLEGROUND | MAP_ARENA)
    }

    pub const fn is_world_map(self) -> bool {
        self.instance_type == MAP_COMMON
    }

    pub const fn is_split_by_faction(self) -> bool {
        matches!(self.id, 609 | 1265 | 1481 | 2175 | 2570)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapDifficultyEntry {
    pub id: u32,
    pub message: String,
    pub map_id: u32,
    pub difficulty_id: u8,
    pub lock_id: u8,
    pub reset_interval: u8,
    pub max_players: u32,
    pub flags: u8,
}

impl MapDifficultyEntry {
    pub const fn is_using_encounter_locks(&self) -> bool {
        self.flags & MAP_DIFFICULTY_FLAG_USE_LOOT_BASED_LOCK != 0
    }
}
