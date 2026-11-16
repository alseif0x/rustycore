use super::apply::apply_one_for_scan;
use super::{
    PLAYER_ITEM_SLOT_END_LIKE_CPP, PlayerSkills, SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP,
    SkillEnchantmentEffects, SkillEnchantmentError, SkillEnchantmentSource,
};

/// C++ `Player::UpdateSkillEnchantments`, preserving live slot reads and
/// immediate application order. The caller explicitly narrows the native
/// skill ID to `u16` before entering this source-width operation.
pub(super) fn update_skill_enchantments<S, E>(
    skills: &mut PlayerSkills,
    skill_id: u16,
    curr_value: u16,
    new_value: u16,
    sources: &S,
    effects: &mut E,
) -> Result<(), SkillEnchantmentError<S::Error, E::Error>>
where
    S: SkillEnchantmentSource,
    E: SkillEnchantmentEffects,
{
    for inventory_slot in 0..PLAYER_ITEM_SLOT_END_LIKE_CPP {
        let Some(mut last_item) = effects
            .item_in_slot(inventory_slot)
            .map_err(SkillEnchantmentError::Effect)?
        else {
            continue;
        };

        for enchantment_slot in 0..SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP as u16 {
            let item = effects
                .item_in_slot(inventory_slot)
                .map_err(SkillEnchantmentError::Effect)?
                .ok_or(SkillEnchantmentError::ItemInvalidated(last_item.item_guid))?;
            last_item = item;
            let enchantment_id = item.enchantment_ids[usize::from(enchantment_slot)];
            if enchantment_id == 0 {
                continue;
            }

            let Some(enchantment) = sources
                .enchantment(enchantment_id)
                .map_err(SkillEnchantmentError::Source)?
            else {
                // C++ returns from the entire scan on a missing direct row.
                return Ok(());
            };

            if enchantment.required_skill_id == u32::from(skill_id)
                && let Some(apply) =
                    threshold_crossing(curr_value, new_value, enchantment.required_skill_rank)
            {
                apply_one_for_scan(
                    skills,
                    item.item_guid,
                    enchantment_slot,
                    apply,
                    sources,
                    effects,
                )?;
            }

            if matches!(enchantment_slot, 2 | 3 | 4) {
                let current = effects
                    .item_in_slot(inventory_slot)
                    .map_err(SkillEnchantmentError::Effect)?
                    .ok_or(SkillEnchantmentError::ItemInvalidated(last_item.item_guid))?;
                if current.socket_colors[usize::from(enchantment_slot - 2)] == 0 {
                    let prismatic_id = current.enchantment_ids[6];
                    if let Some(prismatic) = sources
                        .enchantment(prismatic_id)
                        .map_err(SkillEnchantmentError::Source)?
                        && prismatic.required_skill_id == u32::from(skill_id)
                        && let Some(apply) =
                            threshold_crossing(curr_value, new_value, prismatic.required_skill_rank)
                    {
                        apply_one_for_scan(
                            skills,
                            current.item_guid,
                            enchantment_slot,
                            apply,
                            sources,
                            effects,
                        )?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn threshold_crossing(old: u16, new: u16, threshold: u32) -> Option<bool> {
    if u32::from(old) < threshold && u32::from(new) >= threshold {
        Some(true)
    } else if u32::from(new) < threshold && u32::from(old) >= threshold {
        Some(false)
    } else {
        None
    }
}
