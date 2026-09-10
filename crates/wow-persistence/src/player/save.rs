//! Coherent full-save DTO groups and existing immediate talent-reset/XP persistence requests.
//! Mechanical relocation from lib.rs in #578; public crate-root paths are retained.

use crate::{LogicalDatabaseLikeCpp, PersistenceOutcomeLikeCpp};

/// One SQLx-free semantic Player snapshot.
///
/// This request deliberately describes Player state groups, not prepared
/// statements. The MariaDB adapter owns the current statement decomposition
/// and the exact order in which it appends those statements to the single
/// Characters-database transaction. C++ anchor: `Player.cpp:19312-19655`.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerCharacterSaveRequestLikeCpp {
    pub player_guid: u64,
    pub account_id: u32,
    pub wall_clock_unix_secs: i64,
    pub character: PlayerCharacterSnapshotSaveLikeCpp,
    pub spells: Option<PlayerSpellSaveGroupLikeCpp>,
    pub skills: Option<Vec<PlayerSkillSaveLikeCpp>>,
    pub glyphs: Option<Vec<PlayerGlyphSaveLikeCpp>>,
    pub talents: Option<Vec<PlayerTalentSaveLikeCpp>>,
    pub spell_cooldowns: Option<Vec<PlayerSpellCooldownSaveLikeCpp>>,
    pub spell_charges: Option<Vec<PlayerSpellChargeSaveLikeCpp>>,
    pub action_buttons: Option<PlayerActionButtonsSaveLikeCpp>,
    pub equipment_sets: Option<Vec<PlayerEquipmentSetSaveLikeCpp>>,
    pub void_storage: Option<Vec<PlayerVoidStorageSlotSaveLikeCpp>>,
    pub tutorials: Option<PlayerTutorialsSaveLikeCpp>,
    pub instance_lock_times: Vec<PlayerInstanceLockTimeSaveLikeCpp>,
    pub played_time: PlayerPlayedTimeSaveLikeCpp,
    pub reputations: Vec<PlayerReputationSaveLikeCpp>,
    pub cuf_profiles: Option<Vec<PlayerCufProfileSlotSaveLikeCpp>>,
}

impl PlayerCharacterSaveRequestLikeCpp {
    pub fn committed_groups_like_cpp(&self) -> PlayerCharacterCommittedGroupsLikeCpp {
        let (player_spells, fallback_player_spells) = match &self.spells {
            Some(PlayerSpellSaveGroupLikeCpp::Complete {
                fallback_rows_were_present,
                ..
            }) => (true, *fallback_rows_were_present),
            Some(PlayerSpellSaveGroupLikeCpp::Fallback { .. }) => (false, true),
            None => (false, false),
        };
        PlayerCharacterCommittedGroupsLikeCpp {
            player_spells,
            fallback_player_spells,
            player_skills: self.skills.is_some(),
            equipment_sets: self.equipment_sets.as_ref().is_some_and(|sets| {
                sets.iter()
                    .any(|set| set.state != PlayerEquipmentSetStateLikeCpp::Unchanged)
            }),
            tutorials_changed: self.tutorials.is_some(),
            tutorials_insert: self
                .tutorials
                .as_ref()
                .is_some_and(|tutorials| !tutorials.already_persisted),
            reputation: !self.reputations.is_empty(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlayerCharacterCommittedGroupsLikeCpp {
    pub player_spells: bool,
    pub fallback_player_spells: bool,
    pub player_skills: bool,
    pub equipment_sets: bool,
    pub tutorials_changed: bool,
    pub tutorials_insert: bool,
    pub reputation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerCharacterSaveResultLikeCpp {
    pub outcome: PersistenceOutcomeLikeCpp,
    pub committed: PlayerCharacterCommittedGroupsLikeCpp,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerCharacterSnapshotSaveLikeCpp {
    pub position: PlayerPositionSaveLikeCpp,
    pub level: u8,
    pub xp: u32,
    pub money: u64,
    pub rest_state: u8,
    pub player_flags: u32,
    pub rest_bonus: f32,
    pub logout_time: u64,
    pub is_logout_resting: bool,
    pub health: u32,
    pub powers: Option<[i32; 10]>,
    pub talent_reset_cost: u32,
    pub talent_reset_time: u64,
    pub explored_zones: String,
    pub dungeon_difficulty: u32,
    pub raid_difficulty: u32,
    pub legacy_raid_difficulty: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerPositionSaveLikeCpp {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub orientation: f32,
    pub map_id: u16,
    pub instance_id: u32,
    pub zone_id: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerSpellSaveGroupLikeCpp {
    Complete {
        rows: Vec<PlayerSpellSaveLikeCpp>,
        fallback_rows_were_present: bool,
    },
    Fallback {
        rows: Vec<PlayerFallbackSpellSaveLikeCpp>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerSpellStateLikeCpp {
    Unchanged,
    Changed,
    New,
    Removed,
    Temporary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSpellSaveLikeCpp {
    pub spell_id: i32,
    pub active: bool,
    pub disabled: bool,
    pub dependent: bool,
    pub favorite: bool,
    pub state: PlayerSpellStateLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerFallbackSpellSaveLikeCpp {
    pub spell_id: i32,
    pub active: bool,
    pub dependent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSkillSaveLikeCpp {
    pub skill_id: u16,
    pub value: u16,
    pub max: u16,
    pub profession_slot: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerGlyphSaveLikeCpp {
    pub talent_group: u8,
    pub glyph_slot: u8,
    pub glyph_id: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerTalentSaveLikeCpp {
    pub talent_id: u32,
    pub rank: u8,
    pub talent_group: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSpellCooldownSaveLikeCpp {
    pub spell_id: u32,
    pub item_id: u32,
    pub cooldown_end_unix_secs: i64,
    pub category_id: u32,
    pub category_end_unix_secs: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSpellChargeSaveLikeCpp {
    pub category_id: u32,
    pub recharge_start_unix_secs: i64,
    pub recharge_end_unix_secs: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerActionButtonsSaveLikeCpp {
    pub spec: u8,
    pub trait_config_id: i32,
    pub rows: Vec<PlayerActionButtonSaveLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerActionButtonSaveLikeCpp {
    pub button: u8,
    pub packed_action: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerEquipmentSetTypeLikeCpp {
    Equipment,
    Transmog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerEquipmentSetStateLikeCpp {
    Unchanged,
    Changed,
    New,
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerEquipmentSetSaveLikeCpp {
    pub set_guid: u64,
    pub set_id: u32,
    pub set_type: PlayerEquipmentSetTypeLikeCpp,
    pub state: PlayerEquipmentSetStateLikeCpp,
    pub name: String,
    pub icon: String,
    pub ignore_mask: u32,
    pub assigned_spec_index: i32,
    pub pieces: Vec<u64>,
    pub appearances: Vec<i32>,
    pub enchants: [i32; 2],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerVoidStorageSaveLikeCpp {
    pub item_id: u64,
    pub item_entry: u32,
    pub creator_guid: u64,
    pub fixed_scaling_level: u32,
    pub random_properties_id: i32,
    pub random_properties_seed: i32,
    pub context: u8,
}

/// One retained talent row written by C++ `Player::_SaveTalents` after the
/// active talent group has been reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerTalentResetSaveRowLikeCpp {
    pub talent_id: u32,
    pub rank: u8,
    pub talent_group: u8,
}

/// The complete represented talent-reset transaction.
///
/// `money_before`/`money_after` are part of the durability contract rather
/// than gameplay state here: the MariaDB adapter uses the absolute money row
/// to reconcile a lost COMMIT reply. Equal values deliberately prove nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerTalentResetPersistenceRequestLikeCpp {
    pub player_guid: u64,
    pub money_before: u64,
    pub money_after: u64,
    pub reset_cost: u32,
    pub reset_time_secs: u64,
    pub retained_talents: Vec<PlayerTalentResetSaveRowLikeCpp>,
}

impl PlayerTalentResetPersistenceRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Characters
    }
}

/// The optional online rest-state row that accompanies one represented XP
/// durability write. Gameplay owns the values; the adapter owns their SQL
/// representation and transaction order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerXpRestStateSaveLikeCpp {
    pub rest_state: u8,
    pub player_flags: u32,
    pub rest_bonus: f32,
}

/// One SQLx-free request for Rusty's represented immediate XP durability
/// boundary. Legacy C++ mutates XP in `Player::GiveXP` and persists it through
/// the ordinary Player save; this contract deliberately preserves Rust's
/// current immediate transaction without claiming parity for its timing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerXpPersistenceRequestLikeCpp {
    pub player_guid: u64,
    pub level_changed: bool,
    pub level: u8,
    pub xp: u32,
    pub rest: Option<PlayerXpRestStateSaveLikeCpp>,
}

impl PlayerXpPersistenceRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Characters
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerVoidStorageSlotSaveLikeCpp {
    pub slot: u8,
    pub item: Option<PlayerVoidStorageSaveLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerTutorialsSaveLikeCpp {
    pub tutorials: [u32; 8],
    pub already_persisted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerInstanceLockTimeSaveLikeCpp {
    pub instance_id: u32,
    pub release_time: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerPlayedTimeSaveLikeCpp {
    pub total_time: u32,
    pub level_time: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerReputationSaveLikeCpp {
    pub faction_id: u16,
    pub standing: i32,
    pub flags: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerCufProfileSaveLikeCpp {
    pub profile_name: String,
    pub frame_height: u16,
    pub frame_width: u16,
    pub sort_by: u8,
    pub health_text: u8,
    pub bool_options: u32,
    pub top_point: u8,
    pub bottom_point: u8,
    pub left_point: u8,
    pub top_offset: u16,
    pub bottom_offset: u16,
    pub left_offset: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerCufProfileSlotSaveLikeCpp {
    pub profile_id: u8,
    pub profile: Option<PlayerCufProfileSaveLikeCpp>,
}
