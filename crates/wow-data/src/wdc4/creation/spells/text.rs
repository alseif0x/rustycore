//! Source RecordGetString/LoadTableData at 02245dcd: logical full record
//! extent precedes strings even when encrypted sections are skipped. Only
//! original section zero is readable for the admitted spell prefixes.
use super::super::{CreationDb2, CreationTable, numeric};
use crate::wdc4::CompressionType;
use anyhow::{Context, Result, ensure};

impl CreationDb2 {
    pub(crate) fn spell_text(&self, id: u32, field: usize) -> Result<Vec<u8>> {
        self.spell_text_component(id, field, 0)
    }

    /// Source RecordGetString:798-803 includes a uint32 stride for each
    /// string-array component. Closed schemas retain localized versus plain
    /// string ownership in their typed consumer; no arbitrary text schema.
    pub(crate) fn spell_text_component(
        &self,
        id: u32,
        field: usize,
        component: usize,
    ) -> Result<Vec<u8>> {
        let CreationTable::Spell(table) = self.table else {
            anyhow::bail!("Not a target spell table");
        };
        let &(bits, count) = table
            .schema()
            .numeric
            .get(field)
            .context("Spell string field absent")?;
        ensure!(
            bits == 0 && component < count,
            "Not a source spell string component"
        );
        let index = *self
            .records
            .get(&id)
            .context("Spell string record absent")?;
        let info = self
            .reader
            .field_info
            .get(field)
            .context("Spell string column absent")?;
        ensure!(
            info.compression == CompressionType::None && info.field_offset_bits % 8 == 0,
            "Unsupported spell string displacement"
        );
        let relative = numeric(&self.reader, index, field, component, 32, count)? as usize;
        if relative == 0 {
            return Ok(Vec::new());
        }
        let records = (self.reader.header.record_count as usize)
            .checked_mul(self.reader.header.record_size as usize)
            .context("Spell logical record extent overflow")?;
        let total = records
            .checked_add(self.reader.header.string_table_size as usize)
            .context("Spell logical string extent overflow")?;
        ensure!(
            relative < total,
            "Spell string displacement exceeds source bound"
        );
        // Full reads concatenate all original records. Bounded reads admit
        // only section zero, so physical index is also its source logical index.
        let target = index
            .checked_mul(self.reader.header.record_size as usize)
            .and_then(|at| at.checked_add(usize::from(info.field_offset_bits / 8)))
            .and_then(|at| {
                component
                    .checked_mul(4)
                    .and_then(|stride| at.checked_add(stride))
            })
            .and_then(|at| at.checked_add(relative))
            .context("Spell string address overflow")?;
        let mut offset = target
            .checked_sub(records)
            .context("Spell string points into records")?;
        let mut bytes = Vec::new();
        for pool in &self.reader.string_tables {
            if offset >= pool.len() {
                offset -= pool.len();
                continue;
            }
            let tail = &pool[offset..];
            if let Some(end) = tail.iter().position(|&byte| byte == 0) {
                bytes.extend_from_slice(&tail[..end]);
                return Ok(bytes);
            }
            bytes.extend_from_slice(tail);
            offset = 0;
        }
        // Do not manufacture the zero-filled unknown pools of C++ Skip.
        anyhow::bail!("Spell string references unavailable or unterminated pool")
    }
}
