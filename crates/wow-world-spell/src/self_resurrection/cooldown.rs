use wow_data::{DifficultyStore, SpellStore};
use wow_entities::Player;

use super::{SelfResurrectionError, catalog::SpellSource};

pub(super) struct SelfResurrectionPlan {
    pub(super) has_soulstone: bool,
    pub(super) twisting_nether_rolls: usize,
    pub(super) reincarnation_ready: bool,
}

pub(super) fn reincarnation_ready(
    player: &Player,
    ignored_by_aura: bool,
    target: &SpellSource<'_>,
    spell_id: u32,
    spells: &SpellStore,
    difficulties: Option<&DifficultyStore>,
) -> Result<bool, SelfResurrectionError> {
    if ignored_by_aura {
        return Ok(true);
    }

    let history = &player.unit().subsystems().spells.history;
    if !history.cooldowns_loaded {
        return Err(SelfResurrectionError::SpellHistoryUnavailable);
    }
    if history.cooldowns.contains_key(&spell_id) {
        return Ok(false);
    }

    let category_id = target.category_id(spell_id, spells, difficulties)?;
    Ok(category_id == 0 || !history.category_cooldowns.contains_key(&category_id))
}
