use super::*;
use std::collections::HashMap;
use std::sync::Arc;

#[test]
fn legacy_initial_threat_no_harmful_attribute_excludes_both_polarities() {
    for (counter, is_positive) in [(51_001_u32, true), (51_002, false)] {
        let (mut session, _, _) = make_session();
        let initial_manager = shared_map_manager();
        let player_guid = ObjectGuid::create_player(1, i64::from(counter));
        let creature_guid = test_creature_guid(i64::from(counter));
        let spell_id = counter as i32;
        session.core.player_guid = Some(player_guid);
        session.core.state = SessionState::LoggedIn;
        install_canonical_player_owner_for_test(&mut session, 0, 0);
        let manager = session
            .core
            .canonical_map_manager
            .clone()
            .unwrap_or_else(|| initial_manager.clone());
        register_test_creature(&mut session, manager.clone(), creature_guid, 40);

        if is_positive {
            let mut manager = manager.lock().expect("canonical MapManager");
            let map = manager
                .find_map_mut(0, 0)
                .expect("canonical world Map")
                .map_mut();
            let threat_ref = {
                let creature = map
                    .get_typed_creature_mut(creature_guid)
                    .expect("registered canonical Creature");
                creature
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .set_threat(player_guid, 25.0);
                *creature
                    .unit()
                    .subsystems()
                    .combat
                    .threat_ref(player_guid)
                    .expect("Creature threat reference")
            };
            map.get_typed_player_mut(player_guid)
                .expect("canonical Player")
                .unit_mut()
                .subsystems_mut()
                .combat
                .put_threatened_by_me_ref(creature_guid, threat_ref);
        }

        let mut attributes = [0; 15];
        attributes[4] = wow_data::spell::attributes::SPELL_ATTR4_NO_HARMFUL_THREAT;
        let mut spells = wow_data::SpellStore::new();
        spells.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
        spells.insert(
            spell_id,
            threat_spell_info_like_cpp(
                spell_id,
                wow_data::spell::spell_effect_types::SPELL_EFFECT_THREAT,
                0,
            ),
        );
        session.set_spell_store(Arc::new(spells));
        session.set_spell_threat_store(Arc::new(wow_data::SpellThreatStoreLikeCpp {
            entries_by_spell_id: HashMap::from([(
                spell_id as u32,
                wow_data::SpellThreatEntryLikeCpp {
                    flat_mod: 7,
                    pct_mod: 3.0,
                    ap_pct_mod: 0.0,
                },
            )]),
        }));

        session.apply_spell_initial_threat_like_cpp(
            spell_id,
            player_guid,
            if is_positive {
                player_guid
            } else {
                creature_guid
            },
            is_positive,
        );

        let manager = manager.lock().expect("canonical MapManager");
        let creature = manager
            .find_map(0, 0)
            .expect("canonical world Map")
            .map()
            .get_typed_creature(creature_guid)
            .expect("registered canonical Creature");
        if is_positive {
            assert_eq!(
                creature
                    .unit()
                    .subsystems()
                    .combat
                    .threat_value(player_guid),
                Some(25.0),
                "NO_HARMFUL_THREAT blocks forwarding from a positive cast"
            );
        } else {
            assert!(!creature.is_in_combat());
            assert_eq!(
                creature
                    .unit()
                    .subsystems()
                    .combat
                    .threat_value(player_guid),
                None,
                "NO_HARMFUL_THREAT blocks harmful cast-level threat"
            );
        }
    }
}
