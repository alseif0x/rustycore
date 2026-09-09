//! Condition entry model and load reporting state definitions, part 1 of 3.
//!
//! Separated from the conditions.rs root under #638. Behaviour is preserved.

use super::*;

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
    pub skill_line_store: Option<&'a crate::SkillLineStore>,
    pub map_store: Option<&'a crate::MapStore>,
    pub phase_store: Option<&'a crate::PhaseStore>,
    pub quest_store: Option<&'a crate::quest::QuestStore>,
    pub area_trigger_db2_store: Option<&'a crate::AreaTriggerDb2Store>,
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
