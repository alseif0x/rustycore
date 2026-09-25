use super::*;

#[test]
fn death_sync_preserves_existing_canonical_death_time_ms() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(18_001);
    register_test_creature(&mut session, manager.clone(), guid, 40);

    {
        let mut manager = manager.write().unwrap();
        let world_creature = manager.find_creature_mut(0, 0, guid).unwrap();
        world_creature.creature.mark_ai_dead(1_234);
    }
    session
        .mutate_world_creature(guid, |creature| {
            creature.take_damage(40);
        })
        .unwrap();

    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, guid).unwrap();
    assert_eq!(
        world_creature.creature.ai_ownership().death_time_ms,
        Some(1_234)
    );
    assert_eq!(world_creature.current_hp(), 0);
}

#[tokio::test]
async fn heal_max_health_zero_damage_missing_target_is_noop_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 49);
    let missing_creature_guid = test_creature_guid(18_015);
    session.set_player_guid(Some(player_guid));
    session.set_player_health_like_cpp(65, 100);

    session
        .apply_heal_max_health_like_cpp(0, 0, player_guid, missing_creature_guid)
        .await
        .expect("C++ !unitTarget guard makes heal-max-health a no-op");

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn primary_heal_max_health_nonzero_damage_heals_target_missing_health_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 733_i32;
    let player_guid = ObjectGuid::create_player(1, 50);
    session.set_player_guid(Some(player_guid));
    session.set_player_health_like_cpp(35, 80);

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL_MAX_HEALTH,
            effect_base_points: 1,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented primary heal-max-health should execute");

    assert_eq!(
        session.player_health_like_cpp(),
        80,
        "C++ damage != 0 heals the missing health of the unit target"
    );
    let opcodes = drain_server_opcodes(&send_rx);
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::SpellHealLog,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::CooldownEvent,
        ]
    );
}
