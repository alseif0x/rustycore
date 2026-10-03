//! Source bitset truncation, tokenization and strict decimal StringTo<uint32>.
use super::*;

pub(super) fn load(
    rows: Vec<CreatureImmunityRow>,
    counts: &mut ImmunityCounts,
) -> BTreeMap<i32, CreatureImmunityInfo> {
    let mut result: BTreeMap<i32, CreatureImmunityInfo> = BTreeMap::new();
    for row in rows {
        counts.input_rows += 1;
        let info = result.entry(row.id).or_default();
        let school = row.school as u8;
        let dispel = row.dispel as u16;
        let mechanics = row.mechanics as u64;
        info.school_mask = school & 0x7f;
        info.dispel_mask = dispel & 0xfff;
        info.mechanic_mask = mechanics & ((1u64 << 37) - 1);
        counts.truncated_masks += usize::from(info.school_mask != school)
            + usize::from(info.dispel_mask != dispel)
            + usize::from(info.mechanic_mask != mechanics);
        // Source overwrites masks, but appends lists and ORs flags on repeated ID.
        info.other_mask |= u8::from(row.immune_aoe) | (u8::from(row.immune_chain) << 1);
        tokens(
            &row.effects,
            361,
            &mut info.effect_types,
            &mut counts.invalid_effect_tokens,
        );
        tokens(
            &row.auras,
            665,
            &mut info.aura_types,
            &mut counts.invalid_aura_tokens,
        );
    }
    result
}

fn tokens(input: &[u8], bound: u32, output: &mut Vec<u32>, invalid: &mut usize) {
    for token in input
        .split(|&b| b == b',')
        .filter(|token| !token.is_empty())
    {
        let value = token.iter().try_fold(0u32, |n, &digit| {
            if !digit.is_ascii_digit() {
                return None;
            }
            n.checked_mul(10)?.checked_add(u32::from(digit - b'0'))
        });
        if let Some(value) = value.filter(|&v| v < bound) {
            output.push(value);
        } else {
            *invalid += 1;
        }
    }
}
