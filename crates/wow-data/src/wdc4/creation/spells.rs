//! New target spell admission, retaining the existing checked numeric owner.
//! No permissive legacy getter, arbitrary table or production Player is added.
mod schema;
#[cfg(test)]
mod tests;
mod text;
use super::{CreationDb2, CreationTable};
use crate::wdc4::{Wdc4Reader, available::spells as prefixes};
use anyhow::{Result, ensure};
pub(crate) use schema::SpellTable;
use std::path::Path;

impl CreationDb2 {
    pub(crate) fn open_spell(
        directory: &Path,
        table: SpellTable,
        available: bool,
    ) -> Result<(Self, usize)> {
        if let Some(prefix) = prefixes::contract(table).filter(|_| available) {
            let path = directory.join(format!("{}.available.db2", table.name()));
            let reader = Wdc4Reader::open_available_spell(&path, table)?;
            Ok((
                Self::checked(reader, CreationTable::Spell(table))?,
                prefix.unknown(),
            ))
        } else {
            Ok((Self::open(directory, CreationTable::Spell(table))?, 0))
        }
    }
}

pub(super) fn check_header(reader: &Wdc4Reader, table: SpellTable) -> Result<()> {
    let schema = table.schema();
    ensure!(
        reader.header.flags == schema.flags
            && reader.header.id_index as usize == schema.base.id.unwrap_or(0)
            && reader.header._locale == schema.locale_mask
            && reader.header.total_field_count as usize == schema.base.fields,
        "Wrong native target spell header"
    );
    if reader.header.record_count == 0 {
        ensure!(
            empty(reader, CreationTable::Spell(table)),
            "Contradictory empty target spell table"
        );
    }
    Ok(())
}

pub(super) fn empty(reader: &Wdc4Reader, table: CreationTable) -> bool {
    let CreationTable::Spell(spell) = table else {
        return false;
    };
    let schema = spell.schema();
    reader.header.record_count == 0
        && reader.header.section_count == 0
        && reader.header.record_size == 0
        && reader.header.string_table_size == 0
        && reader.header.field_storage_info_size == 0
        && reader.header.common_data_size == 0
        && reader.header.pallet_data_size == 0
        && reader.header.flags == schema.flags
        && reader.header._parent_lookup_count == schema.parent_lookups
        && reader.field_info.is_empty()
}
