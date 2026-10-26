use wow_data::{
    DifficultyStore, ServersideSpellInfoLikeCpp, ServersideSpellStoreLikeCpp,
    SpellClassOptionsStore, SpellEffectInfo, SpellStore,
};
use wow_entities::Player;

use super::{SelfResurrectionError, cooldown::SelfResurrectionPlan};

const REINCARNATION_PASSIVE: i32 = 20_608;
const REINCARNATION_ACTIVE: u32 = 21_169;
const TWISTING_NETHER_AURA: u32 = 23_701;
const WARLOCK_FAMILY: u32 = 5;
const DUMMY_AURA: i32 = 4;
const SOULSTONE_FAMILY_FLAG_WORD: usize = 1;
const SOULSTONE_FAMILY_FLAG: u32 = 0x0100_0000;
const MAX_EFFECTS: u8 = 32;

#[derive(Clone, Copy)]
pub(super) struct FamilyInfo {
    pub(super) name: u32,
    pub(super) flags: [u32; 4],
}

pub(super) enum SpellSource<'a> {
    Regular { difficulty: u8 },
    Server(&'a ServersideSpellInfoLikeCpp),
}

#[derive(Clone, Copy)]
pub(super) struct ActiveEffect {
    pub(super) effect_type: u32,
    pub(super) aura_type: i32,
    pub(super) class_mask: [u32; 4],
}

impl SpellSource<'_> {
    pub(super) fn family(
        &self,
        spell_id: u32,
        class_options: Option<&SpellClassOptionsStore>,
        difficulties: Option<&DifficultyStore>,
    ) -> Result<FamilyInfo, SelfResurrectionError> {
        match self {
            Self::Regular { difficulty } => {
                let class_options = class_options
                    .ok_or(SelfResurrectionError::ClassOptionsUnavailable { spell_id })?;
                let (name, flags) = wow_data::spell::resolve_spell_family(
                    class_options,
                    spell_id as i32,
                    *difficulty,
                    difficulties,
                )
                .map_err(SelfResurrectionError::DifficultyCatalog)?;
                Ok(FamilyInfo {
                    name: u32::from(name),
                    flags,
                })
            }
            Self::Server(info) => Ok(FamilyInfo {
                name: info.row.spell_family_name,
                flags: info.row.spell_family_flags,
            }),
        }
    }

    pub(super) fn active_effect(
        &self,
        effect_index: u8,
        regular_effects: &[&SpellEffectInfo],
    ) -> Option<ActiveEffect> {
        let index = usize::from(effect_index);
        match self {
            Self::Regular { .. } => regular_effects
                .iter()
                .find(|effect| effect.effect_index == u32::from(effect_index))
                .map(|effect| ActiveEffect {
                    effect_type: effect.effect,
                    aura_type: effect.effect_aura,
                    class_mask: effect.effect_spell_class_mask,
                }),
            Self::Server(info) => info
                .effects
                .get(index)
                .filter(|effect| effect.effect_index == i32::from(effect_index))
                .and_then(|effect| {
                    Some(ActiveEffect {
                        effect_type: u32::try_from(effect.effect).ok()?,
                        aura_type: effect.effect_aura,
                        class_mask: effect.effect_spell_class_mask.map(|word| word as u32),
                    })
                }),
        }
    }

    pub(super) fn category_id(
        &self,
        spell_id: u32,
        spells: &SpellStore,
        difficulties: Option<&DifficultyStore>,
    ) -> Result<u32, SelfResurrectionError> {
        match self {
            Self::Regular { difficulty } => spells
                .category_id_at_difficulty(spell_id, *difficulty, difficulties)
                .map_err(SelfResurrectionError::CategoryMetadata),
            Self::Server(info) => Ok(info.row.category_id),
        }
    }
}

pub(super) fn prepare(
    player: &Player,
    current_difficulty: u8,
    spells: &SpellStore,
    serverside_spells: &ServersideSpellStoreLikeCpp,
    class_options: Option<&SpellClassOptionsStore>,
    difficulties: Option<&DifficultyStore>,
) -> Result<SelfResurrectionPlan, SelfResurrectionError> {
    let reincarnation_known = has_reincarnation_passive(player)?;

    let mut applications = player
        .unit()
        .subsystems()
        .auras
        .runtime_applications_like_cpp()
        .values()
        .filter(|application| application.effect_mask != 0)
        .collect::<Vec<_>>();
    applications.sort_unstable_by_key(|application| application.slot);

    if !player
        .unit()
        .subsystems()
        .auras
        .persisted_player_aura_authority_complete_like_cpp()
        || player
            .unit()
            .subsystems()
            .auras
            .spell_hit_aura_authority_tombstoned_like_cpp()
    {
        return Err(SelfResurrectionError::AuraStateUnavailable);
    }

    let mut has_soulstone = false;
    let mut twisting_nether_rolls = 0;
    let mut ignore_cooldown_auras = Vec::new();

    for application in applications {
        let spell_id = application.spell_id as u32;
        let source = resolve_spell_source(
            spell_id,
            application.difficulty_id,
            spells,
            serverside_spells,
            difficulties,
        )?;
        let regular_effects = match &source {
            SpellSource::Regular { difficulty } => wow_data::spell::resolve_spell_effect_rows(
                spells,
                spell_id as i32,
                *difficulty,
                difficulties,
            )
            .map_err(SelfResurrectionError::EffectCatalog)?,
            SpellSource::Server(_) => Vec::new(),
        };

        let mut family = None;
        for effect_index in 0..MAX_EFFECTS {
            let bit = 1u32 << effect_index;
            if application.effect_mask & bit == 0 {
                continue;
            }
            let effect = source.active_effect(effect_index, &regular_effects).ok_or(
                SelfResurrectionError::ActiveEffectUnavailable {
                    spell_id,
                    effect_index,
                },
            )?;
            if !is_aura_effect(effect.effect_type, effect.aura_type) {
                return Err(SelfResurrectionError::ActiveEffectNotAura {
                    spell_id,
                    effect_index,
                });
            }

            let is_dummy = effect.aura_type == DUMMY_AURA;
            let is_ignore_cooldown = reincarnation_known
                && effect.aura_type
                    == wow_data::spell::aura_types::SPELL_AURA_IGNORE_SPELL_COOLDOWN;
            if !is_dummy && !is_ignore_cooldown {
                continue;
            }

            let current_family = match family {
                Some(family) => family,
                None => {
                    let resolved = source.family(spell_id, class_options, difficulties)?;
                    family = Some(resolved);
                    resolved
                }
            };

            if is_dummy {
                if current_family.name == WARLOCK_FAMILY
                    && current_family.flags[SOULSTONE_FAMILY_FLAG_WORD] & SOULSTONE_FAMILY_FLAG != 0
                {
                    has_soulstone = true;
                } else if spell_id == TWISTING_NETHER_AURA {
                    twisting_nether_rolls += 1;
                }
            }
            if is_ignore_cooldown {
                ignore_cooldown_auras.push((current_family, effect.class_mask));
            }
        }
    }

    let reincarnation_ready = if reincarnation_known {
        let target = resolve_spell_source(
            REINCARNATION_ACTIVE,
            current_difficulty,
            spells,
            serverside_spells,
            difficulties,
        )?;
        let ignore_cooldown = aura_ignores_spell_cooldown(
            &ignore_cooldown_auras,
            &target,
            REINCARNATION_ACTIVE,
            class_options,
            difficulties,
        )?;
        super::cooldown::reincarnation_ready(
            player,
            ignore_cooldown,
            &target,
            REINCARNATION_ACTIVE,
            spells,
            difficulties,
        )?
    } else {
        false
    };

    Ok(SelfResurrectionPlan::new(
        has_soulstone,
        twisting_nether_rolls,
        reincarnation_ready,
    ))
}

fn has_reincarnation_passive(player: &Player) -> Result<bool, SelfResurrectionError> {
    let runtime = player.spell_runtime_like_cpp();
    if let Some(spell) = runtime.rows_like_cpp().get(&REINCARNATION_PASSIVE) {
        return Ok(spell.state != wow_entities::PlayerSpellLoadState::Removed && !spell.disabled);
    }
    if runtime.rows_complete_like_cpp() {
        return Ok(false);
    }
    Err(SelfResurrectionError::SpellbookUnavailable {
        spell_id: REINCARNATION_PASSIVE as u32,
    })
}

fn resolve_spell_source<'a>(
    spell_id: u32,
    requested_difficulty: u8,
    spells: &'a SpellStore,
    serverside_spells: &'a ServersideSpellStoreLikeCpp,
    difficulties: Option<&DifficultyStore>,
) -> Result<SpellSource<'a>, SelfResurrectionError> {
    let has_any_key = spells.contains_spell_info_any_difficulty_like_cpp(spell_id)
        || (u8::MIN..=u8::MAX).any(|difficulty| {
            serverside_spells
                .get_serverside_spell_like_cpp(spell_id, u32::from(difficulty))
                .is_some()
        });

    let resolved = wow_data::spell::resolve_spell_catalog_value(
        requested_difficulty,
        difficulties,
        has_any_key,
        |difficulty| {
            let regular = spells.contains_spell_info_exact_like_cpp(spell_id, difficulty);
            let server =
                serverside_spells.get_serverside_spell_like_cpp(spell_id, u32::from(difficulty));
            match (regular, server) {
                (true, Some(_)) => Some(Err(SelfResurrectionError::SpellInfoSourceCollision {
                    spell_id,
                    difficulty,
                })),
                (true, None) => Some(Ok(SpellSource::Regular { difficulty })),
                (false, Some(info)) => Some(Ok(SpellSource::Server(info))),
                (false, None) => None,
            }
        },
    )
    .map_err(SelfResurrectionError::DifficultyCatalog)?;

    match resolved {
        Some(Ok(source)) => Ok(source),
        Some(Err(error)) => Err(error),
        None => Err(SelfResurrectionError::SpellInfoKeyUnavailable {
            spell_id,
            requested_difficulty,
        }),
    }
}

fn aura_ignores_spell_cooldown(
    auras: &[(FamilyInfo, [u32; 4])],
    target: &SpellSource<'_>,
    target_spell_id: u32,
    class_options: Option<&SpellClassOptionsStore>,
    difficulties: Option<&DifficultyStore>,
) -> Result<bool, SelfResurrectionError> {
    for (aura_family, class_mask) in auras {
        if aura_family.name == 0 {
            return Ok(true);
        }
        let target_family = target.family(target_spell_id, class_options, difficulties)?;
        if affected_spell(target_family, *aura_family, *class_mask) {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) fn active_spell_cooldown_aura_affects_like_cpp(
    player: &Player,
    target_spell_id: u32,
    target_difficulty: u8,
    spells: &SpellStore,
    serverside_spells: &ServersideSpellStoreLikeCpp,
    class_options: Option<&SpellClassOptionsStore>,
    difficulties: Option<&DifficultyStore>,
) -> Result<bool, SelfResurrectionError> {
    let auras = &player.unit().subsystems().auras;
    if !auras.persisted_player_aura_authority_complete_like_cpp()
        || auras.spell_hit_aura_authority_tombstoned_like_cpp()
    {
        return Err(SelfResurrectionError::AuraStateUnavailable);
    }

    let mut applications = auras
        .runtime_applications_like_cpp()
        .values()
        .filter(|application| application.effect_mask != 0)
        .collect::<Vec<_>>();
    applications.sort_unstable_by_key(|application| application.slot);

    let mut ignore_cooldown_auras = Vec::new();
    for application in applications {
        let spell_id = application.spell_id as u32;
        let source = resolve_spell_source(
            spell_id,
            application.difficulty_id,
            spells,
            serverside_spells,
            difficulties,
        )?;
        let regular_effects = match &source {
            SpellSource::Regular { difficulty } => wow_data::spell::resolve_spell_effect_rows(
                spells,
                spell_id as i32,
                *difficulty,
                difficulties,
            )
            .map_err(SelfResurrectionError::EffectCatalog)?,
            SpellSource::Server(_) => Vec::new(),
        };
        let mut family = None;
        for effect_index in 0..MAX_EFFECTS {
            if application.effect_mask & (1u32 << effect_index) == 0 {
                continue;
            }
            let effect = source.active_effect(effect_index, &regular_effects).ok_or(
                SelfResurrectionError::ActiveEffectUnavailable {
                    spell_id,
                    effect_index,
                },
            )?;
            if effect.aura_type != wow_data::spell::aura_types::SPELL_AURA_IGNORE_SPELL_COOLDOWN {
                continue;
            }
            let current_family = match family {
                Some(family) => family,
                None => {
                    let resolved = source.family(spell_id, class_options, difficulties)?;
                    family = Some(resolved);
                    resolved
                }
            };
            ignore_cooldown_auras.push((current_family, effect.class_mask));
        }
    }

    let target = resolve_spell_source(
        target_spell_id,
        target_difficulty,
        spells,
        serverside_spells,
        difficulties,
    )?;
    aura_ignores_spell_cooldown(
        &ignore_cooldown_auras,
        &target,
        target_spell_id,
        class_options,
        difficulties,
    )
}

fn affected_spell(target: FamilyInfo, aura: FamilyInfo, class_mask: [u32; 4]) -> bool {
    if aura.name == 0 {
        return true;
    }
    if aura.name != target.name {
        return false;
    }
    class_mask == [0; 4]
        || class_mask
            .iter()
            .zip(target.flags)
            .any(|(aura_word, target_word)| aura_word & target_word != 0)
}

fn is_aura_effect(effect_type: u32, aura_type: i32) -> bool {
    if aura_type == 0 {
        return false;
    }
    use wow_data::spell::spell_effect_types::*;
    matches!(
        effect_type,
        SPELL_EFFECT_APPLY_AURA
            | SPELL_EFFECT_APPLY_AREA_AURA_PARTY
            | SPELL_EFFECT_APPLY_AREA_AURA_RAID
            | SPELL_EFFECT_APPLY_AREA_AURA_FRIEND
            | SPELL_EFFECT_APPLY_AREA_AURA_ENEMY
            | SPELL_EFFECT_APPLY_AREA_AURA_PET
            | SPELL_EFFECT_APPLY_AREA_AURA_OWNER
            | SPELL_EFFECT_APPLY_AURA_ON_PET
            | SPELL_EFFECT_APPLY_AREA_AURA_SUMMONS
            | SPELL_EFFECT_APPLY_AREA_AURA_PARTY_NONRANDOM
            | SPELL_EFFECT_PERSISTENT_AREA_AURA
    )
}
