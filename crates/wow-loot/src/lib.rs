// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! C++-shaped loot store primitives.
//!
//! This module mirrors the small, reusable parts of TrinityCore's
//! `LootStoreItem`, `LootTemplate`, and private `LootGroup` model. Runtime
//! condition evaluation and `Loot::FillLoot` orchestration are intentionally
//! layered above this crate.

use std::collections::{HashMap, HashSet};

use rand::Rng;
use wow_core::ObjectGuid;

const MIN_NON_ZERO_LOOT_CHANCE_LIKE_CPP: f32 = 0.000001;
pub const MAX_NR_LOOT_ITEMS_LIKE_CPP: usize = 18;
pub const LOOT_METHOD_FREE_FOR_ALL_LIKE_CPP: u8 = 0;
pub const LOOT_METHOD_ROUND_ROBIN_LIKE_CPP: u8 = 1;
pub const LOOT_METHOD_MASTER_LIKE_CPP: u8 = 2;
pub const LOOT_METHOD_GROUP_LIKE_CPP: u8 = 3;
pub const LOOT_METHOD_NEED_BEFORE_GREED_LIKE_CPP: u8 = 4;
pub const LOOT_METHOD_PERSONAL_LIKE_CPP: u8 = 5;
pub const LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP: u8 = 0;
pub const LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP: u8 = 1;
pub const LOOT_SLOT_TYPE_LOCKED_LIKE_CPP: u8 = 2;
pub const LOOT_SLOT_TYPE_MASTER_LIKE_CPP: u8 = 3;
pub const LOOT_SLOT_TYPE_OWNER_LIKE_CPP: u8 = 4;
const CONDITION_MAX_LIKE_CPP: i32 = 59;
const CONDITION_SPAWNMASK_DEPRECATED_LIKE_CPP: i32 = 19;
const CONDITION_ITEM_LIKE_CPP: i32 = 2;
const CONDITION_TEAM_LIKE_CPP: i32 = 6;
const CONDITION_INSTANCE_INFO_LIKE_CPP: i32 = 13;
const CONDITION_CLASS_LIKE_CPP: i32 = 15;
const CONDITION_RACE_LIKE_CPP: i32 = 16;
const CONDITION_GENDER_LIKE_CPP: i32 = 20;
const CONDITION_OBJECT_ENTRY_GUID_LEGACY_LIKE_CPP: i32 = 31;
const CONDITION_TYPE_MASK_LEGACY_LIKE_CPP: i32 = 32;
const CONDITION_DRUNKENSTATE_LIKE_CPP: i32 = 10;
const CONDITION_LEVEL_LIKE_CPP: i32 = 27;
const CONDITION_RELATION_TO_LIKE_CPP: i32 = 33;
const CONDITION_REACTION_TO_LIKE_CPP: i32 = 34;
const CONDITION_DISTANCE_TO_LIKE_CPP: i32 = 35;
const CONDITION_HP_VAL_LIKE_CPP: i32 = 37;
const CONDITION_HP_PCT_LIKE_CPP: i32 = 38;
const CONDITION_STAND_STATE_LIKE_CPP: i32 = 42;
const CONDITION_PET_TYPE_LIKE_CPP: i32 = 45;
const CONDITION_QUESTSTATE_LIKE_CPP: i32 = 47;
const CONDITION_OBJECT_ENTRY_GUID_LIKE_CPP: i32 = 51;
const CONDITION_TYPE_MASK_LIKE_CPP: i32 = 52;
const COMP_TYPE_MAX_LIKE_CPP: u32 = 5;
const MAX_QUEST_STATUS_LIKE_CPP: u32 = 7;
const ALLIANCE_TEAM_LIKE_CPP: u32 = 469;
const HORDE_TEAM_LIKE_CPP: u32 = 67;
const GENDER_NONE_LIKE_CPP: u32 = 2;
const DRUNKEN_SMASHED_LIKE_CPP: u32 = 3;
const CLASSMASK_ALL_PLAYABLE_LIKE_CPP: u32 = (1 << 13) - 1;
const RACEMASK_ALL_PLAYABLE_LIKE_CPP: u32 = 0xFFA1_FFFF;
const TYPEMASK_CONDITION_ALLOWED_LIKE_CPP: u32 = 0x560;
const UNIT_STAND_STATE_SUBMERGED_LIKE_CPP: u32 = 9;
const MAX_PET_TYPE_LIKE_CPP: u32 = 4;
const INSTANCE_INFO_GUID_DATA_LIKE_CPP: u32 = 1;
const TYPEID_OBJECT_LIKE_CPP: u32 = 0;
const TYPEID_ITEM_LIKE_CPP: u32 = 1;
const TYPEID_CONTAINER_LIKE_CPP: u32 = 2;
const TYPEID_UNIT_LIKE_CPP: u32 = 5;
const TYPEID_PLAYER_LIKE_CPP: u32 = 6;
const TYPEID_GAMEOBJECT_LIKE_CPP: u32 = 8;
const TYPEID_DYNAMICOBJECT_LIKE_CPP: u32 = 9;
const TYPEID_CORPSE_LIKE_CPP: u32 = 10;
const TYPEID_AREATRIGGER_LIKE_CPP: u32 = 11;
const TYPEID_SCENEOBJECT_LIKE_CPP: u32 = 12;
const TYPEID_CONVERSATION_LIKE_CPP: u32 = 13;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LootStoreItem {
    pub item_id: u32,
    pub reference: u32,
    pub chance: f32,
    pub needs_quest: bool,
    pub loot_mode: u16,
    pub group_id: u8,
    pub min_count: u8,
    pub max_count: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LootStoreItemContext {
    pub store_kind: LootStoreKind,
    pub entry: u32,
    pub item: LootStoreItem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeneratedLootItem {
    pub item_id: u32,
    pub count: u32,
    pub loot_list_id: u32,
    pub random_properties_id: i32,
    pub random_properties_seed: i32,
    pub context: u8,
    pub free_for_all: bool,
    pub follow_loot_rules: bool,
    pub needs_quest: bool,
    pub is_looted: bool,
    pub is_blocked: bool,
    pub is_under_threshold: bool,
    pub is_counted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeneratedPersonalLootItem {
    pub looter: ObjectGuid,
    pub item: GeneratedLootItem,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LootItemRandomProperties {
    pub id: i32,
    pub seed: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LootItemTemplateMetadata {
    pub max_stack: u32,
    pub has_multi_drop_flag: bool,
    pub has_follow_loot_rules_flag: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LootFillOptions {
    pub loot_mode: u16,
    pub rates_allowed: bool,
    pub referenced_amount_rate: f32,
    pub item_context: u8,
}

impl Default for LootFillOptions {
    fn default() -> Self {
        Self {
            loot_mode: 0x01,
            rates_allowed: true,
            referenced_amount_rate: 1.0,
            item_context: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LootFillError {
    MissingLootTemplate { loot_id: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LootStoreKind {
    Creature,
    Disenchant,
    Fishing,
    Gameobject,
    Item,
    Mail,
    Milling,
    Pickpocketing,
    Prospecting,
    Reference,
    Skinning,
    Spell,
}

impl LootStoreKind {
    pub const ALL_LIKE_CPP: [Self; 12] = [
        Self::Creature,
        Self::Disenchant,
        Self::Fishing,
        Self::Gameobject,
        Self::Item,
        Self::Mail,
        Self::Milling,
        Self::Pickpocketing,
        Self::Prospecting,
        Self::Reference,
        Self::Skinning,
        Self::Spell,
    ];

    #[must_use]
    pub const fn definition_like_cpp(self) -> LootStoreDefinition {
        match self {
            Self::Creature => {
                LootStoreDefinition::new("creature_loot_template", "creature entry", true)
            }
            Self::Disenchant => {
                LootStoreDefinition::new("disenchant_loot_template", "item disenchant id", true)
            }
            Self::Fishing => LootStoreDefinition::new("fishing_loot_template", "area id", true),
            Self::Gameobject => {
                LootStoreDefinition::new("gameobject_loot_template", "gameobject entry", true)
            }
            Self::Item => LootStoreDefinition::new("item_loot_template", "item entry", true),
            Self::Mail => LootStoreDefinition::new("mail_loot_template", "mail template id", false),
            Self::Milling => {
                LootStoreDefinition::new("milling_loot_template", "item entry (herb)", true)
            }
            Self::Pickpocketing => LootStoreDefinition::new(
                "pickpocketing_loot_template",
                "creature pickpocket lootid",
                true,
            ),
            Self::Prospecting => {
                LootStoreDefinition::new("prospecting_loot_template", "item entry (ore)", true)
            }
            Self::Reference => {
                LootStoreDefinition::new("reference_loot_template", "reference id", false)
            }
            Self::Skinning => {
                LootStoreDefinition::new("skinning_loot_template", "creature skinning id", true)
            }
            Self::Spell => LootStoreDefinition::new(
                "spell_loot_template",
                "spell id (random item creating)",
                false,
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LootStoreDefinition {
    pub table_name: &'static str,
    pub entry_name: &'static str,
    pub rates_allowed: bool,
}

impl LootStoreDefinition {
    #[must_use]
    pub const fn new(
        table_name: &'static str,
        entry_name: &'static str,
        rates_allowed: bool,
    ) -> Self {
        Self {
            table_name,
            entry_name,
            rates_allowed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LootTemplateRow {
    pub entry: u32,
    pub item: LootStoreItem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LootStoreLoadError {
    InvalidGroupId {
        table_name: &'static str,
        entry: u32,
        item_id: u32,
        group_id: u8,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct LootStore {
    definition: LootStoreDefinition,
    templates: HashMap<u32, LootTemplate>,
}

pub type LootStores = HashMap<LootStoreKind, LootStore>;

#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn loot_item_ui_type_for_player_like_cpp(
    player_guid: ObjectGuid,
    allowed_looters: &[ObjectGuid],
    is_looted_for_player: bool,
    free_for_all: bool,
    player_has_unlooted_ffa_item: bool,
    needs_quest: bool,
    follow_loot_rules: bool,
    loot_method: u8,
    round_robin_player: ObjectGuid,
    loot_master_guid: ObjectGuid,
    is_under_threshold: bool,
    is_blocked: bool,
    roll_winner_guid: ObjectGuid,
) -> Option<u8> {
    if is_looted_for_player {
        return None;
    }

    if !allowed_looters.contains(&player_guid) {
        return None;
    }

    if free_for_all {
        if player_has_unlooted_ffa_item {
            return Some(if loot_method == LOOT_METHOD_FREE_FOR_ALL_LIKE_CPP {
                LOOT_SLOT_TYPE_OWNER_LIKE_CPP
            } else {
                LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP
            });
        }
        return None;
    }

    if needs_quest && !follow_loot_rules {
        return Some(if loot_method == LOOT_METHOD_FREE_FOR_ALL_LIKE_CPP {
            LOOT_SLOT_TYPE_OWNER_LIKE_CPP
        } else {
            LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP
        });
    }

    match loot_method {
        LOOT_METHOD_FREE_FOR_ALL_LIKE_CPP => Some(LOOT_SLOT_TYPE_OWNER_LIKE_CPP),
        LOOT_METHOD_ROUND_ROBIN_LIKE_CPP => {
            if !round_robin_player.is_empty() && round_robin_player != player_guid {
                return None;
            }
            Some(LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP)
        }
        LOOT_METHOD_MASTER_LIKE_CPP => {
            if is_under_threshold {
                if !round_robin_player.is_empty() && round_robin_player != player_guid {
                    return None;
                }
                return Some(LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP);
            }

            Some(if loot_master_guid == player_guid {
                LOOT_SLOT_TYPE_MASTER_LIKE_CPP
            } else {
                LOOT_SLOT_TYPE_LOCKED_LIKE_CPP
            })
        }
        LOOT_METHOD_GROUP_LIKE_CPP | LOOT_METHOD_NEED_BEFORE_GREED_LIKE_CPP => {
            if is_under_threshold
                && !round_robin_player.is_empty()
                && round_robin_player != player_guid
            {
                return None;
            }

            if is_blocked {
                return Some(LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP);
            }

            if roll_winner_guid.is_empty() {
                return Some(LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP);
            }

            if roll_winner_guid == player_guid {
                return Some(LOOT_SLOT_TYPE_OWNER_LIKE_CPP);
            }

            None
        }
        LOOT_METHOD_PERSONAL_LIKE_CPP => Some(LOOT_SLOT_TYPE_OWNER_LIKE_CPP),
        _ => None,
    }
}

#[must_use]
pub fn generate_money_loot_with_rate_like_cpp<R: Rng + ?Sized>(
    min_amount: u32,
    max_amount: u32,
    rate: f32,
    rng: &mut R,
) -> u32 {
    if max_amount == 0 {
        return 0;
    }

    if max_amount <= min_amount {
        return ((max_amount as f32) * rate) as u32;
    }

    if max_amount - min_amount < 32_700 {
        return ((rng.gen_range(min_amount..=max_amount) as f32) * rate) as u32;
    }

    (((rng.gen_range((min_amount >> 8)..=(max_amount >> 8)) as f32) * rate) as u32) << 8
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LootReferenceUse {
    pub store_kind: LootStoreKind,
    pub entry: u32,
    pub item_id: u32,
    pub reference: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootReferenceCheckReport {
    pub missing_references: Vec<LootReferenceUse>,
    pub unused_reference_ids: Vec<u32>,
}

impl LootReferenceCheckReport {
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.missing_references.is_empty() && self.unused_reference_ids.is_empty()
    }
}

const LOOT_REFERENCE_CHECK_ORDER_LIKE_CPP: [LootStoreKind; 11] = [
    LootStoreKind::Creature,
    LootStoreKind::Fishing,
    LootStoreKind::Gameobject,
    LootStoreKind::Item,
    LootStoreKind::Milling,
    LootStoreKind::Pickpocketing,
    LootStoreKind::Skinning,
    LootStoreKind::Disenchant,
    LootStoreKind::Prospecting,
    LootStoreKind::Mail,
    LootStoreKind::Reference,
];

#[must_use]
pub fn check_loot_references_like_cpp(stores: &LootStores) -> LootReferenceCheckReport {
    let reference_store = stores.get(&LootStoreKind::Reference);
    let mut unused_reference_ids = reference_store
        .map(LootStore::collect_loot_ids_like_cpp)
        .unwrap_or_default();
    let mut missing_references = Vec::new();

    for kind in LOOT_REFERENCE_CHECK_ORDER_LIKE_CPP {
        let Some(store) = stores.get(&kind) else {
            continue;
        };

        for reference_use in store.reference_uses_like_cpp(kind) {
            if reference_store.is_some_and(|store| store.have_loot_for(reference_use.reference)) {
                unused_reference_ids.remove(&reference_use.reference);
            } else {
                missing_references.push(reference_use);
            }
        }
    }

    let mut unused_reference_ids: Vec<u32> = unused_reference_ids.into_iter().collect();
    unused_reference_ids.sort_unstable();

    LootReferenceCheckReport {
        missing_references,
        unused_reference_ids,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LootConditionId {
    pub source_type: i32,
    pub source_group: u32,
    pub source_entry: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootConditionRowLikeCpp {
    pub else_group: u32,
    pub condition_type_or_reference: i32,
    pub condition_target: u8,
    pub value1: u32,
    pub value2: u32,
    pub value3: u32,
    pub string_value1: String,
    pub negative: bool,
    pub script_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissingLootConditionTemplate {
    pub condition_id: LootConditionId,
    pub store_kind: LootStoreKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissingLootConditionItemTemplate {
    pub condition_id: LootConditionId,
    pub store_kind: LootStoreKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissingLootConditionTemplateItem {
    pub condition_id: LootConditionId,
    pub store_kind: LootStoreKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LootConditionReferenceUseLikeCpp {
    pub condition_id: LootConditionId,
    pub reference_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootConditionLinkReport {
    pub linked: usize,
    pub unsupported_source_types: Vec<LootConditionId>,
    pub missing_templates: Vec<MissingLootConditionTemplate>,
    pub missing_item_templates: Vec<MissingLootConditionItemTemplate>,
    pub missing_template_items: Vec<MissingLootConditionTemplateItem>,
    pub missing_reference_templates: Vec<LootConditionReferenceUseLikeCpp>,
}

impl LootConditionLinkReport {
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.unsupported_source_types.is_empty()
            && self.missing_templates.is_empty()
            && self.missing_item_templates.is_empty()
            && self.missing_template_items.is_empty()
            && self.missing_reference_templates.is_empty()
    }
}

#[must_use]
pub fn loot_store_kind_for_condition_source_type_like_cpp(
    source_type: i32,
) -> Option<LootStoreKind> {
    match source_type {
        1 => Some(LootStoreKind::Creature),
        2 => Some(LootStoreKind::Disenchant),
        3 => Some(LootStoreKind::Fishing),
        4 => Some(LootStoreKind::Gameobject),
        5 => Some(LootStoreKind::Item),
        6 => Some(LootStoreKind::Mail),
        7 => Some(LootStoreKind::Milling),
        8 => Some(LootStoreKind::Pickpocketing),
        9 => Some(LootStoreKind::Prospecting),
        10 => Some(LootStoreKind::Reference),
        11 => Some(LootStoreKind::Skinning),
        12 => Some(LootStoreKind::Spell),
        _ => None,
    }
}

#[must_use]
pub const fn condition_source_type_for_loot_store_kind_like_cpp(kind: LootStoreKind) -> i32 {
    match kind {
        LootStoreKind::Creature => 1,
        LootStoreKind::Disenchant => 2,
        LootStoreKind::Fishing => 3,
        LootStoreKind::Gameobject => 4,
        LootStoreKind::Item => 5,
        LootStoreKind::Mail => 6,
        LootStoreKind::Milling => 7,
        LootStoreKind::Pickpocketing => 8,
        LootStoreKind::Prospecting => 9,
        LootStoreKind::Reference => 10,
        LootStoreKind::Skinning => 11,
        LootStoreKind::Spell => 12,
    }
}

#[must_use]
pub fn loot_conditions_allow_player_like_cpp_representable<F>(
    conditions: &[LootConditionRowLikeCpp],
    evaluate: F,
) -> bool
where
    F: FnMut(&LootConditionRowLikeCpp) -> Option<bool>,
{
    let references = HashMap::new();
    loot_conditions_allow_player_with_references_like_cpp_representable(
        conditions,
        &references,
        evaluate,
    )
}

#[must_use]
pub fn loot_conditions_allow_player_with_references_like_cpp_representable<F>(
    conditions: &[LootConditionRowLikeCpp],
    references: &HashMap<u32, Vec<LootConditionRowLikeCpp>>,
    mut evaluate: F,
) -> bool
where
    F: FnMut(&LootConditionRowLikeCpp) -> Option<bool>,
{
    loot_conditions_allow_player_inner_like_cpp(conditions, references, &mut evaluate, 0)
}

#[must_use]
pub fn loot_condition_reference_ids_like_cpp(conditions: &[LootConditionRowLikeCpp]) -> Vec<u32> {
    conditions
        .iter()
        .filter(|condition| condition.condition_type_or_reference < 0)
        .map(|condition| condition.condition_type_or_reference.unsigned_abs())
        .collect()
}

#[must_use]
pub const fn loot_condition_reference_self_references_like_cpp(
    source_type_or_reference_id: i32,
    condition_type_or_reference: i32,
) -> bool {
    condition_type_or_reference < 0 && condition_type_or_reference == source_type_or_reference_id
}

const fn legacy_type_id_to_type_id_like_cpp(legacy_type_id: u32) -> u32 {
    match legacy_type_id {
        0 => TYPEID_OBJECT_LIKE_CPP,
        1 => TYPEID_ITEM_LIKE_CPP,
        2 => TYPEID_CONTAINER_LIKE_CPP,
        3 => TYPEID_UNIT_LIKE_CPP,
        4 => TYPEID_PLAYER_LIKE_CPP,
        5 => TYPEID_GAMEOBJECT_LIKE_CPP,
        6 => TYPEID_DYNAMICOBJECT_LIKE_CPP,
        7 => TYPEID_CORPSE_LIKE_CPP,
        8 => TYPEID_AREATRIGGER_LIKE_CPP,
        9 => TYPEID_SCENEOBJECT_LIKE_CPP,
        10 => TYPEID_CONVERSATION_LIKE_CPP,
        _ => TYPEID_OBJECT_LIKE_CPP,
    }
}

const fn legacy_type_mask_to_type_mask_like_cpp(legacy_type_mask: u32) -> u32 {
    let mut legacy_type_id = 0;
    let mut type_mask = 0;
    while legacy_type_id < 11 {
        if legacy_type_mask & (1 << legacy_type_id) != 0 {
            type_mask |= 1 << legacy_type_id_to_type_id_like_cpp(legacy_type_id);
        }
        legacy_type_id += 1;
    }
    type_mask
}

const fn object_entry_guid_type_id_is_loadable_without_external_stores_like_cpp(
    type_id: u32,
) -> bool {
    matches!(
        type_id,
        TYPEID_UNIT_LIKE_CPP
            | TYPEID_PLAYER_LIKE_CPP
            | TYPEID_GAMEOBJECT_LIKE_CPP
            | TYPEID_CORPSE_LIKE_CPP
    )
}

const fn type_mask_is_loadable_without_external_stores_like_cpp(type_mask: u32) -> bool {
    type_mask != 0 && type_mask & !TYPEMASK_CONDITION_ALLOWED_LIKE_CPP == 0
}

#[must_use]
pub fn loot_condition_row_normalize_without_external_stores_like_cpp(
    mut condition: LootConditionRowLikeCpp,
) -> Option<LootConditionRowLikeCpp> {
    match condition.condition_type_or_reference {
        CONDITION_OBJECT_ENTRY_GUID_LEGACY_LIKE_CPP => {
            condition.condition_type_or_reference = CONDITION_OBJECT_ENTRY_GUID_LIKE_CPP;
            condition.value1 = legacy_type_id_to_type_id_like_cpp(condition.value1);
        }
        CONDITION_TYPE_MASK_LEGACY_LIKE_CPP => {
            condition.condition_type_or_reference = CONDITION_TYPE_MASK_LIKE_CPP;
            condition.value1 = legacy_type_mask_to_type_mask_like_cpp(condition.value1);
        }
        _ => {}
    }

    if loot_condition_row_is_loadable_without_external_stores_like_cpp(&condition) {
        Some(condition)
    } else {
        None
    }
}

#[must_use]
pub const fn loot_condition_row_is_loadable_without_external_stores_like_cpp(
    condition: &LootConditionRowLikeCpp,
) -> bool {
    if condition.condition_type_or_reference < 0 {
        return true;
    }

    if condition.condition_type_or_reference >= CONDITION_MAX_LIKE_CPP {
        return false;
    }

    if condition.condition_target != 0 {
        return false;
    }

    match condition.condition_type_or_reference {
        CONDITION_SPAWNMASK_DEPRECATED_LIKE_CPP => false,
        CONDITION_ITEM_LIKE_CPP => condition.value2 != 0,
        CONDITION_TEAM_LIKE_CPP => {
            condition.value1 == ALLIANCE_TEAM_LIKE_CPP || condition.value1 == HORDE_TEAM_LIKE_CPP
        }
        CONDITION_CLASS_LIKE_CPP => condition.value1 & !CLASSMASK_ALL_PLAYABLE_LIKE_CPP == 0,
        CONDITION_RACE_LIKE_CPP => condition.value1 & !RACEMASK_ALL_PLAYABLE_LIKE_CPP == 0,
        CONDITION_GENDER_LIKE_CPP => condition.value1 <= GENDER_NONE_LIKE_CPP,
        CONDITION_DRUNKENSTATE_LIKE_CPP => condition.value1 <= DRUNKEN_SMASHED_LIKE_CPP,
        CONDITION_INSTANCE_INFO_LIKE_CPP => condition.value3 != INSTANCE_INFO_GUID_DATA_LIKE_CPP,
        CONDITION_LEVEL_LIKE_CPP => condition.value2 < COMP_TYPE_MAX_LIKE_CPP,
        CONDITION_OBJECT_ENTRY_GUID_LEGACY_LIKE_CPP => {
            object_entry_guid_type_id_is_loadable_without_external_stores_like_cpp(
                legacy_type_id_to_type_id_like_cpp(condition.value1),
            )
        }
        CONDITION_OBJECT_ENTRY_GUID_LIKE_CPP => {
            object_entry_guid_type_id_is_loadable_without_external_stores_like_cpp(condition.value1)
        }
        CONDITION_RELATION_TO_LIKE_CPP | CONDITION_REACTION_TO_LIKE_CPP => false,
        CONDITION_DISTANCE_TO_LIKE_CPP => false,
        CONDITION_HP_VAL_LIKE_CPP => condition.value2 < COMP_TYPE_MAX_LIKE_CPP,
        CONDITION_HP_PCT_LIKE_CPP => {
            condition.value1 <= 100 && condition.value2 < COMP_TYPE_MAX_LIKE_CPP
        }
        CONDITION_STAND_STATE_LIKE_CPP => match condition.value1 {
            0 => condition.value2 <= UNIT_STAND_STATE_SUBMERGED_LIKE_CPP,
            1 => condition.value2 <= 1,
            _ => false,
        },
        CONDITION_PET_TYPE_LIKE_CPP => condition.value1 < (1 << MAX_PET_TYPE_LIKE_CPP),
        CONDITION_QUESTSTATE_LIKE_CPP => condition.value2 < (1 << MAX_QUEST_STATUS_LIKE_CPP),
        CONDITION_TYPE_MASK_LEGACY_LIKE_CPP => {
            type_mask_is_loadable_without_external_stores_like_cpp(
                legacy_type_mask_to_type_mask_like_cpp(condition.value1),
            )
        }
        CONDITION_TYPE_MASK_LIKE_CPP => {
            type_mask_is_loadable_without_external_stores_like_cpp(condition.value1)
        }
        _ => true,
    }
}

fn loot_conditions_allow_player_inner_like_cpp<F>(
    conditions: &[LootConditionRowLikeCpp],
    references: &HashMap<u32, Vec<LootConditionRowLikeCpp>>,
    evaluate: &mut F,
    depth: u8,
) -> bool
where
    F: FnMut(&LootConditionRowLikeCpp) -> Option<bool>,
{
    if conditions.is_empty() {
        return true;
    }
    if depth >= 16 {
        return false;
    }

    let mut else_group_store: HashMap<u32, bool> = HashMap::new();
    for condition in conditions {
        let group_meets = else_group_store.entry(condition.else_group).or_insert(true);
        if !*group_meets {
            continue;
        }

        if condition.condition_type_or_reference < 0 {
            let reference_id = condition.condition_type_or_reference.unsigned_abs();
            if let Some(reference_conditions) = references.get(&reference_id) {
                *group_meets = loot_conditions_allow_player_inner_like_cpp(
                    reference_conditions,
                    references,
                    evaluate,
                    depth + 1,
                );
            }
            continue;
        }

        if condition.condition_target != 0
            || !condition.string_value1.is_empty()
            || !condition.script_name.is_empty()
        {
            *group_meets = false;
            continue;
        }
        let Some(mut condition_meets) = evaluate(condition) else {
            *group_meets = false;
            continue;
        };
        if condition.negative {
            condition_meets = !condition_meets;
        }
        *group_meets = condition_meets;
    }

    else_group_store.values().any(|group_meets| *group_meets)
}

#[must_use]
pub fn condition_compare_values_like_cpp(
    comparison_type: u32,
    value: u32,
    expected: u32,
) -> Option<bool> {
    match comparison_type {
        0 => Some(value == expected),
        1 => Some(value > expected),
        2 => Some(value < expected),
        3 => Some(value >= expected),
        4 => Some(value <= expected),
        _ => None,
    }
}

#[must_use]
pub fn check_loot_condition_links_like_cpp<I, F>(
    stores: &LootStores,
    condition_ids: I,
    mut item_exists: F,
) -> LootConditionLinkReport
where
    I: IntoIterator<Item = LootConditionId>,
    F: FnMut(u32) -> bool,
{
    let mut report = LootConditionLinkReport {
        linked: 0,
        unsupported_source_types: Vec::new(),
        missing_templates: Vec::new(),
        missing_item_templates: Vec::new(),
        missing_template_items: Vec::new(),
        missing_reference_templates: Vec::new(),
    };

    for condition_id in condition_ids {
        let Some(store_kind) =
            loot_store_kind_for_condition_source_type_like_cpp(condition_id.source_type)
        else {
            report.unsupported_source_types.push(condition_id);
            continue;
        };

        let Some(store) = stores.get(&store_kind) else {
            report.missing_templates.push(MissingLootConditionTemplate {
                condition_id,
                store_kind,
            });
            continue;
        };

        let Some(template) = store.get_loot_for(condition_id.source_group) else {
            report.missing_templates.push(MissingLootConditionTemplate {
                condition_id,
                store_kind,
            });
            continue;
        };

        if !item_exists(condition_id.source_entry)
            && !template.is_reference_like_cpp(condition_id.source_entry)
        {
            report
                .missing_item_templates
                .push(MissingLootConditionItemTemplate {
                    condition_id,
                    store_kind,
                });
            continue;
        }

        if template.has_condition_link_target_like_cpp(condition_id.source_entry) {
            report.linked = report.linked.saturating_add(1);
        } else {
            report
                .missing_template_items
                .push(MissingLootConditionTemplateItem {
                    condition_id,
                    store_kind,
                });
        }
    }

    report
}

pub fn check_loot_condition_references_like_cpp<I, R>(
    report: &mut LootConditionLinkReport,
    reference_uses: I,
    reference_template_ids: R,
) where
    I: IntoIterator<Item = LootConditionReferenceUseLikeCpp>,
    R: IntoIterator<Item = u32>,
{
    let reference_template_ids: HashSet<u32> = reference_template_ids.into_iter().collect();
    for reference_use in reference_uses {
        if !reference_template_ids.contains(&reference_use.reference_id) {
            report.missing_reference_templates.push(reference_use);
        }
    }
}

impl LootStore {
    #[must_use]
    pub fn new(definition: LootStoreDefinition) -> Self {
        Self {
            definition,
            templates: HashMap::new(),
        }
    }

    #[must_use]
    pub fn for_kind_like_cpp(kind: LootStoreKind) -> Self {
        Self::new(kind.definition_like_cpp())
    }

    #[must_use]
    pub const fn definition(&self) -> LootStoreDefinition {
        self.definition
    }

    #[must_use]
    pub fn templates(&self) -> &HashMap<u32, LootTemplate> {
        &self.templates
    }

    #[must_use]
    pub fn have_loot_for(&self, loot_id: u32) -> bool {
        self.templates.contains_key(&loot_id)
    }

    #[must_use]
    pub fn get_loot_for(&self, loot_id: u32) -> Option<&LootTemplate> {
        self.templates.get(&loot_id)
    }

    pub fn clear_like_cpp(&mut self) {
        self.templates.clear();
    }

    pub fn load_rows_like_cpp<I, F>(
        &mut self,
        rows: I,
        mut item_exists: F,
    ) -> Result<u32, LootStoreLoadError>
    where
        I: IntoIterator<Item = LootTemplateRow>,
        F: FnMut(u32) -> bool,
    {
        self.clear_like_cpp();
        let mut count = 0u32;

        for row in rows {
            if row.item.group_id >= 1 << 7 {
                return Err(LootStoreLoadError::InvalidGroupId {
                    table_name: self.definition.table_name,
                    entry: row.entry,
                    item_id: row.item.item_id,
                    group_id: row.item.group_id,
                });
            }

            let item_exists_for_row = row.item.reference != 0 || item_exists(row.item.item_id);
            if !row.item.is_valid_like_cpp(item_exists_for_row) {
                continue;
            }

            self.templates
                .entry(row.entry)
                .or_default()
                .add_entry_like_cpp(row.item);
            count = count.saturating_add(1);
        }

        Ok(count)
    }

    #[must_use]
    pub fn collect_loot_ids_like_cpp(&self) -> HashSet<u32> {
        self.templates.keys().copied().collect()
    }

    #[must_use]
    pub fn reference_uses_like_cpp(&self, store_kind: LootStoreKind) -> Vec<LootReferenceUse> {
        let mut uses = Vec::new();
        for (&entry, template) in &self.templates {
            template.append_reference_uses_like_cpp(store_kind, entry, &mut uses);
        }
        uses
    }

    #[must_use]
    pub fn condition_ids_for_fill_like_cpp(
        &self,
        loot_id: u32,
        store_kind: LootStoreKind,
        stores: &LootStores,
    ) -> Vec<LootConditionId> {
        let Some(template) = self.get_loot_for(loot_id) else {
            return Vec::new();
        };

        let mut ids = Vec::new();
        template.append_condition_ids_for_fill_like_cpp(stores, store_kind, loot_id, 0, &mut ids);
        ids.sort_by_key(|id| (id.source_type, id.source_group, id.source_entry));
        ids.dedup();
        ids
    }

    #[must_use]
    pub fn have_quest_loot_for_like_cpp(&self, loot_id: u32, stores: &LootStores) -> bool {
        self.get_loot_for(loot_id)
            .is_some_and(|template| template.has_quest_drop_like_cpp(stores, 0, &mut |_| true))
    }

    #[must_use]
    pub fn have_quest_loot_for_player_like_cpp<F>(
        &self,
        loot_id: u32,
        stores: &LootStores,
        mut player_has_quest_for_item: F,
    ) -> bool
    where
        F: FnMut(u32) -> bool,
    {
        self.get_loot_for(loot_id).is_some_and(|template| {
            template.has_quest_drop_for_player_like_cpp(stores, 0, &mut player_has_quest_for_item)
        })
    }

    pub fn fill_loot_like_cpp<R, FTemplate, FRate, FAllowed, FRandom>(
        &self,
        loot_id: u32,
        store_kind: LootStoreKind,
        stores: &LootStores,
        options: LootFillOptions,
        rng: &mut R,
        item_template: FTemplate,
        item_chance_rate: FRate,
        mut item_allowed: FAllowed,
        random_properties: FRandom,
    ) -> Result<Vec<GeneratedLootItem>, LootFillError>
    where
        R: Rng + ?Sized,
        FTemplate: FnMut(u32) -> Option<LootItemTemplateMetadata>,
        FRate: FnMut(LootStoreItem) -> f32,
        FAllowed: FnMut(LootStoreItem) -> bool,
        FRandom: FnMut(u32, &mut R) -> LootItemRandomProperties,
    {
        self.fill_loot_with_context_like_cpp(
            loot_id,
            store_kind,
            stores,
            options,
            rng,
            item_template,
            item_chance_rate,
            |context| item_allowed(context.item),
            random_properties,
        )
    }

    pub fn fill_loot_with_context_like_cpp<R, FTemplate, FRate, FAllowed, FRandom>(
        &self,
        loot_id: u32,
        store_kind: LootStoreKind,
        stores: &LootStores,
        options: LootFillOptions,
        rng: &mut R,
        mut item_template: FTemplate,
        mut item_chance_rate: FRate,
        mut item_allowed: FAllowed,
        mut random_properties: FRandom,
    ) -> Result<Vec<GeneratedLootItem>, LootFillError>
    where
        R: Rng + ?Sized,
        FTemplate: FnMut(u32) -> Option<LootItemTemplateMetadata>,
        FRate: FnMut(LootStoreItem) -> f32,
        FAllowed: FnMut(LootStoreItemContext) -> bool,
        FRandom: FnMut(u32, &mut R) -> LootItemRandomProperties,
    {
        let Some(template) = self.get_loot_for(loot_id) else {
            return Err(LootFillError::MissingLootTemplate { loot_id });
        };

        let mut generated = Vec::with_capacity(MAX_NR_LOOT_ITEMS_LIKE_CPP);
        template.process_like_cpp(
            stores,
            &options,
            rng,
            &mut generated,
            &mut item_template,
            &mut item_chance_rate,
            &mut item_allowed,
            &mut random_properties,
            store_kind,
            loot_id,
            0,
        );

        Ok(generated)
    }

    pub fn fill_personal_loot_with_context_like_cpp<R, FTemplate, FRate, FAllowed, FRandom>(
        &self,
        loot_id: u32,
        store_kind: LootStoreKind,
        stores: &LootStores,
        options: LootFillOptions,
        looters: &[ObjectGuid],
        rng: &mut R,
        mut item_template: FTemplate,
        mut item_chance_rate: FRate,
        mut item_allowed: FAllowed,
        mut random_properties: FRandom,
    ) -> Result<Vec<GeneratedPersonalLootItem>, LootFillError>
    where
        R: Rng + ?Sized,
        FTemplate: FnMut(u32) -> Option<LootItemTemplateMetadata>,
        FRate: FnMut(LootStoreItem) -> f32,
        FAllowed: FnMut(LootStoreItemContext, ObjectGuid) -> bool,
        FRandom: FnMut(u32, &mut R) -> LootItemRandomProperties,
    {
        let Some(template) = self.get_loot_for(loot_id) else {
            return Err(LootFillError::MissingLootTemplate { loot_id });
        };

        let mut generated = Vec::with_capacity(MAX_NR_LOOT_ITEMS_LIKE_CPP);
        template.process_personal_like_cpp(
            stores,
            &options,
            looters,
            rng,
            &mut generated,
            &mut item_template,
            &mut item_chance_rate,
            &mut item_allowed,
            &mut random_properties,
            store_kind,
            loot_id,
        );

        Ok(generated)
    }
}

impl LootStoreItem {
    #[must_use]
    pub fn is_reference(self) -> bool {
        self.reference > 0
    }

    #[must_use]
    pub fn is_valid_like_cpp(self, item_exists: bool) -> bool {
        if self.min_count == 0 {
            return false;
        }

        if self.reference == 0 {
            if self.item_id == 0 || !item_exists {
                return false;
            }

            if self.chance == 0.0 && self.group_id == 0 {
                return false;
            }

            if self.chance != 0.0 && self.chance < MIN_NON_ZERO_LOOT_CHANCE_LIKE_CPP {
                return false;
            }

            if self.max_count < self.min_count {
                return false;
            }

            return true;
        }

        self.needs_quest || self.chance != 0.0
    }

    #[must_use]
    pub fn can_roll_as_plain_entry_like_cpp(self, item_exists: bool, loot_mode: u16) -> bool {
        self.reference == 0
            && self.group_id == 0
            && self.is_valid_like_cpp(item_exists)
            && self.loot_mode & loot_mode != 0
    }

    #[must_use]
    pub fn can_roll_as_reference_entry_like_cpp(self, loot_mode: u16) -> bool {
        self.reference != 0 && self.is_valid_like_cpp(true) && self.loot_mode & loot_mode != 0
    }

    #[must_use]
    pub fn roll_like_cpp<R: Rng + ?Sized>(self, rng: &mut R, chance_multiplier: f32) -> bool {
        if self.chance >= 100.0 {
            return true;
        }

        roll_chance_like_cpp(rng, self.chance * chance_multiplier)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LootTemplate {
    entries: Vec<LootStoreItem>,
    groups: Vec<LootGroup>,
}

impl LootTemplate {
    pub fn add_entry_like_cpp(&mut self, item: LootStoreItem) {
        if item.group_id > 0 && item.reference == 0 {
            let index = usize::from(item.group_id - 1);
            if index >= self.groups.len() {
                self.groups.resize_with(index + 1, LootGroup::default);
            }
            self.groups[index].add_entry_like_cpp(item);
        } else {
            self.entries.push(item);
        }
    }

    #[must_use]
    pub fn entries(&self) -> &[LootStoreItem] {
        &self.entries
    }

    #[must_use]
    pub fn groups(&self) -> &[LootGroup] {
        &self.groups
    }

    #[must_use]
    pub fn is_reference_like_cpp(&self, item_id: u32) -> bool {
        self.entries
            .iter()
            .any(|item| item.item_id == item_id && item.reference > 0)
    }

    #[must_use]
    pub fn has_condition_link_target_like_cpp(&self, source_entry: u32) -> bool {
        if self.entries.iter().any(|item| item.item_id == source_entry) {
            return true;
        }

        self.groups
            .iter()
            .any(|group| group.has_condition_link_target_like_cpp(source_entry))
    }

    fn append_reference_uses_like_cpp(
        &self,
        store_kind: LootStoreKind,
        entry: u32,
        uses: &mut Vec<LootReferenceUse>,
    ) {
        append_reference_items_like_cpp(store_kind, entry, &self.entries, uses);

        for group in &self.groups {
            group.append_reference_uses_like_cpp(store_kind, entry, uses);
        }
    }

    fn append_condition_ids_for_fill_like_cpp(
        &self,
        stores: &LootStores,
        store_kind: LootStoreKind,
        entry: u32,
        group_id: u8,
        ids: &mut Vec<LootConditionId>,
    ) {
        if group_id > 0 {
            let index = usize::from(group_id - 1);
            if let Some(group) = self.groups.get(index) {
                group.append_condition_ids_for_fill_like_cpp(store_kind, entry, ids);
            }
            return;
        }

        for item in &self.entries {
            if item.reference > 0 {
                let Some(reference_store) = stores.get(&LootStoreKind::Reference) else {
                    continue;
                };
                let Some(reference_template) = reference_store.get_loot_for(item.reference) else {
                    continue;
                };
                reference_template.append_condition_ids_for_fill_like_cpp(
                    stores,
                    LootStoreKind::Reference,
                    item.reference,
                    item.group_id,
                    ids,
                );
            } else {
                ids.push(LootConditionId {
                    source_type: condition_source_type_for_loot_store_kind_like_cpp(store_kind),
                    source_group: entry,
                    source_entry: item.item_id,
                });
            }
        }

        for group in &self.groups {
            group.append_condition_ids_for_fill_like_cpp(store_kind, entry, ids);
        }
    }

    fn has_quest_drop_like_cpp<F>(
        &self,
        stores: &LootStores,
        group_id: u8,
        player_has_quest_for_item: &mut F,
    ) -> bool
    where
        F: FnMut(u32) -> bool,
    {
        if group_id > 0 {
            let index = usize::from(group_id - 1);
            return self
                .groups
                .get(index)
                .is_some_and(|group| group.has_quest_drop_like_cpp(player_has_quest_for_item));
        }

        for item in &self.entries {
            if item.reference > 0 {
                let Some(reference_store) = stores.get(&LootStoreKind::Reference) else {
                    continue;
                };
                let Some(reference_template) = reference_store.get_loot_for(item.reference) else {
                    continue;
                };
                if reference_template.has_quest_drop_like_cpp(
                    stores,
                    item.group_id,
                    player_has_quest_for_item,
                ) {
                    return true;
                }
            } else if item.needs_quest {
                return true;
            }
        }

        self.groups
            .iter()
            .any(|group| group.has_quest_drop_like_cpp(player_has_quest_for_item))
    }

    fn has_quest_drop_for_player_like_cpp<F>(
        &self,
        stores: &LootStores,
        group_id: u8,
        player_has_quest_for_item: &mut F,
    ) -> bool
    where
        F: FnMut(u32) -> bool,
    {
        if group_id > 0 {
            let index = usize::from(group_id - 1);
            return self.groups.get(index).is_some_and(|group| {
                group.has_quest_drop_for_player_like_cpp(player_has_quest_for_item)
            });
        }

        for item in &self.entries {
            if item.reference > 0 {
                let Some(reference_store) = stores.get(&LootStoreKind::Reference) else {
                    continue;
                };
                let Some(reference_template) = reference_store.get_loot_for(item.reference) else {
                    continue;
                };
                if reference_template.has_quest_drop_for_player_like_cpp(
                    stores,
                    item.group_id,
                    player_has_quest_for_item,
                ) {
                    return true;
                }
            } else if player_has_quest_for_item(item.item_id) {
                return true;
            }
        }

        self.groups
            .iter()
            .any(|group| group.has_quest_drop_for_player_like_cpp(player_has_quest_for_item))
    }

    fn process_like_cpp<R, FTemplate, FRate, FAllowed, FRandom>(
        &self,
        stores: &LootStores,
        options: &LootFillOptions,
        rng: &mut R,
        generated: &mut Vec<GeneratedLootItem>,
        item_template: &mut FTemplate,
        item_chance_rate: &mut FRate,
        item_allowed: &mut FAllowed,
        random_properties: &mut FRandom,
        store_kind: LootStoreKind,
        entry: u32,
        group_id: u8,
    ) where
        R: Rng + ?Sized,
        FTemplate: FnMut(u32) -> Option<LootItemTemplateMetadata>,
        FRate: FnMut(LootStoreItem) -> f32,
        FAllowed: FnMut(LootStoreItemContext) -> bool,
        FRandom: FnMut(u32, &mut R) -> LootItemRandomProperties,
    {
        if generated.len() >= MAX_NR_LOOT_ITEMS_LIKE_CPP {
            return;
        }

        if group_id > 0 {
            let index = usize::from(group_id - 1);
            if let Some(group) = self.groups.get(index) {
                group.process_like_cpp(
                    options.loot_mode,
                    rng,
                    generated,
                    item_template,
                    item_allowed,
                    random_properties,
                    options.item_context,
                    store_kind,
                    entry,
                );
            }
            return;
        }

        for item in &self.entries {
            if generated.len() >= MAX_NR_LOOT_ITEMS_LIKE_CPP {
                return;
            }

            if item.loot_mode & options.loot_mode == 0 {
                continue;
            }

            let chance_rate = if options.rates_allowed {
                item_chance_rate(*item)
            } else {
                1.0
            };

            if !item.roll_like_cpp(rng, chance_rate) {
                continue;
            }

            if item.reference > 0 {
                let Some(reference_store) = stores.get(&LootStoreKind::Reference) else {
                    continue;
                };
                let Some(reference_template) = reference_store.get_loot_for(item.reference) else {
                    continue;
                };

                let max_count =
                    ((f32::from(item.max_count)) * options.referenced_amount_rate) as u32;
                for _ in 0..max_count {
                    reference_template.process_like_cpp(
                        stores,
                        options,
                        rng,
                        generated,
                        item_template,
                        item_chance_rate,
                        item_allowed,
                        random_properties,
                        LootStoreKind::Reference,
                        item.reference,
                        item.group_id,
                    );
                    if generated.len() >= MAX_NR_LOOT_ITEMS_LIKE_CPP {
                        return;
                    }
                }
            } else if item_allowed(LootStoreItemContext {
                store_kind,
                entry,
                item: *item,
            }) {
                add_generated_loot_item_like_cpp(
                    generated,
                    *item,
                    options.item_context,
                    rng,
                    item_template,
                    random_properties,
                );
            }
        }

        for group in &self.groups {
            if generated.len() >= MAX_NR_LOOT_ITEMS_LIKE_CPP {
                return;
            }
            group.process_like_cpp(
                options.loot_mode,
                rng,
                generated,
                item_template,
                item_allowed,
                random_properties,
                options.item_context,
                store_kind,
                entry,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn process_personal_like_cpp<R, FTemplate, FRate, FAllowed, FRandom>(
        &self,
        stores: &LootStores,
        options: &LootFillOptions,
        looters: &[ObjectGuid],
        rng: &mut R,
        generated: &mut Vec<GeneratedPersonalLootItem>,
        item_template: &mut FTemplate,
        item_chance_rate: &mut FRate,
        item_allowed: &mut FAllowed,
        random_properties: &mut FRandom,
        store_kind: LootStoreKind,
        entry: u32,
    ) where
        R: Rng + ?Sized,
        FTemplate: FnMut(u32) -> Option<LootItemTemplateMetadata>,
        FRate: FnMut(LootStoreItem) -> f32,
        FAllowed: FnMut(LootStoreItemContext, ObjectGuid) -> bool,
        FRandom: FnMut(u32, &mut R) -> LootItemRandomProperties,
    {
        for item in &self.entries {
            if generated.len() >= MAX_NR_LOOT_ITEMS_LIKE_CPP {
                return;
            }

            if item.loot_mode & options.loot_mode == 0 {
                continue;
            }

            let chance_rate = if options.rates_allowed {
                item_chance_rate(*item)
            } else {
                1.0
            };

            if !item.roll_like_cpp(rng, chance_rate) {
                continue;
            }

            if item.reference > 0 {
                let Some(reference_store) = stores.get(&LootStoreKind::Reference) else {
                    continue;
                };
                let Some(reference_template) = reference_store.get_loot_for(item.reference) else {
                    continue;
                };

                let max_count =
                    ((f32::from(item.max_count)) * options.referenced_amount_rate) as u32;
                let mut got_loot = Vec::new();
                for _ in 0..max_count {
                    let eligible = reference_template.personal_looters_for_template_like_cpp(
                        stores,
                        LootStoreKind::Reference,
                        item.reference,
                        item.group_id,
                        looters,
                        item_allowed,
                    );
                    if eligible.is_empty() {
                        break;
                    }

                    let not_yet_looted = eligible
                        .iter()
                        .copied()
                        .filter(|looter| !got_loot.contains(looter))
                        .collect::<Vec<_>>();
                    let candidates = if not_yet_looted.is_empty() {
                        got_loot.clear();
                        eligible
                    } else {
                        not_yet_looted
                    };
                    let chosen_looter = candidates[rng.gen_range(0..candidates.len())];
                    reference_template.process_for_personal_looter_like_cpp(
                        stores,
                        options,
                        rng,
                        generated,
                        item_template,
                        item_chance_rate,
                        item_allowed,
                        random_properties,
                        LootStoreKind::Reference,
                        item.reference,
                        item.group_id,
                        chosen_looter,
                    );
                    got_loot.push(chosen_looter);
                }
            } else {
                let candidates = looters
                    .iter()
                    .copied()
                    .filter(|looter| {
                        item_allowed(
                            LootStoreItemContext {
                                store_kind,
                                entry,
                                item: *item,
                            },
                            *looter,
                        )
                    })
                    .collect::<Vec<_>>();
                if candidates.is_empty() {
                    continue;
                }

                let chosen_looter = candidates[rng.gen_range(0..candidates.len())];
                add_generated_personal_loot_item_like_cpp(
                    generated,
                    chosen_looter,
                    *item,
                    options.item_context,
                    rng,
                    item_template,
                    random_properties,
                );
            }
        }

        for group in &self.groups {
            if generated.len() >= MAX_NR_LOOT_ITEMS_LIKE_CPP {
                return;
            }

            let candidates = looters
                .iter()
                .copied()
                .filter(|looter| {
                    group.has_drop_for_personal_looter_like_cpp(
                        store_kind,
                        entry,
                        *looter,
                        item_allowed,
                    )
                })
                .collect::<Vec<_>>();
            if candidates.is_empty() {
                continue;
            }

            let chosen_looter = candidates[rng.gen_range(0..candidates.len())];
            group.process_personal_root_like_cpp(
                options.loot_mode,
                rng,
                generated,
                item_template,
                random_properties,
                options.item_context,
                chosen_looter,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn process_for_personal_looter_like_cpp<R, FTemplate, FRate, FAllowed, FRandom>(
        &self,
        stores: &LootStores,
        options: &LootFillOptions,
        rng: &mut R,
        generated: &mut Vec<GeneratedPersonalLootItem>,
        item_template: &mut FTemplate,
        item_chance_rate: &mut FRate,
        item_allowed: &mut FAllowed,
        random_properties: &mut FRandom,
        store_kind: LootStoreKind,
        entry: u32,
        group_id: u8,
        looter: ObjectGuid,
    ) where
        R: Rng + ?Sized,
        FTemplate: FnMut(u32) -> Option<LootItemTemplateMetadata>,
        FRate: FnMut(LootStoreItem) -> f32,
        FAllowed: FnMut(LootStoreItemContext, ObjectGuid) -> bool,
        FRandom: FnMut(u32, &mut R) -> LootItemRandomProperties,
    {
        let mut generated_for_looter = Vec::new();
        self.process_like_cpp(
            stores,
            options,
            rng,
            &mut generated_for_looter,
            item_template,
            item_chance_rate,
            &mut |context| item_allowed(context, looter),
            random_properties,
            store_kind,
            entry,
            group_id,
        );
        for mut item in generated_for_looter {
            item.loot_list_id = generated.len() as u32;
            generated.push(GeneratedPersonalLootItem { looter, item });
        }
    }

    fn personal_looters_for_template_like_cpp<FAllowed>(
        &self,
        stores: &LootStores,
        store_kind: LootStoreKind,
        entry: u32,
        group_id: u8,
        looters: &[ObjectGuid],
        item_allowed: &mut FAllowed,
    ) -> Vec<ObjectGuid>
    where
        FAllowed: FnMut(LootStoreItemContext, ObjectGuid) -> bool,
    {
        looters
            .iter()
            .copied()
            .filter(|looter| {
                self.has_drop_for_personal_looter_like_cpp(
                    stores,
                    store_kind,
                    entry,
                    group_id,
                    *looter,
                    item_allowed,
                )
            })
            .collect()
    }

    fn has_drop_for_personal_looter_like_cpp<FAllowed>(
        &self,
        stores: &LootStores,
        store_kind: LootStoreKind,
        entry: u32,
        group_id: u8,
        looter: ObjectGuid,
        item_allowed: &mut FAllowed,
    ) -> bool
    where
        FAllowed: FnMut(LootStoreItemContext, ObjectGuid) -> bool,
    {
        if group_id > 0 {
            let index = usize::from(group_id - 1);
            return self.groups.get(index).is_some_and(|group| {
                group.has_drop_for_personal_looter_like_cpp(store_kind, entry, looter, item_allowed)
            });
        }

        for item in &self.entries {
            if item.reference > 0 {
                let Some(reference_store) = stores.get(&LootStoreKind::Reference) else {
                    continue;
                };
                let Some(reference_template) = reference_store.get_loot_for(item.reference) else {
                    continue;
                };
                if reference_template.has_drop_for_personal_looter_like_cpp(
                    stores,
                    LootStoreKind::Reference,
                    item.reference,
                    item.group_id,
                    looter,
                    item_allowed,
                ) {
                    return true;
                }
            } else if item_allowed(
                LootStoreItemContext {
                    store_kind,
                    entry,
                    item: *item,
                },
                looter,
            ) {
                return true;
            }
        }

        self.groups.iter().any(|group| {
            group.has_drop_for_personal_looter_like_cpp(store_kind, entry, looter, item_allowed)
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LootGroup {
    explicitly_chanced: Vec<LootStoreItem>,
    equal_chanced: Vec<LootStoreItem>,
}

impl LootGroup {
    pub fn add_entry_like_cpp(&mut self, item: LootStoreItem) {
        if item.chance != 0.0 {
            self.explicitly_chanced.push(item);
        } else {
            self.equal_chanced.push(item);
        }
    }

    #[must_use]
    pub fn explicitly_chanced(&self) -> &[LootStoreItem] {
        &self.explicitly_chanced
    }

    #[must_use]
    pub fn equal_chanced(&self) -> &[LootStoreItem] {
        &self.equal_chanced
    }

    fn has_quest_drop_like_cpp<F>(&self, player_has_quest_for_item: &mut F) -> bool
    where
        F: FnMut(u32) -> bool,
    {
        self.explicitly_chanced
            .iter()
            .chain(self.equal_chanced.iter())
            .any(|item| item.needs_quest && player_has_quest_for_item(item.item_id))
    }

    fn has_quest_drop_for_player_like_cpp<F>(&self, player_has_quest_for_item: &mut F) -> bool
    where
        F: FnMut(u32) -> bool,
    {
        self.explicitly_chanced
            .iter()
            .chain(self.equal_chanced.iter())
            .any(|item| player_has_quest_for_item(item.item_id))
    }

    pub fn roll_like_cpp<R, F>(
        &self,
        loot_mode: u16,
        rng: &mut R,
        mut item_allowed: F,
    ) -> Option<LootStoreItem>
    where
        R: Rng + ?Sized,
        F: FnMut(LootStoreItem) -> bool,
    {
        self.roll_with_context_like_cpp(
            loot_mode,
            rng,
            |context| item_allowed(context.item),
            LootStoreKind::Creature,
            0,
        )
    }

    fn roll_with_context_like_cpp<R, F>(
        &self,
        loot_mode: u16,
        rng: &mut R,
        mut item_allowed: F,
        store_kind: LootStoreKind,
        entry: u32,
    ) -> Option<LootStoreItem>
    where
        R: Rng + ?Sized,
        F: FnMut(LootStoreItemContext) -> bool,
    {
        let possible_explicit: Vec<LootStoreItem> = self
            .explicitly_chanced
            .iter()
            .copied()
            .filter(|item| {
                item.loot_mode & loot_mode != 0
                    && item_allowed(LootStoreItemContext {
                        store_kind,
                        entry,
                        item: *item,
                    })
            })
            .collect();

        if !possible_explicit.is_empty() {
            let mut roll = rng.gen_range(0.0f32..100.0f32);
            for item in possible_explicit {
                if item.chance >= 100.0 {
                    return Some(item);
                }

                roll -= item.chance;
                if roll < 0.0 {
                    return Some(item);
                }
            }
        }

        let possible_equal: Vec<LootStoreItem> = self
            .equal_chanced
            .iter()
            .copied()
            .filter(|item| {
                item.loot_mode & loot_mode != 0
                    && item_allowed(LootStoreItemContext {
                        store_kind,
                        entry,
                        item: *item,
                    })
            })
            .collect();

        if possible_equal.is_empty() {
            return None;
        }

        let index = rng.gen_range(0..possible_equal.len());
        Some(possible_equal[index])
    }

    fn append_reference_uses_like_cpp(
        &self,
        store_kind: LootStoreKind,
        entry: u32,
        uses: &mut Vec<LootReferenceUse>,
    ) {
        append_reference_items_like_cpp(store_kind, entry, &self.explicitly_chanced, uses);
        append_reference_items_like_cpp(store_kind, entry, &self.equal_chanced, uses);
    }

    fn append_condition_ids_for_fill_like_cpp(
        &self,
        store_kind: LootStoreKind,
        entry: u32,
        ids: &mut Vec<LootConditionId>,
    ) {
        for item in self
            .explicitly_chanced
            .iter()
            .chain(self.equal_chanced.iter())
        {
            ids.push(LootConditionId {
                source_type: condition_source_type_for_loot_store_kind_like_cpp(store_kind),
                source_group: entry,
                source_entry: item.item_id,
            });
        }
    }

    fn has_condition_link_target_like_cpp(&self, source_entry: u32) -> bool {
        self.explicitly_chanced
            .iter()
            .chain(self.equal_chanced.iter())
            .any(|item| item.item_id == source_entry)
    }

    fn process_like_cpp<R, FTemplate, FAllowed, FRandom>(
        &self,
        loot_mode: u16,
        rng: &mut R,
        generated: &mut Vec<GeneratedLootItem>,
        item_template: &mut FTemplate,
        item_allowed: &mut FAllowed,
        random_properties: &mut FRandom,
        item_context: u8,
        store_kind: LootStoreKind,
        entry: u32,
    ) where
        R: Rng + ?Sized,
        FTemplate: FnMut(u32) -> Option<LootItemTemplateMetadata>,
        FAllowed: FnMut(LootStoreItemContext) -> bool,
        FRandom: FnMut(u32, &mut R) -> LootItemRandomProperties,
    {
        if let Some(item) =
            self.roll_with_context_like_cpp(loot_mode, rng, item_allowed, store_kind, entry)
        {
            add_generated_loot_item_like_cpp(
                generated,
                item,
                item_context,
                rng,
                item_template,
                random_properties,
            );
        }
    }

    fn has_drop_for_personal_looter_like_cpp<FAllowed>(
        &self,
        store_kind: LootStoreKind,
        entry: u32,
        looter: ObjectGuid,
        item_allowed: &mut FAllowed,
    ) -> bool
    where
        FAllowed: FnMut(LootStoreItemContext, ObjectGuid) -> bool,
    {
        self.explicitly_chanced
            .iter()
            .chain(self.equal_chanced.iter())
            .any(|item| {
                item_allowed(
                    LootStoreItemContext {
                        store_kind,
                        entry,
                        item: *item,
                    },
                    looter,
                )
            })
    }

    fn process_personal_root_like_cpp<R, FTemplate, FRandom>(
        &self,
        loot_mode: u16,
        rng: &mut R,
        generated: &mut Vec<GeneratedPersonalLootItem>,
        item_template: &mut FTemplate,
        random_properties: &mut FRandom,
        item_context: u8,
        looter: ObjectGuid,
    ) where
        R: Rng + ?Sized,
        FTemplate: FnMut(u32) -> Option<LootItemTemplateMetadata>,
        FRandom: FnMut(u32, &mut R) -> LootItemRandomProperties,
    {
        if let Some(item) =
            self.roll_with_context_like_cpp(loot_mode, rng, |_| true, LootStoreKind::Creature, 0)
        {
            add_generated_personal_loot_item_like_cpp(
                generated,
                looter,
                item,
                item_context,
                rng,
                item_template,
                random_properties,
            );
        }
    }
}

fn add_generated_loot_item_like_cpp<R, FTemplate, FRandom>(
    generated: &mut Vec<GeneratedLootItem>,
    item: LootStoreItem,
    item_context: u8,
    rng: &mut R,
    item_template: &mut FTemplate,
    random_properties: &mut FRandom,
) where
    R: Rng + ?Sized,
    FTemplate: FnMut(u32) -> Option<LootItemTemplateMetadata>,
    FRandom: FnMut(u32, &mut R) -> LootItemRandomProperties,
{
    let Some(metadata) = item_template(item.item_id) else {
        return;
    };
    let max_stack = metadata.max_stack;
    if max_stack == 0 {
        return;
    }

    let mut count = rng.gen_range(u32::from(item.min_count)..=u32::from(item.max_count));
    let stacks = count / max_stack + u32::from(count % max_stack != 0);

    for _ in 0..stacks {
        if generated.len() >= MAX_NR_LOOT_ITEMS_LIKE_CPP {
            return;
        }

        let stack_count = count.min(max_stack);
        let random_properties = random_properties(item.item_id, rng);
        generated.push(GeneratedLootItem {
            item_id: item.item_id,
            count: stack_count,
            loot_list_id: generated.len() as u32,
            random_properties_id: random_properties.id,
            random_properties_seed: random_properties.seed,
            context: item_context,
            free_for_all: metadata.has_multi_drop_flag,
            follow_loot_rules: !item.needs_quest || metadata.has_follow_loot_rules_flag,
            needs_quest: item.needs_quest,
            is_looted: false,
            is_blocked: false,
            is_under_threshold: false,
            is_counted: false,
        });
        count = count.saturating_sub(max_stack);
    }
}

fn add_generated_personal_loot_item_like_cpp<R, FTemplate, FRandom>(
    generated: &mut Vec<GeneratedPersonalLootItem>,
    looter: ObjectGuid,
    item: LootStoreItem,
    item_context: u8,
    rng: &mut R,
    item_template: &mut FTemplate,
    random_properties: &mut FRandom,
) where
    R: Rng + ?Sized,
    FTemplate: FnMut(u32) -> Option<LootItemTemplateMetadata>,
    FRandom: FnMut(u32, &mut R) -> LootItemRandomProperties,
{
    let Some(metadata) = item_template(item.item_id) else {
        return;
    };
    let max_stack = metadata.max_stack;
    if max_stack == 0 {
        return;
    }

    let mut count = rng.gen_range(u32::from(item.min_count)..=u32::from(item.max_count));
    let stacks = count / max_stack + u32::from(count % max_stack != 0);

    for _ in 0..stacks {
        if generated.len() >= MAX_NR_LOOT_ITEMS_LIKE_CPP {
            return;
        }

        let stack_count = count.min(max_stack);
        let random_properties = random_properties(item.item_id, rng);
        generated.push(GeneratedPersonalLootItem {
            looter,
            item: GeneratedLootItem {
                item_id: item.item_id,
                count: stack_count,
                loot_list_id: generated.len() as u32,
                random_properties_id: random_properties.id,
                random_properties_seed: random_properties.seed,
                context: item_context,
                free_for_all: metadata.has_multi_drop_flag,
                follow_loot_rules: !item.needs_quest || metadata.has_follow_loot_rules_flag,
                needs_quest: item.needs_quest,
                is_looted: false,
                is_blocked: false,
                is_under_threshold: false,
                is_counted: false,
            },
        });
        count = count.saturating_sub(max_stack);
    }
}

fn roll_chance_like_cpp<R: Rng + ?Sized>(rng: &mut R, chance: f32) -> bool {
    if chance <= 0.0 {
        return false;
    }
    if chance >= 100.0 {
        return true;
    }

    rng.gen_range(0.0f32..100.0f32) < chance
}

fn append_reference_items_like_cpp(
    store_kind: LootStoreKind,
    entry: u32,
    items: &[LootStoreItem],
    uses: &mut Vec<LootReferenceUse>,
) {
    for item in items {
        if item.reference > 0 {
            uses.push(LootReferenceUse {
                store_kind,
                entry,
                item_id: item.item_id,
                reference: item.reference,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GeneratedLootItem, LOOT_METHOD_FREE_FOR_ALL_LIKE_CPP, LOOT_METHOD_GROUP_LIKE_CPP,
        LOOT_METHOD_MASTER_LIKE_CPP, LOOT_METHOD_NEED_BEFORE_GREED_LIKE_CPP,
        LOOT_METHOD_PERSONAL_LIKE_CPP, LOOT_METHOD_ROUND_ROBIN_LIKE_CPP,
        LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP, LOOT_SLOT_TYPE_LOCKED_LIKE_CPP,
        LOOT_SLOT_TYPE_MASTER_LIKE_CPP, LOOT_SLOT_TYPE_OWNER_LIKE_CPP,
        LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP, LootConditionId, LootConditionLinkReport,
        LootConditionReferenceUseLikeCpp, LootConditionRowLikeCpp, LootFillError, LootFillOptions,
        LootItemRandomProperties, LootItemTemplateMetadata, LootReferenceCheckReport,
        LootReferenceUse, LootStore, LootStoreItem, LootStoreKind, LootStoreLoadError, LootStores,
        LootTemplate, LootTemplateRow, MissingLootConditionItemTemplate,
        MissingLootConditionTemplate, MissingLootConditionTemplateItem,
        check_loot_condition_links_like_cpp, check_loot_condition_references_like_cpp,
        check_loot_references_like_cpp, condition_compare_values_like_cpp,
        generate_money_loot_with_rate_like_cpp, loot_condition_reference_ids_like_cpp,
        loot_condition_reference_self_references_like_cpp,
        loot_condition_row_is_loadable_without_external_stores_like_cpp,
        loot_condition_row_normalize_without_external_stores_like_cpp,
        loot_conditions_allow_player_like_cpp_representable,
        loot_conditions_allow_player_with_references_like_cpp_representable,
        loot_item_ui_type_for_player_like_cpp,
    };
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    use std::collections::HashMap;
    use wow_core::ObjectGuid;

    const DEFAULT_LOOT_MODE: u16 = 0x01;

    fn item(item_id: u32, reference: u32, chance: f32, group_id: u8) -> LootStoreItem {
        LootStoreItem {
            item_id,
            reference,
            chance,
            needs_quest: false,
            loot_mode: DEFAULT_LOOT_MODE,
            group_id,
            min_count: 1,
            max_count: 1,
        }
    }

    fn item_metadata(max_stack: u32) -> LootItemTemplateMetadata {
        LootItemTemplateMetadata {
            max_stack,
            has_multi_drop_flag: false,
            has_follow_loot_rules_flag: false,
        }
    }

    fn guid(counter: i64) -> ObjectGuid {
        ObjectGuid::create_player(1, counter)
    }

    #[allow(clippy::too_many_arguments)]
    fn ui_type(
        player: ObjectGuid,
        allowed_looters: &[ObjectGuid],
        is_looted_for_player: bool,
        free_for_all: bool,
        player_has_unlooted_ffa_item: bool,
        needs_quest: bool,
        follow_loot_rules: bool,
        loot_method: u8,
        round_robin_player: ObjectGuid,
        loot_master_guid: ObjectGuid,
        is_under_threshold: bool,
        is_blocked: bool,
        roll_winner_guid: ObjectGuid,
    ) -> Option<u8> {
        loot_item_ui_type_for_player_like_cpp(
            player,
            allowed_looters,
            is_looted_for_player,
            free_for_all,
            player_has_unlooted_ffa_item,
            needs_quest,
            follow_loot_rules,
            loot_method,
            round_robin_player,
            loot_master_guid,
            is_under_threshold,
            is_blocked,
            roll_winner_guid,
        )
    }

    fn generated_item(item_id: u32, count: u32, loot_list_id: u32) -> GeneratedLootItem {
        GeneratedLootItem {
            item_id,
            count,
            loot_list_id,
            random_properties_id: item_id as i32 + 1000,
            random_properties_seed: item_id as i32 + 2000,
            context: 0,
            free_for_all: false,
            follow_loot_rules: true,
            needs_quest: false,
            is_looted: false,
            is_blocked: false,
            is_under_threshold: false,
            is_counted: false,
        }
    }

    fn random_properties<R: Rng + ?Sized>(item_id: u32, _rng: &mut R) -> LootItemRandomProperties {
        LootItemRandomProperties {
            id: item_id as i32 + 1000,
            seed: item_id as i32 + 2000,
        }
    }

    fn condition(
        else_group: u32,
        condition_type_or_reference: i32,
        value1: u32,
        negative: bool,
    ) -> LootConditionRowLikeCpp {
        LootConditionRowLikeCpp {
            else_group,
            condition_type_or_reference,
            condition_target: 0,
            value1,
            value2: 0,
            value3: 0,
            string_value1: String::new(),
            negative,
            script_name: String::new(),
        }
    }

    #[test]
    fn loot_store_item_validity_matches_cpp_basic_guards() {
        assert!(item(25, 0, 100.0, 0).is_valid_like_cpp(true));
        assert!(!item(0, 0, 100.0, 0).is_valid_like_cpp(true));
        assert!(!item(25, 0, 100.0, 0).is_valid_like_cpp(false));
        assert!(!item(25, 0, 0.0, 0).is_valid_like_cpp(true));
        assert!(item(25, 0, 0.0, 1).is_valid_like_cpp(true));
        assert!(!item(25, 0, 0.0000001, 0).is_valid_like_cpp(true));
        assert!(item(0, 700, 25.0, 0).is_valid_like_cpp(true));
        assert!(!item(0, 700, 0.0, 0).is_valid_like_cpp(true));
        assert!(
            LootStoreItem {
                needs_quest: true,
                ..item(0, 700, 0.0, 0)
            }
            .is_valid_like_cpp(true)
        );
    }

    #[test]
    fn loot_conditions_else_group_negative_and_unsupported_rows_match_cpp_shape() {
        let conditions = vec![
            condition(0, 25, 100, false),
            condition(0, 25, 200, false),
            condition(1, 25, 300, true),
        ];

        assert!(loot_conditions_allow_player_like_cpp_representable(
            &conditions,
            |condition| Some(condition.value1 == 100 || condition.value1 == 200),
        ));
        assert!(loot_conditions_allow_player_like_cpp_representable(
            &conditions,
            |condition| Some(condition.value1 == 100),
        ));
        assert!(!loot_conditions_allow_player_like_cpp_representable(
            &conditions,
            |condition| Some(condition.value1 == 300),
        ));

        let mut scripted = condition(0, 25, 100, false);
        scripted.script_name = "Unsupported".into();
        assert!(!loot_conditions_allow_player_like_cpp_representable(
            &[scripted],
            |_| Some(true),
        ));
        assert!(loot_conditions_allow_player_like_cpp_representable(
            &[condition(0, -1, 100, false)],
            |_| Some(true),
        ));
    }

    #[test]
    fn loot_conditions_else_group_store_does_not_require_contiguous_rows_like_cpp() {
        let conditions = vec![
            condition(0, 25, 100, false),
            condition(1, 25, 300, false),
            condition(0, 25, 200, false),
        ];

        assert!(loot_conditions_allow_player_like_cpp_representable(
            &conditions,
            |condition| Some(condition.value1 == 100 || condition.value1 == 200),
        ));
        assert!(!loot_conditions_allow_player_like_cpp_representable(
            &conditions,
            |condition| Some(condition.value1 == 100),
        ));
    }

    #[test]
    fn loot_conditions_reference_templates_expand_like_cpp() {
        let conditions = vec![condition(0, -42, 0, false), condition(1, 25, 300, false)];
        let mut references = HashMap::new();
        references.insert(42, vec![condition(0, 25, 100, false)]);

        assert!(
            loot_conditions_allow_player_with_references_like_cpp_representable(
                &conditions,
                &references,
                |condition| Some(condition.value1 == 100),
            )
        );
        assert!(
            !loot_conditions_allow_player_with_references_like_cpp_representable(
                &conditions,
                &references,
                |condition| Some(condition.value1 == 200),
            )
        );
    }

    #[test]
    fn loot_condition_reference_ids_extract_negative_condition_types_like_cpp() {
        assert_eq!(
            loot_condition_reference_ids_like_cpp(&[
                condition(0, -42, 0, false),
                condition(0, 25, 0, false),
                condition(0, -900, 0, false),
            ]),
            vec![42, 900]
        );
    }

    #[test]
    fn loot_condition_reference_self_reference_guard_matches_cpp_load_skip() {
        assert!(loot_condition_reference_self_references_like_cpp(-42, -42));
        assert!(!loot_condition_reference_self_references_like_cpp(1, -42));
        assert!(!loot_condition_reference_self_references_like_cpp(
            -42, -900
        ));
        assert!(!loot_condition_reference_self_references_like_cpp(-42, 25));
    }

    #[test]
    fn loot_condition_row_loadable_guard_matches_cpp_deterministic_load_skips() {
        assert!(
            loot_condition_row_is_loadable_without_external_stores_like_cpp(&condition(
                0, 0, 0, false
            ))
        );

        assert!(
            !loot_condition_row_is_loadable_without_external_stores_like_cpp(&condition(
                0, 59, 0, false
            ))
        );
        assert!(
            !loot_condition_row_is_loadable_without_external_stores_like_cpp(&condition(
                0, 19, 0, false
            ))
        );

        let mut invalid_target = condition(0, 25, 1, false);
        invalid_target.condition_target = 1;
        assert!(!loot_condition_row_is_loadable_without_external_stores_like_cpp(&invalid_target,));

        assert!(
            !loot_condition_row_is_loadable_without_external_stores_like_cpp(&condition(
                0, 2, 6948, false
            ))
        );
        let mut valid_item = condition(0, 2, 6948, false);
        valid_item.value2 = 1;
        assert!(loot_condition_row_is_loadable_without_external_stores_like_cpp(&valid_item));

        assert!(
            loot_condition_row_is_loadable_without_external_stores_like_cpp(&condition(
                0, 6, 469, false
            ))
        );
        assert!(
            !loot_condition_row_is_loadable_without_external_stores_like_cpp(&condition(
                0, 6, 1, false
            ))
        );

        let mut invalid_class = condition(0, 15, 1, false);
        invalid_class.value1 = 1 << 13;
        assert!(!loot_condition_row_is_loadable_without_external_stores_like_cpp(&invalid_class,));
        let mut valid_class = condition(0, 15, 1, false);
        valid_class.value1 = 1 << 12;
        assert!(loot_condition_row_is_loadable_without_external_stores_like_cpp(&valid_class));

        let mut invalid_race = condition(0, 16, 1, false);
        invalid_race.value1 = 1 << 22;
        assert!(!loot_condition_row_is_loadable_without_external_stores_like_cpp(&invalid_race,));
        let mut valid_remapped_race = condition(0, 16, 1, false);
        valid_remapped_race.value1 = 1 << 11;
        assert!(
            loot_condition_row_is_loadable_without_external_stores_like_cpp(&valid_remapped_race,)
        );

        let mut invalid_level = condition(0, 27, 80, false);
        invalid_level.value2 = 5;
        assert!(!loot_condition_row_is_loadable_without_external_stores_like_cpp(&invalid_level,));

        let mut invalid_drunkenstate = condition(0, 10, 4, false);
        invalid_drunkenstate.value1 = 4;
        assert!(
            !loot_condition_row_is_loadable_without_external_stores_like_cpp(&invalid_drunkenstate,)
        );

        assert!(
            !loot_condition_row_is_loadable_without_external_stores_like_cpp(&condition(
                0, 35, 0, false
            ))
        );

        let mut invalid_hp_val = condition(0, 37, 1000, false);
        invalid_hp_val.value2 = 5;
        assert!(!loot_condition_row_is_loadable_without_external_stores_like_cpp(&invalid_hp_val,));
        let mut invalid_hp_pct_value = condition(0, 38, 101, false);
        invalid_hp_pct_value.value1 = 101;
        assert!(
            !loot_condition_row_is_loadable_without_external_stores_like_cpp(&invalid_hp_pct_value,)
        );
        let mut invalid_hp_pct_compare = condition(0, 38, 50, false);
        invalid_hp_pct_compare.value2 = 5;
        assert!(
            !loot_condition_row_is_loadable_without_external_stores_like_cpp(
                &invalid_hp_pct_compare,
            )
        );

        let mut instance_guid_data = condition(0, 13, 1, false);
        instance_guid_data.value3 = 1;
        assert!(
            !loot_condition_row_is_loadable_without_external_stores_like_cpp(&instance_guid_data,)
        );

        assert!(
            !loot_condition_row_is_loadable_without_external_stores_like_cpp(&condition(
                0, 33, 0, false
            ))
        );
        assert!(
            !loot_condition_row_is_loadable_without_external_stores_like_cpp(&condition(
                0, 34, 0, false
            ))
        );

        let mut invalid_stand_state_mode = condition(0, 42, 2, false);
        invalid_stand_state_mode.value1 = 2;
        assert!(
            !loot_condition_row_is_loadable_without_external_stores_like_cpp(
                &invalid_stand_state_mode,
            )
        );
        let mut invalid_stand_state_value = condition(0, 42, 0, false);
        invalid_stand_state_value.value2 = 10;
        assert!(
            !loot_condition_row_is_loadable_without_external_stores_like_cpp(
                &invalid_stand_state_value,
            )
        );

        let mut invalid_pet_type = condition(0, 45, 16, false);
        invalid_pet_type.value1 = 16;
        assert!(
            !loot_condition_row_is_loadable_without_external_stores_like_cpp(&invalid_pet_type,)
        );

        assert!(
            !loot_condition_row_is_loadable_without_external_stores_like_cpp(&condition(
                0, 52, 0, false
            ))
        );
        let mut invalid_type_mask = condition(0, 52, 0x02, false);
        invalid_type_mask.value1 = 0x02;
        assert!(
            !loot_condition_row_is_loadable_without_external_stores_like_cpp(&invalid_type_mask,)
        );
        let mut valid_type_mask = condition(0, 52, 0x40, false);
        valid_type_mask.value1 = 0x40;
        assert!(loot_condition_row_is_loadable_without_external_stores_like_cpp(&valid_type_mask));

        let mut invalid_quest_state = condition(0, 47, 1, false);
        invalid_quest_state.value2 = 128;
        assert!(
            !loot_condition_row_is_loadable_without_external_stores_like_cpp(&invalid_quest_state,)
        );

        let mut reference_with_useless_target = condition(0, -42, 0, false);
        reference_with_useless_target.condition_target = 1;
        assert!(
            loot_condition_row_is_loadable_without_external_stores_like_cpp(
                &reference_with_useless_target,
            )
        );
    }

    #[test]
    fn loot_condition_legacy_object_and_type_mask_rows_normalize_like_cpp_load() {
        let legacy_player_object = condition(0, 31, 4, false);
        let normalized =
            loot_condition_row_normalize_without_external_stores_like_cpp(legacy_player_object)
                .unwrap();
        assert_eq!(normalized.condition_type_or_reference, 51);
        assert_eq!(normalized.value1, 6);

        let legacy_player_mask = condition(0, 32, 1 << 4, false);
        let normalized =
            loot_condition_row_normalize_without_external_stores_like_cpp(legacy_player_mask)
                .unwrap();
        assert_eq!(normalized.condition_type_or_reference, 52);
        assert_eq!(normalized.value1, 0x40);

        let legacy_item_object = condition(0, 31, 1, false);
        assert!(
            !loot_condition_row_is_loadable_without_external_stores_like_cpp(&legacy_item_object)
        );
        assert!(
            loot_condition_row_normalize_without_external_stores_like_cpp(legacy_item_object)
                .is_none()
        );

        let invalid_object_type = condition(0, 51, 7, false);
        assert!(
            !loot_condition_row_is_loadable_without_external_stores_like_cpp(&invalid_object_type)
        );
    }

    #[test]
    fn condition_compare_matches_cpp_compare_values_order() {
        assert_eq!(condition_compare_values_like_cpp(0, 10, 10), Some(true));
        assert_eq!(condition_compare_values_like_cpp(1, 11, 10), Some(true));
        assert_eq!(condition_compare_values_like_cpp(2, 9, 10), Some(true));
        assert_eq!(condition_compare_values_like_cpp(3, 10, 10), Some(true));
        assert_eq!(condition_compare_values_like_cpp(4, 10, 10), Some(true));
        assert_eq!(condition_compare_values_like_cpp(5, 10, 10), None);
    }

    #[test]
    fn loot_template_add_entry_matches_cpp_group_split() {
        let mut template = LootTemplate::default();
        template.add_entry_like_cpp(item(25, 0, 100.0, 0));
        template.add_entry_like_cpp(item(26, 0, 0.0, 2));
        template.add_entry_like_cpp(item(0, 900, 50.0, 2));

        assert_eq!(template.entries().len(), 2);
        assert_eq!(template.groups().len(), 2);
        assert!(template.groups()[0].equal_chanced().is_empty());
        assert_eq!(template.groups()[1].equal_chanced().len(), 1);
        assert_eq!(template.entries()[1].reference, 900);
    }

    #[test]
    fn loot_store_definitions_match_cpp_globals() {
        let definitions: Vec<_> = LootStoreKind::ALL_LIKE_CPP
            .iter()
            .map(|kind| kind.definition_like_cpp())
            .collect();

        assert_eq!(definitions.len(), 12);
        assert_eq!(definitions[0].table_name, "creature_loot_template");
        assert_eq!(definitions[0].entry_name, "creature entry");
        assert!(definitions[0].rates_allowed);
        assert_eq!(definitions[1].table_name, "disenchant_loot_template");
        assert_eq!(definitions[9].table_name, "reference_loot_template");
        assert!(!definitions[9].rates_allowed);
        assert_eq!(definitions[11].table_name, "spell_loot_template");
        assert!(!definitions[11].rates_allowed);
    }

    #[test]
    fn loot_store_load_rows_matches_cpp_clear_validate_and_collect_shape() {
        let mut store = LootStore::for_kind_like_cpp(LootStoreKind::Disenchant);
        let rows = [
            LootTemplateRow {
                entry: 100,
                item: item(25, 0, 100.0, 0),
            },
            LootTemplateRow {
                entry: 100,
                item: item(26, 0, 0.0, 2),
            },
            LootTemplateRow {
                entry: 101,
                item: item(0, 700, 50.0, 0),
            },
            LootTemplateRow {
                entry: 102,
                item: item(0, 0, 100.0, 0),
            },
        ];

        let loaded = store
            .load_rows_like_cpp(rows, |item_id| item_id == 25 || item_id == 26)
            .unwrap();

        assert_eq!(loaded, 3);
        assert!(store.have_loot_for(100));
        assert!(store.have_loot_for(101));
        assert!(!store.have_loot_for(102));
        assert_eq!(store.get_loot_for(100).unwrap().entries().len(), 1);
        assert_eq!(
            store.get_loot_for(100).unwrap().groups()[1]
                .equal_chanced()
                .len(),
            1
        );
        assert_eq!(store.collect_loot_ids_like_cpp().len(), 2);
    }

    #[test]
    fn loot_store_load_rows_rejects_cpp_invalid_group_id() {
        let mut store = LootStore::for_kind_like_cpp(LootStoreKind::Item);
        let err = store
            .load_rows_like_cpp(
                [LootTemplateRow {
                    entry: 100,
                    item: LootStoreItem {
                        group_id: 128,
                        ..item(25, 0, 100.0, 0)
                    },
                }],
                |_| true,
            )
            .unwrap_err();

        assert_eq!(
            err,
            LootStoreLoadError::InvalidGroupId {
                table_name: "item_loot_template",
                entry: 100,
                item_id: 25,
                group_id: 128,
            }
        );
    }

    #[test]
    fn loot_reference_check_matches_cpp_used_missing_and_unused_refs() {
        let mut reference = LootStore::for_kind_like_cpp(LootStoreKind::Reference);
        reference
            .load_rows_like_cpp(
                [
                    LootTemplateRow {
                        entry: 700,
                        item: item(25, 0, 100.0, 0),
                    },
                    LootTemplateRow {
                        entry: 701,
                        item: item(26, 0, 100.0, 0),
                    },
                ],
                |_| true,
            )
            .unwrap();

        let mut creature = LootStore::for_kind_like_cpp(LootStoreKind::Creature);
        creature
            .load_rows_like_cpp(
                [
                    LootTemplateRow {
                        entry: 100,
                        item: item(0, 700, 50.0, 0),
                    },
                    LootTemplateRow {
                        entry: 101,
                        item: item(0, 999, 25.0, 0),
                    },
                ],
                |_| true,
            )
            .unwrap();

        let mut stores = LootStores::new();
        stores.insert(LootStoreKind::Reference, reference);
        stores.insert(LootStoreKind::Creature, creature);

        assert_eq!(
            check_loot_references_like_cpp(&stores),
            LootReferenceCheckReport {
                missing_references: vec![LootReferenceUse {
                    store_kind: LootStoreKind::Creature,
                    entry: 101,
                    item_id: 0,
                    reference: 999,
                }],
                unused_reference_ids: vec![701],
            }
        );
    }

    #[test]
    fn loot_reference_check_includes_reference_store_self_refs_like_cpp() {
        let mut reference = LootStore::for_kind_like_cpp(LootStoreKind::Reference);
        reference
            .load_rows_like_cpp(
                [
                    LootTemplateRow {
                        entry: 700,
                        item: item(0, 701, 100.0, 0),
                    },
                    LootTemplateRow {
                        entry: 701,
                        item: item(25, 0, 100.0, 0),
                    },
                ],
                |_| true,
            )
            .unwrap();

        let mut stores = LootStores::new();
        stores.insert(LootStoreKind::Reference, reference);

        assert_eq!(
            check_loot_references_like_cpp(&stores),
            LootReferenceCheckReport {
                missing_references: Vec::new(),
                unused_reference_ids: vec![700],
            }
        );
    }

    #[test]
    fn loot_condition_link_check_matches_cpp_source_and_item_guards() {
        let mut creature = LootStore::for_kind_like_cpp(LootStoreKind::Creature);
        creature
            .load_rows_like_cpp(
                [
                    LootTemplateRow {
                        entry: 100,
                        item: item(25, 0, 100.0, 0),
                    },
                    LootTemplateRow {
                        entry: 100,
                        item: item(26, 0, 0.0, 2),
                    },
                    LootTemplateRow {
                        entry: 100,
                        item: item(0, 700, 50.0, 0),
                    },
                ],
                |item_id| item_id == 25 || item_id == 26,
            )
            .unwrap();

        let mut stores = LootStores::new();
        stores.insert(LootStoreKind::Creature, creature);

        let report = check_loot_condition_links_like_cpp(
            &stores,
            [
                LootConditionId {
                    source_type: 1,
                    source_group: 100,
                    source_entry: 25,
                },
                LootConditionId {
                    source_type: 1,
                    source_group: 100,
                    source_entry: 26,
                },
                LootConditionId {
                    source_type: 1,
                    source_group: 100,
                    source_entry: 0,
                },
                LootConditionId {
                    source_type: 1,
                    source_group: 999,
                    source_entry: 25,
                },
                LootConditionId {
                    source_type: 1,
                    source_group: 100,
                    source_entry: 27,
                },
                LootConditionId {
                    source_type: 1,
                    source_group: 100,
                    source_entry: 28,
                },
                LootConditionId {
                    source_type: 99,
                    source_group: 100,
                    source_entry: 25,
                },
            ],
            |item_id| matches!(item_id, 25 | 26 | 28),
        );

        assert_eq!(
            report,
            LootConditionLinkReport {
                linked: 3,
                unsupported_source_types: vec![LootConditionId {
                    source_type: 99,
                    source_group: 100,
                    source_entry: 25,
                }],
                missing_templates: vec![MissingLootConditionTemplate {
                    condition_id: LootConditionId {
                        source_type: 1,
                        source_group: 999,
                        source_entry: 25,
                    },
                    store_kind: LootStoreKind::Creature,
                }],
                missing_item_templates: vec![MissingLootConditionItemTemplate {
                    condition_id: LootConditionId {
                        source_type: 1,
                        source_group: 100,
                        source_entry: 27,
                    },
                    store_kind: LootStoreKind::Creature,
                }],
                missing_template_items: vec![MissingLootConditionTemplateItem {
                    condition_id: LootConditionId {
                        source_type: 1,
                        source_group: 100,
                        source_entry: 28,
                    },
                    store_kind: LootStoreKind::Creature,
                }],
                missing_reference_templates: Vec::new(),
            }
        );
    }

    #[test]
    fn loot_condition_reference_check_reports_missing_templates_like_cpp_load_guard() {
        let mut report = LootConditionLinkReport {
            linked: 0,
            unsupported_source_types: Vec::new(),
            missing_templates: Vec::new(),
            missing_item_templates: Vec::new(),
            missing_template_items: Vec::new(),
            missing_reference_templates: Vec::new(),
        };

        let uses = [
            LootConditionReferenceUseLikeCpp {
                condition_id: LootConditionId {
                    source_type: 1,
                    source_group: 100,
                    source_entry: 25,
                },
                reference_id: 42,
            },
            LootConditionReferenceUseLikeCpp {
                condition_id: LootConditionId {
                    source_type: 5,
                    source_group: 200,
                    source_entry: 26,
                },
                reference_id: 77,
            },
        ];
        check_loot_condition_references_like_cpp(&mut report, uses, [42]);

        assert_eq!(
            report.missing_reference_templates,
            vec![LootConditionReferenceUseLikeCpp {
                condition_id: LootConditionId {
                    source_type: 5,
                    source_group: 200,
                    source_entry: 26,
                },
                reference_id: 77,
            }]
        );
        assert!(!report.is_clean());
    }

    #[test]
    fn fill_loot_processes_plain_group_and_reference_entries_like_cpp() {
        let mut creature = LootStore::for_kind_like_cpp(LootStoreKind::Creature);
        creature
            .load_rows_like_cpp(
                [
                    LootTemplateRow {
                        entry: 100,
                        item: item(25, 0, 100.0, 0),
                    },
                    LootTemplateRow {
                        entry: 100,
                        item: item(26, 0, 0.0, 1),
                    },
                    LootTemplateRow {
                        entry: 100,
                        item: item(0, 700, 100.0, 0),
                    },
                ],
                |item_id| matches!(item_id, 25 | 26),
            )
            .unwrap();

        let mut reference = LootStore::for_kind_like_cpp(LootStoreKind::Reference);
        reference
            .load_rows_like_cpp(
                [LootTemplateRow {
                    entry: 700,
                    item: item(27, 0, 100.0, 0),
                }],
                |item_id| item_id == 27,
            )
            .unwrap();

        let mut stores = LootStores::new();
        stores.insert(LootStoreKind::Creature, creature);
        stores.insert(LootStoreKind::Reference, reference);

        let mut rng = StdRng::seed_from_u64(7);
        let generated = stores[&LootStoreKind::Creature]
            .fill_loot_like_cpp(
                100,
                LootStoreKind::Creature,
                &stores,
                LootFillOptions::default(),
                &mut rng,
                |_| Some(item_metadata(20)),
                |_| 1.0,
                |_| true,
                random_properties,
            )
            .unwrap();

        assert_eq!(
            generated,
            vec![
                generated_item(25, 1, 0),
                generated_item(27, 1, 1),
                generated_item(26, 1, 2),
            ]
        );
    }

    #[test]
    fn fill_personal_loot_assigns_plain_entries_to_one_looter_like_cpp() {
        let mut creature = LootStore::for_kind_like_cpp(LootStoreKind::Creature);
        creature
            .load_rows_like_cpp(
                [
                    LootTemplateRow {
                        entry: 100,
                        item: item(25, 0, 100.0, 0),
                    },
                    LootTemplateRow {
                        entry: 100,
                        item: item(26, 0, 100.0, 0),
                    },
                ],
                |item_id| matches!(item_id, 25 | 26),
            )
            .unwrap();

        let mut stores = LootStores::new();
        stores.insert(LootStoreKind::Creature, creature);

        let looters = [guid(42), guid(77)];
        let mut rng = StdRng::seed_from_u64(7);
        let generated = stores[&LootStoreKind::Creature]
            .fill_personal_loot_with_context_like_cpp(
                100,
                LootStoreKind::Creature,
                &stores,
                LootFillOptions::default(),
                &looters,
                &mut rng,
                |_| Some(item_metadata(20)),
                |_| 1.0,
                |_, _| true,
                random_properties,
            )
            .unwrap();

        assert_eq!(generated.len(), 2);
        assert!(generated.iter().all(|item| looters.contains(&item.looter)));
        assert!(generated.iter().all(|item| item.item.loot_list_id < 2));
        assert_eq!(
            generated
                .iter()
                .map(|item| item.item.item_id)
                .collect::<Vec<_>>(),
            vec![25, 26]
        );
    }

    #[test]
    fn fill_personal_loot_reference_cycles_looters_like_cpp() {
        let mut creature = LootStore::for_kind_like_cpp(LootStoreKind::Creature);
        let mut reference_item = item(0, 700, 100.0, 0);
        reference_item.max_count = 2;
        creature
            .load_rows_like_cpp(
                [LootTemplateRow {
                    entry: 100,
                    item: reference_item,
                }],
                |_| true,
            )
            .unwrap();

        let mut reference = LootStore::for_kind_like_cpp(LootStoreKind::Reference);
        reference
            .load_rows_like_cpp(
                [LootTemplateRow {
                    entry: 700,
                    item: item(27, 0, 100.0, 0),
                }],
                |item_id| item_id == 27,
            )
            .unwrap();

        let mut stores = LootStores::new();
        stores.insert(LootStoreKind::Creature, creature);
        stores.insert(LootStoreKind::Reference, reference);

        let looters = [guid(42), guid(77)];
        let mut rng = StdRng::seed_from_u64(3);
        let generated = stores[&LootStoreKind::Creature]
            .fill_personal_loot_with_context_like_cpp(
                100,
                LootStoreKind::Creature,
                &stores,
                LootFillOptions::default(),
                &looters,
                &mut rng,
                |_| Some(item_metadata(20)),
                |_| 1.0,
                |_, _| true,
                random_properties,
            )
            .unwrap();

        let mut assigned = generated.iter().map(|item| item.looter).collect::<Vec<_>>();
        assigned.sort_by_key(|guid| guid.counter());
        assert_eq!(assigned, looters);
        assert_eq!(
            generated
                .iter()
                .map(|item| item.item.item_id)
                .collect::<Vec<_>>(),
            vec![27, 27]
        );
        assert_eq!(
            generated
                .iter()
                .map(|item| item.item.loot_list_id)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
    }

    #[test]
    fn fill_loot_with_context_reports_reference_source_like_cpp_conditions() {
        let mut reference = LootStore::for_kind_like_cpp(LootStoreKind::Reference);
        reference
            .load_rows_like_cpp(
                [LootTemplateRow {
                    entry: 900,
                    item: item(27, 0, 100.0, 0),
                }],
                |item_id| item_id == 27,
            )
            .unwrap();

        let mut creature = LootStore::for_kind_like_cpp(LootStoreKind::Creature);
        creature
            .load_rows_like_cpp(
                [LootTemplateRow {
                    entry: 100,
                    item: LootStoreItem {
                        reference: 900,
                        max_count: 1,
                        ..item(900, 900, 100.0, 0)
                    },
                }],
                |_| false,
            )
            .unwrap();

        let mut stores = LootStores::new();
        stores.insert(LootStoreKind::Creature, creature);
        stores.insert(LootStoreKind::Reference, reference);

        let mut seen = Vec::new();
        let mut rng = StdRng::seed_from_u64(7);
        let generated = stores[&LootStoreKind::Creature]
            .fill_loot_with_context_like_cpp(
                100,
                LootStoreKind::Creature,
                &stores,
                LootFillOptions::default(),
                &mut rng,
                |_| Some(item_metadata(20)),
                |_| 1.0,
                |context| {
                    seen.push((context.store_kind, context.entry, context.item.item_id));
                    true
                },
                random_properties,
            )
            .unwrap();

        assert_eq!(generated, vec![generated_item(27, 1, 0)]);
        assert_eq!(seen, vec![(LootStoreKind::Reference, 900, 27)]);
        assert_eq!(
            stores[&LootStoreKind::Creature].condition_ids_for_fill_like_cpp(
                100,
                LootStoreKind::Creature,
                &stores,
            ),
            vec![LootConditionId {
                source_type: 10,
                source_group: 900,
                source_entry: 27,
            }]
        );
    }

    #[test]
    fn fill_loot_splits_stacks_and_caps_at_cpp_max_loot_items() {
        let mut store = LootStore::for_kind_like_cpp(LootStoreKind::Item);
        store
            .load_rows_like_cpp(
                [LootTemplateRow {
                    entry: 500,
                    item: LootStoreItem {
                        min_count: 40,
                        max_count: 40,
                        ..item(25, 0, 100.0, 0)
                    },
                }],
                |item_id| item_id == 25,
            )
            .unwrap();

        let mut stores = LootStores::new();
        stores.insert(LootStoreKind::Item, store);

        let mut rng = StdRng::seed_from_u64(11);
        let generated = stores[&LootStoreKind::Item]
            .fill_loot_like_cpp(
                500,
                LootStoreKind::Item,
                &stores,
                LootFillOptions::default(),
                &mut rng,
                |_| Some(item_metadata(1)),
                |_| 1.0,
                |_| true,
                random_properties,
            )
            .unwrap();

        assert_eq!(generated.len(), 18);
        assert_eq!(generated[0].loot_list_id, 0);
        assert_eq!(generated[17].loot_list_id, 17);
        assert!(
            generated
                .iter()
                .all(|item| item.item_id == 25 && item.count == 1)
        );
    }

    #[test]
    fn fill_loot_initializes_loot_item_metadata_like_cpp_constructor() {
        let mut store = LootStore::for_kind_like_cpp(LootStoreKind::Item);
        store
            .load_rows_like_cpp(
                [LootTemplateRow {
                    entry: 600,
                    item: LootStoreItem {
                        needs_quest: true,
                        ..item(25, 0, 100.0, 0)
                    },
                }],
                |item_id| item_id == 25,
            )
            .unwrap();

        let mut stores = LootStores::new();
        stores.insert(LootStoreKind::Item, store);

        let mut rng = StdRng::seed_from_u64(13);
        let generated = stores[&LootStoreKind::Item]
            .fill_loot_like_cpp(
                600,
                LootStoreKind::Item,
                &stores,
                LootFillOptions {
                    item_context: 4,
                    ..LootFillOptions::default()
                },
                &mut rng,
                |_| {
                    Some(LootItemTemplateMetadata {
                        max_stack: 20,
                        has_multi_drop_flag: true,
                        has_follow_loot_rules_flag: true,
                    })
                },
                |_| 1.0,
                |_| true,
                |_, _| LootItemRandomProperties {
                    id: 4242,
                    seed: 2424,
                },
            )
            .unwrap();

        assert_eq!(
            generated,
            vec![GeneratedLootItem {
                item_id: 25,
                count: 1,
                loot_list_id: 0,
                random_properties_id: 4242,
                random_properties_seed: 2424,
                context: 4,
                free_for_all: true,
                follow_loot_rules: true,
                needs_quest: true,
                is_looted: false,
                is_blocked: false,
                is_under_threshold: false,
                is_counted: false,
            }]
        );
    }

    #[test]
    fn fill_loot_random_properties_callback_uses_fill_rng_like_cpp() {
        let mut store = LootStore::for_kind_like_cpp(LootStoreKind::Item);
        store
            .load_rows_like_cpp(
                [LootTemplateRow {
                    entry: 601,
                    item: item(25, 0, 100.0, 0),
                }],
                |item_id| item_id == 25,
            )
            .unwrap();

        let mut stores = LootStores::new();
        stores.insert(LootStoreKind::Item, store);

        let mut expected_rng = StdRng::seed_from_u64(14);
        let _: u32 = expected_rng.gen_range(1..=1);
        let expected_id = expected_rng.gen_range(1..=10_000);

        let mut rng = StdRng::seed_from_u64(14);
        let generated = stores[&LootStoreKind::Item]
            .fill_loot_like_cpp(
                601,
                LootStoreKind::Item,
                &stores,
                LootFillOptions::default(),
                &mut rng,
                |_| Some(item_metadata(20)),
                |_| 1.0,
                |_| true,
                |_, rng| LootItemRandomProperties {
                    id: rng.gen_range(1..=10_000),
                    seed: 0,
                },
            )
            .unwrap();

        assert_eq!(generated[0].random_properties_id, expected_id);
    }

    #[test]
    fn fill_loot_reports_missing_template_like_cpp_no_empty_error_branch() {
        let store = LootStore::for_kind_like_cpp(LootStoreKind::Creature);
        let mut stores = LootStores::new();
        stores.insert(LootStoreKind::Creature, store);

        let mut rng = StdRng::seed_from_u64(1);
        let err = stores[&LootStoreKind::Creature]
            .fill_loot_like_cpp(
                999,
                LootStoreKind::Creature,
                &stores,
                LootFillOptions::default(),
                &mut rng,
                |_| Some(item_metadata(1)),
                |_| 1.0,
                |_| true,
                random_properties,
            )
            .unwrap_err();

        assert_eq!(err, LootFillError::MissingLootTemplate { loot_id: 999 });
    }

    #[test]
    fn generate_money_loot_matches_cpp_boundary_branches() {
        let mut rng = StdRng::seed_from_u64(0xC0FFEE);
        assert_eq!(
            generate_money_loot_with_rate_like_cpp(0, 0, 1.0, &mut rng),
            0
        );
        assert_eq!(
            generate_money_loot_with_rate_like_cpp(120, 100, 1.0, &mut rng),
            100
        );
        assert_eq!(
            generate_money_loot_with_rate_like_cpp(120, 100, 2.5, &mut rng),
            250
        );

        let small_range = generate_money_loot_with_rate_like_cpp(100, 200, 1.0, &mut rng);
        assert!((100..=200).contains(&small_range));

        let wide_range = generate_money_loot_with_rate_like_cpp(1_000, 100_000, 1.0, &mut rng);
        assert_eq!(wide_range & 0xFF, 0);
        assert!((((1_000 >> 8) << 8)..=((100_000 >> 8) << 8)).contains(&wide_range));
    }

    #[test]
    fn loot_item_ui_type_hides_looted_or_disallowed_rows_like_cpp() {
        let player = guid(42);
        let other = guid(77);

        assert_eq!(
            ui_type(
                player,
                &[player],
                true,
                false,
                false,
                false,
                true,
                LOOT_METHOD_FREE_FOR_ALL_LIKE_CPP,
                ObjectGuid::EMPTY,
                ObjectGuid::EMPTY,
                false,
                false,
                ObjectGuid::EMPTY,
            ),
            None
        );
        assert_eq!(
            ui_type(
                player,
                &[other],
                false,
                false,
                false,
                false,
                true,
                LOOT_METHOD_FREE_FOR_ALL_LIKE_CPP,
                ObjectGuid::EMPTY,
                ObjectGuid::EMPTY,
                false,
                false,
                ObjectGuid::EMPTY,
            ),
            None
        );
    }

    #[test]
    fn loot_item_ui_type_free_for_all_and_quest_paths_match_cpp() {
        let player = guid(42);

        assert_eq!(
            ui_type(
                player,
                &[player],
                false,
                true,
                true,
                false,
                true,
                LOOT_METHOD_FREE_FOR_ALL_LIKE_CPP,
                ObjectGuid::EMPTY,
                ObjectGuid::EMPTY,
                false,
                false,
                ObjectGuid::EMPTY,
            ),
            Some(LOOT_SLOT_TYPE_OWNER_LIKE_CPP)
        );
        assert_eq!(
            ui_type(
                player,
                &[player],
                false,
                true,
                true,
                false,
                true,
                LOOT_METHOD_GROUP_LIKE_CPP,
                ObjectGuid::EMPTY,
                ObjectGuid::EMPTY,
                false,
                false,
                ObjectGuid::EMPTY,
            ),
            Some(LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP)
        );
        assert_eq!(
            ui_type(
                player,
                &[player],
                false,
                true,
                false,
                false,
                true,
                LOOT_METHOD_FREE_FOR_ALL_LIKE_CPP,
                ObjectGuid::EMPTY,
                ObjectGuid::EMPTY,
                false,
                false,
                ObjectGuid::EMPTY,
            ),
            None
        );
        assert_eq!(
            ui_type(
                player,
                &[player],
                false,
                false,
                false,
                true,
                false,
                LOOT_METHOD_GROUP_LIKE_CPP,
                ObjectGuid::EMPTY,
                ObjectGuid::EMPTY,
                false,
                false,
                ObjectGuid::EMPTY,
            ),
            Some(LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP)
        );
    }

    #[test]
    fn loot_item_ui_type_round_robin_and_master_loot_match_cpp() {
        let player = guid(42);
        let other = guid(77);

        assert_eq!(
            ui_type(
                player,
                &[player],
                false,
                false,
                false,
                false,
                true,
                LOOT_METHOD_ROUND_ROBIN_LIKE_CPP,
                other,
                ObjectGuid::EMPTY,
                false,
                false,
                ObjectGuid::EMPTY,
            ),
            None
        );
        assert_eq!(
            ui_type(
                player,
                &[player],
                false,
                false,
                false,
                false,
                true,
                LOOT_METHOD_ROUND_ROBIN_LIKE_CPP,
                ObjectGuid::EMPTY,
                ObjectGuid::EMPTY,
                false,
                false,
                ObjectGuid::EMPTY,
            ),
            Some(LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP)
        );
        assert_eq!(
            ui_type(
                player,
                &[player],
                false,
                false,
                false,
                false,
                true,
                LOOT_METHOD_MASTER_LIKE_CPP,
                ObjectGuid::EMPTY,
                player,
                false,
                false,
                ObjectGuid::EMPTY,
            ),
            Some(LOOT_SLOT_TYPE_MASTER_LIKE_CPP)
        );
        assert_eq!(
            ui_type(
                player,
                &[player],
                false,
                false,
                false,
                false,
                true,
                LOOT_METHOD_MASTER_LIKE_CPP,
                ObjectGuid::EMPTY,
                other,
                false,
                false,
                ObjectGuid::EMPTY,
            ),
            Some(LOOT_SLOT_TYPE_LOCKED_LIKE_CPP)
        );
        assert_eq!(
            ui_type(
                player,
                &[player],
                false,
                false,
                false,
                false,
                true,
                LOOT_METHOD_MASTER_LIKE_CPP,
                ObjectGuid::EMPTY,
                other,
                true,
                false,
                ObjectGuid::EMPTY,
            ),
            Some(LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP)
        );
    }

    #[test]
    fn loot_item_ui_type_group_roll_paths_match_cpp() {
        let player = guid(42);
        let other = guid(77);

        assert_eq!(
            ui_type(
                player,
                &[player],
                false,
                false,
                false,
                false,
                true,
                LOOT_METHOD_GROUP_LIKE_CPP,
                other,
                ObjectGuid::EMPTY,
                true,
                false,
                ObjectGuid::EMPTY,
            ),
            None
        );
        assert_eq!(
            ui_type(
                player,
                &[player],
                false,
                false,
                false,
                false,
                true,
                LOOT_METHOD_NEED_BEFORE_GREED_LIKE_CPP,
                ObjectGuid::EMPTY,
                ObjectGuid::EMPTY,
                false,
                true,
                ObjectGuid::EMPTY,
            ),
            Some(LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP)
        );
        assert_eq!(
            ui_type(
                player,
                &[player],
                false,
                false,
                false,
                false,
                true,
                LOOT_METHOD_GROUP_LIKE_CPP,
                ObjectGuid::EMPTY,
                ObjectGuid::EMPTY,
                false,
                false,
                ObjectGuid::EMPTY,
            ),
            Some(LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP)
        );
        assert_eq!(
            ui_type(
                player,
                &[player],
                false,
                false,
                false,
                false,
                true,
                LOOT_METHOD_GROUP_LIKE_CPP,
                ObjectGuid::EMPTY,
                ObjectGuid::EMPTY,
                false,
                false,
                player,
            ),
            Some(LOOT_SLOT_TYPE_OWNER_LIKE_CPP)
        );
        assert_eq!(
            ui_type(
                player,
                &[player],
                false,
                false,
                false,
                false,
                true,
                LOOT_METHOD_GROUP_LIKE_CPP,
                ObjectGuid::EMPTY,
                ObjectGuid::EMPTY,
                false,
                false,
                other,
            ),
            None
        );
    }

    #[test]
    fn loot_item_ui_type_personal_loot_matches_cpp() {
        let player = guid(42);

        assert_eq!(
            ui_type(
                player,
                &[player],
                false,
                false,
                false,
                false,
                true,
                LOOT_METHOD_PERSONAL_LIKE_CPP,
                ObjectGuid::EMPTY,
                ObjectGuid::EMPTY,
                false,
                false,
                ObjectGuid::EMPTY,
            ),
            Some(LOOT_SLOT_TYPE_OWNER_LIKE_CPP)
        );
    }

    #[test]
    fn have_quest_loot_follows_reference_templates_like_cpp() {
        let mut gameobject_store = LootStore::for_kind_like_cpp(LootStoreKind::Gameobject);
        gameobject_store
            .load_rows_like_cpp(
                [LootTemplateRow {
                    entry: 700,
                    item: item(701, 701, 100.0, 0),
                }],
                |_| true,
            )
            .unwrap();
        let mut quest_item = item(9001, 0, 100.0, 0);
        quest_item.needs_quest = true;
        let mut reference_store = LootStore::for_kind_like_cpp(LootStoreKind::Reference);
        reference_store
            .load_rows_like_cpp(
                [LootTemplateRow {
                    entry: 701,
                    item: quest_item,
                }],
                |_| true,
            )
            .unwrap();
        let mut stores = LootStores::new();
        stores.insert(LootStoreKind::Gameobject, gameobject_store);
        stores.insert(LootStoreKind::Reference, reference_store);
        let store = stores.get(&LootStoreKind::Gameobject).unwrap();

        assert!(store.have_quest_loot_for_like_cpp(700, &stores));
        assert!(store.have_quest_loot_for_player_like_cpp(700, &stores, |item_id| item_id == 9001));
        assert!(!store.have_quest_loot_for_player_like_cpp(700, &stores, |item_id| item_id == 42));
    }
}
