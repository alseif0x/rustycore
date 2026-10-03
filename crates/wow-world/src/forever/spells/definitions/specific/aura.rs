//! Source order is category/family/dispel/mechanics/frost/IDs/banish/default.
use super::*;
pub(super) fn classify(
    definition: &Definition,
    id: u32,
) -> Result<SpellAuraState, SpellSpecificError> {
    use SpellAuraState::*;
    let fields = &definition.fields;
    let flags = fields.spell_family_flags;
    if fields.category_id == 1133 {
        return Ok(FaerieFire);
    }
    if fields.spell_family_name == 7 && (flags[0] & 0x50 != 0 || flags[1] & 0x4000000 != 0) {
        return Ok(DruidPeriodicHeal);
    }
    if fields.spell_family_name == 8 && flags[0] & 0x10000 != 0 {
        return Ok(RoguePoisoned);
    }
    if fields.dispel == 9 {
        return Ok(Enraged);
    }
    let mut mechanics = 0u64;
    if fields.mechanic != 0 {
        mechanics |= 1u64
            .checked_shl(fields.mechanic)
            .ok_or(SpellSpecificError::UndefinedMechanicShift)?;
    }
    for effect in &definition.effects {
        if effect.effect != 0 && effect.mechanic != 0 {
            mechanics |= 1u64
                .checked_shl(effect.mechanic)
                .ok_or(SpellSpecificError::UndefinedMechanicShift)?;
        }
    }
    if mechanics & (1 << 15) != 0 {
        return Ok(Bleed);
    }
    if fields.school_mask & 0x10 != 0
        && definition
            .effects
            .iter()
            .any(|effect| effect.is_aura() && matches!(effect.aura, 12 | 26 | 455))
    {
        return Ok(Frozen);
    }
    match id {
        1064 => return Ok(Dazed),
        32216 => return Ok(Victorious),
        71465 | 50241 | 81262 => return Ok(RaidEncounter),
        6950 | 9806 | 9991 | 13424 | 13752 | 16432 | 20656 | 25602 | 32129 | 35325 | 35328
        | 35329 | 35331 | 49163 | 65863 | 79559 | 82855 | 102953 | 127907 | 127913 | 129007
        | 130159 | 142537 | 168455 | 176905 | 189502 | 201785 | 201786 | 201935 | 239233
        | 319400 | 321470 | 331134 => return Ok(FaerieFire),
        _ => {}
    }
    if fields.mechanic == 18 {
        return Ok(Banished);
    }
    Ok(None)
}
