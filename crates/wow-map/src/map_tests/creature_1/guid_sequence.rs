use super::*;

#[test]
fn guid_sequence_creature_starts_at_one_like_cpp() {
    let mut map = test_map();

    assert_eq!(map.generate_low_guid_like_cpp(HighGuid::Creature), Ok(1));
    assert_eq!(map.generate_low_guid_like_cpp(HighGuid::Creature), Ok(2));
    assert_eq!(map.get_max_low_guid_like_cpp(HighGuid::Creature), Ok(3));
}
#[test]
fn guid_sequence_creature_and_gameobject_are_independent_like_cpp() {
    let mut map = test_map();

    assert_eq!(map.generate_low_guid_like_cpp(HighGuid::Creature), Ok(1));
    assert_eq!(map.generate_low_guid_like_cpp(HighGuid::GameObject), Ok(1));
    assert_eq!(map.generate_low_guid_like_cpp(HighGuid::Creature), Ok(2));
    assert_eq!(map.get_max_low_guid_like_cpp(HighGuid::GameObject), Ok(2));
}
#[test]
fn guid_sequence_accepts_non_creature_gameobject_map_sources_like_cpp() {
    let mut map = test_map();

    assert_eq!(map.generate_low_guid_like_cpp(HighGuid::AreaTrigger), Ok(1));
    assert_eq!(
        map.generate_low_guid_like_cpp(HighGuid::DynamicObject),
        Ok(1)
    );
    assert_eq!(map.generate_low_guid_like_cpp(HighGuid::AreaTrigger), Ok(2));
    assert_eq!(
        map.get_max_low_guid_like_cpp(HighGuid::DynamicObject),
        Ok(2)
    );
}

#[test]
fn map_settles_addon_aura_cast_guid_and_visual_from_one_shared_sequence_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(48503, 4850301, true);
    let creature_guid = creature.guid();
    let spell_id = 81_001;
    let addon = wow_entities::CreatureAddonLifecycleRecordLikeCpp {
        aura_applications: vec![wow_entities::CreatureAddonAuraApplicationLikeCpp {
            spell_id,
            spell_visual_id: 1_234,
            effect_mask: 1,
            flags: 0,
            effects: Vec::new(),
        }],
        ..wow_entities::CreatureAddonLifecycleRecordLikeCpp::default()
    };

    assert!(creature.apply_creatures_addon_lifecycle_like_cpp(Some(&addon)));
    assert_eq!(
        map.settle_creature_addon_aura_provenance_like_cpp(&mut creature),
        Ok(1)
    );
    let slot = *creature
        .unit()
        .subsystems()
        .auras
        .visible_auras
        .keys()
        .next()
        .expect("the admitted addon aura must have a visible slot");
    let provenance = creature
        .unit()
        .subsystems()
        .auras
        .aura_cast_provenance_like_cpp(slot);
    assert_eq!(provenance.spell_visual_id, 1_234);
    assert_eq!(provenance.cast_id.high_type(), HighGuid::Cast);
    assert_eq!(provenance.cast_id.sub_type(), 3);
    assert_eq!(provenance.cast_id.realm_id(), creature_guid.realm_id());
    assert_eq!(provenance.cast_id.map_id(), creature_guid.map_id());
    assert_eq!(provenance.cast_id.entry(), spell_id);
    assert_eq!(provenance.cast_id.counter(), 1);

    // The next normal Cast allocation observes the addon allocation because
    // both use this Map's one HighGuid::Cast sequence.
    assert_eq!(map.generate_low_guid_like_cpp(HighGuid::Cast), Ok(2));

    // A duplicate Unit admission produces no pending record and therefore no
    // Cast allocation or provenance overwrite.
    assert!(creature.apply_creatures_addon_lifecycle_like_cpp(Some(&addon)));
    assert_eq!(
        map.settle_creature_addon_aura_provenance_like_cpp(&mut creature),
        Ok(0)
    );
    assert_eq!(
        creature
            .unit()
            .subsystems()
            .auras
            .aura_cast_provenance_like_cpp(slot),
        provenance
    );
    assert_eq!(map.generate_low_guid_like_cpp(HighGuid::Cast), Ok(3));
}

#[test]
fn map_settles_canonical_creature_pending_addon_records_without_overlapping_borrows_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(48504, 4850401, true);
    let creature_guid = creature.guid();
    let addon = wow_entities::CreatureAddonLifecycleRecordLikeCpp {
        aura_applications: vec![wow_entities::CreatureAddonAuraApplicationLikeCpp {
            spell_id: 81_002,
            spell_visual_id: 1_235,
            effect_mask: 1,
            flags: 0,
            effects: Vec::new(),
        }],
        ..wow_entities::CreatureAddonLifecycleRecordLikeCpp::default()
    };
    creature.apply_creatures_addon_lifecycle_like_cpp(Some(&addon));
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert_eq!(
        map.settle_creature_addon_aura_provenance_by_guid_like_cpp(creature_guid),
        Ok(Some(1))
    );
    let canonical = map
        .map_object_record(creature_guid)
        .and_then(|record| record.creature())
        .expect("canonical Creature must remain the one aura owner");
    let slot = *canonical
        .unit()
        .subsystems()
        .auras
        .visible_auras
        .keys()
        .next()
        .expect("the canonical addon aura must have a visible slot");
    let provenance = canonical
        .unit()
        .subsystems()
        .auras
        .aura_cast_provenance_like_cpp(slot);
    assert_eq!(provenance.spell_visual_id, 1_235);
    assert_eq!(provenance.cast_id.counter(), 1);
    assert_eq!(provenance.cast_id.entry(), 81_002);
}

#[test]
fn map_rejects_stale_addon_pending_record_before_consuming_cast_sequence_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(48506, 4850601, true);
    let addon = wow_entities::CreatureAddonLifecycleRecordLikeCpp {
        aura_applications: vec![wow_entities::CreatureAddonAuraApplicationLikeCpp {
            spell_id: 81_003,
            spell_visual_id: 1_236,
            effect_mask: 1,
            flags: 0,
            effects: Vec::new(),
        }],
        ..wow_entities::CreatureAddonLifecycleRecordLikeCpp::default()
    };
    creature.apply_creatures_addon_lifecycle_like_cpp(Some(&addon));
    let slot = *creature
        .unit()
        .subsystems()
        .auras
        .visible_auras
        .keys()
        .next()
        .expect("the admitted addon aura must have a visible slot");
    creature
        .unit_mut()
        .subsystems_mut()
        .auras
        .clear_visible(slot);

    assert_eq!(
        map.settle_creature_addon_aura_provenance_like_cpp(&mut creature),
        Ok(0)
    );
    assert_eq!(
        map.generate_low_guid_like_cpp(HighGuid::Cast),
        Ok(1),
        "a stale Unit slot must not consume the Map-owned Cast sequence"
    );
}

#[test]
fn map_reapplied_addon_aura_gets_a_fresh_cast_sequence_value_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(48505, 4850501, true);
    let creature_guid = creature.guid();
    let spell_id = 81_004;
    let addon = wow_entities::CreatureAddonLifecycleRecordLikeCpp {
        aura_applications: vec![wow_entities::CreatureAddonAuraApplicationLikeCpp {
            spell_id,
            spell_visual_id: 1_237,
            effect_mask: 1,
            flags: 0,
            effects: Vec::new(),
        }],
        ..wow_entities::CreatureAddonLifecycleRecordLikeCpp::default()
    };

    creature.apply_creatures_addon_lifecycle_like_cpp(Some(&addon));
    map.settle_creature_addon_aura_provenance_like_cpp(&mut creature)
        .unwrap();
    let first_slot = *creature
        .unit()
        .subsystems()
        .auras
        .visible_auras
        .keys()
        .next()
        .expect("first addon application slot");
    let first = creature
        .unit()
        .subsystems()
        .auras
        .aura_cast_provenance_like_cpp(first_slot)
        .cast_id;

    creature
        .unit_mut()
        .subsystems_mut()
        .auras
        .remove_auras_due_to_spell_like_cpp(spell_id, creature_guid, 0);
    creature
        .unit_mut()
        .subsystems_mut()
        .auras
        .clear_visible(first_slot);
    creature.apply_creatures_addon_lifecycle_like_cpp(Some(&addon));
    assert_eq!(
        map.settle_creature_addon_aura_provenance_like_cpp(&mut creature),
        Ok(1)
    );
    let second_slot = *creature
        .unit()
        .subsystems()
        .auras
        .visible_auras
        .keys()
        .next()
        .expect("fresh addon application slot");
    let second = creature
        .unit()
        .subsystems()
        .auras
        .aura_cast_provenance_like_cpp(second_slot)
        .cast_id;
    assert_eq!(first.counter(), 1);
    assert_eq!(second.counter(), 2);
    assert_ne!(first, second);
}
