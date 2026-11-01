//! Native Player self-resurrection list initialization.
//!
//! The Application owns when this participant runs in the death operation.

mod catalog;
mod cooldown;

#[cfg(test)]
#[path = "../unit_tests/self_resurrection/mod.rs"]
mod tests;

use wow_data::{DifficultyStore, ServersideSpellStoreLikeCpp, SpellClassOptionsStore, SpellStore};
use wow_entities::Player;

use self::{catalog::prepare, cooldown::SelfResurrectionPlan};

const SOULSTONE_SPELL: i32 = 3_026;
const TWISTING_NETHER_SPELL: i32 = 23_700;

/// Metadata or canonical-state gaps that prevent a safe initialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfResurrectionError {
    SpellbookUnavailable {
        spell_id: u32,
    },
    AuraStateUnavailable,
    SpellInfoKeyUnavailable {
        spell_id: u32,
        requested_difficulty: u8,
    },
    SpellInfoSourceCollision {
        spell_id: u32,
        difficulty: u8,
    },
    DifficultyCatalog(wow_data::spell::DataSpellCatalogResolutionError),
    EffectCatalog(wow_data::spell::SpellEffectResolutionError),
    ActiveEffectUnavailable {
        spell_id: u32,
        effect_index: u8,
    },
    ActiveEffectNotAura {
        spell_id: u32,
        effect_index: u8,
    },
    ClassOptionsUnavailable {
        spell_id: u32,
    },
    SpellHistoryUnavailable,
    CategoryMetadata(wow_data::SpellCategoryReadError),
}

/// Rebuild the native Player `SelfResSpells` list from the current canonical
/// aura, spellbook and history owners. The caller supplies the current map
/// difficulty and the immutable process catalogs it already owns.
///
/// All fallible reads finish before the first Player mutation or chance roll.
/// After preflight, the native list is cleared even when the prepared result is
/// empty, then candidates are evaluated and appended in fixed priority order.
pub fn initialize_self_resurrection_spells(
    player: &mut Player,
    current_difficulty: u8,
    spells: &SpellStore,
    serverside_spells: &ServersideSpellStoreLikeCpp,
    class_options: Option<&SpellClassOptionsStore>,
    difficulties: Option<&DifficultyStore>,
    mut roll_chance: impl FnMut(u32) -> bool,
) -> Result<(), SelfResurrectionError> {
    let plan = prepare(
        player,
        current_difficulty,
        spells,
        serverside_spells,
        class_options,
        difficulties,
    )?;

    player.clear_self_res_spells();

    let mut has_twisting_nether = false;
    // Source rolls once for every eligible 23701 DUMMY effect. These rolls
    // all use the same chance and only set one boolean, so counting them keeps
    // the exact RNG call count and result without storing application order.
    for _ in 0..plan.twisting_nether_rolls {
        if roll_chance(10) {
            has_twisting_nether = true;
        }
    }

    if plan.has_soulstone {
        player.add_self_res_spell(SOULSTONE_SPELL);
    }
    if has_twisting_nether {
        player.add_self_res_spell(TWISTING_NETHER_SPELL);
    }
    if plan.reincarnation_ready {
        player.add_self_res_spell(21169);
    }

    Ok(())
}

/// Resolve C++ `HasAuraTypeWithAffectMask(SPELL_AURA_IGNORE_SPELL_COOLDOWN)`
/// against every active Player aura and the target's selected family data.
pub fn active_spell_cooldown_aura_affects_like_cpp(
    player: &Player,
    target_spell_id: u32,
    target_difficulty: u8,
    spells: &SpellStore,
    serverside_spells: &ServersideSpellStoreLikeCpp,
    class_options: Option<&SpellClassOptionsStore>,
    difficulties: Option<&DifficultyStore>,
) -> Result<bool, SelfResurrectionError> {
    catalog::active_spell_cooldown_aura_affects_like_cpp(
        player,
        target_spell_id,
        target_difficulty,
        spells,
        serverside_spells,
        class_options,
        difficulties,
    )
}

impl SelfResurrectionPlan {
    pub(super) const fn new(
        has_soulstone: bool,
        twisting_nether_rolls: usize,
        reincarnation_ready: bool,
    ) -> Self {
        Self {
            has_soulstone,
            twisting_nether_rolls,
            reincarnation_ready,
        }
    }
}
