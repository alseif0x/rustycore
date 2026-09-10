use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};

use bitflags::bitflags;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateMask {
    bits: usize,
    blocks: Vec<u32>,
    blocks_mask: Vec<u32>,
}

impl UpdateMask {
    pub fn new(bits: usize) -> Self {
        let block_count = bits.div_ceil(32);
        let blocks_mask_count = block_count.div_ceil(32);
        Self {
            bits,
            blocks: vec![0; block_count],
            blocks_mask: vec![0; blocks_mask_count],
        }
    }

    pub fn from_blocks(bits: usize, input: &[u32]) -> Self {
        let mut mask = Self::new(bits);
        for (block, value) in mask.blocks.iter_mut().zip(input.iter().copied()) {
            *block = value;
        }
        mask.mask_unused_tail_bits();
        mask.rebuild_blocks_mask();
        mask
    }

    pub fn all_bits(bits: usize) -> Self {
        let mut mask = Self::new(bits);
        mask.set_all();
        mask
    }

    pub const fn bits(&self) -> usize {
        self.bits
    }

    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    pub fn blocks_mask_count(&self) -> usize {
        self.blocks_mask.len()
    }

    pub fn blocks(&self) -> &[u32] {
        &self.blocks
    }

    pub fn blocks_mask(&self) -> &[u32] {
        &self.blocks_mask
    }

    pub fn get_block(&self, index: usize) -> u32 {
        self.blocks[index]
    }

    pub fn get_blocks_mask(&self, index: usize) -> u32 {
        self.blocks_mask[index]
    }

    pub fn is_set(&self, index: usize) -> bool {
        assert!(index < self.bits);
        (self.blocks[block_index(index)] & block_flag(index)) != 0
    }

    pub fn is_any_set(&self) -> bool {
        self.blocks_mask.iter().any(|block_mask| *block_mask != 0)
    }

    pub fn set(&mut self, index: usize) {
        assert!(index < self.bits);
        let block = block_index(index);
        self.blocks[block] |= block_flag(index);
        self.blocks_mask[block_index(block)] |= block_flag(block);
    }

    pub fn reset(&mut self, index: usize) {
        assert!(index < self.bits);
        let block = block_index(index);
        self.blocks[block] &= !block_flag(index);
        if self.blocks[block] == 0 {
            self.blocks_mask[block_index(block)] &= !block_flag(block);
        }
    }

    pub fn reset_all(&mut self) {
        self.blocks.fill(0);
        self.blocks_mask.fill(0);
    }

    pub fn set_all(&mut self) {
        self.blocks.fill(u32::MAX);
        self.mask_unused_tail_bits();
        self.rebuild_blocks_mask();
    }

    fn mask_unused_tail_bits(&mut self) {
        if let Some(last) = self.blocks.last_mut() {
            let used_bits = self.bits % 32;
            if used_bits != 0 {
                *last &= u32::MAX >> (32 - used_bits);
            }
        }
    }

    fn rebuild_blocks_mask(&mut self) {
        self.blocks_mask.fill(0);
        for (block, value) in self.blocks.iter().enumerate() {
            if *value != 0 {
                self.blocks_mask[block_index(block)] |= block_flag(block);
            }
        }
    }
}

impl BitAndAssign<&UpdateMask> for UpdateMask {
    fn bitand_assign(&mut self, rhs: &UpdateMask) {
        assert_eq!(self.bits, rhs.bits);
        for (left, right) in self.blocks.iter_mut().zip(&rhs.blocks) {
            *left &= *right;
        }
        self.rebuild_blocks_mask();
    }
}

impl BitOrAssign<&UpdateMask> for UpdateMask {
    fn bitor_assign(&mut self, rhs: &UpdateMask) {
        assert_eq!(self.bits, rhs.bits);
        for (left, right) in self.blocks.iter_mut().zip(&rhs.blocks) {
            *left |= *right;
        }
        self.mask_unused_tail_bits();
        self.rebuild_blocks_mask();
    }
}

impl BitAnd for UpdateMask {
    type Output = Self;

    fn bitand(mut self, rhs: Self) -> Self::Output {
        self &= &rhs;
        self
    }
}

impl BitOr for UpdateMask {
    type Output = Self;

    fn bitor(mut self, rhs: Self) -> Self::Output {
        self |= &rhs;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UpdateFieldSectionKind {
    ObjectData,
    ItemData,
    ContainerData,
    UnitData,
    PlayerData,
    ActivePlayerData,
    GameObjectData,
    DynamicObjectData,
    CorpseData,
    AreaTriggerData,
    SceneObjectData,
    ConversationData,
}

impl UpdateFieldSectionKind {
    pub const ALL: [Self; 12] = [
        Self::ObjectData,
        Self::ItemData,
        Self::ContainerData,
        Self::UnitData,
        Self::PlayerData,
        Self::ActivePlayerData,
        Self::GameObjectData,
        Self::DynamicObjectData,
        Self::CorpseData,
        Self::AreaTriggerData,
        Self::SceneObjectData,
        Self::ConversationData,
    ];

    pub const fn bit_count(self) -> usize {
        match self {
            Self::ObjectData => OBJECT_DATA_BITS,
            Self::ItemData => ITEM_DATA_BITS,
            Self::ContainerData => CONTAINER_DATA_BITS,
            Self::UnitData => UNIT_DATA_BITS,
            Self::PlayerData => PLAYER_DATA_BITS,
            Self::ActivePlayerData => ACTIVE_PLAYER_DATA_BITS,
            Self::GameObjectData => GAME_OBJECT_DATA_BITS,
            Self::DynamicObjectData => DYNAMIC_OBJECT_DATA_BITS,
            Self::CorpseData => CORPSE_DATA_BITS,
            Self::AreaTriggerData => AREA_TRIGGER_DATA_BITS,
            Self::SceneObjectData => SCENE_OBJECT_DATA_BITS,
            Self::ConversationData => CONVERSATION_DATA_BITS,
        }
    }

    pub const fn client_type_bit(self) -> Option<usize> {
        match self {
            Self::ObjectData => Some(TYPEID_OBJECT),
            Self::ItemData => Some(TYPEID_ITEM),
            Self::ContainerData => Some(TYPEID_CONTAINER),
            Self::UnitData => Some(TYPEID_UNIT),
            Self::PlayerData => Some(TYPEID_PLAYER),
            Self::ActivePlayerData => Some(TYPEID_ACTIVE_PLAYER),
            Self::GameObjectData => Some(TYPEID_GAME_OBJECT),
            Self::DynamicObjectData => Some(TYPEID_DYNAMIC_OBJECT),
            Self::CorpseData => Some(TYPEID_CORPSE),
            Self::AreaTriggerData => Some(TYPEID_AREA_TRIGGER),
            Self::SceneObjectData => Some(TYPEID_SCENE_OBJECT),
            Self::ConversationData => Some(TYPEID_CONVERSATION),
        }
    }

    pub const fn metadata(self) -> UpdateFieldSectionMetadata {
        let bit_count = self.bit_count();
        let block_count = const_div_ceil(bit_count, 32);
        UpdateFieldSectionMetadata {
            kind: self,
            bit_count,
            block_count,
            blocks_mask_count: const_div_ceil(block_count, 32),
            client_type_bit: self.client_type_bit(),
        }
    }

    pub const fn descriptors(self) -> &'static [UpdateFieldDescriptor] {
        match self {
            Self::ObjectData => &[],
            Self::ItemData => ITEM_DATA_DESCRIPTORS,
            Self::ContainerData => CONTAINER_DATA_DESCRIPTORS,
            Self::UnitData => UNIT_DATA_DESCRIPTORS,
            Self::PlayerData => PLAYER_DATA_DESCRIPTORS,
            Self::ActivePlayerData => ACTIVE_PLAYER_DATA_DESCRIPTORS,
            Self::GameObjectData => GAME_OBJECT_DATA_DESCRIPTORS,
            Self::DynamicObjectData => &[],
            Self::CorpseData => CORPSE_DATA_DESCRIPTORS,
            Self::AreaTriggerData => AREA_TRIGGER_DATA_DESCRIPTORS,
            Self::SceneObjectData => &[],
            Self::ConversationData => CONVERSATION_DATA_DESCRIPTORS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateFieldSectionMetadata {
    pub kind: UpdateFieldSectionKind,
    pub bit_count: usize,
    pub block_count: usize,
    pub blocks_mask_count: usize,
    pub client_type_bit: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateFieldDescriptorKind {
    Scalar,
    Dynamic,
    DynamicArray,
    Optional,
    Nested,
    Array,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateFieldDescriptor {
    pub name: &'static str,
    pub kind: UpdateFieldDescriptorKind,
    pub bit: usize,
    pub first_child_bit: Option<usize>,
    pub element_count: Option<usize>,
    pub nested_bit_count: Option<usize>,
}

impl UpdateFieldDescriptor {
    pub const fn scalar(name: &'static str, bit: usize) -> Self {
        Self {
            name,
            kind: UpdateFieldDescriptorKind::Scalar,
            bit,
            first_child_bit: None,
            element_count: None,
            nested_bit_count: None,
        }
    }

    pub const fn dynamic(name: &'static str, bit: usize) -> Self {
        Self {
            name,
            kind: UpdateFieldDescriptorKind::Dynamic,
            bit,
            first_child_bit: None,
            element_count: None,
            nested_bit_count: None,
        }
    }

    pub const fn dynamic_array(
        name: &'static str,
        bit: usize,
        first_child_bit: usize,
        element_count: usize,
    ) -> Self {
        Self {
            name,
            kind: UpdateFieldDescriptorKind::DynamicArray,
            bit,
            first_child_bit: Some(first_child_bit),
            element_count: Some(element_count),
            nested_bit_count: None,
        }
    }

    pub const fn optional(name: &'static str, bit: usize, nested_bit_count: usize) -> Self {
        Self {
            name,
            kind: UpdateFieldDescriptorKind::Optional,
            bit,
            first_child_bit: None,
            element_count: None,
            nested_bit_count: Some(nested_bit_count),
        }
    }

    pub const fn nested(name: &'static str, bit: usize, nested_bit_count: usize) -> Self {
        Self {
            name,
            kind: UpdateFieldDescriptorKind::Nested,
            bit,
            first_child_bit: None,
            element_count: None,
            nested_bit_count: Some(nested_bit_count),
        }
    }

    pub const fn array(
        name: &'static str,
        bit: usize,
        first_child_bit: usize,
        element_count: usize,
        nested_bit_count: Option<usize>,
    ) -> Self {
        Self {
            name,
            kind: UpdateFieldDescriptorKind::Array,
            bit,
            first_child_bit: Some(first_child_bit),
            element_count: Some(element_count),
            nested_bit_count,
        }
    }

    pub const fn last_child_bit(self) -> Option<usize> {
        match (self.first_child_bit, self.element_count) {
            (Some(first), Some(count)) => Some(first + count - 1),
            _ => None,
        }
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct UpdateFieldVisibilityFlags: u8 {
        const OWNER = 0x01;
        const PARTY_MEMBER = 0x02;
        const UNIT_ALL = 0x04;
        const EMPATH = 0x08;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateFieldSectionUpdate {
    pub kind: UpdateFieldSectionKind,
    pub mask: UpdateMask,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValuesUpdateSections {
    pub changed_object_type_mask: u32,
    pub sections: Vec<UpdateFieldSectionUpdate>,
}

impl ValuesUpdateSections {
    pub const fn empty() -> Self {
        Self {
            changed_object_type_mask: 0,
            sections: Vec::new(),
        }
    }

    pub fn push(&mut self, section: UpdateFieldSectionUpdate) {
        assert_eq!(section.mask.bits(), section.kind.bit_count());
        if section.mask.is_any_set() {
            if let Some(type_bit) = section.kind.client_type_bit() {
                self.changed_object_type_mask |= 1 << type_bit;
            }
        }
        self.sections.push(section);
    }

    pub const fn has_data(&self) -> bool {
        self.changed_object_type_mask != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectDataValues {
    pub entry_id: i32,
    pub dynamic_flags: u32,
    pub scale: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectDataUpdate {
    pub mask: UpdateMask,
    pub values: ObjectDataValues,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValuesUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataUpdate>,
}

impl ValuesUpdate {
    pub const fn empty() -> Self {
        Self {
            changed_object_type_mask: 0,
            object_data: None,
        }
    }

    pub const fn has_data(&self) -> bool {
        self.changed_object_type_mask != 0
    }
}

pub const NUM_CLIENT_OBJECT_TYPES: usize = 14;
pub const OBJECT_DATA_BITS: usize = 4;
pub const ITEM_DATA_BITS: usize = 43;
pub const CONTAINER_DATA_BITS: usize = 39;
pub const UNIT_DATA_BITS: usize = 227;
pub const PLAYER_DATA_BITS: usize = 108;
pub const ACTIVE_PLAYER_DATA_BITS: usize = 1525;
pub const GAME_OBJECT_DATA_BITS: usize = 20;
pub const DYNAMIC_OBJECT_DATA_BITS: usize = 7;
pub const CORPSE_DATA_BITS: usize = 32;
pub const AREA_TRIGGER_DATA_BITS: usize = 20;
pub const SCENE_OBJECT_DATA_BITS: usize = 5;
pub const CONVERSATION_DATA_BITS: usize = 4;
pub const TYPEID_OBJECT: usize = 0;
pub const TYPEID_ITEM: usize = 1;
pub const TYPEID_CONTAINER: usize = 2;
pub const TYPEID_UNIT: usize = 5;
pub const TYPEID_PLAYER: usize = 6;
pub const TYPEID_ACTIVE_PLAYER: usize = 7;
pub const TYPEID_GAME_OBJECT: usize = 8;
pub const TYPEID_DYNAMIC_OBJECT: usize = 9;
pub const TYPEID_CORPSE: usize = 10;
pub const TYPEID_AREA_TRIGGER: usize = 11;
pub const TYPEID_SCENE_OBJECT: usize = 12;
pub const TYPEID_CONVERSATION: usize = 13;
pub const OBJECT_DATA_PARENT_BIT: usize = 0;
pub const OBJECT_DATA_ENTRY_ID_BIT: usize = 1;
pub const OBJECT_DATA_DYNAMIC_FLAGS_BIT: usize = 2;
pub const OBJECT_DATA_SCALE_BIT: usize = 3;

const ITEM_BASE_ALLOWED_BLOCKS: [u32; 2] = [0xE029_CE7F, 0x0000_07FF];
const ITEM_OWNER_ALLOWED_BLOCKS: [u32; 2] = [0x1FD6_3180, 0x0000_0000];
const UNIT_BASE_ALLOWED_BLOCKS: [u32; 8] = [
    0xFFFF_DFFF,
    0xFF0F_DFFF,
    0xC001_EFFF,
    0x001E_FFFF,
    0xFFFF_FE00,
    0x0000_3FFF,
    0xFFF0_0000,
    0x0000_0007,
];
const UNIT_OWNER_ALLOWED_BLOCKS: [u32; 8] = [
    0x0000_2000,
    0x00F0_2000,
    0x3FFE_1000,
    0xFFF1_0000,
    0x0000_01FF,
    0xFFFF_C000,
    0x000F_FFFF,
    0x0000_0000,
];
const UNIT_ALL_ALLOWED_BLOCKS: [u32; 8] = [
    0x0000_0000,
    0x0000_0000,
    0x0000_0000,
    0xFFF0_0000,
    0x0000_01FF,
    0x0000_0000,
    0x0000_0000,
    0x0000_0000,
];
const UNIT_EMPATH_ALLOWED_BLOCKS: [u32; 8] = [
    0x0000_0000,
    0x00F0_0000,
    0x0000_0000,
    0x0000_0000,
    0x0000_0000,
    0xC000_0000,
    0x0000_003F,
    0x0000_0000,
];
const PLAYER_BASE_ALLOWED_BLOCKS: [u32; 4] = [0xFFFF_FFFF, 0xE000_0007, 0xFFFF_FFFF, 0x0000_0FFF];
const PLAYER_PARTY_MEMBER_ALLOWED_BLOCKS: [u32; 4] =
    [0x0000_0000, 0x1FFF_FFF8, 0x0000_0000, 0x0000_0000];

pub fn base_allowed_mask_for_section(kind: UpdateFieldSectionKind) -> UpdateMask {
    match kind {
        UpdateFieldSectionKind::ItemData => {
            UpdateMask::from_blocks(kind.bit_count(), &ITEM_BASE_ALLOWED_BLOCKS)
        }
        UpdateFieldSectionKind::UnitData => {
            UpdateMask::from_blocks(kind.bit_count(), &UNIT_BASE_ALLOWED_BLOCKS)
        }
        UpdateFieldSectionKind::PlayerData => {
            UpdateMask::from_blocks(kind.bit_count(), &PLAYER_BASE_ALLOWED_BLOCKS)
        }
        _ => UpdateMask::all_bits(kind.bit_count()),
    }
}

pub fn extra_allowed_mask_for_visibility(
    kind: UpdateFieldSectionKind,
    flags: UpdateFieldVisibilityFlags,
) -> UpdateMask {
    let mut mask = UpdateMask::new(kind.bit_count());
    match kind {
        UpdateFieldSectionKind::ItemData => {
            if flags.contains(UpdateFieldVisibilityFlags::OWNER) {
                mask |= &UpdateMask::from_blocks(kind.bit_count(), &ITEM_OWNER_ALLOWED_BLOCKS);
            }
        }
        UpdateFieldSectionKind::UnitData => {
            if flags.contains(UpdateFieldVisibilityFlags::OWNER) {
                mask |= &UpdateMask::from_blocks(kind.bit_count(), &UNIT_OWNER_ALLOWED_BLOCKS);
            }
            if flags.contains(UpdateFieldVisibilityFlags::UNIT_ALL) {
                mask |= &UpdateMask::from_blocks(kind.bit_count(), &UNIT_ALL_ALLOWED_BLOCKS);
            }
            if flags.contains(UpdateFieldVisibilityFlags::EMPATH) {
                mask |= &UpdateMask::from_blocks(kind.bit_count(), &UNIT_EMPATH_ALLOWED_BLOCKS);
            }
        }
        UpdateFieldSectionKind::PlayerData => {
            if flags.contains(UpdateFieldVisibilityFlags::PARTY_MEMBER) {
                mask |=
                    &UpdateMask::from_blocks(kind.bit_count(), &PLAYER_PARTY_MEMBER_ALLOWED_BLOCKS);
            }
        }
        _ => {}
    }
    mask
}

pub fn allowed_mask_for_visibility(
    kind: UpdateFieldSectionKind,
    flags: UpdateFieldVisibilityFlags,
) -> UpdateMask {
    let mut mask = base_allowed_mask_for_section(kind);
    mask |= &extra_allowed_mask_for_visibility(kind, flags);
    mask
}

pub fn filter_disallowed_fields(
    kind: UpdateFieldSectionKind,
    changes_mask: &mut UpdateMask,
    flags: UpdateFieldVisibilityFlags,
) {
    assert_eq!(changes_mask.bits(), kind.bit_count());
    *changes_mask &= &allowed_mask_for_visibility(kind, flags);
}

const ITEM_DATA_DESCRIPTORS: &[UpdateFieldDescriptor] = &[
    UpdateFieldDescriptor::dynamic("ArtifactPowers", 1),
    UpdateFieldDescriptor::dynamic("Gems", 2),
    UpdateFieldDescriptor::nested("Modifiers", 19, 1),
    UpdateFieldDescriptor::array("SpellCharges", 23, 24, 5, None),
    UpdateFieldDescriptor::array("Enchantment", 29, 30, 13, Some(6)),
];

const CONTAINER_DATA_DESCRIPTORS: &[UpdateFieldDescriptor] =
    &[UpdateFieldDescriptor::array("Slots", 2, 3, 36, None)];

const UNIT_DATA_DESCRIPTORS: &[UpdateFieldDescriptor] = &[
    UpdateFieldDescriptor::dynamic("StateWorldEffectIDs", 1),
    UpdateFieldDescriptor::dynamic("PassiveSpells", 2),
    UpdateFieldDescriptor::dynamic("WorldEffects", 3),
    UpdateFieldDescriptor::dynamic("ChannelObjects", 4),
    UpdateFieldDescriptor::nested("ChannelData", 22, 0),
    UpdateFieldDescriptor::array("NpcFlags", 113, 114, 2, None),
    UpdateFieldDescriptor::array("PowerRegenFlatModifier", 116, 117, 10, None),
    UpdateFieldDescriptor::array("PowerRegenInterruptedFlatModifier", 116, 127, 10, None),
    UpdateFieldDescriptor::array("Power", 116, 137, 10, None),
    UpdateFieldDescriptor::array("MaxPower", 116, 147, 10, None),
    UpdateFieldDescriptor::array("ModPowerRegen", 116, 157, 10, None),
    UpdateFieldDescriptor::array("VirtualItems", 167, 168, 3, Some(4)),
    UpdateFieldDescriptor::array("AttackRoundBaseTime", 171, 172, 2, None),
    UpdateFieldDescriptor::array("Stats", 174, 175, 5, None),
    UpdateFieldDescriptor::array("StatPosBuff", 174, 180, 5, None),
    UpdateFieldDescriptor::array("StatNegBuff", 174, 185, 5, None),
    UpdateFieldDescriptor::array("Resistances", 190, 191, 7, None),
    UpdateFieldDescriptor::array("PowerCostModifier", 190, 198, 7, None),
    UpdateFieldDescriptor::array("PowerCostMultiplier", 190, 205, 7, None),
    UpdateFieldDescriptor::array("ResistanceBuffModsPositive", 212, 213, 7, None),
    UpdateFieldDescriptor::array("ResistanceBuffModsNegative", 212, 220, 7, None),
];

const PLAYER_DATA_DESCRIPTORS: &[UpdateFieldDescriptor] = &[
    UpdateFieldDescriptor::dynamic("Customizations", 1),
    UpdateFieldDescriptor::dynamic("ArenaCooldowns", 2),
    UpdateFieldDescriptor::dynamic("VisualItemReplacements", 3),
    UpdateFieldDescriptor::array("PartyType", 32, 33, 2, None),
    UpdateFieldDescriptor::array("QuestLog", 35, 36, 25, Some(29)),
    UpdateFieldDescriptor::array("VisibleItems", 61, 62, 19, Some(4)),
    UpdateFieldDescriptor::array("AvgItemLevel", 81, 82, 6, None),
    UpdateFieldDescriptor::array("Field_3120", 88, 89, 19, None),
];

const ACTIVE_PLAYER_DATA_DESCRIPTORS: &[UpdateFieldDescriptor] = &[
    UpdateFieldDescriptor::dynamic("KnownTitles", 3),
    UpdateFieldDescriptor::dynamic("DailyQuestsCompleted", 4),
    UpdateFieldDescriptor::dynamic("AvailableQuestLineXQuestIDs", 5),
    UpdateFieldDescriptor::dynamic("Field_1000", 6),
    UpdateFieldDescriptor::dynamic("Heirlooms", 7),
    UpdateFieldDescriptor::dynamic("HeirloomFlags", 8),
    UpdateFieldDescriptor::dynamic("Toys", 9),
    UpdateFieldDescriptor::dynamic("Transmog", 10),
    UpdateFieldDescriptor::dynamic("ConditionalTransmog", 11),
    UpdateFieldDescriptor::dynamic("SelfResSpells", 12),
    UpdateFieldDescriptor::dynamic("CharacterRestrictions", 13),
    UpdateFieldDescriptor::dynamic("SpellPctModByLabel", 14),
    UpdateFieldDescriptor::dynamic("SpellFlatModByLabel", 15),
    UpdateFieldDescriptor::dynamic("TaskQuests", 16),
    UpdateFieldDescriptor::dynamic("TraitConfigs", 17),
    UpdateFieldDescriptor::dynamic("CategoryCooldownMods", 18),
    UpdateFieldDescriptor::dynamic("WeeklySpellUses", 19),
    UpdateFieldDescriptor::dynamic_array("ResearchSites", 20, 21, 1),
    UpdateFieldDescriptor::dynamic_array("ResearchSiteProgress", 22, 23, 1),
    UpdateFieldDescriptor::dynamic_array("Research", 24, 25, 1),
    UpdateFieldDescriptor::nested("Skill", 32, 1793),
    UpdateFieldDescriptor::nested("ResearchHistory", 116, 2),
    UpdateFieldDescriptor::nested("FrozenPerksVendorItem", 117, 0),
    UpdateFieldDescriptor::optional("PetStable", 122, 3),
    UpdateFieldDescriptor::array("InvSlots", 124, 125, 141, None),
    UpdateFieldDescriptor::array("TrackResourceMask", 266, 267, 2, None),
    UpdateFieldDescriptor::array("SpellCritPercentage", 269, 270, 7, None),
    UpdateFieldDescriptor::array("ModDamageDonePos", 269, 277, 7, None),
    UpdateFieldDescriptor::array("ModDamageDoneNeg", 269, 284, 7, None),
    UpdateFieldDescriptor::array("ModDamageDonePercent", 269, 291, 7, None),
    UpdateFieldDescriptor::array("ExploredZones", 298, 299, 240, None),
    UpdateFieldDescriptor::array("RestInfo", 539, 540, 2, Some(3)),
    UpdateFieldDescriptor::array("WeaponDmgMultipliers", 542, 543, 3, None),
    UpdateFieldDescriptor::array("WeaponAtkSpeedMultipliers", 542, 546, 3, None),
    UpdateFieldDescriptor::array("BuybackPrice", 549, 550, 12, None),
    UpdateFieldDescriptor::array("BuybackTimestamp", 549, 562, 12, None),
    UpdateFieldDescriptor::array("CombatRatings", 574, 575, 32, None),
    UpdateFieldDescriptor::array("PvpInfo", 607, 608, 7, Some(19)),
    UpdateFieldDescriptor::array("NoReagentCostMask", 615, 616, 4, None),
    UpdateFieldDescriptor::array("ProfessionSkillLine", 620, 621, 2, None),
    UpdateFieldDescriptor::array("BagSlotFlags", 623, 624, 4, None),
    UpdateFieldDescriptor::array("BankBagSlotFlags", 628, 629, 7, None),
    UpdateFieldDescriptor::array("QuestCompleted", 636, 637, 875, None),
    UpdateFieldDescriptor::array("GlyphSlots", 1512, 1513, 6, None),
    UpdateFieldDescriptor::array("Glyphs", 1512, 1519, 6, None),
];

const GAME_OBJECT_DATA_DESCRIPTORS: &[UpdateFieldDescriptor] = &[
    UpdateFieldDescriptor::dynamic("StateWorldEffectIDs", 1),
    UpdateFieldDescriptor::dynamic("EnableDoodadSets", 2),
    UpdateFieldDescriptor::dynamic("WorldEffects", 3),
    UpdateFieldDescriptor::nested("ParentRotation", 12, 0),
];

const CORPSE_DATA_DESCRIPTORS: &[UpdateFieldDescriptor] = &[
    UpdateFieldDescriptor::dynamic("Customizations", 1),
    UpdateFieldDescriptor::array("Items", 12, 13, 19, None),
];

const AREA_TRIGGER_DATA_DESCRIPTORS: &[UpdateFieldDescriptor] = &[
    UpdateFieldDescriptor::nested("OverrideScaleCurve", 1, 7),
    UpdateFieldDescriptor::nested("ExtraScaleCurve", 2, 7),
    UpdateFieldDescriptor::nested("OverrideMoveCurveX", 3, 7),
    UpdateFieldDescriptor::nested("OverrideMoveCurveY", 4, 7),
    UpdateFieldDescriptor::nested("OverrideMoveCurveZ", 5, 7),
    UpdateFieldDescriptor::nested("VisualAnim", 19, 5),
];

const CONVERSATION_DATA_DESCRIPTORS: &[UpdateFieldDescriptor] = &[
    UpdateFieldDescriptor::dynamic("Lines", 1),
    UpdateFieldDescriptor::dynamic("Actors", 2),
    UpdateFieldDescriptor::scalar("LastLineEndTime", 3),
];

const fn block_index(bit: usize) -> usize {
    bit / 32
}

const fn block_flag(bit: usize) -> u32 {
    1u32 << (bit % 32)
}

const fn const_div_ceil(value: usize, divisor: usize) -> usize {
    (value + divisor - 1) / divisor
}

#[cfg(test)]
#[path = "update_fields/tests/mod.rs"]
mod tests;
