//! 02245dcd SpellMgr.cpp:823-895, source ascending ability/root order.
//! Canonical nodes reference existing definitions by exact resolved keys.
#[cfg(test)]
mod tests;
use super::{Key, SpellDefinitionError, SpellDefinitionSeeds};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpellRankNode {
    pub previous: Option<(u32, i16)>,
    pub next: Option<(u32, i16)>,
    pub first: (u32, i16),
    pub last: (u32, i16),
    pub rank: u8,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpellRankCounts {
    pub ability_rows: usize,
    pub unavailable_baseline_abilities: usize,
    pub skipped_missing_definitions: usize,
    pub overwritten_links: usize,
    pub roots: usize,
    pub nodes: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellRankError {
    RequiresTargetCaps,
    MissingSkillAbilityMap,
    AlreadyApplied,
    DefinitionLookup(SpellDefinitionError),
    ReachableCycle,
    UndefinedRankTraversal,
}
pub(super) struct LoadedRanks {
    pub(super) nodes: BTreeMap<u32, SpellRankNode>,
    pub(super) counts: SpellRankCounts,
}

impl SpellDefinitionSeeds {
    pub fn with_spell_ranks(mut self) -> Result<Self, SpellRankError> {
        if self.ranks.is_some() {
            return Err(SpellRankError::AlreadyApplied);
        }
        if self.target_caps.is_none() {
            return Err(SpellRankError::RequiresTargetCaps);
        }
        let index = self
            .skill_line_abilities
            .as_ref()
            .ok_or(SpellRankError::MissingSkillAbilityMap)?;
        let mut counts = SpellRankCounts {
            unavailable_baseline_abilities: index.catalog().counts()[5],
            ..Default::default()
        };
        let mut chains = BTreeMap::<u32, u32>::new();
        let mut previous = BTreeSet::new();
        for ability in index.catalog().skill_ability_records() {
            counts.ability_rows += 1;
            if ability.supercedes_spell == 0 {
                continue;
            }
            let before = ability.supercedes_spell as u32;
            let after = ability.spell as u32;
            if self
                .get(before, 0)
                .map_err(SpellRankError::DefinitionLookup)?
                .is_none()
                || self
                    .get(after, 0)
                    .map_err(SpellRankError::DefinitionLookup)?
                    .is_none()
            {
                counts.skipped_missing_definitions += 1;
                continue;
            }
            counts.overwritten_links += usize::from(chains.insert(before, after).is_some());
            previous.insert(after); // Includes overwritten link's old destination.
        }
        let mut nodes: BTreeMap<u32, SpellRankNode> = BTreeMap::new();
        for (&root, &second) in &chains {
            if previous.contains(&root) {
                continue;
            }
            counts.roots += 1;
            let first = self.rank_key(root)?;
            let next = self.rank_key(second)?;
            nodes.insert(
                root,
                SpellRankNode {
                    previous: None,
                    next: Some(next),
                    first,
                    last: next,
                    rank: 1,
                },
            );
            nodes.insert(
                second,
                SpellRankNode {
                    previous: Some(first),
                    next: None,
                    first,
                    last: next,
                    rank: 2,
                },
            );
            let mut rank = 3u8;
            let mut current = second;
            let mut visited = BTreeSet::from([root, second]);
            while let Some(&after) = chains.get(&current) {
                if !visited.insert(after) {
                    return Err(SpellRankError::ReachableCycle);
                }
                let before = self.rank_key(current)?;
                let last = self.rank_key(after)?;
                nodes
                    .get_mut(&current)
                    .expect("source initialized predecessor")
                    .next = Some(last);
                nodes.insert(
                    after,
                    SpellRankNode {
                        previous: Some(before),
                        next: None,
                        first,
                        last,
                        rank,
                    },
                );
                rank = rank.wrapping_add(1); // Source uint8 increment wraps.
                let mut back = Some(before);
                let mut back_seen = BTreeSet::new();
                while let Some(key) = back {
                    if !back_seen.insert(key.0) {
                        return Err(SpellRankError::ReachableCycle);
                    }
                    let node = nodes.get_mut(&key.0).expect("initialized previous rank");
                    node.last = last;
                    back = node.previous;
                }
                current = after;
            }
        }
        counts.nodes = nodes.len();
        self.ranks = Some(LoadedRanks { nodes, counts });
        Ok(self)
    }
    fn rank_key(&self, id: u32) -> Result<Key, SpellRankError> {
        self.get(id, 0)
            .map_err(SpellRankError::DefinitionLookup)?
            .map(|view| (view.spell_id(), view.difficulty()))
            .ok_or(SpellRankError::UndefinedRankTraversal)
    }
    pub fn spell_rank_counts(&self) -> Option<SpellRankCounts> {
        self.ranks.as_ref().map(|r| r.counts)
    }
    pub fn spell_rank_node(&self, id: u32) -> Option<&SpellRankNode> {
        self.ranks.as_ref()?.nodes.get(&id)
    }
    pub fn spell_rank(&self, id: u32) -> u8 {
        self.spell_rank_node(id).map_or(0, |node| node.rank)
    }
    pub fn first_spell_in_chain(&self, id: u32) -> u32 {
        self.spell_rank_node(id).map_or(id, |node| node.first.0)
    }
    pub fn last_spell_in_chain(&self, id: u32) -> u32 {
        self.spell_rank_node(id).map_or(id, |node| node.last.0)
    }
    pub fn next_spell_in_chain(&self, id: u32) -> u32 {
        self.spell_rank_node(id)
            .and_then(|node| node.next)
            .map_or(0, |key| key.0)
    }
    pub fn previous_spell_in_chain(&self, id: u32) -> u32 {
        self.spell_rank_node(id)
            .and_then(|node| node.previous)
            .map_or(0, |key| key.0)
    }
    pub fn spell_with_rank(
        &self,
        mut id: u32,
        rank: u32,
        strict: bool,
    ) -> Result<u32, SpellRankError> {
        let mut seen = BTreeSet::new();
        loop {
            if !seen.insert(id) {
                return Err(SpellRankError::ReachableCycle);
            }
            let Some(node) = self.spell_rank_node(id) else {
                return Ok(if strict && rank > 1 { 0 } else { id });
            };
            if rank == u32::from(node.rank) {
                return Ok(id);
            }
            id = (if u32::from(node.rank) < rank {
                node.next
            } else {
                node.previous
            })
            .ok_or(SpellRankError::UndefinedRankTraversal)?
            .0;
        }
    }
}
