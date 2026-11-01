//! Session scenarios exercising represented spell-state responsibilities.
//!
//! Split out of `session_tests.rs` under #626. The root retains the shared
//! replacement-spell fixture; children group ownership and effect behavior.

use super::*;

#[path = "scenarios_spell_state_11/reputation.rs"]
mod reputation;
#[path = "scenarios_spell_state_11/spell_damage_base.rs"]
mod spell_damage_base;
#[path = "scenarios_spell_state_11/spell_damage_modifiers.rs"]
mod spell_damage_modifiers;
#[path = "scenarios_spell_state_11/spell_healing.rs"]
mod spell_healing;
#[path = "scenarios_spell_state_11/spell_state_ownership.rs"]
mod spell_state_ownership;

/// The represented replacement runtime this scenario installs and expects.
fn replacement_spell_runtime_like_cpp(
    replacement_row: wow_entities::PlayerKnownSpellRecord,
) -> wow_entities::PlayerSpellRuntimeState {
    let mut state = wow_entities::PlayerSpellRuntimeState::default();
    state.install_acquisition_snapshot_like_cpp(
        wow_entities::PlayerSpellAcquisitionSnapshotLikeCpp {
            known_spells: vec![900],
            rows: BTreeMap::from([(900, replacement_row)]),
            trait_definition_ids: BTreeMap::from([(900, 11)]),
            override_spells: BTreeMap::from([(800, BTreeSet::from([900]))]),
            ..Default::default()
        },
    );
    state
}
