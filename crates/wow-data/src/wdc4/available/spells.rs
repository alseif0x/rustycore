//! Twenty-four captured build-70170/esES normal prefixes; never arbitrary Skip.
use crate::wdc4::{Wdc4Header, creation::SpellTable, format::SectionHeader};
use anyhow::{Result, ensure};

#[derive(Clone, Copy)]
pub(in crate::wdc4) struct SpellPrefix {
    pub(in crate::wdc4) full: u32,
    pub(in crate::wdc4) bytes: u32,
    pub(in crate::wdc4) first: u32,
    pub(in crate::wdc4) records: u32,
    pub(in crate::wdc4) known: u32,
    pub(in crate::wdc4) record_bytes: u32,
    pub(in crate::wdc4) sections: u32,
    pub(in crate::wdc4) strings: u32,
    pub(in crate::wdc4) copies: u32,
}

pub(in crate::wdc4) fn contract(table: SpellTable) -> Option<SpellPrefix> {
    let (full, bytes, first, records, known, record_bytes, sections, strings, copies) = match table
    {
        SpellTable::SpellName => (711994, 696455, 4260, 18134, 17565, 4, 9, 438291, 14173),
        SpellTable::SpellEffect => (1749502, 1701488, 89932, 43670, 42409, 26, 9, 2, 0),
        SpellTable::SpellMisc => (3016982, 2933534, 15280, 32626, 31720, 80, 9, 2, 0),
        SpellTable::SpellAuraOptions => (209698, 208578, 2388, 12953, 12886, 4, 5, 2, 0),
        SpellTable::SpellAuraRestrictions => (9624, 9134, 904, 333, 316, 14, 5, 2, 0),
        SpellTable::SpellCastingRequirements => (37512, 37028, 1188, 3302, 3258, 7, 5, 2, 0),
        SpellTable::SpellCategories => (189161, 183217, 3224, 10931, 10587, 5, 9, 2, 0),
        SpellTable::SpellClassOptions => (59695, 59623, 5180, 6057, 6049, 5, 3, 2, 0),
        SpellTable::SpellCooldowns => (68837, 67802, 1248, 4501, 4436, 3, 6, 2, 0),
        SpellTable::SpellEquippedItems => (21625, 21438, 668, 1905, 1888, 7, 4, 2, 0),
        SpellTable::SpellInterrupts => (156592, 151816, 3932, 10170, 9858, 3, 9, 2, 0),
        SpellTable::SpellLabel => (61484, 61045, 828, 4662, 4631, 1, 4, 2, 0),
        SpellTable::SpellLevels => (196940, 194513, 2064, 12986, 12829, 3, 7, 2, 0),
        SpellTable::SpellPower => (55536, 54957, 3508, 3466, 3429, 7, 3, 2, 0),
        SpellTable::SpellReagents => (131214, 130834, 97832, 3338, 3300, 6, 5, 2, 0),
        SpellTable::SpellTargetRestrictions => (74862, 72638, 1552, 4578, 4442, 4, 5, 2, 0),
        SpellTable::SpellTotems => (10559, 10541, 828, 1081, 1079, 5, 2, 2, 0),
        SpellTable::SpellXSpellVisual => (319037, 309086, 5352, 20905, 20248, 7, 9, 2, 0),
        SpellTable::SummonProperties => (1970, 1962, 976, 90, 89, 4, 2, 2, 34),
        SpellTable::BattlePetSpecies => (3786, 3746, 1484, 115, 113, 20, 3, 2, 0),
        SpellTable::SpellItemEnchantment => (330036, 329881, 964, 2200, 2199, 124, 2, 47309, 17),
        SpellTable::SpellVisual => (125674, 124866, 3656, 2292, 2273, 36, 7, 2, 3786),
        SpellTable::SpellVisualMissile => (35442, 35390, 6496, 723, 722, 32, 2, 2, 0),
        SpellTable::SpellVisualEffectName => (49315, 49213, 1680, 2689, 2683, 13, 2, 2, 240),
        _ => return None,
    };
    Some(SpellPrefix {
        full,
        bytes,
        first,
        records,
        known,
        record_bytes,
        sections,
        strings,
        copies,
    })
}

impl SpellPrefix {
    pub(in crate::wdc4) fn unknown(self) -> usize {
        (self.records - self.known) as usize
    }

    pub(in crate::wdc4) fn validate(
        self,
        table: SpellTable,
        header: &Wdc4Header,
        sections: &[SectionHeader],
        bytes: usize,
    ) -> Result<()> {
        let schema = table.schema();
        ensure!(
            header.format_version == 5
                && header.table_hash == schema.base.hash
                && header._layout_hash == schema.base.layout
                && header.field_count as usize == schema.base.fields
                && header.total_field_count as usize == schema.base.fields
                && header._locale == schema.locale_mask
                && header.flags == schema.flags
                && header.id_index as usize == schema.base.id.unwrap_or(0)
                && header._parent_lookup_count == schema.parent_lookups
                && header.record_count == self.records
                && header.record_size == self.record_bytes
                && header.section_count == self.sections
                && sections.len() == self.sections as usize
                && bytes == self.bytes as usize
                && self.bytes < self.full
                && self.full <= 4 * 1024 * 1024,
            "Wrong bounded target spell prefix header"
        );
        let mut count = 0u64;
        let mut previous = 0;
        for (index, section) in sections.iter().enumerate() {
            let ids = if schema.base.id.is_none() {
                u64::from(section.record_count) * 4
            } else {
                0
            };
            let parent = if schema.parent_lookups == 1 {
                12 + u64::from(section.record_count) * 8
            } else {
                0
            };
            let end = u64::from(section.file_offset)
                + u64::from(section.record_count) * u64::from(self.record_bytes)
                + u64::from(section.string_table_size)
                + ids
                + u64::from(section.copy_table_count) * 8
                + parent;
            ensure!(
                (section._tact_key_hash == 0) == (index == 0)
                    && section.file_offset > previous
                    && end <= u64::from(self.full)
                    && u64::from(section.id_list_size) == ids
                    && u64::from(section._relationship_data_size) == parent
                    && section._offset_map_id_count == 0,
                "Wrong spell known/unknown section shape"
            );
            if index == 0 {
                ensure!(
                    section.file_offset == self.first
                        && section.record_count == self.known
                        && section.string_table_size == self.strings
                        && section.copy_table_count == self.copies
                        && end == u64::from(self.bytes),
                    "Wrong spell plaintext extent"
                );
            } else {
                ensure!(
                    section.file_offset >= self.bytes
                        && (index != 1 || section.file_offset == self.bytes),
                    "Wrong spell excluded section boundary"
                );
            }
            count += u64::from(section.record_count);
            previous = section.file_offset;
        }
        ensure!(
            count == u64::from(self.records),
            "Wrong spell section record total"
        );
        Ok(())
    }
}

#[cfg(test)]
#[path = "spells/tests.rs"]
mod tests;
