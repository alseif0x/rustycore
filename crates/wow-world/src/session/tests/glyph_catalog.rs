//! Login borrows process glyph data; Session does not cache the catalog.

use super::*;

pub(super) fn catalog(id: u32) -> wow_data::GlyphPropertiesStore {
    wow_data::GlyphPropertiesStore::from_entries([wow_data::GlyphPropertiesEntry {
        id,
        spell_id: 10,
        glyph_type: 1,
        glyph_exclusive_category_id: 0,
        spell_icon_file_data_id: 0,
        glyph_slot_flags: 0,
    }])
}

#[test]
fn glyph_load_uses_supplied_catalog_for_active_and_detached_canonical_player() {
    let (mut session, _, _) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    let catalog = Arc::new(catalog(123));
    let missing = wow_data::GlyphPropertiesStore::from_entries([]);
    let references = Arc::strong_count(&catalog);
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        assert!(session.load_represented_glyph_row_like_cpp(&catalog, 0, 0, 123));
        assert!(!session.load_represented_glyph_row_like_cpp(&missing, 0, 0, 999));
        assert!(!session.load_represented_glyph_row_like_cpp(&missing, 0, 1, 123));
        let runtime = session.player_talent_runtime_snapshot_like_cpp().unwrap();
        assert_eq!(runtime.glyph_groups[0][0], 123);
        assert_eq!(runtime.glyph_groups[0][1], 0);
        assert_eq!(
            Arc::strong_count(&catalog),
            references,
            "no Session catalog cache"
        );
        // Preserve the represented zero-row clearing policy in this refactor.
        assert!(session.load_represented_glyph_row_like_cpp(&missing, 0, 0, 0));
        assert_eq!(
            session
                .player_talent_runtime_snapshot_like_cpp()
                .unwrap()
                .glyph_groups[0][0],
            0
        );
    }
    session.canonical_map_manager = None;
    assert!(!session.load_represented_glyph_row_like_cpp(&catalog, 0, 0, 123));
}
