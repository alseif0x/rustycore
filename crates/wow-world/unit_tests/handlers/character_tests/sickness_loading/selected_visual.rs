use super::selected_restore::{authority, keyed_rows, loaded_sickness};
use super::*;
use std::sync::Arc;

const FALLBACK_VISUAL_ID: u32 = 0xF123_4567;

fn visual_row(
    id: u32,
    difficulty_id: u8,
    caster_player_condition_id: u32,
) -> wow_data::SpellXSpellVisualEntry {
    wow_data::SpellXSpellVisualEntry {
        id,
        difficulty_id,
        spell_visual_id: id.wrapping_add(77),
        probability: 1.0,
        flags: 0,
        priority: 0,
        spell_icon_file_id: 0,
        active_icon_file_id: 0,
        viewer_unit_condition_id: 0,
        viewer_player_condition_id: 0,
        caster_unit_condition_id: 0,
        caster_player_condition_id,
        spell_id: SICKNESS_SPELL_ID as u32,
    }
}

fn install_visuals(
    session: &mut WorldSession,
    rows: impl IntoIterator<Item = wow_data::SpellXSpellVisualEntry>,
) {
    session.set_creature_aggro_config_like_cpp(crate::session::CreatureAggroConfigLikeCpp {
        spell_x_spell_visual_store: Some(Arc::new(wow_data::SpellXSpellVisualStore::from_entries(
            rows,
        ))),
        ..Default::default()
    });
}

#[test]
fn selected_load_inherits_saved_zero_difficulty_and_publishes_priority_relation_bits() {
    let (mut session, player, send_rx) =
        canonical_aura_session([sickness_catalog_info()], SICKNESS_SPELL_ID);
    session.set_difficulty_store(Arc::new(wow_data::DifficultyStore::from_entries([
        wow_data::DifficultyEntry {
            id: 0,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 3,
            toggle_difficulty_id: 0,
        },
        wow_data::DifficultyEntry {
            id: 3,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
    ])));
    install_visuals(
        &mut session,
        [
            visual_row(FALLBACK_VISUAL_ID, 3, 12),
            visual_row(245_040, 3, 0),
        ],
    );
    let (aura, saved) = keyed_rows(player, player, ObjectGuid::EMPTY, 0b111, 90_000);

    assert_eq!(
        session.load_represented_character_auras_like_cpp([aura], saved, 0),
        1
    );
    let loaded = loaded_sickness(&session);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].2.spell_visual_id, FALLBACK_VISUAL_ID as i32);

    let (state, hub) = crate::session::split_spell_state_ref(&session);
    state
        .send_initial_player_auras_like_cpp(hub)
        .expect("selected load publishes the retained creation provenance");
    let bytes = send_rx
        .try_recv()
        .expect("initial selected AuraUpdate is queued");
    let (visual_id, _, _, _) = super::full_aura_visual_duration_fields(&bytes);
    assert_eq!(visual_id, FALLBACK_VISUAL_ID as i32);
}

#[test]
fn selected_load_missing_visual_store_preserves_old_aura_and_marks_authority_incomplete() {
    let (mut session, player, _) =
        canonical_aura_session([sickness_catalog_info()], SICKNESS_SPELL_ID);
    let (old, saved) = keyed_rows(player, player, ObjectGuid::EMPTY, 0b111, 90_000);
    assert_eq!(
        session.load_represented_character_auras_like_cpp([old], saved, 0),
        1
    );
    let old = loaded_sickness(&session).remove(0);
    session.set_creature_aggro_config_like_cpp(Default::default());
    let (candidate, saved) = keyed_rows(player, player, ObjectGuid::EMPTY, 0b111, 80_000);

    assert_eq!(
        session.load_represented_character_auras_like_cpp([candidate], saved, 0),
        0
    );
    let current = loaded_sickness(&session).remove(0);
    assert_eq!(current.0.slot, old.0.slot);
    assert_eq!(current.2.cast_id, old.2.cast_id);
    assert_eq!(current.2.spell_visual_id, old.2.spell_visual_id);
    assert_eq!(authority(&session), Some(false));
}

#[test]
fn selected_load_override_potential_holds_before_conflict_removal() {
    let (mut session, player, _) =
        canonical_aura_session([sickness_catalog_info()], SICKNESS_SPELL_ID);
    let (old, saved) = keyed_rows(player, player, ObjectGuid::EMPTY, 0b111, 90_000);
    assert_eq!(
        session.load_represented_character_auras_like_cpp([old], saved, 0),
        1
    );
    let old = loaded_sickness(&session).remove(0);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(SICKNESS_SPELL_ID, sickness_catalog_info());
    let mut sickness_attributes = [0; 15];
    sickness_attributes[0] = SPELL_ATTR0_AURA_IS_DEBUFF;
    spell_store.insert_spell_misc_attributes_like_cpp(SICKNESS_SPELL_ID, sickness_attributes);
    spell_store.insert(
        91_990,
        wow_data::SpellInfo {
            spell_id: 91_990,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(403),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: 403,
                effect_misc_value_1: SICKNESS_SPELL_ID,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    let (candidate, saved) = keyed_rows(player, player, ObjectGuid::EMPTY, 0b111, 80_000);

    assert_eq!(
        session.load_represented_character_auras_like_cpp([candidate], saved, 0),
        0
    );
    let current = loaded_sickness(&session).remove(0);
    assert_eq!(current.0.slot, old.0.slot);
    assert_eq!(current.2.cast_id, old.2.cast_id);
    assert_eq!(authority(&session), Some(false));
}
