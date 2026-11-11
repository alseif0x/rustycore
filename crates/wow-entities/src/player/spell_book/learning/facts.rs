//! Private learning rules over admitted immutable facts, not source adapters.
use super::{LearningSourceFailure, SpellLearningSource};
use crate::{SpellBookDefinition, SpellBookEffect, SpellLearningSourceError};

pub(super) fn spell<S: SpellLearningSource>(
    sources: &S,
    id: u32,
) -> Result<Option<S::Definition<'_>>, LearningSourceFailure<S>> {
    sources
        .definition(id, 0)
        .map_err(SpellLearningSourceError::Definition)
}
pub(super) fn passive<S: SpellLearningSource>(
    sources: &S,
    id: u32,
) -> Result<bool, LearningSourceFailure<S>> {
    Ok(spell(sources, id)?.is_some_and(|view| view.attributes()[0] & 0x40 != 0))
}
pub(super) fn primary_profession_first<S: SpellLearningSource>(
    sources: &S,
    view: &impl SpellBookDefinition,
) -> bool {
    // 02245dcd SharedDefines.h:6376-6385 / SpellMgr.cpp:107-111:
    // primary profession is category 11 and has no parent skill line.
    // SpellInfo.cpp:1666-1685,4428-4434: an absent ChainEntry has rank 1.
    sources
        .spell_rank_node(view.spell_id())
        .map_or(1, |node| node.rank)
        == 1
        && view.effects().any(|effect| {
            effect.effect_kind() == 118
                && sources
                    .skill_line(effect.misc_value_0() as u32)
                    .is_some_and(|line| line.category == 11 && line.parent_skill == 0)
        })
}
/// Shared AddSpell/RemoveSpell range phase only. Callers own their distinct
/// initial value/minimum and final cap rules. Keep lazy GetLevel ordering.
pub(super) fn learned_skill_range<S: SpellLearningSource>(
    sources: &S,
    skill: u16,
    step: u16,
    mut value: u16,
    mut maximum: u16,
    level: impl FnOnce() -> u8,
) -> Result<(u16, u16), LearningSourceFailure<S>> {
    if maximum == 0 {
        if let Some(rc) = sources
            .skill_race_class_info(u32::from(skill))
            .map_err(SpellLearningSourceError::World)?
        {
            if let Some(line) = sources.skill_line(u32::from(skill)) {
                if sources.skill_tier_value(rc.tier as i32 as u32, 0).is_some() {
                    maximum = sources
                        .skill_tier_value(rc.tier as i32 as u32, (i32::from(step) - 1) as u32)
                        .expect("same existing tier") as u16;
                } else if skill == 960 || line.category == 8 {
                    maximum = 1;
                } else if line.category == 10 {
                    value = 300;
                    maximum = 300;
                } else {
                    maximum = u16::from(level()) * 5;
                }
            }
            if rc.flags & 0x10 != 0 {
                value = maximum;
            }
        }
    }
    Ok((value, maximum))
}
