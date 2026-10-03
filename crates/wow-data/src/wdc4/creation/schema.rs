//! 02245dcd DB2Metadata/LoadInfo. Only creation-relevant numeric fields,
//! not strings, full serializers, arbitrary table types or legacy offsets.

#[derive(Clone, Copy)]
pub(crate) enum CreationTable {
    Model,
    Option,
    Choice,
    Requirement,
    RequiredChoice,
    RaceModel,
    Race,
    Class,
    NameProfanity,
    NameReserved,
    NameReservedLocale,
    Category,
    Map,
    PowerType,
    Specialization,
    ClassPower,
    Movie,
    SkillLine,
    SkillRaceClass,
    SkillAbility,
    Loadout,
    LoadoutItem,
    Item,
    ItemEffect,
    ItemEffectRelation,
    ItemSpec,
    ItemSpecOverride,
    GemProperties,
    Spell(super::spells::SpellTable),
}

pub(in crate::wdc4) struct Schema {
    pub name: &'static str,
    pub hash: u32,
    pub layout: u32,
    pub fields: usize,
    pub id: Option<usize>,
    pub parent: Option<usize>,
}

impl CreationTable {
    pub(super) fn schema(self) -> Schema {
        let (name, hash, layout, fields, id, parent) = match self {
            Self::Model => ("ChrModel", 0x49349C6E, 0x03FAB755, 17, Some(2), Some(4)),
            Self::Option => (
                "ChrCustomizationOption",
                0x2FB7905B,
                0xDCC2A86E,
                13,
                Some(1),
                Some(4),
            ),
            Self::Choice => (
                "ChrCustomizationChoice",
                0x681D0F3D,
                0x9559C358,
                11,
                Some(1),
                Some(2),
            ),
            Self::Requirement => ("ChrCustomizationReq", 0x61431A65, 0xCA154412, 9, None, None),
            Self::RequiredChoice => (
                "ChrCustomizationReqChoice",
                0x9B1BEE48,
                0xF925BC6F,
                1,
                None,
                Some(1),
            ),
            Self::RaceModel => ("ChrRaceXChrModel", 0xA7E150FE, 0xA203BC29, 4, None, Some(0)),
            Self::Race => ("ChrRaces", 0x53F1783C, 0x4F44C796, 51, None, None),
            Self::Class => ("ChrClasses", 0xF5889D8C, 0xAFC9B0C2, 43, Some(29), None),
            Self::NameProfanity => ("NamesProfanity", 0xDA82D96C, 0xF227E638, 2, None, None),
            Self::NameReserved => ("NamesReserved", 0x25C1CB13, 0x2B2D5D97, 1, None, None),
            Self::NameReservedLocale => {
                ("NamesReservedLocale", 0x3ACAE305, 0x7B9823D4, 2, None, None)
            }
            Self::Category => ("Cfg_Categories", 0xC7ED797D, 0x8710BE94, 6, None, None),
            Self::Map => ("Map", 0xBD84CD62, 0xD43AFAC3, 26, None, None),
            Self::PowerType => ("PowerType", 0x8D899A57, 0x14BBEEA1, 13, Some(2), None),
            Self::Specialization => (
                "ChrSpecialization",
                0xA00F8E60,
                0xDAB4CA4B,
                13,
                Some(3),
                Some(4),
            ),
            Self::ClassPower => (
                "ChrClassesXPowerTypes",
                0xC0315ACF,
                0x70DA1F8C,
                1,
                None,
                Some(1),
            ),
            Self::Movie => ("Movie", 0x032DFA13, 0xF53888FA, 6, None, None),
            Self::SkillLine => ("SkillLine", 0xB53DC9D6, 0x6763217C, 15, Some(5), None),
            Self::SkillRaceClass => (
                "SkillRaceClassInfo",
                0x06ADE420,
                0x24277B48,
                7,
                None,
                Some(0),
            ),
            Self::SkillAbility => (
                "SkillLineAbility",
                0xFF4446F6,
                0x224F7EA0,
                18,
                Some(2),
                Some(3),
            ),
            Self::Loadout => ("CharacterLoadout", 0xE00A47FB, 0x713CE8BB, 5, None, None),
            Self::LoadoutItem => (
                "CharacterLoadoutItem",
                0x65C39BA7,
                0x0C7A1862,
                2,
                None,
                Some(0),
            ),
            Self::Item => ("Item", 0x50238EC2, 0x9A2A4834, 16, None, None),
            Self::ItemEffect => ("ItemEffect", 0x4002A5B1, 0x4CA77678, 9, None, None),
            Self::ItemEffectRelation => {
                ("ItemXItemEffect", 0x00CB674F, 0x96F083AD, 1, None, Some(1))
            }
            Self::ItemSpec => ("ItemSpec", 0x08DA6E2A, 0x83F3D113, 6, None, Some(2)),
            Self::ItemSpecOverride => {
                ("ItemSpecOverride", 0x149AAE79, 0xB292998C, 1, None, Some(1))
            }
            Self::GemProperties => ("GemProperties", 0x9C00EA6D, 0x86487AD2, 2, None, None),
            Self::Spell(table) => return table.schema().base,
        };
        Schema {
            name,
            hash,
            layout,
            fields,
            id,
            parent,
        }
    }

    /// Physical target header, distinct from DB2Meta::ParentIndexField.
    /// DB2FileLoader.cpp:1773-1792 allows an in-record parent without a lookup.
    /// Fresh 70170 specialization has ParentIndexField=4, ParentLookupCount=0.
    /// Existing table gates retain their independently captured contracts.
    pub(super) fn parent_lookup_count(self) -> u32 {
        match self {
            Self::Spell(table) => table.schema().parent_lookups,
            Self::Specialization | Self::ItemSpec => 0,
            _ => u32::from(self.schema().parent.is_some()),
        }
    }

    /// (logical element bits, metadata array size). Numeric bits are returned
    /// unchanged; consumers apply the source's i8/i32/f32 interpretation.
    pub(super) fn numeric(self, field: usize) -> Option<(usize, usize)> {
        if let Self::Spell(table) = self {
            return table
                .schema()
                .numeric
                .get(field)
                .copied()
                .filter(|(bits, _)| *bits != 0);
        }
        match (self, field) {
            (Self::Model, 0 | 1) => Some((32, 3)),
            (Self::Model, 2 | 4 | 6) => Some((32, 1)),
            (Self::Model, 3) => Some((8, 1)),
            (Self::Option, 1 | 4 | 10) => Some((32, 1)),
            (Self::Choice, 1 | 2 | 3) => Some((32, 1)),
            (Self::Requirement, 1 | 2 | 4 | 5 | 7) => Some((32, 1)),
            (Self::Requirement, 8) => Some((32, 2)),
            (Self::RequiredChoice, 0 | 1) => Some((32, 1)),
            (Self::RaceModel, 1) => Some((32, 1)),
            (Self::RaceModel, 0 | 2) => Some((8, 1)),
            (Self::Race, 15..=18 | 26 | 28) => Some((32, 1)),
            (Self::Race, 34..=36 | 38 | 40) => Some((8, 1)),
            (Self::Class, 14 | 15) => Some((32, 1)),
            (Self::Class, 27 | 28) => Some((16, 1)),
            (Self::Class, 29..=36) => Some((8, 1)),
            (Self::NameProfanity | Self::NameReservedLocale, 1) => Some((8, 1)),
            (Self::Category, 1) => Some((16, 1)),
            (Self::Category, 2 | 3 | 5) => Some((8, 1)),
            (Self::Category, 4) => Some((32, 1)),
            (Self::Map, 7 | 8 | 9 | 15 | 18) => Some((8, 1)),
            (Self::Map, 10..=14 | 17 | 19) => Some((16, 1)),
            (Self::Map, 16 | 20..=24) => Some((32, 1)),
            (Self::Map, 6) => Some((32, 2)),
            (Self::Map, 25) => Some((32, 3)),
            (Self::PowerType, 3) => Some((8, 1)),
            (Self::PowerType, 2 | 4..=12) => Some((32, 1)),
            (Self::Specialization, 4..=7 | 10) => Some((8, 1)),
            (Self::Specialization, 3 | 8 | 9 | 11) => Some((32, 1)),
            (Self::Specialization, 12) => Some((32, 2)),
            (Self::ClassPower, 0 | 1) => Some((8, 1)),
            (Self::Movie, 1 | 2) => Some((8, 1)),
            (Self::Movie, 3..=5) => Some((32, 1)),
            (Self::SkillLine, 5 | 7 | 9..=14) => Some((32, 1)),
            (Self::SkillLine, 6 | 8) => Some((8, 1)),
            (Self::SkillRaceClass, 0 | 5) => Some((16, 1)),
            (Self::SkillRaceClass, 1..=3) => Some((32, 1)),
            (Self::SkillRaceClass, 4) => Some((8, 1)),
            (Self::SkillRaceClass, 6) => Some((32, 2)),
            (Self::SkillAbility, 2 | 4 | 6..=8 | 11) => Some((32, 1)),
            (Self::SkillAbility, 3 | 5 | 9 | 10 | 13..=15) => Some((16, 1)),
            (Self::SkillAbility, 12) => Some((8, 1)),
            (Self::SkillAbility, 16 | 17) => Some((32, 2)),
            (Self::Loadout, 0 | 2) => Some((8, 1)),
            (Self::Loadout, 1 | 3) => Some((32, 1)),
            (Self::Loadout, 4) => Some((32, 2)),
            (Self::LoadoutItem, 0) => Some((16, 1)),
            (Self::LoadoutItem, 1) => Some((32, 1)),
            (Self::Item, 0 | 5 | 7..=10 | 12..=14) => Some((32, 1)),
            (Self::Item, 1..=4 | 6 | 11 | 15) => Some((8, 1)),
            (Self::ItemEffect, 0 | 1) => Some((8, 1)),
            (Self::ItemEffect, 2 | 5 | 7) => Some((16, 1)),
            (Self::ItemEffect, 3 | 4 | 6 | 8) => Some((32, 1)),
            (Self::ItemEffectRelation, 0 | 1) => Some((32, 1)),
            (Self::ItemSpec, 0..=4) => Some((8, 1)),
            (Self::ItemSpec, 5) => Some((16, 1)),
            (Self::ItemSpecOverride, 0) => Some((16, 1)),
            (Self::ItemSpecOverride, 1) => Some((32, 1)),
            (Self::GemProperties, 0) => Some((16, 1)),
            (Self::GemProperties, 1) => Some((32, 1)),
            _ => None,
        }
    }
}
