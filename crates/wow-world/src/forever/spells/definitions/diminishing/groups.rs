//! Exact ordered family/ID rules, SpellInfo.cpp:2993-3357. No mechanic heuristic.
use super::*;
use DiminishingGroup::*;

pub(super) fn compute(
    definition: &Definition,
    id: u32,
    visual: &mut impl FnMut() -> Result<u32, SpellDiminishingError>,
) -> Result<DiminishingGroup, SpellDiminishingError> {
    if definition.negative_effects.iter().all(|negative| !negative) {
        return Ok(None);
    }
    if definition
        .effects
        .iter()
        .any(|effect| effect.is_aura_kind(11))
    {
        return Ok(Taunt);
    }
    match id {
        20549 | 24394 | 118345 | 118905 => return Ok(Stun),
        107079 => return Ok(Incapacitate),
        155145 => return Ok(Silence),
        108199 | 191244 => return Ok(AoeKnockback),
        _ => {}
    }
    let f = definition.fields.spell_family_flags;
    macro_rules! when {
        ($condition:expr, $group:ident) => {
            if $condition {
                return Ok($group);
            }
        };
    }
    match definition.fields.spell_family_name {
        0 => {
            when!(id == 48400, None);
            when!(id == 47481, Stun);
            when!(matches!(id, 66689 | 64155 | 51750 | 48179), None);
        }
        3 => {
            when!(f[0] & 0x40 != 0, Root);
            when!(f[2] & 0x200 != 0, Root);
            when!(f[0] & 0x800000 != 0, Incapacitate);
            when!(f[0] & 0x1000000 != 0, Incapacitate);
            when!(f[2] & 0x40 != 0, Incapacitate);
            when!(f[2] & 0x800000 != 0, Incapacitate);
        }
        4 => {
            when!(f[1] & 0x8000 != 0, Stun);
            when!(f[2] & 0x1000 != 0, Stun);
            when!(f[0] & 0x40000 != 0, Disorient);
        }
        5 => {
            when!(f[0] & 0x80000 != 0, Incapacitate);
            when!(f[1] & 0x8000000 != 0, Incapacitate);
            when!(f[1] & 0x400 != 0, Disorient);
            when!(f[1] & 8 != 0, Disorient);
            when!(f[1] & 0x1000 != 0, Stun);
            when!(f[0] & 0x1000 != 0, Stun);
            when!(id == 170995, LimitOnly);
        }
        57 => {
            when!(f[0] & 0x8000000 != 0, AoeKnockback);
            when!(f[0] & 0x2000000 != 0, Disorient);
            when!(f[1] & 4 != 0, Stun);
        }
        7 => {
            when!(f[1] & 0x80 != 0, Stun);
            when!(f[0] & 0x2000 != 0, Stun);
            when!(id == 163505, Stun);
            when!(f[1] & 1 != 0, Incapacitate);
            when!(f[1] & 0x20 != 0, Disorient);
            when!(id == 81261, Silence);
            when!(f[1] & 0x1000000 != 0, AoeKnockback);
            when!(id == 118283, AoeKnockback);
            when!(f[0] & 0x200 != 0, Root);
            when!(f[2] & 4 != 0, Root);
        }
        8 => {
            when!(f[0] & 0x800000 != 0, Stun);
            when!(f[0] & 0x400 != 0, Stun);
            when!(f[0] & 0x200000 != 0, Stun);
            when!(f[0] & 8 != 0, Incapacitate);
            when!(f[0] & 0x80 != 0, Incapacitate);
            when!(f[0] & 0x1000000 != 0, Disorient);
            when!(f[1] & 0x20000000 != 0, Silence);
        }
        9 => {
            when!(matches!(id, 53148 | 200108 | 212638), Root);
            when!(id == 117526, Stun);
            when!(f[0] & 8 != 0, Incapacitate);
            when!(f[1] & 0x1000 != 0, Incapacitate);
            when!(f[2] & 0x40 != 0, Disorient);
            when!(f[2] & 0x8000 != 0, Disorient);
            when!(id == 202933, Silence);
        }
        10 => {
            when!(f[0] & 4 != 0, Incapacitate);
            when!(id == 105421, Disorient);
            when!(f[0] & 0x4000 != 0, Silence);
            when!(f[0] & 0x800 != 0, Stun);
        }
        11 => {
            when!(f[1] & 0x8000 != 0, Incapacitate);
            when!(f[1] & 0x2000 != 0, AoeKnockback);
            when!(f[2] & 0x4000 != 0, Root);
            when!(f[3] & 0x2000000 != 0, Stun);
        }
        15 => {
            when!(id == 96294, Root);
            when!(id == 207167, Disorient);
            when!(f[0] & 0x200 != 0, Silence);
            when!(f[2] & 0x100000 != 0, Stun);
            when!(matches!(id, 91800 | 91797 | 207171), Stun);
        }
        6 => {
            // Preserve short circuit and EACH separate GetSpellVisual call:
            // selection draws must not be cached across these comparisons.
            when!(f[2] & 0x20 != 0 && visual()? == 52021, Stun);
            when!(id == 226943, Stun);
            when!(f[0] & 0x20000 != 0 && visual()? == 39068, Incapacitate);
            when!(f[2] & 0x20 != 0 && visual()? == 52019, Incapacitate);
            when!(f[0] & 0x10000 != 0, Disorient);
            when!(f[1] & 0x200000 != 0 && visual()? == 39025, Silence);
            when!(id == 204263, AoeKnockback);
        }
        53 => {
            when!(id == 116706, Root);
            when!(f[1] & 0x800000 != 0 && f[2] & 8 == 0, Stun);
            when!(f[1] & 0x200 != 0, Stun);
            when!(id == 202274, Incapacitate);
            when!(f[2] & 0x800000 != 0, Incapacitate);
            when!(id == 198909, Disorient);
        }
        107 => {
            when!(matches!(id, 179057 | 211881 | 200166 | 205630), Stun);
            when!(matches!(id, 217832 | 221527), Incapacitate);
        }
        _ => {}
    }
    Ok(None)
}
