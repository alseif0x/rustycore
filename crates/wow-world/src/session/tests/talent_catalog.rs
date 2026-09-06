//! Talent-row loading borrows a catalog and mutates only the canonical owner.

use super::*;

#[test]
fn talent_tab_validation_uses_the_supplied_catalog_for_active_and_detached_player() {
    let (mut session, _, _) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    let tabs = install_test_talent_tab_store_like_cpp(&mut session);
    let absent = wow_data::TalentTabStore::from_entries([]);
    session.set_talent_store(Arc::new(wow_data::TalentStore::from_entries([
        test_talent_entry_like_cpp(101, 2, 50_101),
    ])));
    let mut spells = wow_data::SpellStore::new();
    spells.insert(50_101, test_spell_info_like_cpp(50_101));
    session.set_spell_store(Arc::new(spells));
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        assert!(!session.load_represented_talent_row_like_cpp(&absent, 101, 2, 0));
        assert!(session.load_represented_talent_row_like_cpp(&tabs, 101, 2, 0));
        let before = session
            .player_talent_runtime_snapshot_like_cpp()
            .unwrap()
            .talent_groups;
        assert!(!session.load_represented_talent_row_like_cpp(&absent, 101, 2, 1));
        let after = session
            .player_talent_runtime_snapshot_like_cpp()
            .unwrap()
            .talent_groups;
        assert_eq!(after, before);
        assert_eq!(after[0].get(&101), Some(&2));
        assert!(!after[1].contains_key(&101));
    }
    session.canonical_map_manager = None;
    assert!(!session.load_represented_talent_row_like_cpp(&tabs, 101, 2, 0));
}
