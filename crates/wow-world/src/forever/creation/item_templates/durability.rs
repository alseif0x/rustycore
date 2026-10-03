//! Exact FillMaxDurability, 02245dcd ObjectMgr.cpp:3020-3107.
//! Invalid C++ array indexes fail closed; no invented fallback durability.
use super::ItemTemplateError;
pub(super) fn maximum(
    class: u32,
    subclass: u32,
    inventory: u32,
    quality: u32,
    level: u32,
) -> Result<u32, ItemTemplateError> {
    if class != 2 && class != 4 {
        return Ok(0);
    }
    // MAX_ITEM_QUALITY is nine. Source has eight explicit initializers,
    // so WoW-token quality eight has an implicitly zero ninth multiplier.
    const QUALITY: [f32; 9] = [0.92, 0.92, 0.92, 1.11, 1.32, 1.61, 0.0, 0.0, 0.0];
    const ARMOR: [f32; 21] = [
        0.0, 0.60, 0.0, 0.60, 0.0, 1.0, 0.33, 0.72, 0.48, 0.33, 0.33, 0.0, 0.0, 0.0, 0.72, 0.0,
        0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    const WEAPON: [f32; 21] = [
        0.91, 1.0, 1.0, 1.0, 0.91, 1.0, 1.0, 0.91, 1.0, 1.0, 1.0, 0.0, 0.0, 0.66, 0.0, 0.66, 0.0,
        0.0, 1.0, 0.66, 0.66,
    ];
    if class == 4 && inventory > 20 {
        return Ok(0);
    } // Before source quality indexing.
    let quality = *QUALITY
        .get(quality as usize)
        .ok_or(ItemTemplateError::InvalidDurabilityQuality)?;
    let penalty = if level <= 28 {
        0.966f32 - (28 - level) as f32 / 54.0f32
    } else {
        1.0
    };
    let rounded = if class == 4 {
        (25.0f32 * quality * ARMOR[inventory as usize] * penalty).round()
    } else {
        (18.0f32
            * quality
            * *WEAPON
                .get(subclass as usize)
                .ok_or(ItemTemplateError::InvalidDurabilityWeapon)?
            * penalty)
            .round()
    };
    Ok(5 * rounded as u32)
}
