use super::effects::{apply_effect_column, apply_operation};
use super::{
    PLAYER_VISIBLE_ITEM_SLOT_END_LIKE_CPP, PlayerSkills, SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP,
    SKILL_ENCHANTMENT_SLOT_PERMANENT_LIKE_CPP, SkillEnchantmentApplyArgs,
    SkillEnchantmentEffectOperation, SkillEnchantmentEffects, SkillEnchantmentError,
    SkillEnchantmentItemFacts, SkillEnchantmentSource,
};
use wow_core::ObjectGuid;

/// C++ `Player::ApplyEnchantment(Item*, EnchantmentSlot, bool, bool, bool)`.
/// The item is resolved from its canonical GUID and never borrowed across an
/// effect callback.
pub(super) fn apply_enchantment<S, E>(
    skills: &mut PlayerSkills,
    item_guid: Option<ObjectGuid>,
    slot: u16,
    apply: bool,
    args: SkillEnchantmentApplyArgs,
    sources: &S,
    effects: &mut E,
) -> Result<(), SkillEnchantmentError<S::Error, E::Error>>
where
    S: SkillEnchantmentSource,
    E: SkillEnchantmentEffects,
{
    let _ = apply_one(
        skills, item_guid, slot, apply, args, false, sources, effects,
    )?;
    Ok(())
}

/// C++ `Player::ApplyEnchantment(Item*, bool)`: it always uses the native
/// duration and condition defaults while applying slots in ascending order.
pub(super) fn apply_all_enchantments<S, E>(
    skills: &mut PlayerSkills,
    item_guid: Option<ObjectGuid>,
    apply: bool,
    sources: &S,
    effects: &mut E,
) -> Result<(), SkillEnchantmentError<S::Error, E::Error>>
where
    S: SkillEnchantmentSource,
    E: SkillEnchantmentEffects,
{
    let mut item_was_resolved = false;
    for slot in 0..SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP as u16 {
        item_was_resolved |= apply_one(
            skills,
            item_guid,
            slot,
            apply,
            SkillEnchantmentApplyArgs::new(),
            item_was_resolved,
            sources,
            effects,
        )?;
    }
    Ok(())
}

pub(super) fn apply_one_for_scan<S, E>(
    skills: &mut PlayerSkills,
    item_guid: ObjectGuid,
    slot: u16,
    apply: bool,
    sources: &S,
    effects: &mut E,
) -> Result<(), SkillEnchantmentError<S::Error, E::Error>>
where
    S: SkillEnchantmentSource,
    E: SkillEnchantmentEffects,
{
    let _ = apply_one(
        skills,
        Some(item_guid),
        slot,
        apply,
        SkillEnchantmentApplyArgs::new(),
        true,
        sources,
        effects,
    )?;
    Ok(())
}

fn apply_one<S, E>(
    skills: &mut PlayerSkills,
    item_guid: Option<ObjectGuid>,
    slot: u16,
    apply: bool,
    args: SkillEnchantmentApplyArgs,
    item_required: bool,
    sources: &S,
    effects: &mut E,
) -> Result<bool, SkillEnchantmentError<S::Error, E::Error>>
where
    S: SkillEnchantmentSource,
    E: SkillEnchantmentEffects,
{
    let Some(item_guid) = item_guid else {
        return Ok(false);
    };
    let Some(item) = effects
        .item_by_guid(item_guid)
        .map_err(SkillEnchantmentError::Effect)?
    else {
        return if item_required {
            Err(SkillEnchantmentError::ItemInvalidated(item_guid))
        } else {
            Ok(false)
        };
    };

    // Preserve source guard order: item/equipped precede slot validation.
    if !item.equipped {
        return Ok(true);
    }
    if usize::from(slot) >= SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP {
        return Ok(true);
    }
    let enchantment_id = item.enchantment_ids[usize::from(slot)];
    if enchantment_id == 0 {
        return Ok(true);
    }
    let Some(record) = sources
        .enchantment(enchantment_id)
        .map_err(SkillEnchantmentError::Source)?
    else {
        return Ok(true);
    };

    if !args.ignore_condition() && record.condition_id != 0 {
        if !effects
            .enchantment_condition_fits(skills, record.condition_id)
            .map_err(SkillEnchantmentError::Effect)?
        {
            return Ok(true);
        }
    }
    if u32::from(
        effects
            .player_level(skills)
            .map_err(SkillEnchantmentError::Effect)?,
    ) < record.min_level
    {
        return Ok(true);
    }
    if record.required_skill_id != 0
        && record.required_skill_rank > u32::from(skills.value(record.required_skill_id))
    {
        return Ok(true);
    }

    if is_socket_slot(slot) {
        let socket_index = usize::from(slot - 2);
        if item.socket_colors[socket_index] == 0 {
            // In this overload a missing prismatic row returns from the whole
            // operation; only UpdateSkillEnchantments skips that sub-branch.
            let prismatic_id = item.enchantment_ids[6];
            let Some(prismatic) = sources
                .enchantment(prismatic_id)
                .map_err(SkillEnchantmentError::Source)?
            else {
                return Ok(true);
            };
            if prismatic.required_skill_id != 0
                && prismatic.required_skill_rank
                    > u32::from(skills.value(prismatic.required_skill_id))
            {
                return Ok(true);
            }
        }

        if let Some(gem_item_id) = item.socketed_gem_item_ids[socket_index] {
            if let Some(gem_template) = sources
                .item_template(gem_item_id)
                .map_err(SkillEnchantmentError::Source)?
            {
                if gem_template.required_skill_id != 0
                    && gem_template.required_skill_rank
                        > u32::from(skills.value(gem_template.required_skill_id))
                {
                    return Ok(true);
                }
            }
        }
    }

    // Source reads broken status once before the effect loop. Item slot and
    // identity-dependent facts are refreshed before each column and tail.
    if !item.broken {
        for column in 0..3 {
            let current = required_item(effects, item_guid)?;
            apply_effect_column(
                skills, &record, current, slot, column, apply, sources, effects,
            )?;
        }
    }

    let current = required_item(effects, item_guid)?;
    if slot == SKILL_ENCHANTMENT_SLOT_PERMANENT_LIKE_CPP
        && current.inventory_slot < PLAYER_VISIBLE_ITEM_SLOT_END_LIKE_CPP
    {
        apply_operation(
            skills,
            effects,
            item_guid,
            SkillEnchantmentEffectOperation::UpdateVisibleItemVisual {
                item_guid,
                inventory_slot: current.inventory_slot,
            },
        )?;
    }

    if args.apply_duration() {
        if apply {
            let current = required_item(effects, item_guid)?;
            let duration = current.enchantment_durations[usize::from(slot)];
            if duration > 0 {
                apply_operation(
                    skills,
                    effects,
                    item_guid,
                    SkillEnchantmentEffectOperation::AddEnchantmentDuration {
                        item_guid,
                        slot,
                        duration_ms: duration,
                    },
                )?;
            }
        } else {
            apply_operation(
                skills,
                effects,
                item_guid,
                SkillEnchantmentEffectOperation::AddEnchantmentDuration {
                    item_guid,
                    slot,
                    duration_ms: 0,
                },
            )?;
        }
    }

    Ok(true)
}

fn required_item<E, SError>(
    effects: &E,
    item_guid: ObjectGuid,
) -> Result<SkillEnchantmentItemFacts, SkillEnchantmentError<SError, E::Error>>
where
    E: SkillEnchantmentEffects,
{
    effects
        .item_by_guid(item_guid)
        .map_err(SkillEnchantmentError::Effect)?
        .ok_or(SkillEnchantmentError::ItemInvalidated(item_guid))
}

const fn is_socket_slot(slot: u16) -> bool {
    matches!(slot, 2 | 3 | 4)
}
