use super::{Category, LocaleReserved, NameRecords, Profanity, Reserved};
use crate::wdc4::creation::{CreationDb2, CreationTable};
use anyhow::Result;
use std::path::Path;

impl NameRecords {
    // DB2LoadInfo.h / DB2Metadata.h at 02245dcd: language is signed BYTE,
    // locale mask is unsigned BYTE, and Cfg_Categories field 2 is the
    // unsigned BYTE CreateCharsetMask.
    pub(super) fn load_from(directory: &Path) -> Result<Self> {
        let mut records = Self::default();

        let db = CreationDb2::open(directory, CreationTable::NameProfanity)?;
        for id in db.ids() {
            records.profanity.push(Profanity {
                id,
                name: db.string(id, 0)?,
                language: db.bits(id, 1, 0)? as u8 as i8,
            });
        }

        let db = CreationDb2::open(directory, CreationTable::NameReserved)?;
        for id in db.ids() {
            records.reserved.push(Reserved {
                id,
                name: db.string(id, 0)?,
            });
        }

        let db = CreationDb2::open(directory, CreationTable::NameReservedLocale)?;
        for id in db.ids() {
            records.locale_reserved.push(LocaleReserved {
                id,
                name: db.string(id, 0)?,
                locale_mask: db.bits(id, 1, 0)? as u8,
            });
        }

        let db = CreationDb2::open(directory, CreationTable::Category)?;
        for id in db.ids() {
            records.categories.push(Category {
                id,
                creation_charset: db.bits(id, 2, 0)? as u8,
            });
        }

        Ok(records)
    }
}
