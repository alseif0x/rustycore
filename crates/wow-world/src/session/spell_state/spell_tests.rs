use crate::handlers::test_support::world::make_session;

#[test]
fn login_known_spells_filters_complete_has_spell_mirror_like_cpp() {
    let (mut session, _) = make_session();
    session.set_known_spells_like_cpp(vec![100, 200]);
    assert!(
        session.set_complete_represented_player_spell_rows_like_cpp([
            crate::session::RepresentedPlayerSpellLikeCpp {
                spell_id: 100,
                active: false,
                disabled: false,
                dependent: false,
                favorite: false,
                state: crate::session::RepresentedPlayerSpellStateLikeCpp::Unchanged,
            },
            crate::session::RepresentedPlayerSpellLikeCpp {
                spell_id: 200,
                active: true,
                disabled: false,
                dependent: false,
                favorite: false,
                state: crate::session::RepresentedPlayerSpellStateLikeCpp::Unchanged,
            },
            crate::session::RepresentedPlayerSpellLikeCpp {
                spell_id: 300,
                active: true,
                disabled: true,
                dependent: false,
                favorite: false,
                state: crate::session::RepresentedPlayerSpellStateLikeCpp::Unchanged,
            },
        ])
    );

    assert_eq!(session.known_spells_like_cpp(), &[100, 200]);
    assert_eq!(
        session.login_known_spells_after_account_collections_like_cpp(),
        vec![200],
        "C++ Player::SendKnownSpells excludes inactive and disabled PlayerSpellMap rows"
    );
}
