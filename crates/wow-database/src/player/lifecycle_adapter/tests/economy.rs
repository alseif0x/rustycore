use super::super::economy::PlayerTalentResetCommitReconciliationLikeCpp;
use super::super::economy::player_bank_slot_purchase_statement_like_cpp;
use super::super::economy::player_buyback_clear_statements_like_cpp;
use super::super::economy::player_durability_repair_statement_like_cpp;
use super::super::economy::player_money_transaction_statements_like_cpp;
use super::super::economy::player_money_write_statement_like_cpp;
use super::super::economy::player_realm_character_count_statements_like_cpp;
use super::super::economy::player_talent_reset_statements_like_cpp;
use super::super::economy::player_uncage_item_state_statement_like_cpp;
use super::super::economy::player_xp_persistence_statements_like_cpp;
use super::super::economy::reconcile_player_talent_reset_commit_like_cpp;
use super::super::login_reads::player_login_auxiliary_load_statement_like_cpp;
use super::super::login_writes::player_homebind_persistence_statement_like_cpp;
use super::super::login_writes::player_login_item_repair_statements_like_cpp;
use super::super::login_writes::player_login_pet_talent_reset_statements_like_cpp;
use super::super::login_writes::player_online_mark_statement_like_cpp;
use super::super::player_currency_save_statements_like_cpp;
use super::super::transports::player_login_transport_load_statement_like_cpp;
use crate::SqlParam;
use crate::params::PreparedStatement;
use crate::statements::CharStatements;
use crate::statements::StatementDef;
use wow_persistence::*;
use wow_persistence::{
    PlayerBankSlotPurchaseRequestLikeCpp, PlayerBuybackClearRequestLikeCpp,
    PlayerCurrencySaveKindLikeCpp, PlayerCurrencySaveRequestLikeCpp,
    PlayerDurabilityRepairSaveLikeCpp, PlayerHomebindPersistenceRequestLikeCpp,
    PlayerLoginAuxiliaryLoadRequestLikeCpp, PlayerLoginItemRepairActionLikeCpp,
    PlayerLoginItemRepairRequestLikeCpp, PlayerLoginTransportLoadRequestLikeCpp,
    PlayerMoneyTransactionRequestLikeCpp, PlayerMoneyWriteRequestLikeCpp,
    PlayerOnlineMarkRequestLikeCpp, PlayerRealmCharacterCountRefreshRequestLikeCpp,
    PlayerTalentResetPersistenceRequestLikeCpp, PlayerUncageItemStateRequestLikeCpp,
    PlayerXpPersistenceRequestLikeCpp,
};

#[test]
fn money_and_durability_transaction_preserves_statement_and_bind_order_like_cpp() {
    let request = PlayerMoneyTransactionRequestLikeCpp {
        player_guid: 42,
        money_after: 900,
        durability_repairs: vec![
            PlayerDurabilityRepairSaveLikeCpp {
                item_db_guid: 71,
                durability: 80,
            },
            PlayerDurabilityRepairSaveLikeCpp {
                item_db_guid: 72,
                durability: 120,
            },
        ],
    };

    let statements = player_money_transaction_statements_like_cpp(&request);
    assert_eq!(statements.len(), 3);
    assert_eq!(statements[0].sql(), CharStatements::UPD_CHAR_MONEY.sql());
    assert_eq!(
        statements[0].params(),
        vec![SqlParam::U64(900), SqlParam::U64(42)]
    );
    for (statement, item_db_guid, durability) in
        [(&statements[1], 71, 80), (&statements[2], 72, 120)]
    {
        assert_eq!(
            statement.sql(),
            CharStatements::UPD_ITEM_INSTANCE_DURABILITY.sql()
        );
        assert_eq!(
            statement.params(),
            vec![SqlParam::U32(durability), SqlParam::U64(item_db_guid)]
        );
    }
    assert_eq!(
        request.logical_database(),
        LogicalDatabaseLikeCpp::Characters
    );
}

#[test]
fn bank_slot_purchase_preserves_checked_statement_and_bind_order_like_cpp() {
    let request = PlayerBankSlotPurchaseRequestLikeCpp {
        player_guid: 42,
        money_after: 12_345,
        bank_slot_count: 3,
    };
    let statement = player_bank_slot_purchase_statement_like_cpp(&request);
    assert_eq!(
        statement.sql(),
        "UPDATE characters SET money = ?, bankSlots = ? WHERE guid = ?"
    );
    assert_eq!(
        statement.params(),
        vec![SqlParam::U64(12_345), SqlParam::U8(3), SqlParam::U64(42)]
    );
}

#[test]
fn uncage_item_state_preserves_statement_identity_bind_order_and_projection() {
    let request = PlayerUncageItemStateRequestLikeCpp {
        player_guid: 42,
        item_guid: 71,
    };
    let statement = player_uncage_item_state_statement_like_cpp(request);
    assert_eq!(statement.sql(), CharStatements::SEL_UNCAGE_ITEM_STATE.sql());
    assert_eq!(
        statement.params(),
        vec![SqlParam::U64(71), SqlParam::U64(42), SqlParam::U64(71)]
    );
}

#[test]
fn standalone_durability_repair_preserves_statement_and_bind_order_like_cpp() {
    let repair = PlayerDurabilityRepairSaveLikeCpp {
        item_db_guid: 71,
        durability: 80,
    };
    let statement = player_durability_repair_statement_like_cpp(&repair);
    assert_eq!(
        statement.sql(),
        CharStatements::UPD_ITEM_INSTANCE_DURABILITY.sql()
    );
    assert_eq!(
        statement.params(),
        vec![SqlParam::U32(80), SqlParam::U64(71)]
    );
}

#[test]
fn checked_money_write_preserves_nontransactional_statement_shape_like_cpp() {
    let request = PlayerMoneyWriteRequestLikeCpp {
        player_guid: 99,
        money: 1234,
    };
    let statement = player_money_write_statement_like_cpp(&request);
    assert_eq!(statement.sql(), CharStatements::UPD_CHAR_MONEY.sql());
    assert_eq!(
        statement.params(),
        vec![SqlParam::U64(1234), SqlParam::U64(99)]
    );
    assert_eq!(
        request.logical_database(),
        LogicalDatabaseLikeCpp::Characters
    );
}

#[test]
fn currency_save_preserves_cpp_statement_identity_and_bind_order() {
    let request = PlayerCurrencySaveRequestLikeCpp {
        player_guid: 42,
        rows: vec![
            PlayerCurrencySaveRowLikeCpp {
                kind: PlayerCurrencySaveKindLikeCpp::New,
                currency_id: 395,
                quantity: 10,
                weekly_quantity: 11,
                tracked_quantity: 12,
                increased_cap_quantity: 13,
                earned_quantity: 14,
                flags: 15,
            },
            PlayerCurrencySaveRowLikeCpp {
                kind: PlayerCurrencySaveKindLikeCpp::Changed,
                currency_id: 396,
                quantity: 20,
                weekly_quantity: 21,
                tracked_quantity: 22,
                increased_cap_quantity: 23,
                earned_quantity: 24,
                flags: 25,
            },
        ],
    };

    let statements = player_currency_save_statements_like_cpp(&request);
    assert_eq!(statements.len(), 2);
    assert_eq!(
        statements[0].sql(),
        CharStatements::REP_PLAYER_CURRENCY.sql()
    );
    assert_eq!(
        statements[0].params(),
        vec![
            SqlParam::U64(42),
            SqlParam::U16(395),
            SqlParam::U32(10),
            SqlParam::U32(11),
            SqlParam::U32(12),
            SqlParam::U32(13),
            SqlParam::U32(14),
            SqlParam::U8(15),
        ]
    );
    assert_eq!(
        statements[1].sql(),
        CharStatements::UPD_PLAYER_CURRENCY.sql()
    );
    assert_eq!(
        statements[1].params(),
        vec![
            SqlParam::U32(20),
            SqlParam::U32(21),
            SqlParam::U32(22),
            SqlParam::U32(23),
            SqlParam::U32(24),
            SqlParam::U8(25),
            SqlParam::U64(42),
            SqlParam::U16(396),
        ]
    );
    assert_eq!(
        request.logical_database(),
        LogicalDatabaseLikeCpp::Characters
    );
}

#[test]
fn homebind_requests_map_to_cpp_statements_binds_and_live_narrowing() {
    let delete = player_homebind_persistence_statement_like_cpp(
        PlayerHomebindPersistenceRequestLikeCpp::DeleteInvalid { player_guid: 7 },
    );
    assert_eq!(delete.sql(), CharStatements::DEL_PLAYER_HOMEBIND.sql());
    assert_eq!(delete.params(), vec![SqlParam::U64(7)]);

    let insert = player_homebind_persistence_statement_like_cpp(
        PlayerHomebindPersistenceRequestLikeCpp::InsertRepaired {
            player_guid: 8,
            map_id: 530,
            area_id: 3430,
            x: 1.0,
            y: 2.0,
            z: 3.0,
            orientation: 4.0,
        },
    );
    assert_eq!(insert.sql(), CharStatements::INS_PLAYER_HOMEBIND.sql());
    assert_eq!(
        insert.params(),
        vec![
            SqlParam::U64(8),
            SqlParam::U16(530),
            SqlParam::U16(3430),
            SqlParam::F32(1.0),
            SqlParam::F32(2.0),
            SqlParam::F32(3.0),
            SqlParam::F32(4.0),
        ]
    );

    let update = player_homebind_persistence_statement_like_cpp(
        PlayerHomebindPersistenceRequestLikeCpp::UpdateLive {
            player_guid: 9,
            map_id: u32::from(u16::MAX) + 2,
            area_id: u32::from(u16::MAX) + 3,
            x: 5.0,
            y: 6.0,
            z: 7.0,
            orientation: 8.0,
        },
    );
    assert_eq!(update.sql(), CharStatements::UPD_PLAYER_HOMEBIND.sql());
    assert_eq!(
        update.params(),
        vec![
            SqlParam::U16(1),
            SqlParam::U16(2),
            SqlParam::F32(5.0),
            SqlParam::F32(6.0),
            SqlParam::F32(7.0),
            SqlParam::F32(8.0),
            SqlParam::U64(9),
        ]
    );
}

#[test]
fn login_item_repairs_expand_in_exact_cpp_statement_and_bind_order() {
    let statements =
        player_login_item_repair_statements_like_cpp(&PlayerLoginItemRepairRequestLikeCpp {
            actions: vec![
                PlayerLoginItemRepairActionLikeCpp::ClearRefundable {
                    item_guid: 11,
                    new_flags: 12,
                },
                PlayerLoginItemRepairActionLikeCpp::NormalizeOnLoad {
                    item_guid: 21,
                    expiration: 22,
                    flags: 23,
                    durability: 24,
                },
            ],
        });

    assert_eq!(statements.len(), 3);
    assert_eq!(
        statements[0].sql(),
        CharStatements::DEL_ITEM_REFUND_INSTANCE.sql()
    );
    assert_eq!(statements[0].params(), vec![SqlParam::U64(11)]);
    assert_eq!(
        statements[1].sql(),
        CharStatements::UPD_ITEM_INSTANCE_FLAGS.sql()
    );
    assert_eq!(
        statements[1].params(),
        vec![SqlParam::U32(12), SqlParam::U64(11)]
    );
    assert_eq!(
        statements[2].sql(),
        CharStatements::UPD_ITEM_INSTANCE_ON_LOAD.sql()
    );
    assert_eq!(
        statements[2].params(),
        vec![
            SqlParam::U32(22),
            SqlParam::U32(23),
            SqlParam::U32(24),
            SqlParam::U64(21),
        ]
    );
}

#[test]
fn remaining_login_writes_map_to_existing_statement_order_and_bind_domains() {
    let [delete_spells, reset_specializations] =
        player_login_pet_talent_reset_statements_like_cpp(77);
    assert_eq!(
        delete_spells.sql(),
        CharStatements::DEL_ALL_PET_SPELLS_BY_OWNER.sql()
    );
    assert_eq!(delete_spells.params(), vec![SqlParam::U64(77)]);
    assert_eq!(
        reset_specializations.sql(),
        CharStatements::UPD_PET_SPECS_BY_OWNER.sql()
    );
    assert_eq!(reset_specializations.params(), vec![SqlParam::U64(77)]);

    let online =
        player_online_mark_statement_like_cpp(PlayerOnlineMarkRequestLikeCpp { player_guid: 88 });
    assert_eq!(online.sql(), CharStatements::UPD_CHAR_ONLINE.sql());
    assert_eq!(online.params(), vec![SqlParam::U32(88)]);
}

#[test]
fn buyback_clear_maps_to_one_ordered_character_transaction_plan_like_cpp() {
    let _serialized = crate::persistence_trace::capture_flag_test_lock();
    let _capture = crate::persistence_trace::RecordingGuard::enable();
    let statements = player_buyback_clear_statements_like_cpp(&PlayerBuybackClearRequestLikeCpp {
        player_guid: 77,
        item_db_guids: vec![91, 92],
    });

    assert_eq!(
        statements
            .iter()
            .map(PreparedStatement::trace_identity)
            .collect::<Vec<_>>(),
        vec![
            Some("DEL_CHAR_INVENTORY_ITEM"),
            Some("DEL_ITEM_INSTANCE"),
            Some("DEL_CHAR_INVENTORY_ITEM"),
            Some("DEL_ITEM_INSTANCE"),
        ]
    );
    assert!(statements.iter().all(|statement| {
        statement.trace_database() == Some(crate::persistence_trace::LogicalDatabase::Character)
    }));
    assert_eq!(
        statements
            .iter()
            .map(PreparedStatement::params)
            .collect::<Vec<_>>(),
        vec![
            &[crate::SqlParam::U64(77), crate::SqlParam::U64(91)][..],
            &[crate::SqlParam::U64(91)][..],
            &[crate::SqlParam::U64(77), crate::SqlParam::U64(92)][..],
            &[crate::SqlParam::U64(92)][..],
        ]
    );
}

#[test]
fn talent_reset_maps_to_one_ordered_character_transaction_plan_like_cpp() {
    let _serialized = crate::persistence_trace::capture_flag_test_lock();
    let _capture = crate::persistence_trace::RecordingGuard::enable();
    let statements =
        player_talent_reset_statements_like_cpp(&PlayerTalentResetPersistenceRequestLikeCpp {
            player_guid: 77,
            money_before: 1_000,
            money_after: 900,
            reset_cost: 100,
            reset_time_secs: 1234,
            retained_talents: vec![
                PlayerTalentResetSaveRowLikeCpp {
                    talent_id: 11,
                    rank: 2,
                    talent_group: 0,
                },
                PlayerTalentResetSaveRowLikeCpp {
                    talent_id: 22,
                    rank: 3,
                    talent_group: 1,
                },
            ],
        });

    assert_eq!(
        statements
            .iter()
            .map(PreparedStatement::trace_identity)
            .collect::<Vec<_>>(),
        vec![
            Some("UPD_CHAR_MONEY"),
            Some("UPD_CHAR_TALENT_RESET_STATE"),
            Some("DEL_CHAR_TALENT"),
            Some("INS_CHAR_TALENT"),
            Some("INS_CHAR_TALENT"),
        ]
    );
    assert_eq!(
        statements
            .iter()
            .map(PreparedStatement::params)
            .collect::<Vec<_>>(),
        vec![
            &[crate::SqlParam::U64(900), crate::SqlParam::U64(77)][..],
            &[
                crate::SqlParam::U32(100),
                crate::SqlParam::U64(1234),
                crate::SqlParam::U64(77),
            ][..],
            &[crate::SqlParam::U64(77)][..],
            &[
                crate::SqlParam::U64(77),
                crate::SqlParam::U32(11),
                crate::SqlParam::U8(2),
                crate::SqlParam::U8(0),
            ][..],
            &[
                crate::SqlParam::U64(77),
                crate::SqlParam::U32(22),
                crate::SqlParam::U8(3),
                crate::SqlParam::U8(1),
            ][..],
        ]
    );
}

#[test]
fn talent_reset_unknown_commit_requires_exact_changed_money_evidence_like_cpp() {
    use PlayerTalentResetCommitReconciliationLikeCpp::{Applied, Failed, Unknown};

    assert_eq!(
        reconcile_player_talent_reset_commit_like_cpp(1_000, 900, Some(900)),
        Applied
    );
    assert_eq!(
        reconcile_player_talent_reset_commit_like_cpp(1_000, 900, Some(1_000)),
        Failed
    );
    assert_eq!(
        reconcile_player_talent_reset_commit_like_cpp(1_000, 900, Some(950)),
        Unknown
    );
    assert_eq!(
        reconcile_player_talent_reset_commit_like_cpp(1_000, 900, None),
        Unknown
    );
    assert_eq!(
        reconcile_player_talent_reset_commit_like_cpp(1_000, 1_000, Some(1_000)),
        Unknown
    );
}

#[test]
fn xp_rest_request_maps_to_exact_order_and_binds_like_cpp() {
    let _serialized = crate::persistence_trace::capture_flag_test_lock();
    let _capture = crate::persistence_trace::RecordingGuard::enable();
    let statements =
        player_xp_persistence_statements_like_cpp(&PlayerXpPersistenceRequestLikeCpp {
            player_guid: 77,
            level_changed: true,
            level: 12,
            xp: 345,
            rest: Some(wow_persistence::PlayerXpRestStateSaveLikeCpp {
                rest_state: 1,
                player_flags: 0x20,
                rest_bonus: 42.5,
            }),
        });

    assert_eq!(
        statements
            .iter()
            .map(PreparedStatement::trace_identity)
            .collect::<Vec<_>>(),
        vec![Some("UPD_CHAR_LEVEL"), Some("UPD_CHAR_ONLINE_REST_STATE")]
    );
    assert_eq!(
        statements
            .iter()
            .map(PreparedStatement::params)
            .collect::<Vec<_>>(),
        vec![
            &[
                crate::SqlParam::U8(12),
                crate::SqlParam::U32(345),
                crate::SqlParam::U64(77),
            ][..],
            &[
                crate::SqlParam::U8(1),
                crate::SqlParam::U32(0x20),
                crate::SqlParam::F32(42.5),
                crate::SqlParam::U64(77),
            ][..],
        ]
    );

    let xp_only = player_xp_persistence_statements_like_cpp(&PlayerXpPersistenceRequestLikeCpp {
        player_guid: 88,
        level_changed: false,
        level: 12,
        xp: 456,
        rest: None,
    });
    assert_eq!(xp_only.len(), 1);
    assert_eq!(xp_only[0].trace_identity(), Some("UPD_CHAR_XP"));
    assert_eq!(
        xp_only[0].params(),
        &[crate::SqlParam::U32(456), crate::SqlParam::U64(88)]
    );
}

#[test]
fn realm_character_count_refresh_maps_to_characters_then_login_like_cpp() {
    let _serialized = crate::persistence_trace::capture_flag_test_lock();
    let _capture = crate::persistence_trace::RecordingGuard::enable();
    let (count, replace) = player_realm_character_count_statements_like_cpp(
        PlayerRealmCharacterCountRefreshRequestLikeCpp {
            account_id: 77,
            realm_id: 12,
        },
        3,
    );

    assert_eq!(count.trace_identity(), Some("SEL_SUM_CHARS"));
    assert_eq!(replace.trace_identity(), Some("REP_REALM_CHARACTERS"));
    assert_eq!(
        count.trace_database(),
        Some(crate::persistence_trace::LogicalDatabase::Character)
    );
    assert_eq!(
        replace.trace_database(),
        Some(crate::persistence_trace::LogicalDatabase::Login)
    );
    assert_eq!(count.params(), &[crate::SqlParam::U32(77)]);
    assert_eq!(
        replace.params(),
        &[
            crate::SqlParam::U8(3),
            crate::SqlParam::U32(77),
            crate::SqlParam::U32(12),
        ]
    );
}

#[test]
fn login_transport_requests_map_to_world_statements_and_bound_guid_like_cpp() {
    let _serialized = crate::persistence_trace::capture_flag_test_lock();
    let _capture = crate::persistence_trace::RecordingGuard::enable();

    let all =
        player_login_transport_load_statement_like_cpp(PlayerLoginTransportLoadRequestLikeCpp::All);
    let one = player_login_transport_load_statement_like_cpp(
        PlayerLoginTransportLoadRequestLikeCpp::ByGuid { guid_low: 77 },
    );

    assert_eq!(all.trace_identity(), Some("SEL_LOGIN_TRANSPORTS"));
    assert_eq!(one.trace_identity(), Some("SEL_LOGIN_TRANSPORT_BY_GUID"));
    assert_eq!(
        all.trace_database(),
        Some(crate::persistence_trace::LogicalDatabase::World)
    );
    assert_eq!(
        one.trace_database(),
        Some(crate::persistence_trace::LogicalDatabase::World)
    );
    assert!(all.params().is_empty());
    assert_eq!(one.params(), &[crate::SqlParam::U64(77)]);
}

#[test]
fn trait_load_requests_keep_statement_identity_during_persistence_capture_like_cpp() {
    let _serialized = crate::persistence_trace::capture_flag_test_lock();
    let _capture = crate::persistence_trace::RecordingGuard::enable();

    let entries = player_login_auxiliary_load_statement_like_cpp(
        PlayerLoginAuxiliaryLoadRequestLikeCpp::TraitEntries { player_guid: 77 },
    );
    let configs = player_login_auxiliary_load_statement_like_cpp(
        PlayerLoginAuxiliaryLoadRequestLikeCpp::TraitConfigs { player_guid: 77 },
    );

    assert_eq!(entries.trace_identity(), Some("SEL_CHAR_TRAIT_ENTRIES"));
    assert_eq!(
        entries.trace_database(),
        Some(crate::persistence_trace::LogicalDatabase::Character)
    );
    assert_eq!(configs.trace_identity(), Some("SEL_CHAR_TRAIT_CONFIGS"));
    assert_eq!(
        configs.trace_database(),
        Some(crate::persistence_trace::LogicalDatabase::Character)
    );
}
