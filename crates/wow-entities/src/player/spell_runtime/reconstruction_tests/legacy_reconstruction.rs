//! Original Session rule cases, now exercising canonical Player operations.

use super::support::*;
use super::*;

#[test]
fn login_known_spells_deactivate_lower_ranks_like_cpp_addspell() {
    let (mut session, _, _) = make_session();
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [
                wow_data::SpellRankEdgeLikeCpp {
                    spell_id: 20,
                    supercedes_spell_id: 10,
                },
                wow_data::SpellRankEdgeLikeCpp {
                    spell_id: 30,
                    supercedes_spell_id: 20,
                },
                wow_data::SpellRankEdgeLikeCpp {
                    spell_id: 200,
                    supercedes_spell_id: 100,
                },
            ],
            |_| true,
        ),
    ));
    let mut known_spells = vec![10, 40, 20, 100, 30];

    assert_eq!(
        session.deactivate_lower_rank_known_spells_for_send_like_cpp(&mut known_spells),
        2
    );
    assert_eq!(
        known_spells,
        vec![40, 100, 30],
        "C++ Player::AddSpell keeps lower ranks in PlayerSpellMap but marks them inactive when a higher known rank supersedes them, so SendKnownSpells skips them"
    );
}

#[test]
fn loaded_known_spell_dependencies_rebuild_all_active_override_edges_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_learn_spell_store(Arc::new(wow_data::SpellLearnSpellStoreLikeCpp {
        learned_by_spell_id: BTreeMap::from([(
            10,
            vec![
                wow_data::SpellLearnSpellNodeLikeCpp {
                    spell: 20,
                    overrides_spell: 100,
                    active: true,
                    auto_learned: false,
                },
                wow_data::SpellLearnSpellNodeLikeCpp {
                    spell: 30,
                    overrides_spell: 200,
                    active: true,
                    auto_learned: true,
                },
            ],
        )]),
    }));
    session.add_represented_override_spell_like_cpp(999, 888);
    session.reset_represented_talents_like_cpp();

    let mut known_spells = vec![10, 20];
    assert_eq!(
        session.apply_loaded_known_spell_dependencies_like_cpp(&mut known_spells),
        0,
        "an already-persisted dependency and an auto-learned dependency do not append spell rows"
    );
    assert_eq!(known_spells, vec![10, 20]);
    assert_eq!(
        session.represented_override_spells_like_cpp(),
        HashMap::from([(100, BTreeSet::from([20])), (200, BTreeSet::from([30])),]),
        "C++ AddSpell rebuilds active OverridesSpell edges outside the AutoLearned branch"
    );

    session.reset_represented_talents_like_cpp();
    let mut active_projection = Vec::new();
    assert_eq!(
        session.apply_loaded_spell_dependencies_from_roots_like_cpp(&[10], &mut active_projection,),
        1,
        "an inactive non-disabled loaded root still runs AddSpell dependency expansion"
    );
    assert_eq!(active_projection, vec![20]);
    assert_eq!(
        session.represented_override_spells_like_cpp(),
        HashMap::from([(100, BTreeSet::from([20])), (200, BTreeSet::from([30])),])
    );
}
