//! Legacy creature-melee damage and outcome scenarios.

use super::*;



/// C++ `Unit::CalcArmorReducedDamage`'s attacker
/// `SPELL_AURA_MOD_IGNORE_TARGET_RESIST` term through the production swing owner.
///
/// The map-owned `run_legacy_player_melee_tick_once_like_cpp` resolves the
/// attacker's live auras, so the normal-school ignore-resist sum must shrink the
/// victim's armour by `std::floor(AddPct(armor, -amount))` before the reduction
/// curve (`Unit.cpp:1646-1651`), not only in the session path.
#[tokio::test]
async fn map_owned_player_melee_applies_attacker_ignore_target_resist_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let creature_guid = test_creature_guid(99_933);
    let player_guid = ObjectGuid::create_player(1, 5_203);
    let map_store = Arc::new(wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 0,
        instance_type: wow_data::map::MAP_COMMON,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }]));

    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::clone(&map_store));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "IgnoreResistSolo".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_attacking(Some(creature_guid));
            unit.set_target(creature_guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 1_000.0, 1_000.0);
            unit.reset_attack_timer_like_cpp(WeaponAttackType::BaseAttack);
        })
        .unwrap();
    session.set_map_manager(Arc::clone(&manager));
    register_test_creature(&mut session, manager.clone(), creature_guid, 100_000);
    session
        .mutate_world_creature(creature_guid, |creature| {
            // `Unit::IsTotem()` zeroes the victim's dodge, parry and block, so the
            // swing cannot be avoided and the damage term is isolated.
            creature
                .creature
                .add_unit_type_mask_like_cpp(wow_entities::UNIT_MASK_TOTEM);
            creature.creature.unit_mut().set_level(80);
            creature.creature.set_combat_log_stats_like_cpp(
                wow_entities::CreatureCombatLogStatsLikeCpp {
                    armor: 5_000,
                    ..Default::default()
                },
            );
        })
        .unwrap();

    let spell_id = 91_130_i32;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 50,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_IGNORE_TARGET_RESIST),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_IGNORE_TARGET_RESIST,
                effect_misc_value_1: 0x01,
                effect_base_points: 50,
                ..Default::default()
            }],
        },
    );
    let spell_store = Arc::new(spell_store);
    session.set_spell_store(Arc::clone(&spell_store));
    session
        .apply_aura(spell_id, player_guid, 30_000, 1)
        .expect("apply ignore-resist aura");

    let (send_tx, _rx) = flume::bounded::<Vec<u8>>(8);
    let (command_tx, _crx) = flume::bounded::<SessionCommand>(8);
    let registration = PlayerRegistry::new().register_or_replace(
        player_guid,
        broadcast_info_with_command(player_guid, send_tx, command_tx),
        Default::default(),
    );
    let attackers = vec![crate::session::PlayerMeleeAttackerSnapshotLikeCpp {
        registration,
        player_guid,
        map_id: 0,
        instance_id: 0,
        in_combat_mirror: true,
        tap_group_guids: Vec::new(),
    }];
    let config = crate::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(spell_store),
        ..Default::default()
    };
    let mut phase_state = crate::session::PlayerMeleePhaseStateLikeCpp::default();
    let health = |manager: &crate::map_manager::SharedMapManager| {
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, creature_guid)
            .unwrap()
            .current_hp()
    };
    let before = health(&manager);
    let outcome = crate::session::run_legacy_player_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &attackers,
        2_000,
        &mut phase_state,
        &config,
    );
    assert!(!outcome.skipped_owner_not_global, "the map owns this tick");
    assert_eq!(outcome.creature_hits, 1);
    assert_eq!(
        before - health(&manager),
        860,
        "50% ignore-resist halves the 5,000 armour before the reduction curve"
    );
}
