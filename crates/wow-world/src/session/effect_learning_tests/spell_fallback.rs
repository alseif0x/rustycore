use super::*;
use crate::spell_acquisition::*;

#[tokio::test]
async fn spell_learn_spell_fallback_defers_without_complete_spell_rows_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let learned_spell_id = 13_342_i32;
    let player_guid = ObjectGuid::create_player(1, 76);
    session.set_player_guid(Some(player_guid));
    session.set_spell_chain_store(Arc::new(wow_data::SpellChainStoreLikeCpp::default()));
    session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([])));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        learned_spell_id,
        wow_data::SpellInfo {
            spell_id: learned_spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    assert!(
        !session
            .apply_learn_spell_effect_like_cpp(learned_spell_id, player_guid)
            .await
    );
    assert!(
        session
            .player_spell_test_fixture_like_cpp
            .represented_fallback_player_spell_rows_like_cpp
            .is_empty(),
        "an unknown durable row must not be guessed into a targeted UPSERT overlay"
    );
    assert!(
        !session.known_spells_like_cpp().contains(&learned_spell_id),
        "runtime publication must also wait for the complete C++ PlayerSpellMap authority"
    );
    assert!(
        session
            .represented_spell_acquisition_post_commit_actions_like_cpp()
            .is_empty()
    );
    assert!(drain_server_opcodes(&send_rx).is_empty());
}

#[test]
fn fallback_reconciliation_preserves_dependent_promotion_like_cpp() {
    let (mut session, _, _) = make_session();
    let spell_id = 13_351_i32;
    session
        .player_spell_test_fixture_like_cpp
        .represented_fallback_player_spell_rows_like_cpp
        .insert(
            spell_id,
            RepresentedPlayerSpellLikeCpp {
                spell_id,
                active: true,
                disabled: false,
                dependent: true,
                favorite: false,
                state: RepresentedPlayerSpellStateLikeCpp::New,
            },
        );

    assert!(
        session.set_complete_represented_player_spell_rows_like_cpp([
            RepresentedPlayerSpellLikeCpp {
                spell_id,
                active: true,
                disabled: false,
                dependent: false,
                favorite: false,
                state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
            },
        ])
    );

    let reconciled = session
        .player_spell_test_fixture_like_cpp
        .represented_player_spell_rows_like_cpp[&spell_id];
    assert!(reconciled.dependent);
    assert_eq!(
        reconciled.state,
        RepresentedPlayerSpellStateLikeCpp::Changed,
        "C++ AddSpell promotes an existing independent row and marks it changed"
    );
}
