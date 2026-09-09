//! Loot store definitions and templates state definitions, part 1 of 4.
//!
//! Separated from the lib.rs root under #642. Behaviour is preserved.

use super::*;

pub(super) const MIN_NON_ZERO_LOOT_CHANCE_LIKE_CPP: f32 = 0.000001;

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

pub(super) const CONDITION_MAX_LIKE_CPP: i32 = 59;

pub(super) const CONDITION_SPAWNMASK_DEPRECATED_LIKE_CPP: i32 = 19;

pub(super) const CONDITION_ITEM_LIKE_CPP: i32 = 2;

pub(super) const CONDITION_TEAM_LIKE_CPP: i32 = 6;

pub(super) const CONDITION_INSTANCE_INFO_LIKE_CPP: i32 = 13;

pub(super) const CONDITION_CLASS_LIKE_CPP: i32 = 15;

pub(super) const CONDITION_RACE_LIKE_CPP: i32 = 16;

pub(super) const CONDITION_GENDER_LIKE_CPP: i32 = 20;

pub(super) const CONDITION_OBJECT_ENTRY_GUID_LEGACY_LIKE_CPP: i32 = 31;

pub(super) const CONDITION_TYPE_MASK_LEGACY_LIKE_CPP: i32 = 32;

pub(super) const CONDITION_DRUNKENSTATE_LIKE_CPP: i32 = 10;

pub(super) const CONDITION_LEVEL_LIKE_CPP: i32 = 27;

pub(super) const CONDITION_RELATION_TO_LIKE_CPP: i32 = 33;

pub(super) const CONDITION_REACTION_TO_LIKE_CPP: i32 = 34;

pub(super) const CONDITION_DISTANCE_TO_LIKE_CPP: i32 = 35;

pub(super) const CONDITION_HP_VAL_LIKE_CPP: i32 = 37;

pub(super) const CONDITION_HP_PCT_LIKE_CPP: i32 = 38;

pub(super) const CONDITION_STAND_STATE_LIKE_CPP: i32 = 42;

pub(super) const CONDITION_PET_TYPE_LIKE_CPP: i32 = 45;

pub(super) const CONDITION_QUESTSTATE_LIKE_CPP: i32 = 47;

pub(super) const CONDITION_OBJECT_ENTRY_GUID_LIKE_CPP: i32 = 51;

pub(super) const CONDITION_TYPE_MASK_LIKE_CPP: i32 = 52;

pub(super) const COMP_TYPE_MAX_LIKE_CPP: u32 = 5;

pub(super) const MAX_QUEST_STATUS_LIKE_CPP: u32 = 7;

pub(super) const ALLIANCE_TEAM_LIKE_CPP: u32 = 469;

pub(super) const HORDE_TEAM_LIKE_CPP: u32 = 67;

pub(super) const GENDER_NONE_LIKE_CPP: u32 = 2;

pub(super) const DRUNKEN_SMASHED_LIKE_CPP: u32 = 3;

pub(super) const CLASSMASK_ALL_PLAYABLE_LIKE_CPP: u32 = (1 << 13) - 1;

pub(super) const RACEMASK_ALL_PLAYABLE_LIKE_CPP: u32 = 0xFFA1_FFFF;

pub(super) const TYPEMASK_CONDITION_ALLOWED_LIKE_CPP: u32 = 0x560;

pub(super) const UNIT_STAND_STATE_SUBMERGED_LIKE_CPP: u32 = 9;

pub(super) const MAX_PET_TYPE_LIKE_CPP: u32 = 4;

pub(super) const INSTANCE_INFO_GUID_DATA_LIKE_CPP: u32 = 1;

pub(super) const TYPEID_OBJECT_LIKE_CPP: u32 = 0;

pub(super) const TYPEID_ITEM_LIKE_CPP: u32 = 1;

pub(super) const TYPEID_CONTAINER_LIKE_CPP: u32 = 2;

pub(super) const TYPEID_UNIT_LIKE_CPP: u32 = 5;

pub(super) const TYPEID_PLAYER_LIKE_CPP: u32 = 6;

pub(super) const TYPEID_GAMEOBJECT_LIKE_CPP: u32 = 8;

pub(super) const TYPEID_DYNAMICOBJECT_LIKE_CPP: u32 = 9;

pub(super) const TYPEID_CORPSE_LIKE_CPP: u32 = 10;

pub(super) const TYPEID_AREATRIGGER_LIKE_CPP: u32 = 11;

pub(super) const TYPEID_SCENEOBJECT_LIKE_CPP: u32 = 12;

pub(super) const TYPEID_CONVERSATION_LIKE_CPP: u32 = 13;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeneratedLootItem {
    pub item_id: u32,
    pub count: u32,
    pub loot_list_id: u32,
    pub random_properties_id: i32,
    pub random_properties_seed: i32,
    pub context: u8,
    pub store_item_context: LootStoreItemContext,
    pub free_for_all: bool,
    pub follow_loot_rules: bool,
    pub needs_quest: bool,
    pub is_looted: bool,
    pub is_blocked: bool,
    pub is_under_threshold: bool,
    pub is_counted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
    pub(super) definition: LootStoreDefinition,
    pub(super) templates: HashMap<u32, LootTemplate>,
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

pub(super) const LOOT_REFERENCE_CHECK_ORDER_LIKE_CPP: [LootStoreKind; 11] = [
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

pub(super) const fn legacy_type_id_to_type_id_like_cpp(legacy_type_id: u32) -> u32 {
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

pub(super) const fn legacy_type_mask_to_type_mask_like_cpp(legacy_type_mask: u32) -> u32 {
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

pub(super) const fn object_entry_guid_type_id_is_loadable_without_external_stores_like_cpp(
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

pub(super) const fn type_mask_is_loadable_without_external_stores_like_cpp(type_mask: u32) -> bool {
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

pub(super) fn loot_conditions_allow_player_inner_like_cpp<F>(
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
