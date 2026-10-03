//! 02245dcd SpellMgr.cpp:1001-1169, SQL snapshot/primary effects/DB2/reverse.
#[cfg(test)]
mod tests;
use super::{SpellDefinitionError, SpellDefinitionSeeds};
use std::collections::{BTreeMap, BTreeSet};
use wow_persistence::forever::spells::SpellLearnRow;
const TALENT: u32 = 0x00800000; // SpellInfo.h:167, source custom bit.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpellLearnNode {
    pub source: u32,
    pub spell: u32,
    pub overrides_spell: u32,
    pub active: bool,
    pub auto_learned: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LearnSpellCounts {
    pub input_rows: usize,
    pub empty_sql_early_return: bool,
    pub sql_nodes: usize,
    pub effect_nodes: usize,
    pub db2_nodes: usize,
    pub missing_source: usize,
    pub missing_learned: usize,
    pub rejected_talents: usize,
    pub redundant_sql: usize,
    pub db2_already_present: usize,
    pub nodes: usize,
    pub unavailable_baseline_db2: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellLearnError {
    RequiresSpecificAndAuraState,
    AlreadyApplied,
    DefinitionLookup(SpellDefinitionError),
}
pub(super) struct LoadedLearnSpells {
    nodes: Vec<SpellLearnNode>,
    by_source: BTreeMap<u32, Vec<usize>>,
    by_learned: BTreeMap<u32, Vec<usize>>,
    counts: LearnSpellCounts,
}
impl LoadedLearnSpells {
    fn insert(&mut self, node: SpellLearnNode) {
        let index = self.nodes.len();
        self.by_source.entry(node.source).or_default().push(index);
        self.nodes.push(node);
    }
    fn contains(&self, source: u32, spell: u32) -> bool {
        self.by_source
            .get(&source)
            .into_iter()
            .flatten()
            .any(|&index| self.nodes[index].spell == spell)
    }
}
impl SpellDefinitionSeeds {
    pub fn with_learn_spells(mut self, rows: Vec<SpellLearnRow>) -> Result<Self, SpellLearnError> {
        if self.learn_spells.is_some() {
            return Err(SpellLearnError::AlreadyApplied);
        }
        if self.specific.is_none() {
            return Err(SpellLearnError::RequiresSpecificAndAuraState);
        }
        let mut loaded = LoadedLearnSpells {
            nodes: vec![],
            by_source: BTreeMap::new(),
            by_learned: BTreeMap::new(),
            counts: LearnSpellCounts {
                unavailable_baseline_db2: self.catalog.counts()[31].2,
                ..Default::default()
            },
        };
        // Source returns before either automatic source pass on empty SQL.
        // Do not reinterpret that branch as an empty snapshot and continue.
        if rows.is_empty() {
            loaded.counts.empty_sql_early_return = true;
            self.learn_spells = Some(loaded);
            return Ok(self);
        }
        let mut sql_pairs = BTreeSet::new(); // Membership only; no copied nodes.
        for row in rows {
            loaded.counts.input_rows += 1;
            let Some(source) = self
                .get(row.source, 0)
                .map_err(SpellLearnError::DefinitionLookup)?
            else {
                loaded.counts.missing_source += 1;
                continue;
            };
            if self
                .get(row.learned, 0)
                .map_err(SpellLearnError::DefinitionLookup)?
                .is_none()
            {
                loaded.counts.missing_learned += 1;
                continue;
            }
            if source.custom_attributes() & TALENT != 0 {
                loaded.counts.rejected_talents += 1;
                continue;
            }
            sql_pairs.insert((row.source, row.learned));
            loaded.insert(SpellLearnNode {
                source: row.source,
                spell: row.learned,
                overrides_spell: 0,
                active: row.active,
                auto_learned: false,
            });
            loaded.counts.sql_nodes += 1;
        }
        for &key in &self
            .source_order
            .as_ref()
            .expect("specific phase admitted source order")
            .primary
        {
            if key.1 != 0 {
                continue;
            }
            let definition = &self.definitions[&key];
            for effect in &definition.effects {
                if effect.effect != 36 {
                    continue;
                }
                if self
                    .get(effect.trigger_spell, 0)
                    .map_err(SpellLearnError::DefinitionLookup)?
                    .is_none()
                {
                    loaded.counts.missing_learned += 1;
                    continue;
                }
                if sql_pairs.contains(&(key.0, effect.trigger_spell)) {
                    loaded.counts.redundant_sql += 1;
                    continue;
                }
                loaded.insert(SpellLearnNode {
                    source: key.0,
                    spell: effect.trigger_spell,
                    overrides_spell: 0,
                    active: true,
                    auto_learned: effect.implicit_targets[0] == 5
                        || definition.custom_attributes & TALENT != 0
                        || definition.fields.attributes[0] & 0x40 != 0
                        || definition.has_effect(44),
                });
                loaded.counts.effect_nodes += 1;
            }
        }
        for row in self.catalog.spell_learn_spell_records() {
            let learned = row.learn_spell_id as u32;
            if self
                .get(row.spell_id, 0)
                .map_err(SpellLearnError::DefinitionLookup)?
                .is_none()
            {
                loaded.counts.missing_source += 1;
                continue;
            }
            if self
                .get(learned, 0)
                .map_err(SpellLearnError::DefinitionLookup)?
                .is_none()
            {
                loaded.counts.missing_learned += 1;
                continue;
            }
            if sql_pairs.contains(&(row.spell_id, learned)) {
                loaded.counts.redundant_sql += 1;
                continue;
            }
            if loaded.contains(row.spell_id, learned) {
                loaded.counts.db2_already_present += 1;
                continue;
            }
            loaded.insert(SpellLearnNode {
                source: row.spell_id,
                spell: learned,
                overrides_spell: row.overrides_spell_id as u32,
                active: true,
                auto_learned: false,
            });
            loaded.counts.db2_nodes += 1;
        }
        // std::multimap iteration sorts SourceSpell, with insertion order for
        // equal keys. Reverse indices borrow the same canonical node allocation.
        for indices in loaded.by_source.values() {
            for &index in indices {
                loaded
                    .by_learned
                    .entry(loaded.nodes[index].spell)
                    .or_default()
                    .push(index);
            }
        }
        loaded.counts.nodes = loaded.nodes.len();
        self.learn_spells = Some(loaded);
        Ok(self)
    }
    pub fn learn_spell_counts(&self) -> Option<LearnSpellCounts> {
        self.learn_spells.as_ref().map(|l| l.counts)
    }
    pub fn spell_learn_nodes(&self, source: u32) -> Option<impl Iterator<Item = &SpellLearnNode>> {
        let loaded = self.learn_spells.as_ref()?;
        Some(
            loaded
                .by_source
                .get(&source)
                .into_iter()
                .flatten()
                .map(move |&index| &loaded.nodes[index]),
        )
    }
    pub fn spell_learned_by(&self, learned: u32) -> Option<impl Iterator<Item = &SpellLearnNode>> {
        let loaded = self.learn_spells.as_ref()?;
        Some(
            loaded
                .by_learned
                .get(&learned)
                .into_iter()
                .flatten()
                .map(move |&index| &loaded.nodes[index]),
        )
    }
    pub fn is_spell_learn_to(&self, source: u32, learned: u32) -> Option<bool> {
        Some(
            self.spell_learn_nodes(source)?
                .any(|node| node.spell == learned),
        )
    }
}

/// Numeric metadata injection, cfg(test) only, within its actual private owner.
/// Node identities/reverse indices stay unchanged; not DB2 admission evidence.
#[cfg(test)]
pub(crate) fn spell_book_test_learn_flags(
    seeds: &mut SpellDefinitionSeeds,
    source: u32,
    spell: u32,
    override_spell: u32,
    automatic: bool,
) {
    let loaded = seeds.learn_spells.as_mut().expect("test learning phase");
    for node in &mut loaded.nodes {
        if node.source == source && node.spell == spell {
            node.overrides_spell = override_spell;
            node.auto_learned = automatic;
        }
    }
}
