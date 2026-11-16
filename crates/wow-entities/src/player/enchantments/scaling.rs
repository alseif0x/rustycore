use super::{
    PlayerSkills, SkillEnchantmentEffects, SkillEnchantmentError, SkillEnchantmentRecord,
    SkillEnchantmentSource,
};

const SPELL_ITEM_ENCHANTMENT_SCALE_AS_A_GEM_LIKE_CPP: i32 = 0x020;

pub(super) fn amount_for_column<S, E>(
    record: &SkillEnchantmentRecord,
    column: usize,
    skills: &PlayerSkills,
    sources: &S,
    effects: &E,
) -> Result<i32, SkillEnchantmentError<S::Error, E::Error>>
where
    S: SkillEnchantmentSource,
    E: SkillEnchantmentEffects,
{
    let raw_amount = record.effect_amounts[column];
    if record.scaling_class == 0 {
        return Ok(raw_amount);
    }

    let (minimum_item_level, maximum_item_level) = effects
        .item_level_limits(skills)
        .map_err(SkillEnchantmentError::Effect)?;
    let scaling_class = if (minimum_item_level != 0 || maximum_item_level != 0)
        && record.scaling_class_restricted != 0
    {
        record.scaling_class_restricted
    } else {
        record.scaling_class
    };

    let minimum_level = if record.flags & SPELL_ITEM_ENCHANTMENT_SCALE_AS_A_GEM_LIKE_CPP != 0 {
        1
    } else {
        60
    };
    let player_level = effects
        .player_level(skills)
        .map_err(SkillEnchantmentError::Effect)?;
    let maximum_level = if record.max_level != 0 {
        record.max_level as u8
    } else {
        sources
            .spell_scaling_row_count()
            .map_err(SkillEnchantmentError::Source)?
            .wrapping_sub(1) as u8
    };

    // The source tests the minimum first and uses `else if` for maximum.
    let scaling_level = if minimum_level > player_level {
        minimum_level
    } else if maximum_level < player_level {
        maximum_level
    } else {
        player_level
    };

    let Some(scaling_value) = sources
        .spell_scaling_column(scaling_level, scaling_class)
        .map_err(SkillEnchantmentError::Source)?
    else {
        return Ok(raw_amount);
    };

    let product = record.scaling_points[column] * scaling_value;
    if !product.is_finite() || product < i32::MIN as f32 || product >= 2_147_483_648.0_f32 {
        return Err(SkillEnchantmentError::InvalidScalingConversion {
            enchantment_id: record.id,
            column: column as u8,
            product_bits: product.to_bits(),
        });
    }
    Ok(product.trunc() as i32)
}
