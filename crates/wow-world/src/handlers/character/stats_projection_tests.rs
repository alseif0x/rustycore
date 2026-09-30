use std::sync::Arc;

use wow_constants::{EnchantmentSlot, ItemContext, PowerType};
use wow_core::ObjectGuid;

use crate::session::WorldSession;

fn make_session_with_send_capacity(
    capacity: usize,
) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded(1);
    let (send_tx, send_rx) = flume::bounded(capacity);
    let mut session = WorldSession::new(
        1,
        "TestAccount".into(),
        0,
        2,
        9,
        54_261,
        vec![0; 40],
        "esES".into(),
        pkt_rx,
        send_tx,
    );
    session.set_item_guid_generator_like_cpp(Arc::new(wow_core::ObjectGuidGenerator::new(
        wow_core::HighGuid::Item,
        1,
    )));
    (session, send_rx)
}

fn set_loaded_player_identity_like_cpp(
    session: &mut WorldSession,
    map_id: u16,
    race: u8,
    class: u8,
    level: u8,
    gender: u8,
) {
    session.set_loaded_player_identity_like_cpp(map_id, race, class, level, gender);
}

fn set_priest_level80_stats(session: &mut WorldSession, base_mana: u32, intellect: u16) {
    session.set_spell_store(Arc::new(wow_data::SpellStore::new()));
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (1, 5, 80),
        wow_data::PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect,
            spirit: 30,
            base_mana,
        },
    )])));
    let mut class_entry = wow_data::character_progression::ChrClassesEntry::default();
    class_entry.id = 5;
    class_entry.starting_level = 1;
    session.set_chr_classes_store(Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([class_entry]),
    ));
    let mut regen_columns = [0.0; wow_data::RegenMpPerSptGameTableLikeCpp::VALUE_COLUMN_COUNT];
    regen_columns[4] = 0.003345;
    let regen_row = wow_data::RegenMpPerSptEntryLikeCpp::from_columns(regen_columns);
    let mut regen_rows = vec![wow_data::RegenMpPerSptEntryLikeCpp::default(); 79];
    regen_rows.push(regen_row);
    session.set_regen_game_tables(Arc::new(wow_data::RegenGameTablesLikeCpp::from_tables(
        wow_data::RegenMpPerSptGameTableLikeCpp::from_rows(regen_rows),
        wow_data::RegenHpPerSptGameTableLikeCpp::from_rows([]),
        wow_data::OctRegenHpGameTableLikeCpp::from_rows([]),
    )));
}

fn total_stat_percentage_spell_store_like_cpp(
    spell_id: i32,
    is_ability: bool,
) -> wow_data::SpellStore {
    let mut store = wow_data::SpellStore::new();
    store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE,
                effect_base_points: 99,
                effect_die_sides: 1,
                effect_misc_value_2: 1 << 2,
                ..Default::default()
            }],
        },
    );
    let mut attributes = [0; 15];
    if is_ability {
        attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_IS_ABILITY;
    }
    store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    store
}

fn attach_stat_update_player_with_mana_and_health(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    current_mana: i32,
    max_mana: i32,
    current_health: u32,
    max_health: u32,
) {
    let identity = (
        session.player_map_id_like_cpp(),
        session.player_race_like_cpp(),
        session.player_class_like_cpp(),
        session.player_level_like_cpp(),
        session.player_gender_like_cpp(),
    );
    crate::canonical_player_access::install_canonical_player_owner_for_test(
        session,
        u32::from(identity.0),
        0,
    );
    session.set_loaded_player_identity_like_cpp(
        identity.0, identity.1, identity.2, identity.3, identity.4,
    );
    let manager = session
        .canonical_map_manager
        .as_ref()
        .expect("canonical map manager fixture");
    assert!(crate::canonical_player_access::configure_canonical_player_vitals_for_test(
        manager,
        player_guid,
        (
            current_health,
            max_health,
            PowerType::Mana,
            current_mana,
            max_mana,
            0,
        ),
    ));
}

fn make_inventory_item_object_for_test(
    session: &WorldSession,
    item_guid: ObjectGuid,
    entry_id: u32,
    owner_guid: ObjectGuid,
    count: u32,
    durability: u32,
    context: ItemContext,
    slot: u8,
) -> wow_entities::Item {
    session.make_inventory_item_object(
        item_guid,
        entry_id,
        owner_guid,
        count,
        durability,
        context,
        slot,
    )
}

fn insert_inventory_item_object_for_test(session: &mut WorldSession, item: wow_entities::Item) {
    let _ = session.insert_inventory_item_object(item);
}

fn represented_item_bonus_state_for_test(
    session: &WorldSession,
) -> wow_entities::PlayerItemBonusStateLikeCpp {
    session.represented_item_bonus_state_like_cpp()
}

fn canonical_player_health_snapshot_for_test(
    session: &WorldSession,
) -> Option<(u32, u32)> {
    session.canonical_player_health_snapshot_like_cpp()
}

fn canonical_player_power_snapshot_for_test(
    session: &WorldSession,
    power_type: PowerType,
) -> Option<(i32, i32)> {
    session.canonical_player_power_snapshot_like_cpp(power_type)
}

#[test]
fn login_passive_parry_and_block_capabilities_feed_first_stat_projection_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 86);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (1, 1, 80),
        wow_data::PlayerLevelStats {
            strength: 50,
            agility: 30,
            stamina: 40,
            intellect: 10,
            spirit: 20,
            base_mana: 0,
        },
    )])));
    let mut warrior_class = wow_data::character_progression::ChrClassesEntry::default();
    warrior_class.id = 1;
    warrior_class.starting_level = 1;
    session.set_chr_classes_store(Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([warrior_class]),
    ));
    let player_guid = crate::canonical_player_access::install_canonical_player_owner_for_test(
        &mut session,
        571,
        0,
    );
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    let manager = session
        .canonical_map_manager
        .as_ref()
        .expect("canonical map manager fixture");
    assert!(crate::canonical_player_access::configure_canonical_player_vitals_for_test(
        manager,
        player_guid,
        (100, 220, PowerType::Mana, 0, 0, 0),
    ));
    let parry_spell_id = 90_087;
    let block_spell_id = 90_088;
    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, effect) in [
        (
            parry_spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_PARRY,
        ),
        (
            block_spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_BLOCK,
        ),
    ] {
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: effect,
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                aura_type: None,
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect,
                    ..Default::default()
                }],
            },
        );
        let mut attributes = [0; 15];
        attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_PASSIVE;
        spell_store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    }
    session.set_spell_store(Arc::new(spell_store));

    assert_eq!(
        session.canonical_player_parry_block_snapshot_like_cpp(),
        (false, false)
    );
    assert_eq!(
        session.apply_login_known_spell_combat_capabilities_like_cpp(&[
            parry_spell_id,
            block_spell_id,
        ]),
        2
    );
    assert_eq!(
        session.canonical_player_parry_block_snapshot_like_cpp(),
        (true, true)
    );
    let projection = session
        .player_stat_system_projection_like_cpp(
            1,
            1,
            80,
            &wow_entities::RepresentedPlayerGearStatsLikeCpp::default(),
        )
        .expect("warrior stat projection");
    assert_eq!(projection.parry_pct, 5.0);
    assert_eq!(projection.block_pct, 5.0);

    // C++ `CONFIG_STATS_LIMITS_*` (`World.cpp:1664-1668`) caps the published
    // percentages when `Stats.Limits.Enable` is set; the same projection feeds
    // the login create snapshot and the canonical effective-stats snapshot.
    session.set_stats_limits_like_cpp(wow_data::StatsLimitsLikeCpp {
        enabled: true,
        dodge: 1.0,
        parry: 1.0,
        block: 1.0,
        crit: 1.0,
    });
    let limited = session
        .player_stat_system_projection_like_cpp(
            1,
            1,
            80,
            &wow_entities::RepresentedPlayerGearStatsLikeCpp::default(),
        )
        .expect("warrior stat projection with limits");
    assert_eq!(limited.parry_pct, 1.0);
    assert_eq!(limited.block_pct, 1.0);
    assert!(limited.crit_pct <= 1.0);
    assert!(limited.ranged_crit_pct <= 1.0);
    assert!(limited.offhand_crit_pct <= 1.0);
}

#[test]
fn login_stat_update_derives_and_syncs_loaded_enchantment_bonuses_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 81);
    let item_guid = ObjectGuid::create_item(1, 82);
    session.set_player_guid(Some(player_guid));
    set_loaded_player_identity_like_cpp(&mut session, 571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1000, 40);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 777, 1320, 77, 110);
    session
        .mutate_canonical_player_like_cpp(|player| player.unit_mut().set_level(80))
        .unwrap();
    session.set_spell_item_enchantment_store(Arc::new(
        wow_data::SpellItemEnchantmentStore::from_entries([
            wow_data::SpellItemEnchantmentEntry {
                id: 920,
                effect_arg: [
                    wow_constants::ItemModType::Stamina as u32,
                    wow_constants::ItemModType::Mana as u32,
                    wow_constants::ItemModType::Strength as u32,
                ],
                effect_points_min: [3, 4, 2],
                effect_scaling_points: [0.0; 3],
                item_visual: 0,
                flags: wow_constants::SpellItemEnchantmentFlags::empty(),
                required_skill_id: 0,
                required_skill_rank: 0,
                item_level: 1,
                charges: 0,
                effect: [wow_constants::ItemEnchantmentType::Stat as u8; 3],
                condition_id: 0,
                min_level: 1,
                max_level: 0,
            },
            wow_data::SpellItemEnchantmentEntry {
                id: 921,
                effect_arg: [
                    wow_constants::ItemModType::ManaRegeneration as u32,
                    wow_constants::spell::SpellSchools::Fire as u32,
                    0,
                ],
                effect_points_min: [25, 9, 0],
                effect_scaling_points: [0.0; 3],
                item_visual: 0,
                flags: wow_constants::SpellItemEnchantmentFlags::empty(),
                required_skill_id: 0,
                required_skill_rank: 0,
                item_level: 1,
                charges: 0,
                effect: [
                    wow_constants::ItemEnchantmentType::Stat as u8,
                    wow_constants::ItemEnchantmentType::Resistance as u8,
                    wow_constants::ItemEnchantmentType::None as u8,
                ],
                condition_id: 0,
                min_level: 1,
                max_level: 0,
            },
        ]),
    ));
    let mut item = make_inventory_item_object_for_test(
        &session,
        item_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        wow_entities::EQUIPMENT_SLOT_CHEST,
    );
    item.set_enchantment(EnchantmentSlot::EnhancementPermanent, 920, 0, 0);
    item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 921, 0, 0);
    insert_inventory_item_object_for_test(&mut session, item);

    let outcome = session.apply_loaded_equipped_item_enchantments_like_cpp(item_guid);
    assert!(outcome.send_stat_update);
    assert_eq!(
        represented_item_bonus_state_for_test(&session).stats_base,
        [2, 0, 3, 0, 0]
    );
    assert_eq!(represented_item_bonus_state_for_test(&session).mana_base, 4);
    assert_eq!(
        represented_item_bonus_state_for_test(&session)
            .mana_regen_bonus,
        25
    );
    let (_, changes) = session
        .player_stat_changes_with_represented_item_bonuses_like_cpp(true)
        .expect("login stat changes with loaded enchantments");

    assert_eq!(changes.health, 13, "current health clamps to the new max");
    assert_eq!(
        changes.max_health, 13,
        "CreateHealth is zero and stamina supplies max HP"
    );
    assert_eq!(changes.power0, 777, "current mana remains authoritative");
    assert_eq!(
        changes.max_power0, 1324,
        "flat mana is derived before max power"
    );
    assert_eq!(changes.stats, [12, 10, 13, 40, 30]);
    assert_eq!(
        changes.attack_power, -20,
        "ChrClasses priest AP coefficients"
    );
    assert_eq!(changes.mana_regen_combat, 5.0);
    assert_eq!(changes.mana_regen_mp5, 0.0);
    let expected_spirit_regen = 40.0_f32.sqrt() * 30.0 * 0.003345;
    assert!((changes.mana_regen - (5.0 + expected_spirit_regen)).abs() < 0.0001);
    assert_eq!(
        canonical_player_health_snapshot_for_test(&session),
        Some((13, 13))
    );
    assert_eq!(
        canonical_player_power_snapshot_for_test(&session, PowerType::Mana),
        Some((777, 1324))
    );
    let effective = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("effective combat stats are owned by the canonical Player");
    assert_eq!(effective.stats, [12, 10, 13, 40, 30]);
    assert_eq!(effective.max_mana, 1324);
    assert_eq!(effective.combat_ratings, [0; 32]);
    assert_eq!(effective.resistances, [20, 0, 9, 0, 0, 0, 0]);

    let spell_id = 90_084;
    session.set_spell_store(Arc::new(total_stat_percentage_spell_store_like_cpp(
        spell_id, false,
    )));
    session.set_state(crate::session::SessionState::LoggedIn);
    session
        .apply_aura(spell_id, player_guid, 30_000, 1)
        .expect("apply total-stat aura over loaded enchantments");
    assert_eq!(
        canonical_player_health_snapshot_for_test(&session),
        Some((13, 80)),
        "absolute aura recalc keeps the loaded +3 stamina enchant before doubling stamina"
    );

    let slot = session
        .visible_aura_slot_for_spell_like_cpp(spell_id)
        .expect("total-stat aura slot");
    session.remove_aura(slot).expect("remove total-stat aura");
    assert_eq!(
        canonical_player_health_snapshot_for_test(&session),
        Some((13, 13)),
        "absolute aura removal keeps the loaded enchantment bonus"
    );
}
