//! 02245dcd SpellMgr.cpp:897-956, exact admission and observed SQL order.
#[cfg(test)]
mod tests;
use super::{SpellDefinitionError, SpellDefinitionSeeds, SpellRankNode};
use std::collections::BTreeMap;
use wow_persistence::forever::spells::SpellRequiredRow;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RequiredSpellCounts {
    pub input_rows: usize,
    pub missing_source: usize,
    pub missing_required: usize,
    pub same_rank_chain: usize,
    pub duplicates: usize,
    pub relations: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellRequiredError {
    RequiresRanks,
    AlreadyApplied,
    DefinitionLookup(SpellDefinitionError),
}
pub(super) struct LoadedRequired {
    rows: Vec<SpellRequiredRow>,
    by_source: BTreeMap<u32, Vec<usize>>,
    by_required: BTreeMap<u32, Vec<usize>>,
    counts: RequiredSpellCounts,
}
impl SpellDefinitionSeeds {
    pub fn with_spell_required(
        mut self,
        rows: Vec<SpellRequiredRow>,
    ) -> Result<Self, SpellRequiredError> {
        if self.required.is_some() {
            return Err(SpellRequiredError::AlreadyApplied);
        }
        if self.ranks.is_none() {
            return Err(SpellRequiredError::RequiresRanks);
        }
        let mut loaded = LoadedRequired {
            rows: vec![],
            by_source: BTreeMap::new(),
            by_required: BTreeMap::new(),
            counts: Default::default(),
        };
        for row in rows {
            loaded.counts.input_rows += 1;
            let Some(source) = self
                .get(row.spell, 0)
                .map_err(SpellRequiredError::DefinitionLookup)?
            else {
                loaded.counts.missing_source += 1;
                continue;
            };
            let Some(required) = self
                .get(row.required, 0)
                .map_err(SpellRequiredError::DefinitionLookup)?
            else {
                loaded.counts.missing_required += 1;
                continue;
            };
            // IsRankOf compares first-rank SpellInfo identity, even for an
            // unranked self requirement; not just two nonzero chain IDs.
            let first = |id, difficulty| {
                self.spell_rank_node(id)
                    .map_or((id, difficulty), |node: &SpellRankNode| node.first)
            };
            if first(source.spell_id(), source.difficulty())
                == first(required.spell_id(), required.difficulty())
            {
                loaded.counts.same_rank_chain += 1;
                continue;
            }
            if loaded
                .by_required
                .get(&row.required)
                .into_iter()
                .flatten()
                .any(|&index| loaded.rows[index].spell == row.spell)
            {
                loaded.counts.duplicates += 1;
                continue;
            }
            let index = loaded.rows.len();
            loaded.by_source.entry(row.spell).or_default().push(index);
            loaded
                .by_required
                .entry(row.required)
                .or_default()
                .push(index);
            loaded.rows.push(row);
            loaded.counts.relations += 1;
        }
        self.required = Some(loaded);
        Ok(self)
    }
    pub fn required_spell_counts(&self) -> Option<RequiredSpellCounts> {
        self.required.as_ref().map(|r| r.counts)
    }
    pub fn spells_required_for(&self, spell: u32) -> Option<impl Iterator<Item = u32>> {
        let loaded = self.required.as_ref()?;
        Some(
            loaded
                .by_source
                .get(&spell)
                .into_iter()
                .flatten()
                .map(move |&index| loaded.rows[index].required),
        )
    }
    pub fn spells_requiring(&self, required: u32) -> Option<impl Iterator<Item = u32>> {
        let loaded = self.required.as_ref()?;
        Some(
            loaded
                .by_required
                .get(&required)
                .into_iter()
                .flatten()
                .map(move |&index| loaded.rows[index].spell),
        )
    }
    pub fn is_spell_requiring(&self, spell: u32, required: u32) -> Option<bool> {
        Some(
            self.spells_requiring(required)?
                .any(|source| source == spell),
        )
    }
}
