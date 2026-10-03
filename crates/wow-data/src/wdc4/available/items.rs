//! Frozen plaintext sections acquired from 70170/esES. Not full-table coverage.
//! 02245dcd DB2FileLoader::LoadTableData/Skip; ItemPrefixes.h acquisition gates.
use super::super::{Wdc4Header, format::SectionHeader};
use anyhow::{Result, ensure};

#[derive(Clone, Copy)]
pub(in crate::wdc4) enum ItemPrefix {
    Basic,
    Effect,
    Relation,
}

pub(super) struct Contract {
    pub hash: u32,
    pub layout: u32,
    pub fields: u32,
    pub record_size: u32,
    pub parent: u32,
    pub full: u32,
    pub counts: &'static [u32],
    pub offsets: &'static [u32],
    pub copies: &'static [u32],
}

impl ItemPrefix {
    pub(super) fn contract(self) -> Contract {
        match self {
            Self::Basic => Contract {
                hash: 0x5023_8EC2,
                layout: 0x9A2A_4834,
                fields: 16,
                record_size: 9,
                parent: 0,
                full: 302290,
                counts: &[9033, 1, 1, 1, 1, 53, 1, 1],
                offsets: &[1676, 301411, 301424, 301437, 301450, 301463, 302264, 302277],
                copies: &[22788, 0, 0, 0, 0, 14, 0, 0],
            },
            Self::Effect => Contract {
                hash: 0x4002_A5B1,
                layout: 0x4CA7_7678,
                fields: 9,
                record_size: 7,
                parent: 0,
                full: 125562,
                counts: &[7580, 7, 2, 2, 2, 25, 2],
                offsets: &[1572, 125074, 125151, 125173, 125195, 125217, 125540],
                copies: &[5015, 0, 0, 0, 0, 6, 0],
            },
            Self::Relation => Contract {
                hash: 0x00CB_674F,
                layout: 0x96F0_83AD,
                fields: 1,
                record_size: 3,
                parent: 1,
                full: 190146,
                counts: &[12588, 2, 2, 2, 32, 2],
                offsets: &[652, 189486, 189528, 189570, 189612, 190104],
                copies: &[0, 0, 0, 0, 0, 0],
            },
        }
    }

    pub(in crate::wdc4) fn byte_limit(self) -> usize {
        self.contract().offsets[1] as usize
    }

    pub(in crate::wdc4) fn validate(
        self,
        header: &Wdc4Header,
        sections: &[SectionHeader],
        bytes: usize,
    ) -> Result<()> {
        let c = self.contract();
        ensure!(
            header.format_version == 5
                && header.table_hash == c.hash
                && header._layout_hash == c.layout
                && header.field_count == c.fields
                && header.total_field_count == c.fields
                && header.record_size == c.record_size
                && header.record_count == c.counts.iter().sum::<u32>()
                && header.flags == 4
                && header.id_index == 0
                && header._parent_lookup_count == c.parent
                && header.field_storage_info_size == c.fields * 24
                && header.section_count as usize == c.counts.len()
                && sections.len() == c.counts.len()
                && bytes == self.byte_limit(),
            "Wrong available target item schema/extent"
        );
        for (i, s) in sections.iter().enumerate() {
            let strings = u32::from(i == 0) * 2;
            let parent_bytes = if c.parent == 0 {
                0
            } else {
                12 + c.counts[i] * 8
            };
            ensure!(
                (s._tact_key_hash == 0) == (i == 0)
                    && s.file_offset == c.offsets[i]
                    && s.record_count == c.counts[i]
                    && s.string_table_size == strings
                    && s.id_list_size == c.counts[i] * 4
                    && s.copy_table_count == c.copies[i]
                    && s._offset_map_id_count == 0
                    && s._relationship_data_size == parent_bytes,
                "Wrong available target item section"
            );
            let end = u64::from(s.file_offset)
                + u64::from(s.record_count) * u64::from(c.record_size)
                + u64::from(strings)
                + u64::from(s.id_list_size)
                + u64::from(s.copy_table_count) * 8
                + u64::from(parent_bytes);
            ensure!(
                end == u64::from(c.offsets.get(i + 1).copied().unwrap_or(c.full)),
                "Wrong available target item section extent"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
