use super::{
    CharacterLoadoutEntry, CharacterLoadoutItemEntry, CharacterLoadoutItemStore,
    CharacterLoadoutStore,
};

#[test]
fn character_loadout_filter_rejects_invalid_race_masks() {
    let store = CharacterLoadoutStore::from_entries([CharacterLoadoutEntry {
        id: 1,
        race_mask: -1,
        chr_class_id: 1,
        purpose: 9,
        item_context: 0,
    }]);

    for race in [0, 65, 255] {
        assert!(
            store.new_character_loadouts_for(race, 1).is_empty(),
            "race {race} has no representable DB2 race-mask bit"
        );
    }
}

#[test]
fn character_loadout_filter_uses_purpose_race_and_class() {
    let store = CharacterLoadoutStore::from_entries([
        CharacterLoadoutEntry {
            id: 1,
            race_mask: 1 << 2,
            chr_class_id: 2,
            purpose: 9,
            item_context: 0,
        },
        CharacterLoadoutEntry {
            id: 2,
            race_mask: 1 << 2,
            chr_class_id: 3,
            purpose: 9,
            item_context: 0,
        },
        CharacterLoadoutEntry {
            id: 3,
            race_mask: 1 << 2,
            chr_class_id: 2,
            purpose: 8,
            item_context: 0,
        },
        CharacterLoadoutEntry {
            id: 4,
            race_mask: 1 << 3,
            chr_class_id: 2,
            purpose: 9,
            item_context: 0,
        },
    ]);

    assert_eq!(
        store
            .new_character_loadouts_for(3, 2)
            .iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        vec![1]
    );
}

#[test]
fn character_loadout_rows_and_items_follow_db2_row_id_order() {
    let loadouts = CharacterLoadoutStore::from_entries([
        CharacterLoadoutEntry {
            id: 20,
            race_mask: 1,
            chr_class_id: 1,
            purpose: 9,
            item_context: 0,
        },
        CharacterLoadoutEntry {
            id: 3,
            race_mask: 1,
            chr_class_id: 1,
            purpose: 9,
            item_context: 0,
        },
    ]);
    assert_eq!(
        loadouts
            .new_character_loadouts_for(1, 1)
            .iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        vec![3, 20]
    );

    let items = CharacterLoadoutItemStore::from_entries([
        CharacterLoadoutItemEntry {
            id: 20,
            character_loadout_id: 3,
            item_id: 9000,
        },
        CharacterLoadoutItemEntry {
            id: 2,
            character_loadout_id: 3,
            item_id: 7000,
        },
        CharacterLoadoutItemEntry {
            id: 4,
            character_loadout_id: 8,
            item_id: 5000,
        },
        CharacterLoadoutItemEntry {
            id: 7,
            character_loadout_id: 3,
            item_id: 2000,
        },
    ]);
    assert_eq!(items.items_for_loadout(3), vec![7000, 2000, 9000]);
}
