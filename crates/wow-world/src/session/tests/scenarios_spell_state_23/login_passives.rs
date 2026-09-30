use super::*;

#[test]
fn login_passive_known_spell_auras_apply_like_cpp_addspell() {
    let (mut session, _, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 421);
    session.set_player_guid(Some(player_guid));
    let mut known_spells = vec![822, 28877, 14_914, 60_000, 60_001, 60_002, 60_003];

    let mut spell_store = SpellStore::new();
    for spell_id in [822, 28877, 14_914, 14_908, 60_000, 60_001, 60_002, 60_003] {
        let has_aura = spell_id != 60_001;
        spell_store.insert(
            spell_id,
            SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: if has_aura {
                    wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
                } else {
                    0
                },
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                aura_type: has_aura
                    .then_some(wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: has_aura
                    .then(|| {
                        vec![wow_data::SpellEffectInfo {
                            effect_index: 0,
                            effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                            effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE,
                            ..Default::default()
                        }]
                    })
                    .unwrap_or_default(),
            },
        );
    }
    for spell_id in [822, 28877, 14_908, 60_000, 60_001, 60_002, 60_003] {
        let mut attributes = [0u32; 15];
        attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_PASSIVE;
        spell_store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    }
    spell_store.insert_spell_shapeshift_masks_like_cpp(60_000, 1 << 4, 0);
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp {
                spell_id: 14_914,
                supercedes_spell_id: 14_908,
            }],
            |spell_id| matches!(spell_id, 14_908 | 14_914),
        ),
    ));
    session.set_spell_aura_restrictions_store(Arc::new(SpellAuraRestrictionsStore::from_entries(
        [wow_data::SpellAuraRestrictionsEntry {
            id: 1,
            difficulty_id: 0,
            caster_aura_state: 7,
            target_aura_state: 0,
            exclude_caster_aura_state: 0,
            exclude_target_aura_state: 0,
            caster_aura_spell: 0,
            target_aura_spell: 0,
            exclude_caster_aura_spell: 0,
            exclude_target_aura_spell: 0,
            spell_id: 60_002,
        }],
    )));
    session.set_spell_equipped_items_store(Arc::new(SpellEquippedItemsStore::from_entries([
        SpellEquippedItemsEntry {
            id: 1,
            spell_id: 60_003,
            equipped_item_class: ItemClass::Weapon as i8,
            equipped_item_inv_types: 0,
            equipped_item_subclass: 1_i32 << (ItemSubClassWeapon::Axe as u32),
        },
    ])));

    assert_eq!(
        session.apply_loaded_known_spell_dependencies_like_cpp(&mut known_spells),
        0
    );
    assert!(
        !known_spells.contains(&14_908),
        "C++ keeps the previous rank inactive when a higher known rank supersedes it"
    );
    session.set_known_spells_like_cpp(known_spells.clone());
    assert_eq!(session.apply_login_passive_known_spell_auras_like_cpp(), 2);
    assert_eq!(
        session.apply_loaded_known_spell_previous_rank_passive_auras_like_cpp(&known_spells),
        1,
        "C++ AddSpell recursively adds/casts previous passive ranks during load"
    );
    let visible: Vec<_> = session
        .visible_auras
        .values()
        .map(|aura| (aura.slot, aura.spell_id, aura.caster_guid, aura.aura_flags))
        .collect();
    assert!(visible.contains(&(
        0,
        822,
        player_guid,
        AFLAG_NOCASTER_LIKE_CPP | 0x0000_0100 | 0x0000_0200
    )));
    assert!(visible.contains(&(
        1,
        28877,
        player_guid,
        AFLAG_NOCASTER_LIKE_CPP | 0x0000_0100 | 0x0000_0200
    )));
    assert!(visible.contains(&(
        2,
        14908,
        player_guid,
        AFLAG_NOCASTER_LIKE_CPP | 0x0000_0100 | 0x0000_0200
    )));
    assert!(
        !session
            .visible_auras
            .values()
            .any(|aura| matches!(aura.spell_id, 60_000 | 60_001 | 60_002 | 60_003))
    );
}
#[test]
fn login_total_stat_percentage_aura_records_cpp_multiplier_and_misc_b_mask() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 15);
    session.set_player_guid(Some(player_guid));
    session.set_known_spells_like_cpp(vec![20_598]);

    let mut spell_store = SpellStore::new();
    spell_store.insert(
        20_598,
        SpellInfo {
            spell_id: 20_598,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE,
                effect_base_points: 2,
                effect_die_sides: 1,
                // C++ Unit::UpdateStatBuffMod filters this aura type with
                // MiscValueA, independently of the total-stat mask below.
                effect_misc_value_1: 4,
                // C++ HandleModTotalPercentStat selects with MiscValueB.
                // Zero applies the effect to all five primary stats.
                effect_misc_value_2: 0,
                ..Default::default()
            }],
            ..test_spell_info_like_cpp(20_598)
        },
    );
    let mut attributes = [0u32; 15];
    attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_PASSIVE;
    spell_store.insert_spell_misc_attributes_like_cpp(20_598, attributes);
    session.set_spell_store(Arc::new(spell_store));

    assert_eq!(session.apply_login_passive_known_spell_auras_like_cpp(), 1);
    assert_eq!(
        session.represented_total_stat_multipliers_like_cpp(),
        [1.03; 5]
    );
    assert_eq!(
        session.represented_total_stat_buff_multipliers_like_cpp(),
        [1.0, 1.0, 1.0, 1.0, 1.03]
    );
    let aura = session
        .visible_auras
        .values()
        .find(|aura| aura.spell_id == 20_598)
        .expect("Human Spirit aura");
    assert_eq!(
        aura.represented_effect,
        Some(RepresentedAuraEffectLikeCpp::ModTotalStatPercentage)
    );
    assert_eq!(aura.represented_amount, 3);
    assert_eq!(aura.represented_misc_value, Some(0));

    session.remove_aura(aura.slot).expect("remove Human Spirit");
    assert_eq!(
        session.represented_total_stat_multipliers_like_cpp(),
        [1.0; 5]
    );
    assert_eq!(
        session.represented_total_stat_buff_multipliers_like_cpp(),
        [1.0; 5]
    );
}
