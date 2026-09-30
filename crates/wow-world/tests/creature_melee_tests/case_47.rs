use super::*;

/// `SPELL_ATTR2_NO_INITIAL_THREAT` does not prevent the recursive share
/// damage, but `ThreatManager::AddThreat` skips a target that is not engaged.
/// Once that same target already has combat, a later share creates threat.
#[test]
fn legacy_creature_melee_share_honors_no_initial_threat_like_cpp() {
    use wow_world::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let attacker_guid = test_creature_guid(91_401);
    let victim_guid = test_creature_guid(91_402);
    let secondary_guid = test_creature_guid(91_403);
    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    for guid in [attacker_guid, victim_guid, secondary_guid] {
        register_test_creature(&mut session, manager.clone(), guid, 100);
    }
    session
        .fixture_melee_mutate_creature(attacker_guid, |creature| {
            creature.creature.ai_ownership_mut().min_damage = 10;
            creature.creature.ai_ownership_mut().max_damage = 10;
            creature.creature.set_flags_extra_runtime_like_cpp(
                wow_constants::CreatureFlagsExtra::NO_CRIT.bits(),
            );
            creature.enter_combat(victim_guid);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    let hit_spell_id = 91_404;
    let share_spell_id = 91_405;
    let mut spells = wow_data::SpellStore::new();
    spells.insert(
        hit_spell_id,
        damage_aura_spell_like_cpp(
            hit_spell_id,
            wow_data::spell::aura_types::SPELL_AURA_MOD_HIT_CHANCE,
            5,
            0,
        ),
    );
    spells.insert(
        share_spell_id,
        damage_aura_spell_like_cpp(
            share_spell_id,
            wow_data::spell::aura_types::SPELL_AURA_SHARE_DAMAGE_PCT,
            50,
            0x01,
        ),
    );
    let mut attributes = [0; 15];
    attributes[2] = wow_data::spell::attributes::SPELL_ATTR2_NO_INITIAL_THREAT;
    spells.insert_spell_misc_attributes_like_cpp(share_spell_id, attributes);
    let spells = Arc::new(spells);
    session
        .fixture_melee_mutate_creature(attacker_guid, |creature| {
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .add_applied(wow_entities::AppliedAuraRef::new(
                    hit_spell_id as u32,
                    attacker_guid,
                    0,
                    1,
                ));
        })
        .unwrap();
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .get_typed_creature_mut(victim_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .auras
        .add_applied(wow_entities::AppliedAuraRef::new(
            share_spell_id as u32,
            secondary_guid,
            0,
            1,
        ));
    let config = wow_world::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(spells),
        ..Default::default()
    };
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let first = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(first.legacy_creature_victim_syncs, 2);
    {
        let canonical = canonical.lock().unwrap();
        let map = canonical.find_map(0, 0).unwrap().map();
        let (health, threat, ai_state) = map
            .with_creature_like_cpp(secondary_guid, |secondary| {
                (
                    secondary.unit().data().health,
                    secondary
                        .unit()
                        .subsystems()
                        .combat
                        .threat_value(attacker_guid),
                    secondary.ai_ownership().state,
                )
            })
            .unwrap();
        assert_eq!(health, 95);
        assert_eq!(threat, None);
        assert_ne!(ai_state, wow_entities::CreatureAiState::InCombat);
    }

    {
        let mut canonical = canonical.lock().unwrap();
        canonical
            .find_map_mut(0, 0)
            .unwrap()
            .map_mut()
            .get_typed_creature_mut(secondary_guid)
            .unwrap()
            .unit_mut()
            .subsystems_mut()
            .combat
            .set_in_combat_with(attacker_guid, false, false);
    }
    session
        .fixture_melee_mutate_creature(secondary_guid, |creature| {
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .set_in_combat_with(attacker_guid, false, false);
        })
        .unwrap();
    session
        .fixture_melee_mutate_creature(attacker_guid, |creature| {
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();
    let second = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(second.legacy_creature_victim_syncs, 2);
    for threat in [
        canonical
            .lock()
            .unwrap()
            .find_map(0, 0)
            .unwrap()
            .map()
            .with_creature_like_cpp(secondary_guid, |secondary| {
                secondary
                    .unit()
                    .subsystems()
                    .combat
                    .threat_value(attacker_guid)
            })
            .flatten(),
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, secondary_guid)
            .unwrap()
            .creature
            .unit()
            .subsystems()
            .combat
            .threat_value(attacker_guid),
    ] {
        assert_eq!(threat, Some(5.0));
    }
}
