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
    NameProfanity,
    NameReserved,
    NameReservedLocale,
    Category,
}

pub(super) struct Schema {
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
            Self::NameProfanity => ("NamesProfanity", 0xDA82D96C, 0xF227E638, 2, None, None),
            Self::NameReserved => ("NamesReserved", 0x25C1CB13, 0x2B2D5D97, 1, None, None),
            Self::NameReservedLocale => {
                ("NamesReservedLocale", 0x3ACAE305, 0x7B9823D4, 2, None, None)
            }
            Self::Category => ("Cfg_Categories", 0xC7ED797D, 0x8710BE94, 6, None, None),
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

    /// (logical element bits, metadata array size). Numeric bits are returned
    /// unchanged; consumers apply the source's i8/i32/f32 interpretation.
    pub(super) fn numeric(self, field: usize) -> Option<(usize, usize)> {
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
            (Self::Race, 15 | 26 | 28) => Some((32, 1)),
            (Self::Race, 36 | 38) => Some((8, 1)),
            (Self::NameProfanity | Self::NameReservedLocale, 1) => Some((8, 1)),
            (Self::Category, 1) => Some((16, 1)),
            (Self::Category, 2 | 3 | 5) => Some((8, 1)),
            (Self::Category, 4) => Some((32, 1)),
            _ => None,
        }
    }
}
