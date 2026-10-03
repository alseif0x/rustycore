//! Null-caster GetSpellXSpellVisualId/GetSpellVisual, :4526-4573.
use super::*;
use wow_data::forever_spells::SpellCatalog;

pub(super) fn null_caster(
    catalog: &SpellCatalog,
    definition: &Definition,
    select: &mut impl FnMut(&[f64]) -> Result<usize, SpellDiminishingError>,
    selections: &mut usize,
) -> Result<u32, SpellDiminishingError> {
    let eligible = |id| -> Result<bool, SpellDiminishingError> {
        let row = catalog
            .spell_x_spell_visual(id)
            .ok_or(SpellDiminishingError::MissingVisualRecord)?;
        // Player condition short-circuits before the UnitCondition lookup.
        if row.caster_player_condition_id != 0 {
            return Ok(false);
        }
        // Source looks up zero too. A missing nonzero condition does NOT
        // reject the visual; a present condition rejects with null caster.
        Ok(catalog
            .unit_condition(u32::from(row.caster_unit_condition_id))
            .is_none())
    };
    for (position, &id) in definition.visuals.iter().enumerate() {
        if !eligible(id)? {
            continue;
        }
        let first = catalog.spell_x_spell_visual(id).expect("checked visual");
        let mut candidates = vec![id];
        for &other_id in &definition.visuals[position + 1..] {
            let other = catalog
                .spell_x_spell_visual(other_id)
                .ok_or(SpellDiminishingError::MissingVisualRecord)?;
            if other.priority != first.priority {
                break;
            }
            if eligible(other_id)? {
                candidates.push(other_id);
            }
        }
        let selected = if candidates.len() == 1 {
            candidates[0] // Ignores Probability, including NaN/negative.
        } else {
            let weights: Vec<_> = candidates
                .iter()
                .map(|id| {
                    f64::from(
                        catalog
                            .spell_x_spell_visual(*id)
                            .expect("checked candidate")
                            .probability,
                    )
                })
                .collect();
            *selections += 1;
            let position = select(&weights)?;
            *candidates
                .get(position)
                .ok_or(SpellDiminishingError::InvalidSelection)?
        };
        return Ok(catalog
            .spell_x_spell_visual(selected)
            .expect("checked selection")
            .spell_visual_id);
    }
    // GetSpellVisual looks up GetSpellXSpellVisualId's zero return, too.
    Ok(catalog
        .spell_x_spell_visual(0)
        .map_or(0, |row| row.spell_visual_id))
}
