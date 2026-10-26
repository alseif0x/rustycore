use super::super::save_plan::player_character_save_statements_like_cpp;
use super::{equipment_row, minimal_character_request};
use crate::params::PreparedStatement;
use crate::statements::CharStatements;
use crate::statements::StatementDef;
use wow_persistence::*;
use wow_persistence::{
    PlayerCharacterSaveRequestLikeCpp, PlayerCufProfileSaveLikeCpp, PlayerEquipmentSetStateLikeCpp,
    PlayerEquipmentSetTypeLikeCpp, PlayerSpellSaveGroupLikeCpp, PlayerSpellStateLikeCpp,
    PlayerVoidStorageSaveLikeCpp,
};

#[test]
fn character_save_adapter_preserves_the_frozen_statement_order_like_cpp() {
    let mut equipment_insert = equipment_row();
    equipment_insert.set_guid = 10;
    let mut equipment_update = equipment_row();
    equipment_update.set_guid = 11;
    equipment_update.state = PlayerEquipmentSetStateLikeCpp::Changed;
    let mut equipment_delete = equipment_row();
    equipment_delete.set_guid = 12;
    equipment_delete.state = PlayerEquipmentSetStateLikeCpp::Deleted;
    let mut transmog_insert = equipment_row();
    transmog_insert.set_guid = 13;
    transmog_insert.set_type = PlayerEquipmentSetTypeLikeCpp::Transmog;
    let mut transmog_update = equipment_row();
    transmog_update.set_guid = 14;
    transmog_update.set_type = PlayerEquipmentSetTypeLikeCpp::Transmog;
    transmog_update.state = PlayerEquipmentSetStateLikeCpp::Changed;
    let mut transmog_delete = equipment_row();
    transmog_delete.set_guid = 15;
    transmog_delete.set_type = PlayerEquipmentSetTypeLikeCpp::Transmog;
    transmog_delete.state = PlayerEquipmentSetStateLikeCpp::Deleted;

    let request = PlayerCharacterSaveRequestLikeCpp {
        player_guid: 1,
        account_id: 2,
        wall_clock_unix_secs: 1_700_000_000,
        character: PlayerCharacterSnapshotSaveLikeCpp {
            position: PlayerPositionSaveLikeCpp {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                orientation: 0.5,
                map_id: 0,
                instance_id: 0,
                zone_id: 0,
            },
            level: 1,
            xp: 0,
            money: 7,
            rest_state: 0,
            player_flags: 0,
            rest_bonus: 0.0,
            logout_time: 1_700_000_000,
            is_logout_resting: false,
            health: 9,
            powers: Some([0; 10]),
            talent_reset_cost: 0,
            talent_reset_time: 0,
            explored_zones: String::new(),
            dungeon_difficulty: 0,
            raid_difficulty: 0,
            legacy_raid_difficulty: 0,
            at_login_flags: 0x104,
            death_expire_time: 1_700_000_123,
        },
        item_durability: Some(vec![
            PlayerItemDurabilitySave {
                item_guid: 22,
                durability: 82,
            },
            PlayerItemDurabilitySave {
                item_guid: 21,
                durability: 81,
            },
        ]),
        inventory_items: None,
        currencies: None,
        resurrection_sickness: None,
        ghost_wisp: None,
        spells: Some(PlayerSpellSaveGroupLikeCpp::Complete {
            rows: vec![PlayerSpellSaveLikeCpp {
                spell_id: 100,
                active: true,
                disabled: false,
                dependent: false,
                favorite: true,
                state: PlayerSpellStateLikeCpp::Changed,
            }],
            fallback_rows_were_present: false,
        }),
        skills: Some(vec![PlayerSkillSaveLikeCpp {
            skill_id: 6,
            value: 300,
            max: 300,
            profession_slot: -1,
        }]),
        glyphs: Some(
            (0..24)
                .map(|glyph_slot| PlayerGlyphSaveLikeCpp {
                    talent_group: 0,
                    glyph_slot,
                    glyph_id: 0,
                })
                .collect(),
        ),
        talents: Some(vec![PlayerTalentSaveLikeCpp {
            talent_id: 200,
            rank: 1,
            talent_group: 0,
        }]),
        spell_cooldowns: Some(vec![PlayerSpellCooldownSaveLikeCpp {
            spell_id: 300,
            item_id: 0,
            cooldown_end_unix_secs: 1_700_000_010,
            category_id: 30,
            category_end_unix_secs: 1_700_000_020,
        }]),
        spell_charges: Some(vec![PlayerSpellChargeSaveLikeCpp {
            category_id: 31,
            recharge_start_unix_secs: 1_700_000_001,
            recharge_end_unix_secs: 1_700_000_030,
        }]),
        action_buttons: Some(PlayerActionButtonsSaveLikeCpp {
            spec: 0,
            trait_config_id: 0,
            rows: vec![PlayerActionButtonSaveLikeCpp {
                button: 0,
                packed_action: 0x0100_0064,
            }],
        }),
        equipment_sets: Some(vec![
            equipment_insert,
            equipment_update,
            equipment_delete,
            transmog_insert,
            transmog_update,
            transmog_delete,
        ]),
        void_storage: Some(vec![
            PlayerVoidStorageSlotSaveLikeCpp {
                slot: 0,
                item: Some(PlayerVoidStorageSaveLikeCpp {
                    item_id: 400,
                    item_entry: 401,
                    creator_guid: 402,
                    fixed_scaling_level: 80,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    context: 0,
                }),
            },
            PlayerVoidStorageSlotSaveLikeCpp {
                slot: 1,
                item: None,
            },
        ]),
        tutorials: Some(PlayerTutorialsSaveLikeCpp {
            tutorials: [0; 8],
            already_persisted: false,
        }),
        instance_lock_times: vec![PlayerInstanceLockTimeSaveLikeCpp {
            instance_id: 500,
            release_time: 1_700_000_100,
        }],
        played_time: PlayerPlayedTimeSaveLikeCpp {
            total_time: 11,
            level_time: 5,
        },
        reputations: vec![PlayerReputationSaveLikeCpp {
            faction_id: 600,
            standing: 1_000,
            flags: 1,
        }],
        cuf_profiles: Some(vec![
            PlayerCufProfileSlotSaveLikeCpp {
                profile_id: 0,
                profile: Some(PlayerCufProfileSaveLikeCpp {
                    profile_name: "raid".to_owned(),
                    frame_height: 40,
                    frame_width: 80,
                    sort_by: 0,
                    health_text: 0,
                    bool_options: 0,
                    top_point: 0,
                    bottom_point: 0,
                    left_point: 0,
                    top_offset: 0,
                    bottom_offset: 0,
                    left_offset: 0,
                }),
            },
            PlayerCufProfileSlotSaveLikeCpp {
                profile_id: 1,
                profile: None,
            },
        ]),
    };
    let statements = player_character_save_statements_like_cpp(&request);
    let durability_sql = CharStatements::UPD_ITEM_INSTANCE_DURABILITY_OWNED.sql();
    let durability_binds = statements
        .iter()
        .filter(|statement| statement.sql() == durability_sql)
        .map(PreparedStatement::params)
        .collect::<Vec<_>>();
    assert_eq!(
        durability_binds,
        vec![
            &[
                crate::SqlParam::U32(81),
                crate::SqlParam::U64(21),
                crate::SqlParam::U64(1),
                crate::SqlParam::U64(1),
            ][..],
            &[
                crate::SqlParam::U32(82),
                crate::SqlParam::U64(22),
                crate::SqlParam::U64(1),
                crate::SqlParam::U64(1),
            ][..],
        ],
        "the adapter sorts per-item durability statements by item GUID"
    );
    let mut runs: Vec<(String, usize)> = Vec::new();
    for statement in statements {
        let sql = statement.sql().to_owned();
        match runs.last_mut() {
            Some((previous, count)) if *previous == sql => *count += 1,
            _ => runs.push((sql, 1)),
        }
    }
    let golden: Vec<(String, usize)> = serde_json::from_str(include_str!(
        "../../../../../wow-world/tests/fixtures/player-save-plan-order.json"
    ))
    .expect("frozen order fixture parses");
    assert_eq!(
        runs,
        golden,
        "replace player-save-plan-order.json with:\n{}",
        serde_json::to_string_pretty(&runs).unwrap_or_default()
    );
}

#[test]
fn lifecycle_aura_spell_reconciles_all_keys_before_sorted_inserts() {
    let mut request = minimal_character_request();
    let without_authority =
        super::super::save_plan::player_character_save_statements_like_cpp(&request);
    assert!(without_authority.iter().all(|statement| {
        statement.sql() != CharStatements::DEL_CHAR_AURA_BY_SPELL.sql()
            && statement.sql() != CharStatements::DEL_CHAR_AURA_EFFECT_BY_SPELL.sql()
            && statement.sql() != CharStatements::DEL_CHAR_AURA_BY_KEY.sql()
            && statement.sql() != CharStatements::DEL_CHAR_AURA_EFFECT_BY_KEY.sql()
    }));

    request.action_buttons = Some(PlayerActionButtonsSaveLikeCpp {
        spec: 1,
        trait_config_id: 2,
        rows: vec![PlayerActionButtonSaveLikeCpp {
            button: 3,
            packed_action: 4,
        }],
    });
    let saved_aura = |amount| PlayerLifecycleAuraSave {
        difficulty: 0,
        stack_count: 1,
        max_duration: 60_000,
        remaining_duration: 45_000,
        charges: 0,
        recalculate_mask: 0,
        cast_item_id: 0,
        cast_item_level: -1,
        effects: vec![PlayerLifecycleAuraEffectSave {
            index: 0,
            amount,
            base_amount: amount - 2,
        }],
    };
    request.resurrection_sickness = Some(PlayerLifecycleAuraSpellSave {
        spell_id: 15_007,
        families: vec![
            PlayerLifecycleAuraFamilySave {
                spell_id: 15_007,
                caster_guid_binary: [0x21; 16],
                item_guid_binary: [0x22; 16],
                auras: vec![saved_aura(21)],
            },
            PlayerLifecycleAuraFamilySave {
                spell_id: 15_008,
                caster_guid_binary: [0x31; 16],
                item_guid_binary: [0x32; 16],
                auras: vec![saved_aura(31)],
            },
            PlayerLifecycleAuraFamilySave {
                spell_id: 15_007,
                caster_guid_binary: [0x11; 16],
                item_guid_binary: [0x12; 16],
                auras: vec![saved_aura(11)],
            },
        ],
    });
    request.equipment_sets = Some(vec![equipment_row()]);

    let statements = super::super::save_plan::player_character_save_statements_like_cpp(&request);
    let index_of = |sql: &str| {
        statements
            .iter()
            .position(|statement| statement.sql() == sql)
            .expect("expected save statement")
    };
    let action = index_of(CharStatements::INS_CHAR_ACTION.sql());
    let effect_delete = index_of(CharStatements::DEL_CHAR_AURA_EFFECT_BY_SPELL.sql());
    let aura_delete = index_of(CharStatements::DEL_CHAR_AURA_BY_SPELL.sql());
    let aura_insert = index_of(CharStatements::INS_AURA.sql());
    let effect_insert = index_of(CharStatements::INS_AURA_EFFECT.sql());
    let equipment = index_of(CharStatements::INS_EQUIP_SET.sql());
    assert!(action < effect_delete);
    assert!(effect_delete < aura_delete);
    assert!(aura_delete < aura_insert);
    assert!(aura_insert < effect_insert);
    assert!(effect_insert < equipment);
    let aura_sql = statements
        .iter()
        .filter(|statement| {
            let sql = statement.sql();
            sql == CharStatements::DEL_CHAR_AURA_EFFECT_BY_SPELL.sql()
                || sql == CharStatements::DEL_CHAR_AURA_BY_SPELL.sql()
                || sql == CharStatements::DEL_CHAR_AURA_BY_KEY.sql()
                || sql == CharStatements::DEL_CHAR_AURA_EFFECT_BY_KEY.sql()
                || sql == CharStatements::INS_AURA.sql()
                || sql == CharStatements::INS_AURA_EFFECT.sql()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        aura_sql
            .iter()
            .map(|statement| statement.sql())
            .collect::<Vec<_>>(),
        vec![
            CharStatements::DEL_CHAR_AURA_EFFECT_BY_SPELL.sql(),
            CharStatements::DEL_CHAR_AURA_BY_SPELL.sql(),
            CharStatements::INS_AURA.sql(),
            CharStatements::INS_AURA_EFFECT.sql(),
            CharStatements::INS_AURA.sql(),
            CharStatements::INS_AURA_EFFECT.sql(),
        ]
    );
    assert_eq!(
        aura_sql[0].params(),
        &[crate::SqlParam::U64(1), crate::SqlParam::U32(15_007)]
    );
    assert_eq!(aura_sql[1].params(), aura_sql[0].params());
    for (aura_index, effect_index, caster, item, amount) in [
        (2, 3, [0x11; 16], [0x12; 16], 11),
        (4, 5, [0x21; 16], [0x22; 16], 21),
    ] {
        let statement = aura_sql[aura_index];
        assert_eq!(
            &statement.params()[..5],
            &[
                crate::SqlParam::U64(1),
                crate::SqlParam::Bytes(caster.to_vec()),
                crate::SqlParam::Bytes(item.to_vec()),
                crate::SqlParam::U32(15_007),
                crate::SqlParam::U32(1),
            ]
        );
        let effect_statement = aura_sql[effect_index];
        assert_eq!(effect_statement.params()[6], crate::SqlParam::I32(amount));
    }

    request
        .resurrection_sickness
        .as_mut()
        .unwrap()
        .families
        .clear();
    let empty_spell = super::super::save_plan::player_character_save_statements_like_cpp(&request);
    assert_eq!(
        empty_spell
            .iter()
            .filter(|statement| {
                statement.sql() == CharStatements::DEL_CHAR_AURA_BY_SPELL.sql()
                    || statement.sql() == CharStatements::DEL_CHAR_AURA_EFFECT_BY_SPELL.sql()
            })
            .count(),
        2,
        "Some(empty) still clears the selected spell's complete keyset"
    );
    assert!(empty_spell.iter().all(|statement| {
        statement.sql() != CharStatements::INS_AURA.sql()
            && statement.sql() != CharStatements::INS_AURA_EFFECT.sql()
            && statement.sql() != CharStatements::DEL_CHAR_AURA_BY_KEY.sql()
            && statement.sql() != CharStatements::DEL_CHAR_AURA_EFFECT_BY_KEY.sql()
    }));
}

#[test]
fn ghost_wisp_lifecycle_aura_families_reconcile_by_spell_then_insert_full_keys() {
    let mut request = minimal_character_request();
    let without_authority = player_character_save_statements_like_cpp(&request);
    assert!(without_authority.iter().all(|statement| {
        statement.sql() != CharStatements::DEL_CHAR_AURA_BY_SPELL.sql()
            && statement.sql() != CharStatements::DEL_CHAR_AURA_EFFECT_BY_SPELL.sql()
            && statement.sql() != CharStatements::DEL_CHAR_AURA_BY_KEY.sql()
            && statement.sql() != CharStatements::DEL_CHAR_AURA_EFFECT_BY_KEY.sql()
            && statement.sql() != CharStatements::INS_AURA.sql()
            && statement.sql() != CharStatements::INS_AURA_EFFECT.sql()
    }));

    request.ghost_wisp = Some(Vec::new());
    let empty_snapshot = player_character_save_statements_like_cpp(&request);
    let cleanup_sql = empty_snapshot
        .iter()
        .filter(|statement| {
            statement.sql() == CharStatements::DEL_CHAR_AURA_EFFECT_BY_SPELL.sql()
                || statement.sql() == CharStatements::DEL_CHAR_AURA_BY_SPELL.sql()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        cleanup_sql
            .iter()
            .map(|statement| statement.sql())
            .collect::<Vec<_>>(),
        vec![
            CharStatements::DEL_CHAR_AURA_EFFECT_BY_SPELL.sql(),
            CharStatements::DEL_CHAR_AURA_BY_SPELL.sql(),
            CharStatements::DEL_CHAR_AURA_EFFECT_BY_SPELL.sql(),
            CharStatements::DEL_CHAR_AURA_BY_SPELL.sql(),
        ],
        "Some(empty) still authorizes both fixed spell-family cleanups"
    );
    assert_eq!(
        cleanup_sql
            .iter()
            .map(|statement| statement.params())
            .collect::<Vec<_>>(),
        vec![
            &[crate::SqlParam::U64(1), crate::SqlParam::U32(8_326)][..],
            &[crate::SqlParam::U64(1), crate::SqlParam::U32(8_326)][..],
            &[crate::SqlParam::U64(1), crate::SqlParam::U32(20_584)][..],
            &[crate::SqlParam::U64(1), crate::SqlParam::U32(20_584)][..],
        ]
    );
    assert!(empty_snapshot.iter().all(|statement| {
        statement.sql() != CharStatements::DEL_CHAR_AURA_BY_KEY.sql()
            && statement.sql() != CharStatements::DEL_CHAR_AURA_EFFECT_BY_KEY.sql()
            && statement.sql() != CharStatements::INS_AURA.sql()
            && statement.sql() != CharStatements::INS_AURA_EFFECT.sql()
    }));

    let caster = [0xA5; 16];
    let item = [0xB6; 16];
    let foreign_caster = [0xC7; 16];
    let ghost_aura = PlayerLifecycleAuraSave {
        difficulty: 1,
        stack_count: 2,
        max_duration: -1,
        remaining_duration: -1,
        charges: 3,
        recalculate_mask: 0b111,
        cast_item_id: 4_321,
        cast_item_level: 77,
        effects: vec![
            PlayerLifecycleAuraEffectSave {
                index: 2,
                amount: 50,
                base_amount: 49,
            },
            PlayerLifecycleAuraEffectSave {
                index: 0,
                amount: -20,
                base_amount: -21,
            },
            PlayerLifecycleAuraEffectSave {
                index: 1,
                amount: 50,
                base_amount: 49,
            },
        ],
    };
    request.ghost_wisp = Some(vec![
        PlayerLifecycleAuraFamilySave {
            spell_id: 8_326,
            caster_guid_binary: caster,
            item_guid_binary: item,
            auras: vec![ghost_aura.clone()],
        },
        PlayerLifecycleAuraFamilySave {
            spell_id: 8_326,
            caster_guid_binary: foreign_caster,
            item_guid_binary: [0; 16],
            auras: vec![ghost_aura.clone()],
        },
        PlayerLifecycleAuraFamilySave {
            spell_id: 20_584,
            caster_guid_binary: caster,
            item_guid_binary: [0; 16],
            auras: Vec::new(),
        },
        PlayerLifecycleAuraFamilySave {
            spell_id: 1_861,
            caster_guid_binary: caster,
            item_guid_binary: item,
            auras: vec![ghost_aura],
        },
    ]);

    let statements = player_character_save_statements_like_cpp(&request);
    let aura_statements = statements
        .iter()
        .filter(|statement| {
            let sql = statement.sql();
            sql == CharStatements::DEL_CHAR_AURA_EFFECT_BY_SPELL.sql()
                || sql == CharStatements::DEL_CHAR_AURA_BY_SPELL.sql()
                || sql == CharStatements::DEL_CHAR_AURA_EFFECT_BY_KEY.sql()
                || sql == CharStatements::DEL_CHAR_AURA_BY_KEY.sql()
                || sql == CharStatements::INS_AURA.sql()
                || sql == CharStatements::INS_AURA_EFFECT.sql()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        aura_statements
            .iter()
            .map(|statement| statement.sql())
            .collect::<Vec<_>>(),
        vec![
            CharStatements::DEL_CHAR_AURA_EFFECT_BY_SPELL.sql(),
            CharStatements::DEL_CHAR_AURA_BY_SPELL.sql(),
            CharStatements::DEL_CHAR_AURA_EFFECT_BY_SPELL.sql(),
            CharStatements::DEL_CHAR_AURA_BY_SPELL.sql(),
            CharStatements::INS_AURA.sql(),
            CharStatements::INS_AURA_EFFECT.sql(),
            CharStatements::INS_AURA_EFFECT.sql(),
            CharStatements::INS_AURA_EFFECT.sql(),
            CharStatements::INS_AURA.sql(),
            CharStatements::INS_AURA_EFFECT.sql(),
            CharStatements::INS_AURA_EFFECT.sql(),
            CharStatements::INS_AURA_EFFECT.sql(),
        ],
        "selected spell cleanup precedes all full-key inserts; unselected rows are omitted"
    );

    assert_eq!(
        aura_statements[0].params(),
        &[crate::SqlParam::U64(1), crate::SqlParam::U32(8_326)]
    );
    assert_eq!(
        aura_statements[1].params(),
        &[crate::SqlParam::U64(1), crate::SqlParam::U32(8_326)]
    );
    assert_eq!(
        aura_statements[2].params(),
        &[crate::SqlParam::U64(1), crate::SqlParam::U32(20_584)]
    );
    assert_eq!(
        aura_statements[3].params(),
        &[crate::SqlParam::U64(1), crate::SqlParam::U32(20_584)]
    );
    assert_eq!(
        aura_statements[4].params(),
        &[
            crate::SqlParam::U64(1),
            crate::SqlParam::Bytes(caster.to_vec()),
            crate::SqlParam::Bytes(item.to_vec()),
            crate::SqlParam::U32(8_326),
            crate::SqlParam::U32(0b111),
            crate::SqlParam::U32(0b111),
            crate::SqlParam::U8(1),
            crate::SqlParam::U8(2),
            crate::SqlParam::I32(-1),
            crate::SqlParam::I32(-1),
            crate::SqlParam::U8(3),
            crate::SqlParam::U32(4_321),
            crate::SqlParam::I32(77),
        ]
    );
    for (statement, index, amount, base_amount) in [
        (&aura_statements[5], 0, -20, -21),
        (&aura_statements[6], 1, 50, 49),
        (&aura_statements[7], 2, 50, 49),
    ] {
        assert_eq!(
            statement.params(),
            &[
                crate::SqlParam::U64(1),
                crate::SqlParam::Bytes(caster.to_vec()),
                crate::SqlParam::Bytes(item.to_vec()),
                crate::SqlParam::U32(8_326),
                crate::SqlParam::U32(0b111),
                crate::SqlParam::U8(index),
                crate::SqlParam::I32(amount),
                crate::SqlParam::I32(base_amount),
            ]
        );
    }
    let foreign_ghost_key = vec![
        crate::SqlParam::U64(1),
        crate::SqlParam::Bytes(foreign_caster.to_vec()),
        crate::SqlParam::Bytes(vec![0; 16]),
        crate::SqlParam::U32(8_326),
        crate::SqlParam::U32(0b111),
    ];
    assert_eq!(aura_statements[8].params()[..5], foreign_ghost_key[..]);
    assert_eq!(aura_statements[8].params()[5], crate::SqlParam::U32(0b111));
    for (statement, index, amount, base_amount) in [
        (&aura_statements[9], 0, -20, -21),
        (&aura_statements[10], 1, 50, 49),
        (&aura_statements[11], 2, 50, 49),
    ] {
        assert_eq!(
            statement.params()[..6],
            [
                crate::SqlParam::U64(1),
                crate::SqlParam::Bytes(foreign_caster.to_vec()),
                crate::SqlParam::Bytes(vec![0; 16]),
                crate::SqlParam::U32(8_326),
                crate::SqlParam::U32(0b111),
                crate::SqlParam::U8(index),
            ]
        );
        assert_eq!(statement.params()[6], crate::SqlParam::I32(amount));
        assert_eq!(statement.params()[7], crate::SqlParam::I32(base_amount));
    }
}

#[test]
fn fallback_spells_expand_inside_the_spell_group_without_a_statement_port_like_cpp() {
    let mut request = minimal_character_request();
    request.spells = Some(PlayerSpellSaveGroupLikeCpp::Fallback {
        rows: vec![
            PlayerFallbackSpellSaveLikeCpp {
                spell_id: 20,
                active: true,
                dependent: false,
            },
            PlayerFallbackSpellSaveLikeCpp {
                spell_id: 30,
                active: false,
                dependent: true,
            },
        ],
    });
    let statements = player_character_save_statements_like_cpp(&request);
    let sql = statements
        .iter()
        .map(PreparedStatement::sql)
        .collect::<Vec<_>>();
    let fallback = sql
        .iter()
        .position(|sql| *sql == CharStatements::UPSERT_CHAR_SPELL_LEARN_FALLBACK.sql())
        .expect("non-dependent fallback spell is upserted");
    assert_eq!(
        sql.get(fallback + 1),
        Some(&CharStatements::DEL_CHAR_SPELL_BY_SPELL.sql()),
        "dependent fallback removal retains its position in the spell group"
    );
}

#[test]
fn cooldown_and_charge_groups_drop_expired_rows_but_keep_the_group_replace_like_cpp() {
    let mut request = minimal_character_request();
    request.spell_cooldowns = Some(vec![
        PlayerSpellCooldownSaveLikeCpp {
            spell_id: 1,
            item_id: 0,
            cooldown_end_unix_secs: 1_699_999_999,
            category_id: 0,
            category_end_unix_secs: 1_699_999_999,
        },
        PlayerSpellCooldownSaveLikeCpp {
            spell_id: 2,
            item_id: 0,
            cooldown_end_unix_secs: 1_700_000_001,
            category_id: 0,
            category_end_unix_secs: 1_699_999_999,
        },
    ]);
    request.spell_charges = Some(vec![
        PlayerSpellChargeSaveLikeCpp {
            category_id: 3,
            recharge_start_unix_secs: 1_699_999_990,
            recharge_end_unix_secs: 1_699_999_999,
        },
        PlayerSpellChargeSaveLikeCpp {
            category_id: 4,
            recharge_start_unix_secs: 1_700_000_000,
            recharge_end_unix_secs: 1_700_000_001,
        },
    ]);
    let statements = player_character_save_statements_like_cpp(&request);
    let sql = statements
        .iter()
        .map(PreparedStatement::sql)
        .collect::<Vec<_>>();
    assert_eq!(
        sql.iter()
            .filter(|sql| **sql == CharStatements::INS_CHAR_SPELL_COOLDOWN.sql())
            .count(),
        1
    );
    assert_eq!(
        sql.iter()
            .filter(|sql| **sql == CharStatements::INS_CHAR_SPELL_CHARGES.sql())
            .count(),
        1
    );
    assert!(sql.contains(&CharStatements::DEL_CHAR_SPELL_COOLDOWNS.sql()));
    assert!(sql.contains(&CharStatements::DEL_CHAR_SPELL_CHARGES.sql()));
}
