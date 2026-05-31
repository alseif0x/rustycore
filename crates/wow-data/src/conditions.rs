// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ConditionMgr` data rows.

use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};

use anyhow::Result;
use num_traits::FromPrimitive;
use tracing::info;
use wow_constants::{
    ComparisonType, ConditionInstanceInfo, ConditionSourceType, ConditionType, Gender,
    RelationType, Team, TypeId, TypeMask, UnitStandStateType,
};
use wow_database::{SqlResult, WorldDatabase, WorldStatements};

pub const GRID_MAP_TYPE_MASK_CORPSE: u32 = 0x01;
pub const GRID_MAP_TYPE_MASK_CREATURE: u32 = 0x02;
pub const GRID_MAP_TYPE_MASK_DYNAMIC_OBJECT: u32 = 0x04;
pub const GRID_MAP_TYPE_MASK_GAME_OBJECT: u32 = 0x08;
pub const GRID_MAP_TYPE_MASK_PLAYER: u32 = 0x10;
pub const GRID_MAP_TYPE_MASK_AREA_TRIGGER: u32 = 0x20;
pub const GRID_MAP_TYPE_MASK_SCENE_OBJECT: u32 = 0x40;
pub const GRID_MAP_TYPE_MASK_CONVERSATION: u32 = 0x80;
pub const GRID_MAP_TYPE_MASK_ALL: u32 = 0xFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ConditionId {
    pub source_group: u32,
    pub source_entry: i32,
    pub source_id: u32,
}

impl ConditionId {
    pub const fn new(source_group: u32, source_entry: i32, source_id: u32) -> Self {
        Self {
            source_group,
            source_entry,
            source_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Condition {
    pub source_type: ConditionSourceType,
    pub source_group: u32,
    pub source_entry: i32,
    pub source_id: u32,
    pub else_group: u32,
    pub condition_type: ConditionType,
    pub condition_value1: u32,
    pub condition_value2: u32,
    pub condition_value3: u32,
    pub condition_string_value1: String,
    pub error_type: u32,
    pub error_text_id: u32,
    pub reference_id: u32,
    pub script_id: u32,
    pub condition_target: u8,
    pub negative_condition: bool,
}

impl Default for Condition {
    fn default() -> Self {
        Self {
            source_type: ConditionSourceType::None,
            source_group: 0,
            source_entry: 0,
            source_id: 0,
            else_group: 0,
            condition_type: ConditionType::None,
            condition_value1: 0,
            condition_value2: 0,
            condition_value3: 0,
            condition_string_value1: String::new(),
            error_type: 0,
            error_text_id: 0,
            reference_id: 0,
            script_id: 0,
            condition_target: 0,
            negative_condition: false,
        }
    }
}

impl Condition {
    /// C++ `Condition::isLoaded`.
    pub const fn is_loaded_like_cpp(&self) -> bool {
        self.condition_type as u32 > ConditionType::None as u32
            || self.reference_id != 0
            || self.script_id != 0
    }

    pub const fn id_like_cpp(&self) -> ConditionId {
        ConditionId::new(self.source_group, self.source_entry, self.source_id)
    }

    /// C++ `Condition::GetMaxAvailableConditionTargets`.
    pub const fn max_available_condition_targets_like_cpp(&self) -> u32 {
        match self.source_type {
            ConditionSourceType::Spell
            | ConditionSourceType::SpellImplicitTarget
            | ConditionSourceType::CreatureTemplateVehicle
            | ConditionSourceType::VehicleSpell
            | ConditionSourceType::SpellClickEvent
            | ConditionSourceType::GossipMenu
            | ConditionSourceType::GossipMenuOption
            | ConditionSourceType::SmartEvent
            | ConditionSourceType::NpcVendor
            | ConditionSourceType::SpellProc => 2,
            _ => 1,
        }
    }

    /// C++ `Condition::ToString`.
    pub fn to_string_like_cpp(&self, ext: bool) -> String {
        let mut text = format!(
            "[Condition SourceType: {} ({})",
            self.source_type as u32,
            condition_source_type_name_like_cpp(self.source_type)
        );

        if condition_source_can_have_group_set_like_cpp(self.source_type) {
            text.push_str(&format!(", SourceGroup: {}", self.source_group));
        }

        text.push_str(&format!(", SourceEntry: {}", self.source_entry));

        if condition_source_can_have_id_set_like_cpp(self.source_type) {
            text.push_str(&format!(", SourceId: {}", self.source_id));
        }

        if ext {
            text.push_str(&format!(
                ", ConditionType: {} ({})",
                self.condition_type as u32,
                condition_type_name_like_cpp(self.condition_type)
            ));
        }

        text.push(']');
        text
    }

    /// C++ `Condition::GetSearcherTypeMaskForCondition`.
    pub fn get_searcher_type_mask_for_condition_like_cpp(&self) -> u32 {
        if self.negative_condition {
            return GRID_MAP_TYPE_MASK_ALL;
        }

        match self.condition_type {
            ConditionType::None
            | ConditionType::ZoneId
            | ConditionType::ActiveEvent
            | ConditionType::InstanceInfo
            | ConditionType::MapId
            | ConditionType::AreaId
            | ConditionType::NearCreature
            | ConditionType::NearGameObject
            | ConditionType::DistanceTo
            | ConditionType::WorldState
            | ConditionType::PhaseId
            | ConditionType::RealmAchievement
            | ConditionType::TerrainSwap
            | ConditionType::DifficultyId
            | ConditionType::ScenarioStep => GRID_MAP_TYPE_MASK_ALL,
            ConditionType::Aura
            | ConditionType::Class
            | ConditionType::Race
            | ConditionType::Level
            | ConditionType::RelationTo
            | ConditionType::ReactionTo
            | ConditionType::Alive
            | ConditionType::HpVal
            | ConditionType::HpPct
            | ConditionType::UnitState
            | ConditionType::InWater
            | ConditionType::StandState
            | ConditionType::Charmed => GRID_MAP_TYPE_MASK_CREATURE | GRID_MAP_TYPE_MASK_PLAYER,
            ConditionType::Item
            | ConditionType::ItemEquipped
            | ConditionType::ReputationRank
            | ConditionType::Achievement
            | ConditionType::Team
            | ConditionType::Skill
            | ConditionType::QuestRewarded
            | ConditionType::QuestTaken
            | ConditionType::QuestComplete
            | ConditionType::QuestNone
            | ConditionType::Spell
            | ConditionType::DrunkenState
            | ConditionType::Title
            | ConditionType::Gender
            | ConditionType::DailyQuestDone
            | ConditionType::PetType
            | ConditionType::Taxi
            | ConditionType::QuestState
            | ConditionType::QuestObjectiveProgress
            | ConditionType::GameMaster
            | ConditionType::BattlePetCount
            | ConditionType::SceneInProgress
            | ConditionType::PlayerCondition => GRID_MAP_TYPE_MASK_PLAYER,
            ConditionType::ObjectEntryGuid | ConditionType::ObjectEntryGuidLegacy => {
                match self.condition_value1 {
                    value if value == TypeId::Unit as u32 => GRID_MAP_TYPE_MASK_CREATURE,
                    value if value == TypeId::Player as u32 => GRID_MAP_TYPE_MASK_PLAYER,
                    value if value == TypeId::GameObject as u32 => GRID_MAP_TYPE_MASK_GAME_OBJECT,
                    value if value == TypeId::Corpse as u32 => GRID_MAP_TYPE_MASK_CORPSE,
                    value if value == TypeId::AreaTrigger as u32 => GRID_MAP_TYPE_MASK_AREA_TRIGGER,
                    _ => 0,
                }
            }
            ConditionType::TypeMask | ConditionType::TypeMaskLegacy => {
                let condition_mask = TypeMask::from_bits_truncate(self.condition_value1);
                let mut mask = 0;
                if condition_mask.intersects(TypeMask::UNIT) {
                    mask |= GRID_MAP_TYPE_MASK_CREATURE | GRID_MAP_TYPE_MASK_PLAYER;
                }
                if condition_mask.intersects(TypeMask::PLAYER) {
                    mask |= GRID_MAP_TYPE_MASK_PLAYER;
                }
                if condition_mask.intersects(TypeMask::GAME_OBJECT) {
                    mask |= GRID_MAP_TYPE_MASK_GAME_OBJECT;
                }
                if condition_mask.intersects(TypeMask::CORPSE) {
                    mask |= GRID_MAP_TYPE_MASK_CORPSE;
                }
                if condition_mask.intersects(TypeMask::AREA_TRIGGER) {
                    mask |= GRID_MAP_TYPE_MASK_AREA_TRIGGER;
                }
                mask
            }
            ConditionType::CreatureType => GRID_MAP_TYPE_MASK_CREATURE,
            ConditionType::PrivateObject => GRID_MAP_TYPE_MASK_ALL & !GRID_MAP_TYPE_MASK_PLAYER,
            ConditionType::SpawnMaskDeprecated | ConditionType::StringId | ConditionType::Max => {
                panic!(
                    "Condition::GetSearcherTypeMaskForCondition - missing condition handling for {:?}",
                    self.condition_type
                )
            }
        }
    }
}

/// C++ `ConditionMgr::CanHaveSourceGroupSet`.
pub const fn condition_source_can_have_group_set_like_cpp(
    source_type: ConditionSourceType,
) -> bool {
    matches!(
        source_type,
        ConditionSourceType::CreatureLootTemplate
            | ConditionSourceType::DisenchantLootTemplate
            | ConditionSourceType::FishingLootTemplate
            | ConditionSourceType::GameObjectLootTemplate
            | ConditionSourceType::ItemLootTemplate
            | ConditionSourceType::MailLootTemplate
            | ConditionSourceType::MillingLootTemplate
            | ConditionSourceType::PickpocketingLootTemplate
            | ConditionSourceType::ProspectingLootTemplate
            | ConditionSourceType::ReferenceLootTemplate
            | ConditionSourceType::SkinningLootTemplate
            | ConditionSourceType::SpellLootTemplate
            | ConditionSourceType::GossipMenu
            | ConditionSourceType::GossipMenuOption
            | ConditionSourceType::VehicleSpell
            | ConditionSourceType::SpellImplicitTarget
            | ConditionSourceType::SpellClickEvent
            | ConditionSourceType::SmartEvent
            | ConditionSourceType::NpcVendor
            | ConditionSourceType::Phase
            | ConditionSourceType::Graveyard
            | ConditionSourceType::AreaTrigger
            | ConditionSourceType::TrainerSpell
            | ConditionSourceType::ObjectIdVisibility
            | ConditionSourceType::ReferenceCondition
    )
}

/// C++ `ConditionMgr::CanHaveSourceIdSet`.
pub const fn condition_source_can_have_id_set_like_cpp(source_type: ConditionSourceType) -> bool {
    matches!(source_type, ConditionSourceType::SmartEvent)
}

/// C++ `ConditionMgr::CanHaveConditionType`.
pub const fn condition_source_can_have_condition_type_like_cpp(
    source_type: ConditionSourceType,
    condition_type: ConditionType,
) -> bool {
    match source_type {
        ConditionSourceType::SpawnGroup => matches!(
            condition_type,
            ConditionType::None
                | ConditionType::ActiveEvent
                | ConditionType::InstanceInfo
                | ConditionType::MapId
                | ConditionType::WorldState
                | ConditionType::RealmAchievement
                | ConditionType::DifficultyId
                | ConditionType::ScenarioStep
        ),
        _ => true,
    }
}

pub const fn condition_source_type_name_like_cpp(source_type: ConditionSourceType) -> &'static str {
    match source_type {
        ConditionSourceType::None => "None",
        ConditionSourceType::CreatureLootTemplate => "Creature Loot",
        ConditionSourceType::DisenchantLootTemplate => "Disenchant Loot",
        ConditionSourceType::FishingLootTemplate => "Fishing Loot",
        ConditionSourceType::GameObjectLootTemplate => "GameObject Loot",
        ConditionSourceType::ItemLootTemplate => "Item Loot",
        ConditionSourceType::MailLootTemplate => "Mail Loot",
        ConditionSourceType::MillingLootTemplate => "Milling Loot",
        ConditionSourceType::PickpocketingLootTemplate => "Pickpocketing Loot",
        ConditionSourceType::ProspectingLootTemplate => "Prospecting Loot",
        ConditionSourceType::ReferenceLootTemplate => "Reference Loot",
        ConditionSourceType::SkinningLootTemplate => "Skinning Loot",
        ConditionSourceType::SpellLootTemplate => "Spell Loot",
        ConditionSourceType::SpellImplicitTarget => "Spell Impl. Target",
        ConditionSourceType::GossipMenu => "Gossip Menu",
        ConditionSourceType::GossipMenuOption => "Gossip Menu Option",
        ConditionSourceType::CreatureTemplateVehicle => "Creature Vehicle",
        ConditionSourceType::Spell => "Spell Expl. Target",
        ConditionSourceType::SpellClickEvent => "Spell Click Event",
        ConditionSourceType::QuestAvailable => "Quest Available",
        ConditionSourceType::VehicleSpell => "Vehicle Spell",
        ConditionSourceType::SmartEvent => "SmartScript",
        ConditionSourceType::NpcVendor => "Npc Vendor",
        ConditionSourceType::SpellProc => "Spell Proc",
        ConditionSourceType::TerrainSwap => "Terrain Swap",
        ConditionSourceType::Phase => "Phase",
        ConditionSourceType::Graveyard => "Graveyard",
        ConditionSourceType::AreaTrigger => "AreaTrigger",
        ConditionSourceType::ConversationLine => "ConversationLine",
        ConditionSourceType::AreaTriggerClientTriggered => "AreaTrigger Client Triggered",
        ConditionSourceType::TrainerSpell => "Trainer Spell",
        ConditionSourceType::ObjectIdVisibility => "Object Visibility (by ID)",
        ConditionSourceType::SpawnGroup => "Spawn Group",
        ConditionSourceType::ReferenceCondition => "Reference",
        ConditionSourceType::Max => "Unknown",
    }
}

pub const fn condition_type_name_like_cpp(condition_type: ConditionType) -> &'static str {
    match condition_type {
        ConditionType::None => "None",
        ConditionType::Aura => "Aura",
        ConditionType::Item => "Item Stored",
        ConditionType::ItemEquipped => "Item Equipped",
        ConditionType::ZoneId => "Zone",
        ConditionType::ReputationRank => "Reputation",
        ConditionType::Team => "Team",
        ConditionType::Skill => "Skill",
        ConditionType::QuestRewarded => "Quest Rewarded",
        ConditionType::QuestTaken => "Quest Taken",
        ConditionType::DrunkenState => "Drunken",
        ConditionType::WorldState => "WorldState",
        ConditionType::ActiveEvent => "Active Event",
        ConditionType::InstanceInfo => "Instance Info",
        ConditionType::QuestNone => "Quest None",
        ConditionType::Class => "Class",
        ConditionType::Race => "Race",
        ConditionType::Achievement => "Achievement",
        ConditionType::Title => "Title",
        ConditionType::SpawnMaskDeprecated => "SpawnMask",
        ConditionType::Gender => "Gender",
        ConditionType::UnitState => "Unit State",
        ConditionType::MapId => "Map",
        ConditionType::AreaId => "Area",
        ConditionType::CreatureType => "CreatureType",
        ConditionType::Spell => "Spell Known",
        ConditionType::PhaseId => "Phase",
        ConditionType::Level => "Level",
        ConditionType::QuestComplete => "Quest Completed",
        ConditionType::NearCreature => "Near Creature",
        ConditionType::NearGameObject => "Near GameObject",
        ConditionType::ObjectEntryGuidLegacy | ConditionType::ObjectEntryGuid => {
            "Object Entry or Guid"
        }
        ConditionType::TypeMaskLegacy | ConditionType::TypeMask => "Object TypeMask",
        ConditionType::RelationTo => "Relation",
        ConditionType::ReactionTo => "Reaction",
        ConditionType::DistanceTo => "Distance",
        ConditionType::Alive => "Alive",
        ConditionType::HpVal => "Health Value",
        ConditionType::HpPct => "Health Pct",
        ConditionType::RealmAchievement => "Realm Achievement",
        ConditionType::InWater => "In Water",
        ConditionType::TerrainSwap => "Terrain Swap",
        ConditionType::StandState => "Sit/stand state",
        ConditionType::DailyQuestDone => "Daily Quest Completed",
        ConditionType::Charmed => "Charmed",
        ConditionType::PetType => "Pet type",
        ConditionType::Taxi => "On Taxi",
        ConditionType::QuestState => "Quest state mask",
        ConditionType::QuestObjectiveProgress => "Quest objective progress",
        ConditionType::DifficultyId => "Map Difficulty",
        ConditionType::GameMaster => "Is Gamemaster",
        ConditionType::BattlePetCount => "BattlePet Species Learned",
        ConditionType::ScenarioStep => "On Scenario Step",
        ConditionType::SceneInProgress => "Scene In Progress",
        ConditionType::PlayerCondition => "Player Condition",
        ConditionType::PrivateObject => "Private Object",
        ConditionType::StringId => "String ID",
        ConditionType::Max => "Unknown",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionTypeInfoLikeCpp {
    pub name: &'static str,
    pub has_condition_value1: bool,
    pub has_condition_value2: bool,
    pub has_condition_value3: bool,
    pub has_condition_string_value1: bool,
}

/// C++ `ConditionMgr::StaticConditionTypeData`.
pub const fn condition_type_info_like_cpp(
    condition_type: ConditionType,
) -> ConditionTypeInfoLikeCpp {
    let name = condition_type_name_like_cpp(condition_type);
    let (
        has_condition_value1,
        has_condition_value2,
        has_condition_value3,
        has_condition_string_value1,
    ) = match condition_type {
        ConditionType::None
        | ConditionType::Alive
        | ConditionType::InWater
        | ConditionType::Charmed
        | ConditionType::Taxi
        | ConditionType::PrivateObject => (false, false, false, false),
        ConditionType::Aura
        | ConditionType::Item
        | ConditionType::InstanceInfo
        | ConditionType::NearCreature
        | ConditionType::ObjectEntryGuidLegacy
        | ConditionType::DistanceTo
        | ConditionType::ObjectEntryGuid
        | ConditionType::BattlePetCount => (true, true, true, false),
        ConditionType::ReputationRank
        | ConditionType::Skill
        | ConditionType::WorldState
        | ConditionType::Level
        | ConditionType::NearGameObject
        | ConditionType::RelationTo
        | ConditionType::ReactionTo
        | ConditionType::HpVal
        | ConditionType::HpPct
        | ConditionType::StandState
        | ConditionType::QuestState => (true, true, false, false),
        ConditionType::QuestObjectiveProgress => (true, false, true, false),
        ConditionType::StringId => (false, false, false, true),
        ConditionType::ItemEquipped
        | ConditionType::ZoneId
        | ConditionType::Team
        | ConditionType::QuestRewarded
        | ConditionType::QuestTaken
        | ConditionType::DrunkenState
        | ConditionType::ActiveEvent
        | ConditionType::QuestNone
        | ConditionType::Class
        | ConditionType::Race
        | ConditionType::Achievement
        | ConditionType::Title
        | ConditionType::SpawnMaskDeprecated
        | ConditionType::Gender
        | ConditionType::UnitState
        | ConditionType::MapId
        | ConditionType::AreaId
        | ConditionType::CreatureType
        | ConditionType::Spell
        | ConditionType::PhaseId
        | ConditionType::QuestComplete
        | ConditionType::TypeMaskLegacy
        | ConditionType::RealmAchievement
        | ConditionType::TerrainSwap
        | ConditionType::DailyQuestDone
        | ConditionType::PetType
        | ConditionType::DifficultyId
        | ConditionType::GameMaster
        | ConditionType::TypeMask
        | ConditionType::ScenarioStep
        | ConditionType::SceneInProgress
        | ConditionType::PlayerCondition => (true, false, false, false),
        ConditionType::Max => (false, false, false, false),
    };

    ConditionTypeInfoLikeCpp {
        name,
        has_condition_value1,
        has_condition_value2,
        has_condition_value3,
        has_condition_string_value1,
    }
}

pub fn useless_condition_value_fields_like_cpp(condition: &Condition) -> Vec<u8> {
    let info = condition_type_info_like_cpp(condition.condition_type);
    let mut fields = Vec::new();

    if condition.condition_value1 != 0 && !info.has_condition_value1 {
        fields.push(1);
    }
    if condition.condition_value2 != 0 && !info.has_condition_value2 {
        fields.push(2);
    }
    if condition.condition_value3 != 0 && !info.has_condition_value3 {
        fields.push(3);
    }
    if !condition.condition_string_value1.is_empty() && !info.has_condition_string_value1 {
        fields.push(4);
    }
    if condition.condition_type == ConditionType::ObjectEntryGuid
        && matches!(
            condition.condition_value1,
            value if value == TypeId::Player as u32 || value == TypeId::Corpse as u32
        )
    {
        if condition.condition_value2 != 0 && !fields.contains(&2) {
            fields.push(2);
        }
        if condition.condition_value3 != 0 && !fields.contains(&3) {
            fields.push(3);
        }
    }

    fields
}

pub const CLASSMASK_ALL_PLAYABLE_LIKE_CPP: u32 = (1 << (1 - 1))
    | (1 << (2 - 1))
    | (1 << (3 - 1))
    | (1 << (4 - 1))
    | (1 << (5 - 1))
    | (1 << (6 - 1))
    | (1 << (7 - 1))
    | (1 << (8 - 1))
    | (1 << (9 - 1))
    | (1 << (10 - 1))
    | (1 << (11 - 1))
    | (1 << (12 - 1))
    | (1 << (13 - 1));

pub const RACEMASK_ALL_PLAYABLE_LIKE_CPP: u64 = (1 << (1 - 1))
    | (1 << (2 - 1))
    | (1 << (3 - 1))
    | (1 << (4 - 1))
    | (1 << (5 - 1))
    | (1 << (6 - 1))
    | (1 << (7 - 1))
    | (1 << (8 - 1))
    | (1 << (9 - 1))
    | (1 << (10 - 1))
    | (1 << (11 - 1))
    | (1 << (22 - 1))
    | (1 << (24 - 1))
    | (1 << (25 - 1))
    | (1 << (26 - 1))
    | (1 << (27 - 1))
    | (1 << (28 - 1))
    | (1 << (29 - 1))
    | (1 << (30 - 1))
    | (1 << (31 - 1))
    | (1 << (32 - 1))
    | (1 << 11)
    | (1 << 12)
    | (1 << 13)
    | (1 << 14)
    | (1 << 16)
    | (1 << 15);

pub const UNIT_STATE_ALL_STATE_SUPPORTED_LIKE_CPP: u32 = 0x3ff7_ffff;
pub const MAX_QUEST_STATUS_LIKE_CPP: u32 = 7;
pub const MAX_SPELL_EFFECTS_LIKE_CPP: u32 = 32;
pub const DRUNKEN_SMASHED_LIKE_CPP: u32 = 3;
pub const CREATURE_TYPE_GAS_CLOUD_LIKE_CPP: u32 = 13;
pub const MAX_PET_TYPE_LIKE_CPP: u32 = 4;
pub const DEFAULT_MAX_BATTLE_PETS_PER_SPECIES_LIKE_CPP: u32 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionTypeValidationErrorLikeCpp {
    InvalidTeam(u32),
    InvalidQuestStateMask(u32),
    InvalidClassMask(u32),
    InvalidRaceMask(u64),
    InvalidGender(u32),
    InvalidSkillValue(u32),
    InvalidSpellEffectIndex(u32),
    ZeroItemCount,
    InvalidComparisonType {
        field: u8,
        value: u32,
    },
    InvalidDrunkenState(u32),
    InvalidObjectTypeId(u32),
    InvalidTypeMask(u32),
    InvalidTargetSelector {
        field: u8,
        value: u32,
        max: u32,
    },
    SelfTargetSelector {
        field: u8,
        value: u32,
    },
    InvalidRelationType(u32),
    InvalidReactionRankMask(u32),
    DeprecatedSpawnMask,
    InvalidUnitState(u32),
    InvalidCreatureType(u32),
    InvalidStandState {
        value1: u32,
        value2: u32,
    },
    InvalidPetTypeMask(u32),
    InvalidBattlePetCount(u32),
    UnsupportedInstanceInfoGuidData,
    NonExistingItem {
        condition_type: ConditionType,
        item_id: u32,
    },
    NonExistingSpell {
        condition_type: ConditionType,
        spell_id: u32,
    },
    NonExistingArea {
        condition_type: ConditionType,
        area_id: u32,
    },
    ZoneIdUsesSubzone(u32),
    NonExistingSkill(u32),
    SkillValueAboveConfigMax {
        skill_id: u32,
        value: u32,
        max: u32,
    },
    NonExistingMap {
        condition_type: ConditionType,
        map_id: u32,
    },
    NonExistingPhase(u32),
    NonExistingQuest {
        condition_type: ConditionType,
        quest_id: u32,
    },
    NonExistingQuestObjective(u32),
    QuestObjectiveCountAboveLimit {
        objective_id: u32,
        count: u32,
        limit: i32,
    },
    NonExistingDifficulty(u32),
    NonExistingFaction(u32),
    NonExistingAchievement {
        condition_type: ConditionType,
        achievement_id: u32,
    },
    NonExistingTitle(u32),
    NonExistingBattlePetSpecies(u32),
    NonExistingScenarioStep(u32),
    NonExistingSceneScriptPackage(u32),
    NonExistingPlayerCondition(u32),
    NonExistingCreatureTemplate {
        condition_type: ConditionType,
        entry: u32,
    },
    NonExistingGameObjectTemplate {
        condition_type: ConditionType,
        entry: u32,
    },
    NonExistingCreatureGuid(u32),
    NonExistingGameObjectGuid(u32),
    CreatureGuidEntryMismatch {
        guid: u32,
        expected_entry: u32,
        actual_entry: u32,
    },
    GameObjectGuidEntryMismatch {
        guid: u32,
        expected_entry: u32,
        actual_entry: u32,
    },
    NonExistingActiveEvent(u32),
    NonExistingWorldState(u32),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConditionTypeValidationReportLikeCpp {
    pub useless_value_fields: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionSourceValidationErrorLikeCpp {
    InvalidSourceType(ConditionSourceType),
    InvalidSpellImplicitTargetEffectMask(u32),
    InvalidAreaTriggerSourceEntry(i32),
    InvalidObjectIdVisibilityObjectType(u32),
    UncheckedObjectIdVisibilityObjectType(u32),
    NonExistingLootTemplate {
        source_type: ConditionSourceType,
        source_group: u32,
    },
    NonExistingLootSourceEntry {
        source_type: ConditionSourceType,
        source_group: u32,
        source_entry: i32,
    },
    NonExistingQuestAvailable(u32),
    NonExistingSourceSpell {
        source_type: ConditionSourceType,
        spell_id: i32,
    },
    NonExistingClientAreaTrigger(i32),
    NonExistingTerrainSwapMap(u32),
    NonExistingPhaseArea(u32),
    NonExistingNpcVendorItem(i32),
    NonExistingGraveyard {
        safe_loc_id: i32,
        zone_id: u32,
    },
    NonExistingSpawnGroup(i32),
    SystemSpawnGroup(i32),
    NonExistingSourceCreatureTemplate {
        source_type: ConditionSourceType,
        entry: i32,
    },
    NonExistingSourceGameObjectTemplate {
        source_type: ConditionSourceType,
        entry: i32,
    },
    NonExistingTrainer(i32),
    NonExistingConversationLineTemplate(i32),
    NonExistingAreaTriggerTemplate {
        id: u32,
        is_custom: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternallySkippedConditionLikeCpp {
    pub condition: Condition,
    pub reason: ConditionRowSkipReason,
}

#[derive(Clone, Copy, Default)]
pub struct ConditionExternalValidationStoresLikeCpp<'a> {
    pub item_store: Option<&'a crate::ItemStore>,
    pub spell_store: Option<&'a crate::SpellStore>,
    pub area_table_store: Option<&'a crate::AreaTableStore>,
    pub skill_store: Option<&'a crate::SkillStore>,
    pub map_store: Option<&'a crate::MapStore>,
    pub phase_store: Option<&'a crate::PhaseStore>,
    pub quest_store: Option<&'a crate::quest::QuestStore>,
    pub area_trigger_store: Option<&'a crate::AreaTriggerStore>,
    pub graveyard_store: Option<&'a crate::GraveyardStore>,
    pub spawn_group_store: Option<&'a crate::SpawnGroupTemplateStore>,
    pub creature_template_store: Option<&'a crate::WorldIdStore>,
    pub gameobject_template_store: Option<&'a crate::WorldIdStore>,
    pub trainer_store: Option<&'a crate::WorldIdStore>,
    pub conversation_line_template_store: Option<&'a crate::WorldIdStore>,
    pub area_trigger_template_store: Option<&'a crate::AreaTriggerTemplateStore>,
    pub creature_spawn_store: Option<&'a crate::WorldSpawnIdStore>,
    pub gameobject_spawn_store: Option<&'a crate::WorldSpawnIdStore>,
    pub active_event_store: Option<&'a crate::WorldIdStore>,
    pub world_state_store: Option<&'a crate::WorldIdStore>,
    pub difficulty_store: Option<&'a crate::DifficultyStore>,
    pub faction_store: Option<&'a crate::Db2IdStore>,
    pub achievement_store: Option<&'a crate::Db2IdStore>,
    pub char_titles_store: Option<&'a crate::Db2IdStore>,
    pub battle_pet_species_store: Option<&'a crate::Db2IdStore>,
    pub scenario_step_store: Option<&'a crate::Db2IdStore>,
    pub scene_script_package_store: Option<&'a crate::Db2IdStore>,
    pub player_condition_store: Option<&'a crate::PlayerConditionStore>,
    pub max_skill_value: Option<u32>,
    pub loot_template_exists: Option<&'a dyn Fn(ConditionSourceType, u32) -> bool>,
    pub loot_source_entry_exists: Option<&'a dyn Fn(ConditionSourceType, u32, i32) -> bool>,
}

pub fn validate_condition_type_static_like_cpp(
    condition: &mut Condition,
) -> Result<ConditionTypeValidationReportLikeCpp, ConditionTypeValidationErrorLikeCpp> {
    use ConditionTypeValidationErrorLikeCpp as Error;

    match condition.condition_type {
        ConditionType::Aura => {
            if condition.condition_value2 >= MAX_SPELL_EFFECTS_LIKE_CPP {
                return Err(Error::InvalidSpellEffectIndex(condition.condition_value2));
            }
        }
        ConditionType::Item => {
            if condition.condition_value2 == 0 {
                return Err(Error::ZeroItemCount);
            }
        }
        ConditionType::Team => {
            if condition.condition_value1 != Team::Alliance as u32
                && condition.condition_value1 != Team::Horde as u32
            {
                return Err(Error::InvalidTeam(condition.condition_value1));
            }
        }
        ConditionType::Skill => {
            if condition.condition_value2 < 1 {
                return Err(Error::InvalidSkillValue(condition.condition_value2));
            }
        }
        ConditionType::QuestState => {
            if condition.condition_value2 >= (1 << MAX_QUEST_STATUS_LIKE_CPP) {
                return Err(Error::InvalidQuestStateMask(condition.condition_value2));
            }
        }
        ConditionType::Class => {
            let invalid_mask = condition.condition_value1 & !CLASSMASK_ALL_PLAYABLE_LIKE_CPP;
            if invalid_mask != 0 {
                return Err(Error::InvalidClassMask(invalid_mask));
            }
        }
        ConditionType::Race => {
            let invalid_mask =
                u64::from(condition.condition_value1) & !RACEMASK_ALL_PLAYABLE_LIKE_CPP;
            if invalid_mask != 0 {
                return Err(Error::InvalidRaceMask(invalid_mask));
            }
        }
        ConditionType::Gender => {
            if condition.condition_value1 > Gender::Female as u32 {
                return Err(Error::InvalidGender(condition.condition_value1));
            }
        }
        ConditionType::Level => {
            if condition.condition_value2 >= ComparisonType::Max as u32 {
                return Err(Error::InvalidComparisonType {
                    field: 2,
                    value: condition.condition_value2,
                });
            }
        }
        ConditionType::DrunkenState => {
            if condition.condition_value1 > DRUNKEN_SMASHED_LIKE_CPP {
                return Err(Error::InvalidDrunkenState(condition.condition_value1));
            }
        }
        ConditionType::ObjectEntryGuidLegacy => {
            condition.condition_type = ConditionType::ObjectEntryGuid;
            condition.condition_value1 =
                convert_legacy_type_id_like_cpp(condition.condition_value1);
            validate_object_entry_guid_type_like_cpp(condition)?;
        }
        ConditionType::ObjectEntryGuid => validate_object_entry_guid_type_like_cpp(condition)?,
        ConditionType::TypeMaskLegacy => {
            condition.condition_type = ConditionType::TypeMask;
            condition.condition_value1 =
                convert_legacy_type_mask_like_cpp(condition.condition_value1);
            validate_type_mask_like_cpp(condition)?;
        }
        ConditionType::TypeMask => validate_type_mask_like_cpp(condition)?,
        ConditionType::RelationTo => {
            validate_target_selector_like_cpp(condition, 1)?;
            if condition.condition_value2 >= RelationType::Max as u32 {
                return Err(Error::InvalidRelationType(condition.condition_value2));
            }
        }
        ConditionType::ReactionTo => {
            validate_target_selector_like_cpp(condition, 1)?;
            if condition.condition_value2 == 0 {
                return Err(Error::InvalidReactionRankMask(condition.condition_value2));
            }
        }
        ConditionType::DistanceTo => {
            validate_target_selector_like_cpp(condition, 1)?;
            if condition.condition_value3 >= ComparisonType::Max as u32 {
                return Err(Error::InvalidComparisonType {
                    field: 3,
                    value: condition.condition_value3,
                });
            }
        }
        ConditionType::HpVal => {
            if condition.condition_value2 >= ComparisonType::Max as u32 {
                return Err(Error::InvalidComparisonType {
                    field: 2,
                    value: condition.condition_value2,
                });
            }
        }
        ConditionType::HpPct => {
            if condition.condition_value1 > 100 {
                return Err(Error::InvalidComparisonType {
                    field: 1,
                    value: condition.condition_value1,
                });
            }
            if condition.condition_value2 >= ComparisonType::Max as u32 {
                return Err(Error::InvalidComparisonType {
                    field: 2,
                    value: condition.condition_value2,
                });
            }
        }
        ConditionType::SpawnMaskDeprecated => return Err(Error::DeprecatedSpawnMask),
        ConditionType::UnitState => {
            if condition.condition_value1 & UNIT_STATE_ALL_STATE_SUPPORTED_LIKE_CPP == 0 {
                return Err(Error::InvalidUnitState(condition.condition_value1));
            }
        }
        ConditionType::CreatureType => {
            if condition.condition_value1 == 0
                || condition.condition_value1 > CREATURE_TYPE_GAS_CLOUD_LIKE_CPP
            {
                return Err(Error::InvalidCreatureType(condition.condition_value1));
            }
        }
        ConditionType::StandState => {
            let valid = match condition.condition_value1 {
                0 => condition.condition_value2 <= UnitStandStateType::Submerged as u32,
                1 => condition.condition_value2 <= 1,
                _ => false,
            };
            if !valid {
                return Err(Error::InvalidStandState {
                    value1: condition.condition_value1,
                    value2: condition.condition_value2,
                });
            }
        }
        ConditionType::PetType => {
            if condition.condition_value1 >= (1 << MAX_PET_TYPE_LIKE_CPP) {
                return Err(Error::InvalidPetTypeMask(condition.condition_value1));
            }
        }
        ConditionType::InstanceInfo => {
            if condition.condition_value3 == ConditionInstanceInfo::GuidData as u32 {
                return Err(Error::UnsupportedInstanceInfoGuidData);
            }
        }
        ConditionType::BattlePetCount => {
            if condition.condition_value2 > DEFAULT_MAX_BATTLE_PETS_PER_SPECIES_LIKE_CPP {
                return Err(Error::InvalidBattlePetCount(condition.condition_value2));
            }
            if condition.condition_value3 >= ComparisonType::Max as u32 {
                return Err(Error::InvalidComparisonType {
                    field: 3,
                    value: condition.condition_value3,
                });
            }
        }
        ConditionType::AreaId
        | ConditionType::Alive
        | ConditionType::InWater
        | ConditionType::TerrainSwap
        | ConditionType::Charmed
        | ConditionType::Taxi
        | ConditionType::GameMaster
        | ConditionType::PrivateObject
        | ConditionType::None
        | ConditionType::StringId
        | ConditionType::ItemEquipped
        | ConditionType::ZoneId
        | ConditionType::ReputationRank
        | ConditionType::WorldState
        | ConditionType::ActiveEvent
        | ConditionType::QuestRewarded
        | ConditionType::QuestTaken
        | ConditionType::QuestNone
        | ConditionType::Achievement
        | ConditionType::Title
        | ConditionType::MapId
        | ConditionType::Spell
        | ConditionType::PhaseId
        | ConditionType::QuestComplete
        | ConditionType::NearCreature
        | ConditionType::NearGameObject
        | ConditionType::RealmAchievement
        | ConditionType::DailyQuestDone
        | ConditionType::QuestObjectiveProgress
        | ConditionType::DifficultyId
        | ConditionType::ScenarioStep
        | ConditionType::SceneInProgress
        | ConditionType::PlayerCondition
        | ConditionType::Max => {}
    }

    Ok(ConditionTypeValidationReportLikeCpp {
        useless_value_fields: useless_condition_value_fields_like_cpp(condition),
    })
}

pub fn validate_condition_source_static_like_cpp(
    condition: &Condition,
) -> Result<(), ConditionSourceValidationErrorLikeCpp> {
    use ConditionSourceValidationErrorLikeCpp as Error;

    match condition.source_type {
        ConditionSourceType::None | ConditionSourceType::Max => {
            return Err(Error::InvalidSourceType(condition.source_type));
        }
        ConditionSourceType::SpellImplicitTarget => {
            if condition.source_group == 0 {
                return Err(Error::InvalidSpellImplicitTargetEffectMask(
                    condition.source_group,
                ));
            }
        }
        ConditionSourceType::AreaTrigger => {
            if condition.source_entry != 0 && condition.source_entry != 1 {
                return Err(Error::InvalidAreaTriggerSourceEntry(condition.source_entry));
            }
        }
        ConditionSourceType::ObjectIdVisibility => {
            if condition.source_group == 0 || condition.source_group >= TypeId::Max as u32 {
                return Err(Error::InvalidObjectIdVisibilityObjectType(
                    condition.source_group,
                ));
            }

            if condition.source_group != TypeId::Unit as u32
                && condition.source_group != TypeId::GameObject as u32
            {
                return Err(Error::UncheckedObjectIdVisibilityObjectType(
                    condition.source_group,
                ));
            }
        }
        ConditionSourceType::ReferenceCondition => {
            return Err(Error::InvalidSourceType(condition.source_type));
        }
        ConditionSourceType::CreatureLootTemplate
        | ConditionSourceType::DisenchantLootTemplate
        | ConditionSourceType::FishingLootTemplate
        | ConditionSourceType::GameObjectLootTemplate
        | ConditionSourceType::ItemLootTemplate
        | ConditionSourceType::MailLootTemplate
        | ConditionSourceType::MillingLootTemplate
        | ConditionSourceType::PickpocketingLootTemplate
        | ConditionSourceType::ProspectingLootTemplate
        | ConditionSourceType::ReferenceLootTemplate
        | ConditionSourceType::SkinningLootTemplate
        | ConditionSourceType::SpellLootTemplate
        | ConditionSourceType::GossipMenu
        | ConditionSourceType::GossipMenuOption
        | ConditionSourceType::CreatureTemplateVehicle
        | ConditionSourceType::Spell
        | ConditionSourceType::SpellClickEvent
        | ConditionSourceType::QuestAvailable
        | ConditionSourceType::VehicleSpell
        | ConditionSourceType::SmartEvent
        | ConditionSourceType::NpcVendor
        | ConditionSourceType::SpellProc
        | ConditionSourceType::TerrainSwap
        | ConditionSourceType::Phase
        | ConditionSourceType::Graveyard
        | ConditionSourceType::ConversationLine
        | ConditionSourceType::AreaTriggerClientTriggered
        | ConditionSourceType::TrainerSpell
        | ConditionSourceType::SpawnGroup => {}
    }

    Ok(())
}

pub fn validate_condition_type_external_like_cpp(
    condition: &Condition,
    stores: ConditionExternalValidationStoresLikeCpp<'_>,
) -> Result<(), ConditionTypeValidationErrorLikeCpp> {
    use ConditionTypeValidationErrorLikeCpp as Error;

    if condition.reference_id != 0 {
        return Ok(());
    }

    match condition.condition_type {
        ConditionType::Aura | ConditionType::Spell => {
            if let Some(store) = stores.spell_store
                && store.get(condition.condition_value1 as i32).is_none()
            {
                return Err(Error::NonExistingSpell {
                    condition_type: condition.condition_type,
                    spell_id: condition.condition_value1,
                });
            }
        }
        ConditionType::Item | ConditionType::ItemEquipped => {
            if let Some(store) = stores.item_store
                && store.get(condition.condition_value1).is_none()
            {
                return Err(Error::NonExistingItem {
                    condition_type: condition.condition_type,
                    item_id: condition.condition_value1,
                });
            }
        }
        ConditionType::ZoneId => {
            if let Some(store) = stores.area_table_store {
                let Some(area) = store.get(condition.condition_value1) else {
                    return Err(Error::NonExistingArea {
                        condition_type: condition.condition_type,
                        area_id: condition.condition_value1,
                    });
                };

                if area.parent_area_id != 0 && area.is_subzone_like_cpp() {
                    return Err(Error::ZoneIdUsesSubzone(condition.condition_value1));
                }
            }
        }
        ConditionType::Skill => {
            if let Some(store) = stores.skill_store
                && !store.contains_skill_line_like_cpp(condition.condition_value1)
            {
                return Err(Error::NonExistingSkill(condition.condition_value1));
            }
            if let Some(max) = stores.max_skill_value
                && condition.condition_value2 > max
            {
                return Err(Error::SkillValueAboveConfigMax {
                    skill_id: condition.condition_value1,
                    value: condition.condition_value2,
                    max,
                });
            }
        }
        ConditionType::MapId => {
            if let Some(store) = stores.map_store
                && store.get(condition.condition_value1).is_none()
            {
                return Err(Error::NonExistingMap {
                    condition_type: condition.condition_type,
                    map_id: condition.condition_value1,
                });
            }
        }
        ConditionType::PhaseId => {
            if let Some(store) = stores.phase_store
                && !store.contains(condition.condition_value1)
            {
                return Err(Error::NonExistingPhase(condition.condition_value1));
            }
        }
        ConditionType::QuestState
        | ConditionType::QuestRewarded
        | ConditionType::QuestTaken
        | ConditionType::QuestNone
        | ConditionType::QuestComplete
        | ConditionType::DailyQuestDone => {
            if let Some(store) = stores.quest_store
                && store.get(condition.condition_value1).is_none()
            {
                return Err(Error::NonExistingQuest {
                    condition_type: condition.condition_type,
                    quest_id: condition.condition_value1,
                });
            }
        }
        ConditionType::QuestObjectiveProgress => {
            if let Some(store) = stores.quest_store {
                let Some(objective) = store.objective_like_cpp(condition.condition_value1) else {
                    return Err(Error::NonExistingQuestObjective(condition.condition_value1));
                };
                let limit = objective.condition_progress_limit_like_cpp();
                if i32::try_from(condition.condition_value3).is_ok_and(|count| count > limit) {
                    return Err(Error::QuestObjectiveCountAboveLimit {
                        objective_id: condition.condition_value1,
                        count: condition.condition_value3,
                        limit,
                    });
                }
            }
        }
        ConditionType::DifficultyId => {
            if let Some(store) = stores.difficulty_store
                && !store.contains(condition.condition_value1)
            {
                return Err(Error::NonExistingDifficulty(condition.condition_value1));
            }
        }
        ConditionType::ReputationRank => {
            if let Some(store) = stores.faction_store
                && !store.contains(condition.condition_value1)
            {
                return Err(Error::NonExistingFaction(condition.condition_value1));
            }
        }
        ConditionType::Achievement | ConditionType::RealmAchievement => {
            if let Some(store) = stores.achievement_store
                && !store.contains(condition.condition_value1)
            {
                return Err(Error::NonExistingAchievement {
                    condition_type: condition.condition_type,
                    achievement_id: condition.condition_value1,
                });
            }
        }
        ConditionType::Title => {
            if let Some(store) = stores.char_titles_store
                && !store.contains(condition.condition_value1)
            {
                return Err(Error::NonExistingTitle(condition.condition_value1));
            }
        }
        ConditionType::BattlePetCount => {
            if let Some(store) = stores.battle_pet_species_store
                && !store.contains(condition.condition_value1)
            {
                return Err(Error::NonExistingBattlePetSpecies(
                    condition.condition_value1,
                ));
            }
        }
        ConditionType::ScenarioStep => {
            if let Some(store) = stores.scenario_step_store
                && !store.contains(condition.condition_value1)
            {
                return Err(Error::NonExistingScenarioStep(condition.condition_value1));
            }
        }
        ConditionType::SceneInProgress => {
            if let Some(store) = stores.scene_script_package_store
                && !store.contains(condition.condition_value1)
            {
                return Err(Error::NonExistingSceneScriptPackage(
                    condition.condition_value1,
                ));
            }
        }
        ConditionType::PlayerCondition => {
            if let Some(store) = stores.player_condition_store
                && !store.contains(condition.condition_value1)
            {
                return Err(Error::NonExistingPlayerCondition(
                    condition.condition_value1,
                ));
            }
        }
        ConditionType::NearCreature => {
            if let Some(store) = stores.creature_template_store
                && !store.contains(condition.condition_value1)
            {
                return Err(Error::NonExistingCreatureTemplate {
                    condition_type: condition.condition_type,
                    entry: condition.condition_value1,
                });
            }
        }
        ConditionType::NearGameObject => {
            if let Some(store) = stores.gameobject_template_store
                && !store.contains(condition.condition_value1)
            {
                return Err(Error::NonExistingGameObjectTemplate {
                    condition_type: condition.condition_type,
                    entry: condition.condition_value1,
                });
            }
        }
        ConditionType::ObjectEntryGuid => match condition.condition_value1 {
            value if value == TypeId::Unit as u32 => {
                if condition.condition_value2 != 0
                    && let Some(store) = stores.creature_template_store
                    && !store.contains(condition.condition_value2)
                {
                    return Err(Error::NonExistingCreatureTemplate {
                        condition_type: condition.condition_type,
                        entry: condition.condition_value2,
                    });
                }
                if condition.condition_value3 != 0
                    && let Some(store) = stores.creature_spawn_store
                {
                    let Some(actual_entry) = store.entry_for_guid(condition.condition_value3)
                    else {
                        return Err(Error::NonExistingCreatureGuid(condition.condition_value3));
                    };
                    if condition.condition_value2 != 0 && actual_entry != condition.condition_value2
                    {
                        return Err(Error::CreatureGuidEntryMismatch {
                            guid: condition.condition_value3,
                            expected_entry: condition.condition_value2,
                            actual_entry,
                        });
                    }
                }
            }
            value if value == TypeId::GameObject as u32 => {
                if condition.condition_value2 != 0
                    && let Some(store) = stores.gameobject_template_store
                    && !store.contains(condition.condition_value2)
                {
                    return Err(Error::NonExistingGameObjectTemplate {
                        condition_type: condition.condition_type,
                        entry: condition.condition_value2,
                    });
                }
                if condition.condition_value3 != 0
                    && let Some(store) = stores.gameobject_spawn_store
                {
                    let Some(actual_entry) = store.entry_for_guid(condition.condition_value3)
                    else {
                        return Err(Error::NonExistingGameObjectGuid(condition.condition_value3));
                    };
                    if condition.condition_value2 != 0 && actual_entry != condition.condition_value2
                    {
                        return Err(Error::GameObjectGuidEntryMismatch {
                            guid: condition.condition_value3,
                            expected_entry: condition.condition_value2,
                            actual_entry,
                        });
                    }
                }
            }
            _ => {}
        },
        ConditionType::ActiveEvent => {
            if let Some(store) = stores.active_event_store
                && !store.contains(condition.condition_value1)
            {
                return Err(Error::NonExistingActiveEvent(condition.condition_value1));
            }
        }
        ConditionType::WorldState => {
            if let Some(store) = stores.world_state_store
                && !store.contains(condition.condition_value1)
            {
                return Err(Error::NonExistingWorldState(condition.condition_value1));
            }
        }
        _ => {}
    }

    Ok(())
}

pub fn validate_condition_source_external_like_cpp(
    condition: &Condition,
    stores: ConditionExternalValidationStoresLikeCpp<'_>,
) -> Result<(), ConditionSourceValidationErrorLikeCpp> {
    let mut condition = condition.clone();
    validate_and_normalize_condition_source_external_like_cpp(&mut condition, stores)
}

pub fn validate_and_normalize_condition_source_external_like_cpp(
    condition: &mut Condition,
    stores: ConditionExternalValidationStoresLikeCpp<'_>,
) -> Result<(), ConditionSourceValidationErrorLikeCpp> {
    use ConditionSourceValidationErrorLikeCpp as Error;

    match condition.source_type {
        source_type if condition_source_is_loot_template_like_cpp(source_type) => {
            if let Some(template_exists) = stores.loot_template_exists
                && !template_exists(source_type, condition.source_group)
            {
                return Err(Error::NonExistingLootTemplate {
                    source_type,
                    source_group: condition.source_group,
                });
            }

            if let Some(source_entry_exists) = stores.loot_source_entry_exists
                && !source_entry_exists(source_type, condition.source_group, condition.source_entry)
            {
                return Err(Error::NonExistingLootSourceEntry {
                    source_type,
                    source_group: condition.source_group,
                    source_entry: condition.source_entry,
                });
            }
        }
        ConditionSourceType::TerrainSwap => {
            if let Some(store) = stores.map_store
                && store.get(condition.source_entry as u32).is_none()
            {
                return Err(Error::NonExistingTerrainSwapMap(
                    condition.source_entry as u32,
                ));
            }
        }
        ConditionSourceType::Phase => {
            if condition.source_entry != 0
                && let Some(store) = stores.area_table_store
                && store.get(condition.source_entry as u32).is_none()
            {
                return Err(Error::NonExistingPhaseArea(condition.source_entry as u32));
            }
        }
        ConditionSourceType::QuestAvailable => {
            if let Some(store) = stores.quest_store
                && store.get(condition.source_entry as u32).is_none()
            {
                return Err(Error::NonExistingQuestAvailable(
                    condition.source_entry as u32,
                ));
            }
        }
        ConditionSourceType::NpcVendor => {
            if let Some(store) = stores.creature_template_store
                && !store.contains(condition.source_group)
            {
                return Err(Error::NonExistingSourceCreatureTemplate {
                    source_type: condition.source_type,
                    entry: condition.source_group as i32,
                });
            }
            if let Some(store) = stores.item_store
                && u32::try_from(condition.source_entry)
                    .ok()
                    .and_then(|item_id| store.get(item_id))
                    .is_none()
            {
                return Err(Error::NonExistingNpcVendorItem(condition.source_entry));
            }
        }
        ConditionSourceType::CreatureTemplateVehicle => {
            if let Some(store) = stores.creature_template_store
                && u32::try_from(condition.source_entry)
                    .ok()
                    .is_none_or(|entry| !store.contains(entry))
            {
                return Err(Error::NonExistingSourceCreatureTemplate {
                    source_type: condition.source_type,
                    entry: condition.source_entry,
                });
            }
        }
        ConditionSourceType::VehicleSpell | ConditionSourceType::SpellClickEvent => {
            if let Some(store) = stores.creature_template_store
                && !store.contains(condition.source_group)
            {
                return Err(Error::NonExistingSourceCreatureTemplate {
                    source_type: condition.source_type,
                    entry: condition.source_group as i32,
                });
            }

            if let Some(store) = stores.spell_store
                && store.get(condition.source_entry).is_none()
            {
                return Err(Error::NonExistingSourceSpell {
                    source_type: condition.source_type,
                    spell_id: condition.source_entry,
                });
            }
        }
        ConditionSourceType::SpellImplicitTarget => {
            if let Some(store) = stores.spell_store {
                let Some(spell) = store.get(condition.source_entry) else {
                    return Err(Error::NonExistingSourceSpell {
                        source_type: condition.source_type,
                        spell_id: condition.source_entry,
                    });
                };
                let normalized_mask =
                    spell.normalized_implicit_target_effect_mask_like_cpp(condition.source_group);
                if normalized_mask == 0 {
                    return Err(Error::InvalidSpellImplicitTargetEffectMask(
                        condition.source_group,
                    ));
                }
                condition.source_group = normalized_mask;
            }
        }
        ConditionSourceType::Spell | ConditionSourceType::SpellProc => {
            if let Some(store) = stores.spell_store
                && store.get(condition.source_entry).is_none()
            {
                return Err(Error::NonExistingSourceSpell {
                    source_type: condition.source_type,
                    spell_id: condition.source_entry,
                });
            }
        }
        ConditionSourceType::TrainerSpell => {
            if let Some(store) = stores.trainer_store
                && !store.contains(condition.source_group)
            {
                return Err(Error::NonExistingTrainer(condition.source_group as i32));
            }

            if let Some(store) = stores.spell_store
                && store.get(condition.source_entry).is_none()
            {
                return Err(Error::NonExistingSourceSpell {
                    source_type: condition.source_type,
                    spell_id: condition.source_entry,
                });
            }
        }
        ConditionSourceType::ConversationLine => {
            if let Some(store) = stores.conversation_line_template_store
                && u32::try_from(condition.source_entry)
                    .ok()
                    .is_none_or(|entry| !store.contains(entry))
            {
                return Err(Error::NonExistingConversationLineTemplate(
                    condition.source_entry,
                ));
            }
        }
        ConditionSourceType::AreaTrigger => {
            if let Some(store) = stores.area_trigger_template_store {
                let id = condition.source_group;
                let is_custom = condition.source_entry == 1;
                if !store.contains(id, is_custom) {
                    return Err(Error::NonExistingAreaTriggerTemplate { id, is_custom });
                }
            }
        }
        ConditionSourceType::AreaTriggerClientTriggered => {
            if let Some(store) = stores.area_trigger_store
                && u32::try_from(condition.source_entry)
                    .ok()
                    .and_then(|id| store.get_trigger(id))
                    .is_none()
            {
                return Err(Error::NonExistingClientAreaTrigger(condition.source_entry));
            }
        }
        ConditionSourceType::Graveyard => {
            if let Some(store) = stores.graveyard_store {
                let Some(safe_loc_id) = u32::try_from(condition.source_entry).ok() else {
                    return Err(Error::NonExistingGraveyard {
                        safe_loc_id: condition.source_entry,
                        zone_id: condition.source_group,
                    });
                };

                if store
                    .find_graveyard_data_like_cpp(safe_loc_id, condition.source_group)
                    .is_none()
                {
                    return Err(Error::NonExistingGraveyard {
                        safe_loc_id: condition.source_entry,
                        zone_id: condition.source_group,
                    });
                }
            }
        }
        ConditionSourceType::SpawnGroup => {
            if let Some(store) = stores.spawn_group_store {
                let Some(spawn_group_id) = u32::try_from(condition.source_entry).ok() else {
                    return Err(Error::NonExistingSpawnGroup(condition.source_entry));
                };
                let Some(spawn_group) = store.get(spawn_group_id) else {
                    return Err(Error::NonExistingSpawnGroup(condition.source_entry));
                };
                if spawn_group.is_system_like_cpp() {
                    return Err(Error::SystemSpawnGroup(condition.source_entry));
                }
            }
        }
        ConditionSourceType::ObjectIdVisibility => match condition.source_group {
            value if value == TypeId::Unit as u32 => {
                if let Some(store) = stores.creature_template_store
                    && u32::try_from(condition.source_entry)
                        .ok()
                        .is_none_or(|entry| !store.contains(entry))
                {
                    return Err(Error::NonExistingSourceCreatureTemplate {
                        source_type: condition.source_type,
                        entry: condition.source_entry,
                    });
                }
            }
            value if value == TypeId::GameObject as u32 => {
                if let Some(store) = stores.gameobject_template_store
                    && u32::try_from(condition.source_entry)
                        .ok()
                        .is_none_or(|entry| !store.contains(entry))
                {
                    return Err(Error::NonExistingSourceGameObjectTemplate {
                        source_type: condition.source_type,
                        entry: condition.source_entry,
                    });
                }
            }
            _ => {}
        },
        _ => {}
    }

    Ok(())
}

const fn condition_source_is_loot_template_like_cpp(source_type: ConditionSourceType) -> bool {
    matches!(
        source_type,
        ConditionSourceType::CreatureLootTemplate
            | ConditionSourceType::DisenchantLootTemplate
            | ConditionSourceType::FishingLootTemplate
            | ConditionSourceType::GameObjectLootTemplate
            | ConditionSourceType::ItemLootTemplate
            | ConditionSourceType::MailLootTemplate
            | ConditionSourceType::MillingLootTemplate
            | ConditionSourceType::PickpocketingLootTemplate
            | ConditionSourceType::ProspectingLootTemplate
            | ConditionSourceType::ReferenceLootTemplate
            | ConditionSourceType::SkinningLootTemplate
            | ConditionSourceType::SpellLootTemplate
    )
}

pub fn apply_external_condition_validation_like_cpp(
    report: &mut ConditionLoadReport,
    stores: ConditionExternalValidationStoresLikeCpp<'_>,
) -> Vec<ExternallySkippedConditionLikeCpp> {
    let mut kept = Vec::with_capacity(report.conditions.len());
    let mut skipped = Vec::new();

    for mut condition in report.conditions.drain(..) {
        if let Err(reason) = validate_condition_type_external_like_cpp(&condition, stores) {
            skipped.push(ExternallySkippedConditionLikeCpp {
                condition,
                reason: ConditionRowSkipReason::ConditionTypeValidationFailed(reason),
            });
            continue;
        }

        if let Err(reason) =
            validate_and_normalize_condition_source_external_like_cpp(&mut condition, stores)
        {
            skipped.push(ExternallySkippedConditionLikeCpp {
                condition,
                reason: ConditionRowSkipReason::ConditionSourceValidationFailed(reason),
            });
            continue;
        }

        kept.push(condition);
    }

    report.conditions = kept;
    skipped
}

const fn convert_legacy_type_id_like_cpp(legacy_type_id: u32) -> u32 {
    match legacy_type_id {
        0 => TypeId::Object as u32,
        1 => TypeId::Item as u32,
        2 => TypeId::Container as u32,
        3 => TypeId::Unit as u32,
        4 => TypeId::Player as u32,
        5 => TypeId::GameObject as u32,
        6 => TypeId::DynamicObject as u32,
        7 => TypeId::Corpse as u32,
        8 => TypeId::AreaTrigger as u32,
        9 => TypeId::SceneObject as u32,
        10 => TypeId::Conversation as u32,
        _ => TypeId::Object as u32,
    }
}

fn convert_legacy_type_mask_like_cpp(legacy_type_mask: u32) -> u32 {
    let mut type_mask = 0;
    for legacy_type_id in 0..11 {
        if legacy_type_mask & (1 << legacy_type_id) != 0 {
            type_mask |= 1 << convert_legacy_type_id_like_cpp(legacy_type_id);
        }
    }

    type_mask
}

fn validate_object_entry_guid_type_like_cpp(
    condition: &Condition,
) -> Result<(), ConditionTypeValidationErrorLikeCpp> {
    match TypeId::from_u32(condition.condition_value1) {
        Some(TypeId::Unit | TypeId::GameObject | TypeId::Player | TypeId::Corpse) => Ok(()),
        _ => Err(ConditionTypeValidationErrorLikeCpp::InvalidObjectTypeId(
            condition.condition_value1,
        )),
    }
}

fn validate_type_mask_like_cpp(
    condition: &Condition,
) -> Result<(), ConditionTypeValidationErrorLikeCpp> {
    let allowed_mask =
        (TypeMask::UNIT | TypeMask::PLAYER | TypeMask::GAME_OBJECT | TypeMask::CORPSE).bits();
    if condition.condition_value1 == 0 || condition.condition_value1 & !allowed_mask != 0 {
        return Err(ConditionTypeValidationErrorLikeCpp::InvalidTypeMask(
            condition.condition_value1,
        ));
    }

    Ok(())
}

fn validate_target_selector_like_cpp(
    condition: &Condition,
    field: u8,
) -> Result<(), ConditionTypeValidationErrorLikeCpp> {
    let value = match field {
        1 => condition.condition_value1,
        _ => unreachable!("unsupported condition target selector field"),
    };
    let max = condition.max_available_condition_targets_like_cpp();

    if value >= max {
        return Err(ConditionTypeValidationErrorLikeCpp::InvalidTargetSelector {
            field,
            value,
            max,
        });
    }

    if value == u32::from(condition.condition_target) {
        return Err(ConditionTypeValidationErrorLikeCpp::SelfTargetSelector { field, value });
    }

    Ok(())
}

pub type ConditionContainer = Vec<Condition>;
pub type ConditionsByEntryMap = HashMap<ConditionId, Arc<ConditionContainer>>;

#[derive(Debug, Clone, Default)]
pub struct ConditionsReference {
    conditions: Weak<ConditionContainer>,
}

impl ConditionsReference {
    pub fn new(conditions: &Arc<ConditionContainer>) -> Self {
        Self {
            conditions: Arc::downgrade(conditions),
        }
    }

    pub fn upgrade(&self) -> Option<Arc<ConditionContainer>> {
        self.conditions.upgrade()
    }

    pub fn is_expired(&self) -> bool {
        self.conditions.strong_count() == 0
    }
}

#[derive(Debug, Clone, Default)]
pub struct ConditionEntriesByTypeStore {
    entries: HashMap<ConditionSourceType, ConditionsByEntryMap>,
}

impl ConditionEntriesByTypeStore {
    pub fn from_conditions_like_cpp(conditions: impl IntoIterator<Item = Condition>) -> Self {
        let mut store = Self::default();
        for condition in conditions {
            store.add_condition_like_cpp(condition);
        }
        store
    }

    pub fn add_condition_like_cpp(&mut self, condition: Condition) {
        self.entries
            .entry(condition.source_type)
            .or_default()
            .entry(condition.id_like_cpp())
            .or_insert_with(|| Arc::new(Vec::new()));

        let bucket = self
            .entries
            .get_mut(&condition.source_type)
            .and_then(|by_id| by_id.get_mut(&condition.id_like_cpp()))
            .expect("condition bucket must exist after insertion");
        Arc::make_mut(bucket).push(condition);
    }

    pub fn conditions_for_like_cpp(
        &self,
        source_type: ConditionSourceType,
        id: ConditionId,
    ) -> Option<&Arc<ConditionContainer>> {
        self.entries
            .get(&source_type)
            .and_then(|by_id| by_id.get(&id))
    }

    pub fn entries_for_source_type_like_cpp(
        &self,
        source_type: ConditionSourceType,
    ) -> Option<&ConditionsByEntryMap> {
        self.entries.get(&source_type)
    }

    pub fn reference_for_like_cpp(
        &self,
        source_type: ConditionSourceType,
        id: ConditionId,
    ) -> Option<ConditionsReference> {
        self.conditions_for_like_cpp(source_type, id)
            .map(ConditionsReference::new)
    }

    pub fn bucket_count(&self) -> usize {
        self.entries.values().map(HashMap::len).sum()
    }

    pub fn condition_count(&self) -> usize {
        self.entries
            .values()
            .flat_map(HashMap::values)
            .map(|conditions| conditions.len())
            .sum()
    }

    /// C++ `SpellsUsedInSpellClickConditions` load-time index.
    pub fn spells_used_in_spell_click_conditions_like_cpp(&self) -> std::collections::HashSet<u32> {
        self.entries_for_source_type_like_cpp(ConditionSourceType::SpellClickEvent)
            .into_iter()
            .flat_map(HashMap::values)
            .flat_map(|conditions| conditions.iter())
            .filter(|condition| condition.condition_type == ConditionType::Aura)
            .map(|condition| condition.condition_value1)
            .collect()
    }

    /// C++ `ConditionMgr::IsSpellUsedInSpellClickConditions`.
    pub fn is_spell_used_in_spell_click_conditions_like_cpp(&self, spell_id: u32) -> bool {
        self.entries_for_source_type_like_cpp(ConditionSourceType::SpellClickEvent)
            .into_iter()
            .flat_map(HashMap::values)
            .flat_map(|conditions| conditions.iter())
            .any(|condition| {
                condition.condition_type == ConditionType::Aura
                    && condition.condition_value1 == spell_id
            })
    }

    /// C++ `ConditionMgr::GetSearcherTypeMaskForConditionList`.
    pub fn get_searcher_type_mask_for_condition_list_like_cpp(
        &self,
        conditions: &[Condition],
    ) -> u32 {
        if conditions.is_empty() {
            return GRID_MAP_TYPE_MASK_ALL;
        }

        let mut else_group_searcher_type_masks = std::collections::BTreeMap::<u32, u32>::new();
        for condition in conditions {
            assert!(
                condition.is_loaded_like_cpp(),
                "ConditionMgr::GetSearcherTypeMaskForConditionList - not yet loaded condition found in list"
            );

            let group_mask = else_group_searcher_type_masks
                .entry(condition.else_group)
                .or_insert(GRID_MAP_TYPE_MASK_ALL);
            if *group_mask == 0 {
                continue;
            }

            if condition.reference_id != 0 {
                let reference_conditions = self
                    .conditions_for_like_cpp(
                        ConditionSourceType::ReferenceCondition,
                        ConditionId::new(condition.reference_id, 0, 0),
                    )
                    .expect(
                        "ConditionMgr::GetSearcherTypeMaskForConditionList - incorrect reference",
                    );
                *group_mask &= self.get_searcher_type_mask_for_condition_list_like_cpp(
                    reference_conditions.as_slice(),
                );
            } else {
                *group_mask &= condition.get_searcher_type_mask_for_condition_like_cpp();
            }
        }

        else_group_searcher_type_masks
            .values()
            .fold(0, |mask, group_mask| mask | group_mask)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionDbRowLikeCpp {
    pub source_type_or_reference_id: i32,
    pub source_group: u32,
    pub source_entry: i32,
    pub source_id: u32,
    pub else_group: u32,
    pub condition_type_or_reference: i32,
    pub condition_target: u8,
    pub condition_value1: u32,
    pub condition_value2: u32,
    pub condition_value3: u32,
    pub condition_string_value1: String,
    pub negative_condition: bool,
    pub error_type: u32,
    pub error_text_id: u32,
    pub script_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionRowSkipReason {
    SelfReference(i32),
    InvalidConditionType(i32),
    InvalidSourceType(i32),
    SourceGroupNotAllowed {
        source_type: ConditionSourceType,
        source_group: u32,
    },
    SourceIdNotAllowed {
        source_type: ConditionSourceType,
        source_id: u32,
    },
    ConditionTargetOutOfRange {
        source_type: ConditionSourceType,
        condition_target: u8,
        max_available_targets: u32,
    },
    ConditionTypeValidationFailed(ConditionTypeValidationErrorLikeCpp),
    ConditionSourceValidationFailed(ConditionSourceValidationErrorLikeCpp),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedConditionRow {
    pub row: ConditionDbRowLikeCpp,
    pub reason: ConditionRowSkipReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionLoadWarningLikeCpp {
    ReferenceUselessConditionTarget {
        source_type_or_reference_id: i32,
        condition_target: u8,
    },
    ReferenceUselessValue {
        source_type_or_reference_id: i32,
        field: u8,
        value: u32,
    },
    ReferenceUselessNegativeCondition {
        source_type_or_reference_id: i32,
    },
    ReferenceTemplateUselessSourceGroup {
        source_type_or_reference_id: i32,
        source_group: u32,
    },
    ReferenceTemplateUselessSourceEntry {
        source_type_or_reference_id: i32,
        source_entry: i32,
    },
    ReferenceTemplateUselessSourceId {
        source_type_or_reference_id: i32,
        source_id: u32,
    },
    ErrorTypeResetForNonSpell {
        source_type: ConditionSourceType,
        error_type: u32,
    },
    ErrorTextIdResetWithoutErrorType {
        source_type: ConditionSourceType,
        error_text_id: u32,
    },
    UselessConditionValue {
        condition_type: ConditionType,
        field: u8,
        value: u32,
    },
    UselessConditionStringValue {
        condition_type: ConditionType,
        field: u8,
        value: String,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConditionLoadReport {
    pub conditions: Vec<Condition>,
    pub skipped: Vec<SkippedConditionRow>,
    pub warnings: Vec<ConditionLoadWarningLikeCpp>,
}

impl ConditionLoadReport {
    pub fn parsed_count(&self) -> usize {
        self.conditions.len()
    }

    pub fn into_store_like_cpp(self) -> ConditionEntriesByTypeStore {
        ConditionEntriesByTypeStore::from_conditions_like_cpp(self.conditions)
    }
}

/// C++ `ConditionMgr::LoadConditions` row-to-`Condition` conversion before source/type validation.
pub fn parse_condition_row_like_cpp(
    row: ConditionDbRowLikeCpp,
    mut script_id_for_name: impl FnMut(&str) -> u32,
) -> Result<Condition, SkippedConditionRow> {
    let mut condition = Condition {
        source_group: row.source_group,
        source_entry: row.source_entry,
        source_id: row.source_id,
        else_group: row.else_group,
        condition_target: row.condition_target,
        condition_value1: row.condition_value1,
        condition_value2: row.condition_value2,
        condition_value3: row.condition_value3,
        condition_string_value1: row.condition_string_value1.clone(),
        negative_condition: row.negative_condition,
        error_type: row.error_type,
        error_text_id: row.error_text_id,
        script_id: script_id_for_name(&row.script_name),
        ..Condition::default()
    };

    if row.condition_type_or_reference >= 0 {
        condition.condition_type = ConditionType::from_i32(row.condition_type_or_reference)
            .filter(|condition_type| *condition_type != ConditionType::Max)
            .ok_or_else(|| SkippedConditionRow {
                row: row.clone(),
                reason: ConditionRowSkipReason::InvalidConditionType(
                    row.condition_type_or_reference,
                ),
            })?;
    }

    if row.source_type_or_reference_id >= 0 {
        condition.source_type = ConditionSourceType::from_i32(row.source_type_or_reference_id)
            .ok_or_else(|| SkippedConditionRow {
                row: row.clone(),
                reason: ConditionRowSkipReason::InvalidSourceType(row.source_type_or_reference_id),
            })?;
    }

    if row.condition_type_or_reference < 0 {
        if row.condition_type_or_reference == row.source_type_or_reference_id {
            return Err(SkippedConditionRow {
                row: row.clone(),
                reason: ConditionRowSkipReason::SelfReference(row.source_type_or_reference_id),
            });
        }

        condition.reference_id = u32::try_from(-row.condition_type_or_reference).unwrap_or(0);
    }

    if row.source_type_or_reference_id < 0 {
        condition.source_type = ConditionSourceType::ReferenceCondition;
        condition.source_group = u32::try_from(-row.source_type_or_reference_id).unwrap_or(0);
    }

    Ok(condition)
}

pub fn condition_load_warnings_like_cpp(
    row: &ConditionDbRowLikeCpp,
) -> Vec<ConditionLoadWarningLikeCpp> {
    let mut warnings = Vec::new();

    if row.condition_type_or_reference < 0 {
        if row.condition_target != 0 {
            warnings.push(
                ConditionLoadWarningLikeCpp::ReferenceUselessConditionTarget {
                    source_type_or_reference_id: row.source_type_or_reference_id,
                    condition_target: row.condition_target,
                },
            );
        }
        if row.condition_value1 != 0 {
            warnings.push(ConditionLoadWarningLikeCpp::ReferenceUselessValue {
                source_type_or_reference_id: row.source_type_or_reference_id,
                field: 1,
                value: row.condition_value1,
            });
        }
        if row.condition_value2 != 0 {
            warnings.push(ConditionLoadWarningLikeCpp::ReferenceUselessValue {
                source_type_or_reference_id: row.source_type_or_reference_id,
                field: 2,
                value: row.condition_value2,
            });
        }
        if row.condition_value3 != 0 {
            warnings.push(ConditionLoadWarningLikeCpp::ReferenceUselessValue {
                source_type_or_reference_id: row.source_type_or_reference_id,
                field: 3,
                value: row.condition_value3,
            });
        }
        if row.negative_condition {
            warnings.push(
                ConditionLoadWarningLikeCpp::ReferenceUselessNegativeCondition {
                    source_type_or_reference_id: row.source_type_or_reference_id,
                },
            );
        }
    }

    if row.source_type_or_reference_id < 0 {
        if row.source_group != 0 {
            warnings.push(
                ConditionLoadWarningLikeCpp::ReferenceTemplateUselessSourceGroup {
                    source_type_or_reference_id: row.source_type_or_reference_id,
                    source_group: row.source_group,
                },
            );
        }
        if row.source_entry != 0 {
            warnings.push(
                ConditionLoadWarningLikeCpp::ReferenceTemplateUselessSourceEntry {
                    source_type_or_reference_id: row.source_type_or_reference_id,
                    source_entry: row.source_entry,
                },
            );
        }
        if row.source_id != 0 {
            warnings.push(
                ConditionLoadWarningLikeCpp::ReferenceTemplateUselessSourceId {
                    source_type_or_reference_id: row.source_type_or_reference_id,
                    source_id: row.source_id,
                },
            );
        }
    }

    warnings
}

fn condition_normalization_warnings_like_cpp(
    condition: &Condition,
) -> Vec<ConditionLoadWarningLikeCpp> {
    let mut warnings = Vec::new();

    if condition.error_type != 0 && condition.source_type != ConditionSourceType::Spell {
        warnings.push(ConditionLoadWarningLikeCpp::ErrorTypeResetForNonSpell {
            source_type: condition.source_type,
            error_type: condition.error_type,
        });
    }

    if condition.error_text_id != 0
        && (condition.error_type == 0 || condition.source_type != ConditionSourceType::Spell)
    {
        warnings.push(
            ConditionLoadWarningLikeCpp::ErrorTextIdResetWithoutErrorType {
                source_type: condition.source_type,
                error_text_id: condition.error_text_id,
            },
        );
    }

    warnings
}

fn condition_type_useless_value_warnings_like_cpp(
    condition: &Condition,
) -> Vec<ConditionLoadWarningLikeCpp> {
    useless_condition_value_fields_like_cpp(condition)
        .into_iter()
        .map(|field| match field {
            1 => ConditionLoadWarningLikeCpp::UselessConditionValue {
                condition_type: condition.condition_type,
                field,
                value: condition.condition_value1,
            },
            2 => ConditionLoadWarningLikeCpp::UselessConditionValue {
                condition_type: condition.condition_type,
                field,
                value: condition.condition_value2,
            },
            3 => ConditionLoadWarningLikeCpp::UselessConditionValue {
                condition_type: condition.condition_type,
                field,
                value: condition.condition_value3,
            },
            4 => ConditionLoadWarningLikeCpp::UselessConditionStringValue {
                condition_type: condition.condition_type,
                field,
                value: condition.condition_string_value1.clone(),
            },
            _ => unreachable!("condition value field must be 1..=4"),
        })
        .collect()
}

pub fn normalize_loaded_condition_shape_like_cpp(
    condition: &mut Condition,
) -> Result<(), ConditionRowSkipReason> {
    normalize_loaded_condition_shape_inner_like_cpp(condition, false)
}

fn normalize_loaded_condition_shape_for_row_like_cpp(
    condition: &mut Condition,
    row: &ConditionDbRowLikeCpp,
) -> Result<(), ConditionRowSkipReason> {
    normalize_loaded_condition_shape_inner_like_cpp(condition, row.source_type_or_reference_id < 0)
}

fn normalize_loaded_condition_shape_inner_like_cpp(
    condition: &mut Condition,
    is_reference_template: bool,
) -> Result<(), ConditionRowSkipReason> {
    if condition.reference_id == 0 {
        validate_condition_type_static_like_cpp(condition)
            .map_err(ConditionRowSkipReason::ConditionTypeValidationFailed)?;

        let max_available_targets = condition.max_available_condition_targets_like_cpp();
        if u32::from(condition.condition_target) >= max_available_targets {
            return Err(ConditionRowSkipReason::ConditionTargetOutOfRange {
                source_type: condition.source_type,
                condition_target: condition.condition_target,
                max_available_targets,
            });
        }
    }

    if !is_reference_template {
        validate_condition_source_static_like_cpp(condition)
            .map_err(ConditionRowSkipReason::ConditionSourceValidationFailed)?;
    }

    if condition.source_group != 0
        && !condition_source_can_have_group_set_like_cpp(condition.source_type)
    {
        return Err(ConditionRowSkipReason::SourceGroupNotAllowed {
            source_type: condition.source_type,
            source_group: condition.source_group,
        });
    }

    if condition.source_id != 0 && !condition_source_can_have_id_set_like_cpp(condition.source_type)
    {
        return Err(ConditionRowSkipReason::SourceIdNotAllowed {
            source_type: condition.source_type,
            source_id: condition.source_id,
        });
    }

    if condition.error_type != 0 && condition.source_type != ConditionSourceType::Spell {
        condition.error_type = 0;
    }

    if condition.error_text_id != 0 && condition.error_type == 0 {
        condition.error_text_id = 0;
    }

    Ok(())
}

pub fn parse_condition_rows_like_cpp(
    rows: impl IntoIterator<Item = ConditionDbRowLikeCpp>,
    mut script_id_for_name: impl FnMut(&str) -> u32,
) -> ConditionLoadReport {
    let mut report = ConditionLoadReport::default();
    for row in rows {
        report
            .warnings
            .extend(condition_load_warnings_like_cpp(&row));
        match parse_condition_row_like_cpp(row.clone(), &mut script_id_for_name) {
            Ok(mut condition) => {
                report
                    .warnings
                    .extend(condition_normalization_warnings_like_cpp(&condition));
                match normalize_loaded_condition_shape_for_row_like_cpp(&mut condition, &row) {
                    Ok(()) => {
                        if condition.reference_id == 0 {
                            report
                                .warnings
                                .extend(condition_type_useless_value_warnings_like_cpp(&condition));
                        }
                        report.conditions.push(condition);
                    }
                    Err(reason) => report.skipped.push(SkippedConditionRow { row, reason }),
                }
            }
            Err(skipped) => report.skipped.push(skipped),
        }
    }
    report
}

pub async fn load_condition_rows_like_cpp(
    db: &WorldDatabase,
    script_id_for_name: impl FnMut(&str) -> u32,
) -> Result<ConditionLoadReport> {
    let stmt = db.prepare(WorldStatements::SEL_CONDITIONS);
    let mut result = db.query(&stmt).await?;
    if result.is_empty() {
        return Ok(ConditionLoadReport::default());
    }

    let mut rows = Vec::new();
    loop {
        rows.push(ConditionDbRowLikeCpp {
            source_type_or_reference_id: read_db_i32_like_cpp(&result, 0),
            source_group: read_db_u32_like_cpp(&result, 1),
            source_entry: read_db_i32_like_cpp(&result, 2),
            source_id: read_db_i32_like_cpp(&result, 3) as u32,
            else_group: read_db_u32_like_cpp(&result, 4),
            condition_type_or_reference: read_db_i32_like_cpp(&result, 5),
            condition_target: read_db_u8_like_cpp(&result, 6),
            condition_value1: read_db_u32_like_cpp(&result, 7),
            condition_value2: read_db_u32_like_cpp(&result, 8),
            condition_value3: read_db_u32_like_cpp(&result, 9),
            condition_string_value1: result.read_string(10),
            negative_condition: read_db_bool_like_cpp(&result, 11),
            error_type: read_db_u32_like_cpp(&result, 12),
            error_text_id: read_db_u32_like_cpp(&result, 13),
            script_name: result.read_string(14),
        });

        if !result.next_row() {
            break;
        }
    }

    let report = parse_condition_rows_like_cpp(rows, script_id_for_name);
    info!(
        "Parsed {} conditions rows ({} skipped before validation, {} load warnings)",
        report.parsed_count(),
        report.skipped.len(),
        report.warnings.len()
    );
    Ok(report)
}

fn read_db_i32_like_cpp(result: &SqlResult, column: usize) -> i32 {
    if let Some(value) = result.try_read::<i32>(column) {
        return value;
    }
    if let Some(value) = result.try_read::<u32>(column) {
        return normalize_unsigned_db_i32_like_cpp(value);
    }
    if let Some(value) = result.try_read::<i16>(column) {
        return i32::from(value);
    }
    if let Some(value) = result.try_read::<u16>(column) {
        return i32::from(value);
    }
    if let Some(value) = result.try_read::<i8>(column) {
        return i32::from(value);
    }
    if let Some(value) = result.try_read::<u8>(column) {
        return i32::from(value);
    }
    if let Some(value) = result.try_read::<i64>(column) {
        return i32::try_from(value).unwrap_or(0);
    }
    if let Some(value) = result.try_read::<u64>(column) {
        return i32::try_from(value).unwrap_or(0);
    }
    0
}

fn read_db_u32_like_cpp(result: &SqlResult, column: usize) -> u32 {
    if let Some(value) = result.try_read::<u32>(column) {
        return value;
    }
    if let Some(value) = result.try_read::<i32>(column) {
        return normalize_signed_db_u32_like_cpp(value);
    }
    if let Some(value) = result.try_read::<u16>(column) {
        return u32::from(value);
    }
    if let Some(value) = result.try_read::<i16>(column) {
        return normalize_signed_db_u32_like_cpp(i32::from(value));
    }
    if let Some(value) = result.try_read::<u8>(column) {
        return u32::from(value);
    }
    if let Some(value) = result.try_read::<i8>(column) {
        return normalize_signed_db_u32_like_cpp(i32::from(value));
    }
    if let Some(value) = result.try_read::<u64>(column) {
        return u32::try_from(value).unwrap_or(0);
    }
    if let Some(value) = result.try_read::<i64>(column) {
        return if (0..=i64::from(u32::MAX)).contains(&value) {
            value as u32
        } else {
            0
        };
    }
    0
}

fn read_db_u8_like_cpp(result: &SqlResult, column: usize) -> u8 {
    if let Some(value) = result.try_read::<u8>(column) {
        return value;
    }
    if let Some(value) = result.try_read::<i8>(column) {
        return normalize_signed_db_u8_like_cpp(i32::from(value));
    }
    if let Some(value) = result.try_read::<u16>(column) {
        return u8::try_from(value).unwrap_or(0);
    }
    if let Some(value) = result.try_read::<i16>(column) {
        return normalize_signed_db_u8_like_cpp(i32::from(value));
    }
    if let Some(value) = result.try_read::<u32>(column) {
        return u8::try_from(value).unwrap_or(0);
    }
    if let Some(value) = result.try_read::<i32>(column) {
        return normalize_signed_db_u8_like_cpp(value);
    }
    0
}

fn read_db_bool_like_cpp(result: &SqlResult, column: usize) -> bool {
    read_db_u8_like_cpp(result, column) == 1
}

fn normalize_signed_db_u32_like_cpp(value: i32) -> u32 {
    value as u32
}

fn normalize_unsigned_db_i32_like_cpp(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(0)
}

fn normalize_signed_db_u8_like_cpp(value: i32) -> u8 {
    let converted = value as u8;
    if i32::from(converted) == value || (converted as i8) as i32 == value {
        converted
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn condition_sql_columns_normalize_like_cpp_field_accessors() {
        assert_eq!(normalize_signed_db_u32_like_cpp(-1), u32::MAX);
        assert_eq!(normalize_signed_db_u8_like_cpp(-1), u8::MAX);
        assert_eq!(normalize_signed_db_u8_like_cpp(0x100), 0);
        assert_eq!(normalize_unsigned_db_i32_like_cpp(i32::MAX as u32 + 1), 0);

        let row = ConditionDbRowLikeCpp {
            source_type_or_reference_id: ConditionSourceType::SmartEvent as i32,
            source_group: 1,
            source_entry: -5,
            source_id: (-1_i32) as u32,
            else_group: 0,
            condition_type_or_reference: ConditionType::None as i32,
            condition_target: 0,
            condition_value1: 0,
            condition_value2: 0,
            condition_value3: 0,
            condition_string_value1: String::new(),
            negative_condition: false,
            error_type: 0,
            error_text_id: 0,
            script_name: String::new(),
        };
        let condition = parse_condition_row_like_cpp(row, |_| 0).unwrap();
        assert_eq!(condition.source_id, u32::MAX);
    }

    fn spell_info(spell_id: i32) -> crate::SpellInfo {
        crate::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            effects: Vec::new(),
        }
    }

    fn quest_template(
        id: u32,
        objectives: Vec<crate::quest::QuestObjective>,
    ) -> crate::quest::QuestTemplate {
        crate::quest::QuestTemplate {
            id,
            quest_type: 2,
            quest_level: 1,
            quest_max_scaling_level: 0,
            quest_package_id: 0,
            min_level: 1,
            quest_sort_id: 0,
            quest_info_id: 0,
            suggested_group_num: 0,
            reward_next_quest: 0,
            reward_xp_difficulty: 0,
            reward_xp_multiplier: 1.0,
            reward_money_difficulty: 0,
            reward_money_multiplier: 1.0,
            reward_bonus_money: 0,
            reward_display_spell: [0; crate::quest::QUEST_REWARD_DISPLAY_SPELL_COUNT],
            reward_spell: 0,
            reward_honor: 0,
            reward_title_id: 0,
            reward_skill_line_id: 0,
            reward_skill_points: 0,
            reward_mail_template_id: 0,
            reward_mail_delay_secs: 0,
            reward_mail_sender_entry: 0,
            reward_faction_ids: [0; crate::quest::QUEST_REWARD_REPUTATIONS_COUNT],
            reward_faction_values: [0; crate::quest::QUEST_REWARD_REPUTATIONS_COUNT],
            reward_faction_overrides: [0; crate::quest::QUEST_REWARD_REPUTATIONS_COUNT],
            reward_faction_cap_in: [0; crate::quest::QUEST_REWARD_REPUTATIONS_COUNT],
            reward_faction_flags: 0,
            source_item_id: 0,
            source_item_count: 0,
            source_spell_id: 0,
            limit_time_secs: 0,
            expansion: 0,
            flags: 0,
            flags_ex: 0,
            flags_ex2: 0,
            special_flags: 0,
            event_id_for_quest: 0,
            reward_items: [0; crate::quest::QUEST_REWARD_ITEM_COUNT],
            reward_amounts: [0; crate::quest::QUEST_REWARD_ITEM_COUNT],
            reward_currencies: [0; crate::quest::QUEST_REWARD_CURRENCY_COUNT],
            reward_currency_amounts: [0; crate::quest::QUEST_REWARD_CURRENCY_COUNT],
            item_drop: [0; crate::quest::QUEST_ITEM_DROP_COUNT],
            item_drop_quantity: [0; crate::quest::QUEST_ITEM_DROP_COUNT],
            log_title: String::new(),
            log_description: String::new(),
            quest_description: String::new(),
            area_description: String::new(),
            quest_completion_log: String::new(),
            objectives,
            allowable_races: 0,
            allowable_classes: 0,
            max_level: 0,
            prev_quest_id: 0,
            next_quest_id: 0,
            exclusive_group: 0,
            breadcrumb_for_quest_id: 0,
            dependent_previous_quests: Vec::new(),
            dependent_breadcrumb_quests: Vec::new(),
            required_min_rep_faction: 0,
            required_min_rep_value: 0,
            required_max_rep_faction: 0,
            required_max_rep_value: 0,
            required_skill_id: 0,
            required_skill_points: 0,
            reward_choice_items: [(0, 0); crate::quest::QUEST_REWARD_CHOICES_COUNT],
            reward_choice_item_types: [0; crate::quest::QUEST_REWARD_CHOICES_COUNT],
        }
    }

    fn quest_objective(id: u32, obj_type: u8, amount: i32) -> crate::quest::QuestObjective {
        crate::quest::QuestObjective {
            id,
            quest_id: 42,
            obj_type,
            order: 0,
            storage_index: 0,
            object_id: 0,
            amount,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        }
    }

    fn condition_row(
        source_type: ConditionSourceType,
        condition_type: ConditionType,
    ) -> ConditionDbRowLikeCpp {
        ConditionDbRowLikeCpp {
            source_type_or_reference_id: source_type as i32,
            source_group: 0,
            source_entry: 0,
            source_id: 0,
            else_group: 0,
            condition_type_or_reference: condition_type as i32,
            condition_target: 0,
            condition_value1: 0,
            condition_value2: 0,
            condition_value3: 0,
            condition_string_value1: String::new(),
            negative_condition: false,
            error_type: 0,
            error_text_id: 0,
            script_name: String::new(),
        }
    }

    #[test]
    fn condition_default_matches_cpp_constructor() {
        let condition = Condition::default();

        assert_eq!(condition.source_type, ConditionSourceType::None);
        assert_eq!(condition.source_group, 0);
        assert_eq!(condition.source_entry, 0);
        assert_eq!(condition.source_id, 0);
        assert_eq!(condition.else_group, 0);
        assert_eq!(condition.condition_type, ConditionType::None);
        assert_eq!(condition.condition_value1, 0);
        assert_eq!(condition.condition_value2, 0);
        assert_eq!(condition.condition_value3, 0);
        assert!(condition.condition_string_value1.is_empty());
        assert_eq!(condition.error_type, 0);
        assert_eq!(condition.error_text_id, 0);
        assert_eq!(condition.reference_id, 0);
        assert_eq!(condition.script_id, 0);
        assert_eq!(condition.condition_target, 0);
        assert!(!condition.negative_condition);
        assert!(!condition.is_loaded_like_cpp());
    }

    #[test]
    fn condition_is_loaded_matches_cpp() {
        let mut condition = Condition {
            condition_type: ConditionType::Aura,
            ..Condition::default()
        };
        assert!(condition.is_loaded_like_cpp());

        condition.condition_type = ConditionType::None;
        condition.reference_id = 42;
        assert!(condition.is_loaded_like_cpp());

        condition.reference_id = 0;
        condition.script_id = 7;
        assert!(condition.is_loaded_like_cpp());
    }

    #[test]
    fn source_group_and_id_flags_match_cpp() {
        assert!(condition_source_can_have_group_set_like_cpp(
            ConditionSourceType::CreatureLootTemplate,
        ));
        assert!(condition_source_can_have_group_set_like_cpp(
            ConditionSourceType::SpellClickEvent,
        ));
        assert!(condition_source_can_have_group_set_like_cpp(
            ConditionSourceType::ReferenceCondition,
        ));
        assert!(!condition_source_can_have_group_set_like_cpp(
            ConditionSourceType::QuestAvailable,
        ));

        assert!(condition_source_can_have_id_set_like_cpp(
            ConditionSourceType::SmartEvent,
        ));
        assert!(!condition_source_can_have_id_set_like_cpp(
            ConditionSourceType::SpellClickEvent,
        ));
    }

    #[test]
    fn source_condition_type_allowance_matches_cpp_spawn_group_special_case() {
        assert!(condition_source_can_have_condition_type_like_cpp(
            ConditionSourceType::SpawnGroup,
            ConditionType::MapId,
        ));
        assert!(condition_source_can_have_condition_type_like_cpp(
            ConditionSourceType::SpawnGroup,
            ConditionType::ScenarioStep,
        ));
        assert!(!condition_source_can_have_condition_type_like_cpp(
            ConditionSourceType::SpawnGroup,
            ConditionType::Aura,
        ));
        assert!(condition_source_can_have_condition_type_like_cpp(
            ConditionSourceType::NpcVendor,
            ConditionType::Aura,
        ));
    }

    #[test]
    fn max_available_condition_targets_matches_cpp_source_groups() {
        assert_eq!(
            Condition {
                source_type: ConditionSourceType::SpellClickEvent,
                ..Condition::default()
            }
            .max_available_condition_targets_like_cpp(),
            2
        );
        assert_eq!(
            Condition {
                source_type: ConditionSourceType::SmartEvent,
                ..Condition::default()
            }
            .max_available_condition_targets_like_cpp(),
            2
        );
        assert_eq!(
            Condition {
                source_type: ConditionSourceType::Phase,
                ..Condition::default()
            }
            .max_available_condition_targets_like_cpp(),
            1
        );
    }

    #[test]
    fn condition_to_string_matches_cpp_shape() {
        let condition = Condition {
            source_type: ConditionSourceType::SmartEvent,
            source_group: 8,
            source_entry: -7,
            source_id: 9,
            condition_type: ConditionType::ObjectEntryGuid,
            ..Condition::default()
        };

        assert_eq!(
            condition.to_string_like_cpp(false),
            "[Condition SourceType: 22 (SmartScript), SourceGroup: 8, SourceEntry: -7, SourceId: 9]"
        );
        assert_eq!(
            condition.to_string_like_cpp(true),
            "[Condition SourceType: 22 (SmartScript), SourceGroup: 8, SourceEntry: -7, SourceId: 9, ConditionType: 51 (Object Entry or Guid)]"
        );
    }

    #[test]
    fn condition_to_string_handles_reference_and_private_object_name() {
        let condition = Condition {
            source_type: ConditionSourceType::ReferenceCondition,
            source_group: 55,
            condition_type: ConditionType::PrivateObject,
            ..Condition::default()
        };

        assert_eq!(
            condition.to_string_like_cpp(true),
            "[Condition SourceType: 34 (Reference), SourceGroup: 55, SourceEntry: 0, ConditionType: 57 (Private Object)]"
        );
    }

    #[test]
    fn condition_type_info_matches_cpp_value_slot_metadata() {
        let aura = condition_type_info_like_cpp(ConditionType::Aura);
        assert_eq!(aura.name, "Aura");
        assert!(aura.has_condition_value1);
        assert!(aura.has_condition_value2);
        assert!(aura.has_condition_value3);
        assert!(!aura.has_condition_string_value1);

        let alive = condition_type_info_like_cpp(ConditionType::Alive);
        assert_eq!(alive.name, "Alive");
        assert!(!alive.has_condition_value1);
        assert!(!alive.has_condition_value2);
        assert!(!alive.has_condition_value3);
        assert!(!alive.has_condition_string_value1);

        let objective = condition_type_info_like_cpp(ConditionType::QuestObjectiveProgress);
        assert_eq!(objective.name, "Quest objective progress");
        assert!(objective.has_condition_value1);
        assert!(!objective.has_condition_value2);
        assert!(objective.has_condition_value3);
        assert!(!objective.has_condition_string_value1);

        let string_id = condition_type_info_like_cpp(ConditionType::StringId);
        assert_eq!(string_id.name, "String ID");
        assert!(!string_id.has_condition_value1);
        assert!(!string_id.has_condition_value2);
        assert!(!string_id.has_condition_value3);
        assert!(string_id.has_condition_string_value1);
    }

    #[test]
    fn condition_type_info_keeps_private_object_slot_explicit() {
        let private_object = condition_type_info_like_cpp(ConditionType::PrivateObject);

        assert_eq!(private_object.name, "Private Object");
        assert!(!private_object.has_condition_value1);
        assert!(!private_object.has_condition_value2);
        assert!(!private_object.has_condition_value3);
        assert!(!private_object.has_condition_string_value1);
    }

    #[test]
    fn useless_condition_value_fields_match_cpp_static_metadata() {
        let alive = Condition {
            condition_type: ConditionType::Alive,
            condition_value1: 1,
            condition_value2: 2,
            condition_value3: 3,
            condition_string_value1: String::from("unused"),
            ..Condition::default()
        };
        assert_eq!(
            useless_condition_value_fields_like_cpp(&alive),
            vec![1, 2, 3, 4]
        );

        let aura = Condition {
            condition_type: ConditionType::Aura,
            condition_value1: 1,
            condition_value2: 2,
            condition_value3: 3,
            condition_string_value1: String::from("unused"),
            ..Condition::default()
        };
        assert_eq!(useless_condition_value_fields_like_cpp(&aura), vec![4]);

        let player_object_entry_guid = Condition {
            condition_type: ConditionType::ObjectEntryGuid,
            condition_value1: TypeId::Player as u32,
            condition_value2: 42,
            condition_value3: 77,
            ..Condition::default()
        };
        assert_eq!(
            useless_condition_value_fields_like_cpp(&player_object_entry_guid),
            vec![2, 3]
        );
    }

    #[test]
    fn condition_type_static_validation_matches_cpp_pure_rejections() {
        let mut invalid_aura = Condition {
            condition_type: ConditionType::Aura,
            condition_value2: MAX_SPELL_EFFECTS_LIKE_CPP,
            ..Condition::default()
        };
        assert_eq!(
            validate_condition_type_static_like_cpp(&mut invalid_aura).unwrap_err(),
            ConditionTypeValidationErrorLikeCpp::InvalidSpellEffectIndex(
                MAX_SPELL_EFFECTS_LIKE_CPP,
            )
        );

        let mut zero_item_count = Condition {
            condition_type: ConditionType::Item,
            condition_value1: 25,
            condition_value2: 0,
            ..Condition::default()
        };
        assert_eq!(
            validate_condition_type_static_like_cpp(&mut zero_item_count).unwrap_err(),
            ConditionTypeValidationErrorLikeCpp::ZeroItemCount
        );

        let mut invalid_team = Condition {
            condition_type: ConditionType::Team,
            condition_value1: 123,
            ..Condition::default()
        };
        assert_eq!(
            validate_condition_type_static_like_cpp(&mut invalid_team).unwrap_err(),
            ConditionTypeValidationErrorLikeCpp::InvalidTeam(123)
        );

        let mut invalid_level = Condition {
            condition_type: ConditionType::Level,
            condition_value2: ComparisonType::Max as u32,
            ..Condition::default()
        };
        assert_eq!(
            validate_condition_type_static_like_cpp(&mut invalid_level).unwrap_err(),
            ConditionTypeValidationErrorLikeCpp::InvalidComparisonType {
                field: 2,
                value: ComparisonType::Max as u32,
            }
        );

        let mut invalid_stand_state = Condition {
            condition_type: ConditionType::StandState,
            condition_value1: 0,
            condition_value2: UnitStandStateType::Max as u32,
            ..Condition::default()
        };
        assert_eq!(
            validate_condition_type_static_like_cpp(&mut invalid_stand_state).unwrap_err(),
            ConditionTypeValidationErrorLikeCpp::InvalidStandState {
                value1: 0,
                value2: UnitStandStateType::Max as u32,
            }
        );

        let mut invalid_battle_pet_count = Condition {
            condition_type: ConditionType::BattlePetCount,
            condition_value2: DEFAULT_MAX_BATTLE_PETS_PER_SPECIES_LIKE_CPP + 1,
            condition_value3: ComparisonType::Eq as u32,
            ..Condition::default()
        };
        assert_eq!(
            validate_condition_type_static_like_cpp(&mut invalid_battle_pet_count).unwrap_err(),
            ConditionTypeValidationErrorLikeCpp::InvalidBattlePetCount(
                DEFAULT_MAX_BATTLE_PETS_PER_SPECIES_LIKE_CPP + 1,
            )
        );

        let mut string_id = Condition {
            condition_type: ConditionType::StringId,
            condition_string_value1: "spawn-id".to_string(),
            ..Condition::default()
        };
        assert_eq!(
            validate_condition_type_static_like_cpp(&mut string_id),
            Ok(ConditionTypeValidationReportLikeCpp {
                useless_value_fields: Vec::new(),
            })
        );
    }

    #[test]
    fn condition_type_static_validation_matches_cpp_target_selector_rules() {
        let mut self_relation = Condition {
            source_type: ConditionSourceType::SpellClickEvent,
            condition_target: 1,
            condition_type: ConditionType::RelationTo,
            condition_value1: 1,
            condition_value2: RelationType::InParty as u32,
            ..Condition::default()
        };
        assert_eq!(
            validate_condition_type_static_like_cpp(&mut self_relation).unwrap_err(),
            ConditionTypeValidationErrorLikeCpp::SelfTargetSelector { field: 1, value: 1 }
        );

        let mut invalid_distance = Condition {
            source_type: ConditionSourceType::SpellClickEvent,
            condition_target: 0,
            condition_type: ConditionType::DistanceTo,
            condition_value1: 2,
            condition_value3: ComparisonType::Eq as u32,
            ..Condition::default()
        };
        assert_eq!(
            validate_condition_type_static_like_cpp(&mut invalid_distance).unwrap_err(),
            ConditionTypeValidationErrorLikeCpp::InvalidTargetSelector {
                field: 1,
                value: 2,
                max: 2,
            }
        );

        let mut valid_reaction = Condition {
            source_type: ConditionSourceType::SpellClickEvent,
            condition_target: 0,
            condition_type: ConditionType::ReactionTo,
            condition_value1: 1,
            condition_value2: 1,
            ..Condition::default()
        };
        assert!(validate_condition_type_static_like_cpp(&mut valid_reaction).is_ok());
    }

    #[test]
    fn condition_type_static_validation_normalizes_legacy_object_types_like_cpp() {
        let mut legacy_object_entry = Condition {
            condition_type: ConditionType::ObjectEntryGuidLegacy,
            condition_value1: 3,
            ..Condition::default()
        };

        validate_condition_type_static_like_cpp(&mut legacy_object_entry).unwrap();

        assert_eq!(
            legacy_object_entry.condition_type,
            ConditionType::ObjectEntryGuid
        );
        assert_eq!(legacy_object_entry.condition_value1, TypeId::Unit as u32);

        let mut legacy_type_mask = Condition {
            condition_type: ConditionType::TypeMaskLegacy,
            condition_value1: (1 << 3) | (1 << 5),
            ..Condition::default()
        };

        validate_condition_type_static_like_cpp(&mut legacy_type_mask).unwrap();

        assert_eq!(legacy_type_mask.condition_type, ConditionType::TypeMask);
        assert_eq!(
            legacy_type_mask.condition_value1,
            (TypeMask::UNIT | TypeMask::GAME_OBJECT).bits()
        );
    }

    #[test]
    fn condition_external_type_validation_uses_loaded_stores_like_cpp() {
        let item_store = crate::ItemStore::from_records([crate::ItemRecord {
            id: 100,
            class_id: 0,
            subclass_id: 0,
            material: 0,
            inventory_type: 0,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
        }]);
        let mut spell_store = crate::SpellStore::new();
        spell_store.insert(200, spell_info(200));
        let area_store = crate::AreaTableStore::from_entries([
            crate::AreaTableEntry {
                id: 300,
                continent_id: 0,
                parent_area_id: 0,
                mount_flags: 0,
                flags: 0,
            },
            crate::AreaTableEntry {
                id: 301,
                continent_id: 0,
                parent_area_id: 300,
                mount_flags: 0,
                flags: crate::area::AREA_FLAG_IS_SUBZONE_LIKE_CPP,
            },
        ]);
        let skill_store = crate::SkillStore::from_skill_lines_like_cpp([400]);
        let map_store = crate::MapStore::from_entries([crate::MapEntry {
            id: 500,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            instance_type: 0,
            flags1: 0,
        }]);
        let phase_store =
            crate::PhaseStore::from_entries([crate::PhaseEntry { id: 600, flags: 0 }]);
        let quest_store = crate::quest::QuestStore::from_quests_like_cpp([quest_template(
            700,
            vec![quest_objective(800, 0, 5), quest_objective(801, 10, 99)],
        )]);
        let difficulty_store = crate::DifficultyStore::from_ids([900]);
        let faction_store = crate::Db2IdStore::from_ids("Faction.db2", [910]);
        let achievement_store = crate::Db2IdStore::from_ids("Achievement.db2", [920]);
        let char_titles_store = crate::Db2IdStore::from_ids("CharTitles.db2", [930]);
        let battle_pet_species_store = crate::Db2IdStore::from_ids("BattlePetSpecies.db2", [940]);
        let scenario_step_store = crate::Db2IdStore::from_ids("ScenarioStep.db2", [950]);
        let scene_script_package_store =
            crate::Db2IdStore::from_ids("SceneScriptPackage.db2", [960]);
        let player_condition_store =
            crate::PlayerConditionStore::from_entries([crate::PlayerConditionEntry {
                id: 970,
                ..crate::PlayerConditionEntry::default()
            }]);
        let creature_template_store =
            crate::WorldIdStore::from_ids("creature_template", [980, 981]);
        let gameobject_template_store =
            crate::WorldIdStore::from_ids("gameobject_template", [990, 991]);
        let creature_spawn_store =
            crate::WorldSpawnIdStore::from_entries("creature", [(1000, 980)]);
        let gameobject_spawn_store =
            crate::WorldSpawnIdStore::from_entries("gameobject", [(1001, 990)]);
        let active_event_store = crate::WorldIdStore::from_ids("game_event", [1100]);
        let world_state_store = crate::WorldIdStore::from_ids("world_state", [1200]);
        let stores = ConditionExternalValidationStoresLikeCpp {
            item_store: Some(&item_store),
            spell_store: Some(&spell_store),
            area_table_store: Some(&area_store),
            skill_store: Some(&skill_store),
            map_store: Some(&map_store),
            phase_store: Some(&phase_store),
            quest_store: Some(&quest_store),
            difficulty_store: Some(&difficulty_store),
            faction_store: Some(&faction_store),
            achievement_store: Some(&achievement_store),
            char_titles_store: Some(&char_titles_store),
            battle_pet_species_store: Some(&battle_pet_species_store),
            scenario_step_store: Some(&scenario_step_store),
            scene_script_package_store: Some(&scene_script_package_store),
            player_condition_store: Some(&player_condition_store),
            creature_template_store: Some(&creature_template_store),
            gameobject_template_store: Some(&gameobject_template_store),
            creature_spawn_store: Some(&creature_spawn_store),
            gameobject_spawn_store: Some(&gameobject_spawn_store),
            active_event_store: Some(&active_event_store),
            world_state_store: Some(&world_state_store),
            max_skill_value: Some(450),
            ..ConditionExternalValidationStoresLikeCpp::default()
        };

        for condition in [
            Condition {
                condition_type: ConditionType::Item,
                condition_value1: 100,
                condition_value2: 1,
                ..Condition::default()
            },
            Condition {
                condition_type: ConditionType::Aura,
                condition_value1: 200,
                ..Condition::default()
            },
            Condition {
                condition_type: ConditionType::ZoneId,
                condition_value1: 300,
                ..Condition::default()
            },
            Condition {
                condition_type: ConditionType::Skill,
                condition_value1: 400,
                condition_value2: 450,
                ..Condition::default()
            },
            Condition {
                condition_type: ConditionType::MapId,
                condition_value1: 500,
                ..Condition::default()
            },
            Condition {
                condition_type: ConditionType::PhaseId,
                condition_value1: 600,
                ..Condition::default()
            },
            Condition {
                condition_type: ConditionType::QuestRewarded,
                condition_value1: 700,
                ..Condition::default()
            },
            Condition {
                condition_type: ConditionType::QuestObjectiveProgress,
                condition_value1: 800,
                condition_value3: 5,
                ..Condition::default()
            },
            Condition {
                condition_type: ConditionType::QuestObjectiveProgress,
                condition_value1: 801,
                condition_value3: 1,
                ..Condition::default()
            },
            Condition {
                condition_type: ConditionType::DifficultyId,
                condition_value1: 900,
                ..Condition::default()
            },
            Condition {
                condition_type: ConditionType::ReputationRank,
                condition_value1: 910,
                ..Condition::default()
            },
            Condition {
                condition_type: ConditionType::Achievement,
                condition_value1: 920,
                ..Condition::default()
            },
            Condition {
                condition_type: ConditionType::RealmAchievement,
                condition_value1: 920,
                ..Condition::default()
            },
            Condition {
                condition_type: ConditionType::Title,
                condition_value1: 930,
                ..Condition::default()
            },
            Condition {
                condition_type: ConditionType::BattlePetCount,
                condition_value1: 940,
                condition_value3: 0,
                ..Condition::default()
            },
            Condition {
                condition_type: ConditionType::ScenarioStep,
                condition_value1: 950,
                ..Condition::default()
            },
            Condition {
                condition_type: ConditionType::SceneInProgress,
                condition_value1: 960,
                ..Condition::default()
            },
            Condition {
                condition_type: ConditionType::PlayerCondition,
                condition_value1: 970,
                ..Condition::default()
            },
            Condition {
                condition_type: ConditionType::NearCreature,
                condition_value1: 980,
                ..Condition::default()
            },
            Condition {
                condition_type: ConditionType::ObjectEntryGuid,
                condition_value1: TypeId::Unit as u32,
                condition_value2: 980,
                condition_value3: 1000,
                ..Condition::default()
            },
            Condition {
                condition_type: ConditionType::NearGameObject,
                condition_value1: 990,
                ..Condition::default()
            },
            Condition {
                condition_type: ConditionType::ObjectEntryGuid,
                condition_value1: TypeId::GameObject as u32,
                condition_value2: 990,
                condition_value3: 1001,
                ..Condition::default()
            },
            Condition {
                condition_type: ConditionType::ActiveEvent,
                condition_value1: 1100,
                ..Condition::default()
            },
            Condition {
                condition_type: ConditionType::WorldState,
                condition_value1: 1200,
                ..Condition::default()
            },
        ] {
            assert_eq!(
                validate_condition_type_external_like_cpp(&condition, stores),
                Ok(())
            );
        }

        assert!(matches!(
            validate_condition_type_external_like_cpp(
                &Condition {
                    condition_type: ConditionType::ItemEquipped,
                    condition_value1: 999,
                    ..Condition::default()
                },
                stores
            ),
            Err(ConditionTypeValidationErrorLikeCpp::NonExistingItem { item_id: 999, .. })
        ));
        assert!(matches!(
            validate_condition_type_external_like_cpp(
                &Condition {
                    condition_type: ConditionType::ZoneId,
                    condition_value1: 999,
                    ..Condition::default()
                },
                stores
            ),
            Err(ConditionTypeValidationErrorLikeCpp::NonExistingArea {
                condition_type: ConditionType::ZoneId,
                area_id: 999
            })
        ));
        assert_eq!(
            validate_condition_type_external_like_cpp(
                &Condition {
                    condition_type: ConditionType::AreaId,
                    condition_value1: 301,
                    ..Condition::default()
                },
                stores
            ),
            Ok(())
        );
        assert!(matches!(
            validate_condition_type_external_like_cpp(
                &Condition {
                    condition_type: ConditionType::ZoneId,
                    condition_value1: 301,
                    ..Condition::default()
                },
                stores
            ),
            Err(ConditionTypeValidationErrorLikeCpp::ZoneIdUsesSubzone(301))
        ));
        assert!(matches!(
            validate_condition_type_external_like_cpp(
                &Condition {
                    condition_type: ConditionType::Skill,
                    condition_value1: 400,
                    condition_value2: 451,
                    ..Condition::default()
                },
                stores
            ),
            Err(
                ConditionTypeValidationErrorLikeCpp::SkillValueAboveConfigMax {
                    value: 451,
                    max: 450,
                    ..
                }
            )
        ));
        assert!(matches!(
            validate_condition_type_external_like_cpp(
                &Condition {
                    condition_type: ConditionType::QuestComplete,
                    condition_value1: 999,
                    ..Condition::default()
                },
                stores
            ),
            Err(ConditionTypeValidationErrorLikeCpp::NonExistingQuest { quest_id: 999, .. })
        ));
        assert!(matches!(
            validate_condition_type_external_like_cpp(
                &Condition {
                    condition_type: ConditionType::QuestObjectiveProgress,
                    condition_value1: 999,
                    ..Condition::default()
                },
                stores
            ),
            Err(ConditionTypeValidationErrorLikeCpp::NonExistingQuestObjective(999))
        ));
        assert!(matches!(
            validate_condition_type_external_like_cpp(
                &Condition {
                    condition_type: ConditionType::QuestObjectiveProgress,
                    condition_value1: 801,
                    condition_value3: 2,
                    ..Condition::default()
                },
                stores
            ),
            Err(
                ConditionTypeValidationErrorLikeCpp::QuestObjectiveCountAboveLimit {
                    objective_id: 801,
                    count: 2,
                    limit: 1
                }
            )
        ));
        assert!(matches!(
            validate_condition_type_external_like_cpp(
                &Condition {
                    condition_type: ConditionType::DifficultyId,
                    condition_value1: 999,
                    ..Condition::default()
                },
                stores
            ),
            Err(ConditionTypeValidationErrorLikeCpp::NonExistingDifficulty(
                999
            ))
        ));
        assert!(matches!(
            validate_condition_type_external_like_cpp(
                &Condition {
                    condition_type: ConditionType::ReputationRank,
                    condition_value1: 999,
                    ..Condition::default()
                },
                stores
            ),
            Err(ConditionTypeValidationErrorLikeCpp::NonExistingFaction(999))
        ));
        assert!(matches!(
            validate_condition_type_external_like_cpp(
                &Condition {
                    condition_type: ConditionType::RealmAchievement,
                    condition_value1: 999,
                    ..Condition::default()
                },
                stores
            ),
            Err(
                ConditionTypeValidationErrorLikeCpp::NonExistingAchievement {
                    condition_type: ConditionType::RealmAchievement,
                    achievement_id: 999
                }
            )
        ));
        assert!(matches!(
            validate_condition_type_external_like_cpp(
                &Condition {
                    condition_type: ConditionType::Title,
                    condition_value1: 999,
                    ..Condition::default()
                },
                stores
            ),
            Err(ConditionTypeValidationErrorLikeCpp::NonExistingTitle(999))
        ));
        assert!(matches!(
            validate_condition_type_external_like_cpp(
                &Condition {
                    condition_type: ConditionType::BattlePetCount,
                    condition_value1: 999,
                    ..Condition::default()
                },
                stores
            ),
            Err(ConditionTypeValidationErrorLikeCpp::NonExistingBattlePetSpecies(999))
        ));
        assert!(matches!(
            validate_condition_type_external_like_cpp(
                &Condition {
                    condition_type: ConditionType::ScenarioStep,
                    condition_value1: 999,
                    ..Condition::default()
                },
                stores
            ),
            Err(ConditionTypeValidationErrorLikeCpp::NonExistingScenarioStep(999))
        ));
        assert!(matches!(
            validate_condition_type_external_like_cpp(
                &Condition {
                    condition_type: ConditionType::SceneInProgress,
                    condition_value1: 999,
                    ..Condition::default()
                },
                stores
            ),
            Err(ConditionTypeValidationErrorLikeCpp::NonExistingSceneScriptPackage(999))
        ));
        assert!(matches!(
            validate_condition_type_external_like_cpp(
                &Condition {
                    condition_type: ConditionType::PlayerCondition,
                    condition_value1: 999,
                    ..Condition::default()
                },
                stores
            ),
            Err(ConditionTypeValidationErrorLikeCpp::NonExistingPlayerCondition(999))
        ));
        assert!(matches!(
            validate_condition_type_external_like_cpp(
                &Condition {
                    condition_type: ConditionType::NearCreature,
                    condition_value1: 999,
                    ..Condition::default()
                },
                stores
            ),
            Err(
                ConditionTypeValidationErrorLikeCpp::NonExistingCreatureTemplate {
                    condition_type: ConditionType::NearCreature,
                    entry: 999
                }
            )
        ));
        assert!(matches!(
            validate_condition_type_external_like_cpp(
                &Condition {
                    condition_type: ConditionType::ObjectEntryGuid,
                    condition_value1: TypeId::GameObject as u32,
                    condition_value2: 999,
                    ..Condition::default()
                },
                stores
            ),
            Err(
                ConditionTypeValidationErrorLikeCpp::NonExistingGameObjectTemplate {
                    condition_type: ConditionType::ObjectEntryGuid,
                    entry: 999
                }
            )
        ));
        assert!(matches!(
            validate_condition_type_external_like_cpp(
                &Condition {
                    condition_type: ConditionType::ObjectEntryGuid,
                    condition_value1: TypeId::Unit as u32,
                    condition_value3: 9999,
                    ..Condition::default()
                },
                stores
            ),
            Err(ConditionTypeValidationErrorLikeCpp::NonExistingCreatureGuid(9999))
        ));
        assert!(matches!(
            validate_condition_type_external_like_cpp(
                &Condition {
                    condition_type: ConditionType::ObjectEntryGuid,
                    condition_value1: TypeId::Unit as u32,
                    condition_value2: 981,
                    condition_value3: 1000,
                    ..Condition::default()
                },
                stores
            ),
            Err(
                ConditionTypeValidationErrorLikeCpp::CreatureGuidEntryMismatch {
                    guid: 1000,
                    expected_entry: 981,
                    actual_entry: 980
                }
            )
        ));
        assert!(matches!(
            validate_condition_type_external_like_cpp(
                &Condition {
                    condition_type: ConditionType::ObjectEntryGuid,
                    condition_value1: TypeId::GameObject as u32,
                    condition_value3: 9999,
                    ..Condition::default()
                },
                stores
            ),
            Err(ConditionTypeValidationErrorLikeCpp::NonExistingGameObjectGuid(9999))
        ));
        assert!(matches!(
            validate_condition_type_external_like_cpp(
                &Condition {
                    condition_type: ConditionType::ObjectEntryGuid,
                    condition_value1: TypeId::GameObject as u32,
                    condition_value2: 991,
                    condition_value3: 1001,
                    ..Condition::default()
                },
                stores
            ),
            Err(
                ConditionTypeValidationErrorLikeCpp::GameObjectGuidEntryMismatch {
                    guid: 1001,
                    expected_entry: 991,
                    actual_entry: 990
                }
            )
        ));
        assert!(matches!(
            validate_condition_type_external_like_cpp(
                &Condition {
                    condition_type: ConditionType::ActiveEvent,
                    condition_value1: 999,
                    ..Condition::default()
                },
                stores
            ),
            Err(ConditionTypeValidationErrorLikeCpp::NonExistingActiveEvent(
                999
            ))
        ));
        assert!(matches!(
            validate_condition_type_external_like_cpp(
                &Condition {
                    condition_type: ConditionType::WorldState,
                    condition_value1: 999,
                    ..Condition::default()
                },
                stores
            ),
            Err(ConditionTypeValidationErrorLikeCpp::NonExistingWorldState(
                999
            ))
        ));
    }

    #[test]
    fn condition_external_source_validation_uses_loaded_stores_like_cpp() {
        let item_store = crate::ItemStore::from_records([crate::ItemRecord {
            id: 100,
            class_id: 0,
            subclass_id: 0,
            material: 0,
            inventory_type: 0,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
        }]);
        let area_store = crate::AreaTableStore::from_entries([
            crate::AreaTableEntry {
                id: 7,
                continent_id: 0,
                parent_area_id: 0,
                mount_flags: 0,
                flags: 0,
            },
            crate::AreaTableEntry {
                id: 8,
                continent_id: 0,
                parent_area_id: 7,
                mount_flags: 0,
                flags: crate::area::AREA_FLAG_IS_SUBZONE_LIKE_CPP,
            },
        ]);
        let map_store = crate::MapStore::from_entries([crate::MapEntry {
            id: 571,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            instance_type: 0,
            flags1: 0,
        }]);
        let quest_store =
            crate::quest::QuestStore::from_quests_like_cpp([quest_template(700, Vec::new())]);
        let mut spell_store = crate::SpellStore::new();
        spell_store.insert(200, spell_info(200));
        spell_store.insert(
            201,
            crate::SpellInfo {
                spell_id: 201,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: 0,
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                aura_type: None,
                display_flags: 0,
                effects: vec![
                    crate::SpellEffectInfo {
                        effect_index: 0,
                        effect: 0,
                        chain_targets: 0,
                        implicit_target_1: 6,
                        implicit_target_2: 0,
                        ..Default::default()
                    },
                    crate::SpellEffectInfo {
                        effect_index: 1,
                        effect: 0,
                        chain_targets: 0,
                        implicit_target_1: 7,
                        implicit_target_2: 0,
                        ..Default::default()
                    },
                ],
            },
        );
        let mut area_trigger_store = crate::AreaTriggerStore::new();
        area_trigger_store.insert(crate::AreaTriggerData {
            trigger_id: 300,
            map_id: 571,
            pos: wow_core::Position::ZERO,
            shape: crate::TriggerShape::Sphere,
            radius: 1.0,
            extents: [0.0; 3],
            height: 0.0,
            yaw: 0.0,
            vertices: Vec::new(),
            teleport: None,
        });
        let mut graveyard_store = crate::GraveyardStore::default();
        graveyard_store.load_graveyard_zones_from_rows_like_cpp(
            [crate::GraveyardZoneRow {
                safe_loc_id: 400,
                ghost_zone_id: 7,
            }],
            |_| true,
            |_| true,
        );
        let (spawn_group_store, _) = crate::SpawnGroupTemplateStore::from_rows_like_cpp([
            crate::SpawnGroupTemplateRow {
                group_id: 500,
                name: "manual".to_string(),
                flags: 0,
            },
            crate::SpawnGroupTemplateRow {
                group_id: 501,
                name: "system".to_string(),
                flags: crate::spawn_group::SPAWN_GROUP_FLAG_SYSTEM_LIKE_CPP,
            },
        ]);
        let creature_template_store = crate::WorldIdStore::from_ids("creature_template", [600]);
        let gameobject_template_store = crate::WorldIdStore::from_ids("gameobject_template", [601]);
        let trainer_store = crate::WorldIdStore::from_ids("trainer", [700]);
        let conversation_line_template_store =
            crate::WorldIdStore::from_ids("conversation_line_template", [800]);
        let area_trigger_template_store =
            crate::AreaTriggerTemplateStore::from_keys([(900, false), (901, true)]);
        let loot_template_exists = |source_type: ConditionSourceType, source_group: u32| {
            source_type == ConditionSourceType::CreatureLootTemplate && source_group == 123
        };
        let loot_source_entry_exists =
            |source_type: ConditionSourceType, source_group: u32, source_entry: i32| {
                source_type == ConditionSourceType::CreatureLootTemplate
                    && source_group == 123
                    && (source_entry == 456 || source_entry == 900)
            };
        let stores = ConditionExternalValidationStoresLikeCpp {
            item_store: Some(&item_store),
            area_table_store: Some(&area_store),
            map_store: Some(&map_store),
            quest_store: Some(&quest_store),
            spell_store: Some(&spell_store),
            area_trigger_store: Some(&area_trigger_store),
            graveyard_store: Some(&graveyard_store),
            spawn_group_store: Some(&spawn_group_store),
            creature_template_store: Some(&creature_template_store),
            gameobject_template_store: Some(&gameobject_template_store),
            trainer_store: Some(&trainer_store),
            conversation_line_template_store: Some(&conversation_line_template_store),
            area_trigger_template_store: Some(&area_trigger_template_store),
            loot_template_exists: Some(&loot_template_exists),
            loot_source_entry_exists: Some(&loot_source_entry_exists),
            ..ConditionExternalValidationStoresLikeCpp::default()
        };

        assert_eq!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::QuestAvailable,
                    source_entry: 700,
                    ..Condition::default()
                },
                stores
            ),
            Ok(())
        );
        assert!(matches!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::QuestAvailable,
                    source_entry: 999,
                    ..Condition::default()
                },
                stores
            ),
            Err(ConditionSourceValidationErrorLikeCpp::NonExistingQuestAvailable(999))
        ));
        assert_eq!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::NpcVendor,
                    source_group: 600,
                    source_entry: 100,
                    ..Condition::default()
                },
                stores
            ),
            Ok(())
        );
        assert!(matches!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::NpcVendor,
                    source_group: 600,
                    source_entry: 999,
                    ..Condition::default()
                },
                stores
            ),
            Err(ConditionSourceValidationErrorLikeCpp::NonExistingNpcVendorItem(999))
        ));
        assert_eq!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::Spell,
                    source_entry: 200,
                    ..Condition::default()
                },
                stores
            ),
            Ok(())
        );
        assert_eq!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::GossipMenu,
                    source_group: 800,
                    source_entry: 900,
                    ..Condition::default()
                },
                stores
            ),
            Ok(())
        );
        assert_eq!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::GossipMenuOption,
                    source_group: 800,
                    source_entry: 1,
                    ..Condition::default()
                },
                stores
            ),
            Ok(())
        );
        let mut spell_implicit = Condition {
            source_type: ConditionSourceType::SpellImplicitTarget,
            source_group: 0b11,
            source_entry: 201,
            ..Condition::default()
        };
        assert_eq!(
            validate_and_normalize_condition_source_external_like_cpp(&mut spell_implicit, stores),
            Ok(())
        );
        assert_eq!(spell_implicit.source_group, 0b10);
        assert!(matches!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::SpellImplicitTarget,
                    source_group: 0b1,
                    source_entry: 201,
                    ..Condition::default()
                },
                stores
            ),
            Err(ConditionSourceValidationErrorLikeCpp::InvalidSpellImplicitTargetEffectMask(1))
        ));
        assert_eq!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::SpellClickEvent,
                    source_group: 600,
                    source_entry: 200,
                    ..Condition::default()
                },
                stores
            ),
            Ok(())
        );
        assert!(matches!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::SpellClickEvent,
                    source_group: 600,
                    source_entry: 999,
                    ..Condition::default()
                },
                stores
            ),
            Err(
                ConditionSourceValidationErrorLikeCpp::NonExistingSourceSpell {
                    source_type: ConditionSourceType::SpellClickEvent,
                    spell_id: 999
                }
            )
        ));
        assert_eq!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::TrainerSpell,
                    source_group: 700,
                    source_entry: 200,
                    ..Condition::default()
                },
                stores
            ),
            Ok(())
        );
        assert!(matches!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::TrainerSpell,
                    source_group: 999,
                    source_entry: 200,
                    ..Condition::default()
                },
                stores
            ),
            Err(ConditionSourceValidationErrorLikeCpp::NonExistingTrainer(
                999
            ))
        ));
        assert!(matches!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::TrainerSpell,
                    source_group: 700,
                    source_entry: 999,
                    ..Condition::default()
                },
                stores
            ),
            Err(
                ConditionSourceValidationErrorLikeCpp::NonExistingSourceSpell {
                    source_type: ConditionSourceType::TrainerSpell,
                    spell_id: 999
                }
            )
        ));
        assert!(matches!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::VehicleSpell,
                    source_group: 999,
                    source_entry: 200,
                    ..Condition::default()
                },
                stores
            ),
            Err(
                ConditionSourceValidationErrorLikeCpp::NonExistingSourceCreatureTemplate {
                    source_type: ConditionSourceType::VehicleSpell,
                    entry: 999
                }
            )
        ));
        assert_eq!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::ConversationLine,
                    source_entry: 800,
                    ..Condition::default()
                },
                stores
            ),
            Ok(())
        );
        assert!(matches!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::ConversationLine,
                    source_entry: 999,
                    ..Condition::default()
                },
                stores
            ),
            Err(ConditionSourceValidationErrorLikeCpp::NonExistingConversationLineTemplate(999))
        ));
        assert_eq!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::AreaTrigger,
                    source_group: 900,
                    source_entry: 0,
                    ..Condition::default()
                },
                stores
            ),
            Ok(())
        );
        assert_eq!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::AreaTrigger,
                    source_group: 901,
                    source_entry: 1,
                    ..Condition::default()
                },
                stores
            ),
            Ok(())
        );
        assert!(matches!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::AreaTrigger,
                    source_group: 900,
                    source_entry: 1,
                    ..Condition::default()
                },
                stores
            ),
            Err(
                ConditionSourceValidationErrorLikeCpp::NonExistingAreaTriggerTemplate {
                    id: 900,
                    is_custom: true
                }
            )
        ));
        assert_eq!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::CreatureTemplateVehicle,
                    source_entry: 600,
                    ..Condition::default()
                },
                stores
            ),
            Ok(())
        );
        assert!(matches!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::CreatureTemplateVehicle,
                    source_entry: 999,
                    ..Condition::default()
                },
                stores
            ),
            Err(
                ConditionSourceValidationErrorLikeCpp::NonExistingSourceCreatureTemplate {
                    source_type: ConditionSourceType::CreatureTemplateVehicle,
                    entry: 999
                }
            )
        ));
        assert_eq!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::AreaTriggerClientTriggered,
                    source_entry: 300,
                    ..Condition::default()
                },
                stores
            ),
            Ok(())
        );
        assert!(matches!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::AreaTriggerClientTriggered,
                    source_entry: 999,
                    ..Condition::default()
                },
                stores
            ),
            Err(ConditionSourceValidationErrorLikeCpp::NonExistingClientAreaTrigger(999))
        ));
        assert_eq!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::Graveyard,
                    source_group: 7,
                    source_entry: 400,
                    ..Condition::default()
                },
                stores
            ),
            Ok(())
        );
        assert!(matches!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::Graveyard,
                    source_group: 7,
                    source_entry: 999,
                    ..Condition::default()
                },
                stores
            ),
            Err(
                ConditionSourceValidationErrorLikeCpp::NonExistingGraveyard {
                    safe_loc_id: 999,
                    zone_id: 7
                }
            )
        ));
        assert_eq!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::SpawnGroup,
                    source_entry: 500,
                    ..Condition::default()
                },
                stores
            ),
            Ok(())
        );
        assert!(matches!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::SpawnGroup,
                    source_entry: 999,
                    ..Condition::default()
                },
                stores
            ),
            Err(ConditionSourceValidationErrorLikeCpp::NonExistingSpawnGroup(999))
        ));
        assert!(matches!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::SpawnGroup,
                    source_entry: 501,
                    ..Condition::default()
                },
                stores
            ),
            Err(ConditionSourceValidationErrorLikeCpp::SystemSpawnGroup(501))
        ));
        assert_eq!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::ObjectIdVisibility,
                    source_group: TypeId::Unit as u32,
                    source_entry: 600,
                    ..Condition::default()
                },
                stores
            ),
            Ok(())
        );
        assert_eq!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::ObjectIdVisibility,
                    source_group: TypeId::GameObject as u32,
                    source_entry: 601,
                    ..Condition::default()
                },
                stores
            ),
            Ok(())
        );
        assert!(matches!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::ObjectIdVisibility,
                    source_group: TypeId::GameObject as u32,
                    source_entry: 999,
                    ..Condition::default()
                },
                stores
            ),
            Err(
                ConditionSourceValidationErrorLikeCpp::NonExistingSourceGameObjectTemplate {
                    source_type: ConditionSourceType::ObjectIdVisibility,
                    entry: 999
                }
            )
        ));
        assert_eq!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::CreatureLootTemplate,
                    source_group: 123,
                    source_entry: 456,
                    ..Condition::default()
                },
                stores
            ),
            Ok(())
        );
        assert!(matches!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::CreatureLootTemplate,
                    source_group: 999,
                    source_entry: 456,
                    ..Condition::default()
                },
                stores
            ),
            Err(
                ConditionSourceValidationErrorLikeCpp::NonExistingLootTemplate {
                    source_type: ConditionSourceType::CreatureLootTemplate,
                    source_group: 999
                }
            )
        ));
        assert!(matches!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::CreatureLootTemplate,
                    source_group: 123,
                    source_entry: 777,
                    ..Condition::default()
                },
                stores
            ),
            Err(
                ConditionSourceValidationErrorLikeCpp::NonExistingLootSourceEntry {
                    source_type: ConditionSourceType::CreatureLootTemplate,
                    source_group: 123,
                    source_entry: 777
                }
            )
        ));
        assert_eq!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::TerrainSwap,
                    source_entry: 571,
                    ..Condition::default()
                },
                stores
            ),
            Ok(())
        );
        assert!(matches!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::TerrainSwap,
                    source_entry: 999,
                    ..Condition::default()
                },
                stores
            ),
            Err(ConditionSourceValidationErrorLikeCpp::NonExistingTerrainSwapMap(999))
        ));
        assert!(matches!(
            validate_condition_source_external_like_cpp(
                &Condition {
                    source_type: ConditionSourceType::Phase,
                    source_entry: 99,
                    ..Condition::default()
                },
                stores
            ),
            Err(ConditionSourceValidationErrorLikeCpp::NonExistingPhaseArea(
                99
            ))
        ));
    }

    #[test]
    fn parse_condition_rows_applies_static_condition_type_validation_like_cpp() {
        let max_condition_type = ConditionDbRowLikeCpp {
            condition_type_or_reference: ConditionType::Max as i32,
            ..condition_row(ConditionSourceType::SpellClickEvent, ConditionType::None)
        };
        let mut deprecated = condition_row(
            ConditionSourceType::SpellClickEvent,
            ConditionType::SpawnMaskDeprecated,
        );
        deprecated.condition_target = 1;
        let mut legacy = condition_row(
            ConditionSourceType::SpellClickEvent,
            ConditionType::TypeMaskLegacy,
        );
        legacy.condition_target = 1;
        legacy.condition_value1 = 1 << 4;

        let report = parse_condition_rows_like_cpp([max_condition_type, deprecated, legacy], |_| 0);

        assert_eq!(report.conditions.len(), 1);
        assert_eq!(report.conditions[0].condition_type, ConditionType::TypeMask);
        assert_eq!(
            report.conditions[0].condition_value1,
            TypeMask::PLAYER.bits()
        );
        assert_eq!(
            report.skipped[0].reason,
            ConditionRowSkipReason::InvalidConditionType(ConditionType::Max as i32)
        );
        assert_eq!(
            report.skipped[1].reason,
            ConditionRowSkipReason::ConditionTypeValidationFailed(
                ConditionTypeValidationErrorLikeCpp::DeprecatedSpawnMask
            )
        );
    }

    #[test]
    fn condition_source_static_validation_matches_cpp_pure_rejections() {
        let mut spell_implicit = condition_row(
            ConditionSourceType::SpellImplicitTarget,
            ConditionType::Aura,
        );
        spell_implicit.source_group = 0;
        let mut area_trigger = condition_row(ConditionSourceType::AreaTrigger, ConditionType::Aura);
        area_trigger.source_group = 77;
        area_trigger.source_entry = 2;
        let mut object_visibility =
            condition_row(ConditionSourceType::ObjectIdVisibility, ConditionType::Aura);
        object_visibility.source_group = TypeId::Player as u32;

        let report =
            parse_condition_rows_like_cpp([spell_implicit, area_trigger, object_visibility], |_| 0);

        assert_eq!(report.conditions.len(), 0);
        assert_eq!(
            report.skipped[0].reason,
            ConditionRowSkipReason::ConditionSourceValidationFailed(
                ConditionSourceValidationErrorLikeCpp::InvalidSpellImplicitTargetEffectMask(0)
            )
        );
        assert_eq!(
            report.skipped[1].reason,
            ConditionRowSkipReason::ConditionSourceValidationFailed(
                ConditionSourceValidationErrorLikeCpp::InvalidAreaTriggerSourceEntry(2)
            )
        );
        assert_eq!(
            report.skipped[2].reason,
            ConditionRowSkipReason::ConditionSourceValidationFailed(
                ConditionSourceValidationErrorLikeCpp::UncheckedObjectIdVisibilityObjectType(
                    TypeId::Player as u32,
                )
            )
        );
    }

    #[test]
    fn condition_source_static_validation_keeps_reference_templates_internal_like_cpp() {
        let mut positive_internal =
            condition_row(ConditionSourceType::ReferenceCondition, ConditionType::Aura);
        positive_internal.source_group = 7;
        let mut negative_template = condition_row(ConditionSourceType::Phase, ConditionType::Aura);
        negative_template.source_type_or_reference_id = -7;
        negative_template.source_group = 0;

        let report = parse_condition_rows_like_cpp([positive_internal, negative_template], |_| 0);

        assert_eq!(report.conditions.len(), 1);
        assert_eq!(
            report.conditions[0].source_type,
            ConditionSourceType::ReferenceCondition
        );
        assert_eq!(report.conditions[0].source_group, 7);
        assert_eq!(
            report.skipped[0].reason,
            ConditionRowSkipReason::ConditionSourceValidationFailed(
                ConditionSourceValidationErrorLikeCpp::InvalidSourceType(
                    ConditionSourceType::ReferenceCondition,
                )
            )
        );
    }

    #[test]
    fn condition_searcher_type_mask_matches_cpp_direct_cases() {
        assert_eq!(
            Condition {
                condition_type: ConditionType::None,
                ..Condition::default()
            }
            .get_searcher_type_mask_for_condition_like_cpp(),
            GRID_MAP_TYPE_MASK_ALL
        );
        assert_eq!(
            Condition {
                condition_type: ConditionType::Aura,
                ..Condition::default()
            }
            .get_searcher_type_mask_for_condition_like_cpp(),
            GRID_MAP_TYPE_MASK_CREATURE | GRID_MAP_TYPE_MASK_PLAYER
        );
        assert_eq!(
            Condition {
                condition_type: ConditionType::Item,
                ..Condition::default()
            }
            .get_searcher_type_mask_for_condition_like_cpp(),
            GRID_MAP_TYPE_MASK_PLAYER
        );
        assert_eq!(
            Condition {
                condition_type: ConditionType::CreatureType,
                ..Condition::default()
            }
            .get_searcher_type_mask_for_condition_like_cpp(),
            GRID_MAP_TYPE_MASK_CREATURE
        );
        assert_eq!(
            Condition {
                condition_type: ConditionType::PrivateObject,
                ..Condition::default()
            }
            .get_searcher_type_mask_for_condition_like_cpp(),
            GRID_MAP_TYPE_MASK_ALL & !GRID_MAP_TYPE_MASK_PLAYER
        );
        assert_eq!(
            Condition {
                condition_type: ConditionType::Team,
                negative_condition: true,
                ..Condition::default()
            }
            .get_searcher_type_mask_for_condition_like_cpp(),
            GRID_MAP_TYPE_MASK_ALL
        );
    }

    #[test]
    fn condition_searcher_type_mask_object_entry_and_type_mask_match_cpp() {
        assert_eq!(
            Condition {
                condition_type: ConditionType::ObjectEntryGuid,
                condition_value1: TypeId::Unit as u32,
                ..Condition::default()
            }
            .get_searcher_type_mask_for_condition_like_cpp(),
            GRID_MAP_TYPE_MASK_CREATURE
        );
        assert_eq!(
            Condition {
                condition_type: ConditionType::ObjectEntryGuid,
                condition_value1: TypeId::Player as u32,
                ..Condition::default()
            }
            .get_searcher_type_mask_for_condition_like_cpp(),
            GRID_MAP_TYPE_MASK_PLAYER
        );
        assert_eq!(
            Condition {
                condition_type: ConditionType::ObjectEntryGuid,
                condition_value1: TypeId::GameObject as u32,
                ..Condition::default()
            }
            .get_searcher_type_mask_for_condition_like_cpp(),
            GRID_MAP_TYPE_MASK_GAME_OBJECT
        );
        assert_eq!(
            Condition {
                condition_type: ConditionType::TypeMask,
                condition_value1: (TypeMask::UNIT | TypeMask::GAME_OBJECT).bits(),
                ..Condition::default()
            }
            .get_searcher_type_mask_for_condition_like_cpp(),
            GRID_MAP_TYPE_MASK_CREATURE
                | GRID_MAP_TYPE_MASK_PLAYER
                | GRID_MAP_TYPE_MASK_GAME_OBJECT
        );
    }

    #[test]
    fn condition_searcher_type_mask_list_ands_groups_ors_else_groups_and_expands_refs() {
        let reference_condition = Condition {
            source_type: ConditionSourceType::ReferenceCondition,
            source_group: 77,
            condition_type: ConditionType::Item,
            ..Condition::default()
        };
        let store = ConditionEntriesByTypeStore::from_conditions_like_cpp([reference_condition]);
        let conditions = vec![
            Condition {
                else_group: 0,
                condition_type: ConditionType::Aura,
                ..Condition::default()
            },
            Condition {
                else_group: 0,
                reference_id: 77,
                ..Condition::default()
            },
            Condition {
                else_group: 1,
                condition_type: ConditionType::CreatureType,
                ..Condition::default()
            },
        ];

        assert_eq!(
            store.get_searcher_type_mask_for_condition_list_like_cpp(&conditions),
            GRID_MAP_TYPE_MASK_PLAYER | GRID_MAP_TYPE_MASK_CREATURE
        );
    }

    #[test]
    fn condition_id_matches_cpp_key_fields() {
        let condition = Condition {
            source_group: 12,
            source_entry: -45,
            source_id: 67,
            ..Condition::default()
        };

        assert_eq!(condition.id_like_cpp(), ConditionId::new(12, -45, 67));
    }

    #[test]
    fn parse_condition_row_maps_positive_source_and_type_like_cpp() {
        let condition = parse_condition_row_like_cpp(
            ConditionDbRowLikeCpp {
                source_type_or_reference_id: ConditionSourceType::Phase as i32,
                source_group: 12,
                source_entry: 34,
                source_id: 0,
                else_group: 2,
                condition_type_or_reference: ConditionType::Aura as i32,
                condition_target: 1,
                condition_value1: 100,
                condition_value2: 2,
                condition_value3: 3,
                condition_string_value1: String::from("string"),
                negative_condition: true,
                error_type: 4,
                error_text_id: 5,
                script_name: String::from("script"),
            },
            |name| u32::from(name == "script") * 77,
        )
        .unwrap();

        assert_eq!(condition.source_type, ConditionSourceType::Phase);
        assert_eq!(condition.condition_type, ConditionType::Aura);
        assert_eq!(condition.source_group, 12);
        assert_eq!(condition.source_entry, 34);
        assert_eq!(condition.else_group, 2);
        assert_eq!(condition.condition_target, 1);
        assert_eq!(condition.condition_value1, 100);
        assert_eq!(condition.condition_value2, 2);
        assert_eq!(condition.condition_value3, 3);
        assert_eq!(condition.condition_string_value1, "string");
        assert!(condition.negative_condition);
        assert_eq!(condition.error_type, 4);
        assert_eq!(condition.error_text_id, 5);
        assert_eq!(condition.script_id, 77);
    }

    #[test]
    fn parse_condition_row_maps_reference_template_like_cpp() {
        let condition = parse_condition_row_like_cpp(
            ConditionDbRowLikeCpp {
                source_type_or_reference_id: -500,
                source_group: 9,
                source_entry: 8,
                source_id: 7,
                else_group: 0,
                condition_type_or_reference: -(ConditionType::Aura as i32),
                condition_target: 0,
                condition_value1: 0,
                condition_value2: 0,
                condition_value3: 0,
                condition_string_value1: String::new(),
                negative_condition: false,
                error_type: 0,
                error_text_id: 0,
                script_name: String::new(),
            },
            |_| 0,
        )
        .unwrap();

        assert_eq!(
            condition.source_type,
            ConditionSourceType::ReferenceCondition
        );
        assert_eq!(condition.source_group, 500);
        assert_eq!(condition.source_entry, 8);
        assert_eq!(condition.source_id, 7);
        assert_eq!(condition.reference_id, ConditionType::Aura as u32);
        assert_eq!(condition.condition_type, ConditionType::None);
    }

    #[test]
    fn parse_condition_rows_records_cpp_reference_useless_data_warnings_without_skipping() {
        let row = ConditionDbRowLikeCpp {
            source_type_or_reference_id: ConditionSourceType::Phase as i32,
            source_group: 0,
            source_entry: 0,
            source_id: 0,
            else_group: 0,
            condition_type_or_reference: -42,
            condition_target: 1,
            condition_value1: 10,
            condition_value2: 20,
            condition_value3: 30,
            condition_string_value1: String::new(),
            negative_condition: true,
            error_type: 0,
            error_text_id: 0,
            script_name: String::new(),
        };

        let report = parse_condition_rows_like_cpp([row], |_| 0);

        assert_eq!(report.conditions.len(), 1);
        assert!(report.skipped.is_empty());
        assert_eq!(
            report.warnings,
            vec![
                ConditionLoadWarningLikeCpp::ReferenceUselessConditionTarget {
                    source_type_or_reference_id: ConditionSourceType::Phase as i32,
                    condition_target: 1,
                },
                ConditionLoadWarningLikeCpp::ReferenceUselessValue {
                    source_type_or_reference_id: ConditionSourceType::Phase as i32,
                    field: 1,
                    value: 10,
                },
                ConditionLoadWarningLikeCpp::ReferenceUselessValue {
                    source_type_or_reference_id: ConditionSourceType::Phase as i32,
                    field: 2,
                    value: 20,
                },
                ConditionLoadWarningLikeCpp::ReferenceUselessValue {
                    source_type_or_reference_id: ConditionSourceType::Phase as i32,
                    field: 3,
                    value: 30,
                },
                ConditionLoadWarningLikeCpp::ReferenceUselessNegativeCondition {
                    source_type_or_reference_id: ConditionSourceType::Phase as i32,
                },
            ]
        );
    }

    #[test]
    fn parse_condition_rows_records_cpp_reference_template_useless_data_warnings() {
        let row = ConditionDbRowLikeCpp {
            source_type_or_reference_id: -500,
            source_group: 9,
            source_entry: 8,
            source_id: 7,
            else_group: 0,
            condition_type_or_reference: ConditionType::Aura as i32,
            condition_target: 0,
            condition_value1: 0,
            condition_value2: 0,
            condition_value3: 0,
            condition_string_value1: String::new(),
            negative_condition: false,
            error_type: 0,
            error_text_id: 0,
            script_name: String::new(),
        };

        let report = parse_condition_rows_like_cpp([row], |_| 0);

        assert_eq!(
            report.warnings,
            vec![
                ConditionLoadWarningLikeCpp::ReferenceTemplateUselessSourceGroup {
                    source_type_or_reference_id: -500,
                    source_group: 9,
                },
                ConditionLoadWarningLikeCpp::ReferenceTemplateUselessSourceEntry {
                    source_type_or_reference_id: -500,
                    source_entry: 8,
                },
                ConditionLoadWarningLikeCpp::ReferenceTemplateUselessSourceId {
                    source_type_or_reference_id: -500,
                    source_id: 7,
                },
            ]
        );
        assert_eq!(
            report.skipped[0].reason,
            ConditionRowSkipReason::SourceIdNotAllowed {
                source_type: ConditionSourceType::ReferenceCondition,
                source_id: 7,
            }
        );
    }

    #[test]
    fn parse_condition_row_skips_self_reference_like_cpp() {
        let row = ConditionDbRowLikeCpp {
            source_type_or_reference_id: -42,
            source_group: 0,
            source_entry: 0,
            source_id: 0,
            else_group: 0,
            condition_type_or_reference: -42,
            condition_target: 0,
            condition_value1: 0,
            condition_value2: 0,
            condition_value3: 0,
            condition_string_value1: String::new(),
            negative_condition: false,
            error_type: 0,
            error_text_id: 0,
            script_name: String::new(),
        };

        let skipped = parse_condition_row_like_cpp(row, |_| 0).unwrap_err();

        assert_eq!(skipped.reason, ConditionRowSkipReason::SelfReference(-42));
    }

    #[test]
    fn parse_condition_rows_applies_cpp_source_group_and_id_shape_validation() {
        let mut invalid_group =
            condition_row(ConditionSourceType::QuestAvailable, ConditionType::Aura);
        invalid_group.source_group = 5;
        let mut invalid_id = condition_row(ConditionSourceType::Phase, ConditionType::Aura);
        invalid_id.source_id = 9;
        let report = parse_condition_rows_like_cpp([invalid_group, invalid_id], |_| 0);

        assert_eq!(report.conditions.len(), 0);
        assert_eq!(
            report.skipped[0].reason,
            ConditionRowSkipReason::SourceGroupNotAllowed {
                source_type: ConditionSourceType::QuestAvailable,
                source_group: 5,
            }
        );
        assert_eq!(
            report.skipped[1].reason,
            ConditionRowSkipReason::SourceIdNotAllowed {
                source_type: ConditionSourceType::Phase,
                source_id: 9,
            }
        );
    }

    #[test]
    fn parse_condition_rows_validates_condition_target_for_non_references_like_cpp() {
        let mut invalid = condition_row(ConditionSourceType::Phase, ConditionType::Aura);
        invalid.condition_target = 1;
        let mut valid = condition_row(ConditionSourceType::SpellClickEvent, ConditionType::Aura);
        valid.condition_target = 1;
        let mut reference = condition_row(ConditionSourceType::Phase, ConditionType::Aura);
        reference.condition_type_or_reference = -77;
        reference.condition_target = 2;

        let report = parse_condition_rows_like_cpp([invalid, valid, reference], |_| 0);

        assert_eq!(report.conditions.len(), 2);
        assert_eq!(
            report.conditions[0].source_type,
            ConditionSourceType::SpellClickEvent
        );
        assert_eq!(report.conditions[1].reference_id, 77);
        assert_eq!(
            report.skipped[0].reason,
            ConditionRowSkipReason::ConditionTargetOutOfRange {
                source_type: ConditionSourceType::Phase,
                condition_target: 1,
                max_available_targets: 1,
            }
        );
    }

    #[test]
    fn parse_condition_rows_records_cpp_useless_condition_value_warnings() {
        let mut alive = condition_row(ConditionSourceType::Phase, ConditionType::Alive);
        alive.condition_value1 = 10;
        alive.condition_value2 = 20;
        alive.condition_value3 = 30;
        alive.condition_string_value1 = "unused".to_string();
        let mut player_guid =
            condition_row(ConditionSourceType::Phase, ConditionType::ObjectEntryGuid);
        player_guid.condition_value1 = TypeId::Player as u32;
        player_guid.condition_value2 = 42;
        player_guid.condition_value3 = 77;

        let report = parse_condition_rows_like_cpp([alive, player_guid], |_| 0);

        assert_eq!(report.conditions.len(), 2);
        assert_eq!(
            report.warnings,
            vec![
                ConditionLoadWarningLikeCpp::UselessConditionValue {
                    condition_type: ConditionType::Alive,
                    field: 1,
                    value: 10,
                },
                ConditionLoadWarningLikeCpp::UselessConditionValue {
                    condition_type: ConditionType::Alive,
                    field: 2,
                    value: 20,
                },
                ConditionLoadWarningLikeCpp::UselessConditionValue {
                    condition_type: ConditionType::Alive,
                    field: 3,
                    value: 30,
                },
                ConditionLoadWarningLikeCpp::UselessConditionStringValue {
                    condition_type: ConditionType::Alive,
                    field: 4,
                    value: "unused".to_string(),
                },
                ConditionLoadWarningLikeCpp::UselessConditionValue {
                    condition_type: ConditionType::ObjectEntryGuid,
                    field: 2,
                    value: 42,
                },
                ConditionLoadWarningLikeCpp::UselessConditionValue {
                    condition_type: ConditionType::ObjectEntryGuid,
                    field: 3,
                    value: 77,
                },
            ]
        );
    }

    #[test]
    fn parse_condition_rows_normalizes_error_fields_like_cpp() {
        let mut non_spell = condition_row(ConditionSourceType::Phase, ConditionType::Aura);
        non_spell.error_type = 7;
        non_spell.error_text_id = 8;
        let mut spell_with_error = condition_row(ConditionSourceType::Spell, ConditionType::Aura);
        spell_with_error.error_type = 7;
        spell_with_error.error_text_id = 8;
        let mut spell_without_error =
            condition_row(ConditionSourceType::Spell, ConditionType::Aura);
        spell_without_error.error_text_id = 8;

        let report = parse_condition_rows_like_cpp(
            [non_spell, spell_with_error, spell_without_error],
            |_| 0,
        );

        assert_eq!(report.conditions.len(), 3);
        assert_eq!(report.conditions[0].error_type, 0);
        assert_eq!(report.conditions[0].error_text_id, 0);
        assert_eq!(report.conditions[1].error_type, 7);
        assert_eq!(report.conditions[1].error_text_id, 8);
        assert_eq!(report.conditions[2].error_type, 0);
        assert_eq!(report.conditions[2].error_text_id, 0);
        assert_eq!(
            report.warnings,
            vec![
                ConditionLoadWarningLikeCpp::ErrorTypeResetForNonSpell {
                    source_type: ConditionSourceType::Phase,
                    error_type: 7,
                },
                ConditionLoadWarningLikeCpp::ErrorTextIdResetWithoutErrorType {
                    source_type: ConditionSourceType::Phase,
                    error_text_id: 8,
                },
                ConditionLoadWarningLikeCpp::ErrorTextIdResetWithoutErrorType {
                    source_type: ConditionSourceType::Spell,
                    error_text_id: 8,
                },
            ]
        );
    }

    #[test]
    fn condition_entries_store_groups_by_source_type_and_id_like_cpp() {
        let first = Condition {
            source_type: ConditionSourceType::Phase,
            source_group: 7,
            source_entry: 20,
            source_id: 0,
            condition_type: ConditionType::Aura,
            condition_value1: 100,
            ..Condition::default()
        };
        let second = Condition {
            condition_value1: 101,
            ..first.clone()
        };
        let other = Condition {
            source_type: ConditionSourceType::TerrainSwap,
            source_group: 7,
            source_entry: 20,
            source_id: 0,
            condition_type: ConditionType::MapId,
            ..Condition::default()
        };

        let store = ConditionEntriesByTypeStore::from_conditions_like_cpp([
            first.clone(),
            second.clone(),
            other,
        ]);

        assert_eq!(store.bucket_count(), 2);
        assert_eq!(store.condition_count(), 3);
        let phase_bucket = store
            .conditions_for_like_cpp(ConditionSourceType::Phase, first.id_like_cpp())
            .unwrap();
        assert_eq!(phase_bucket.as_slice(), &[first, second]);
    }

    #[test]
    fn spell_click_aura_spell_index_matches_cpp_load_builder() {
        let aura_spell_click = Condition {
            source_type: ConditionSourceType::SpellClickEvent,
            source_group: 7,
            source_entry: 20,
            condition_type: ConditionType::Aura,
            condition_value1: 100,
            ..Condition::default()
        };
        let non_aura_spell_click = Condition {
            source_type: ConditionSourceType::SpellClickEvent,
            source_group: 7,
            source_entry: 21,
            condition_type: ConditionType::MapId,
            condition_value1: 571,
            ..Condition::default()
        };
        let aura_other_source = Condition {
            source_type: ConditionSourceType::Spell,
            source_group: 0,
            source_entry: 100,
            condition_type: ConditionType::Aura,
            condition_value1: 200,
            ..Condition::default()
        };
        let store = ConditionEntriesByTypeStore::from_conditions_like_cpp([
            aura_spell_click,
            non_aura_spell_click,
            aura_other_source,
        ]);

        assert_eq!(
            store.spells_used_in_spell_click_conditions_like_cpp(),
            std::collections::HashSet::from([100])
        );
        assert!(store.is_spell_used_in_spell_click_conditions_like_cpp(100));
        assert!(!store.is_spell_used_in_spell_click_conditions_like_cpp(200));
    }

    #[test]
    fn conditions_reference_expires_after_store_reload_like_cpp() {
        let condition = Condition {
            source_type: ConditionSourceType::Phase,
            source_group: 7,
            source_entry: 20,
            condition_type: ConditionType::Aura,
            ..Condition::default()
        };
        let reference = {
            let store = ConditionEntriesByTypeStore::from_conditions_like_cpp([condition.clone()]);
            store
                .reference_for_like_cpp(ConditionSourceType::Phase, condition.id_like_cpp())
                .unwrap()
        };

        assert!(reference.upgrade().is_none());
        assert!(reference.is_expired());
    }
}
