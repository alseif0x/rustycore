//! Spell scenarios for [`super`].
//!
//! Split out of spawn_store_loader_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn area_trigger_spawn_resets_invalid_spell_for_visuals_like_cpp() {
    let maps = map_store(&[1]);
    let difficulties = map_difficulty_store(&[(1, 0)]);
    let area_trigger_templates = valid_area_trigger_template_store();
    let mut row = area_trigger_row(302, "0");
    row.spell_for_visuals = Some(-7);
    let mut report = SpawnKindLoadReport::default();
    let mut runtime_rows = BTreeMap::new();

    let spawn = area_trigger_row_to_spawn_data_like_cpp(
        &row,
        &maps,
        &difficulties,
        &area_trigger_templates,
        &mut |_| false,
        &mut |_| wow_data::ScriptIdLikeCpp(0),
        &mut runtime_rows,
        &mut report,
    )
    .expect("invalid SpellForVisuals is reset, not a skipped spawn");

    assert_eq!(spawn.spawn_id, 302);
    assert_eq!(report.corrected_invalid_spell_for_visuals, [(302, -7)]);
    assert_eq!(runtime_rows.get(&302).unwrap().spell_for_visuals, None);
}
