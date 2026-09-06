//! Crate-root public contract regression suite; all original registrations retained.
use super::*;

fn battle_pet_purchase_command_like_cpp(request_key: [u8; 16]) -> BattlePetPurchaseCommandLikeCpp {
    BattlePetPurchaseCommandLikeCpp {
        request_key,
        character_guid: 7,
        account_id: 11,
        trainer_id: 13,
        spell_id: 17,
        species: 19,
        breed: 23,
        quality: 2,
        display_id: 29,
        level: 1,
        price: 31,
        money_before: 100,
        money_after: 69,
        status: BattlePetPurchaseStatusLikeCpp::PendingApplication,
        published: false,
        failure_reason: None,
    }
}

#[test]
fn battle_pet_purchase_status_codes_and_terminal_set_are_stable_like_cpp() {
    for (code, status, terminal) in [
        (0, BattlePetPurchaseStatusLikeCpp::PendingApplication, false),
        (1, BattlePetPurchaseStatusLikeCpp::Completed, true),
        (
            2,
            BattlePetPurchaseStatusLikeCpp::CompensationPending,
            false,
        ),
        (3, BattlePetPurchaseStatusLikeCpp::Compensated, true),
        (4, BattlePetPurchaseStatusLikeCpp::TerminalFailure, true),
    ] {
        assert_eq!(status.as_u8_like_cpp(), code);
        assert_eq!(
            BattlePetPurchaseStatusLikeCpp::from_u8_like_cpp(code),
            Some(status)
        );
        assert_eq!(status.is_terminal_like_cpp(), terminal);
    }
    assert_eq!(BattlePetPurchaseStatusLikeCpp::from_u8_like_cpp(5), None);
}

#[test]
fn battle_pet_purchase_charge_reconciliation_requires_the_complete_identity() {
    let expected = battle_pet_purchase_command_like_cpp([1; 16]);
    assert_eq!(
        reconcile_battle_pet_purchase_charge_like_cpp(Some(&expected), &expected),
        BattlePetPurchaseChargeOutcomeLikeCpp::Charged
    );
    let mut collision = expected.clone();
    collision.species += 1;
    assert_eq!(
        reconcile_battle_pet_purchase_charge_like_cpp(Some(&collision), &expected),
        BattlePetPurchaseChargeOutcomeLikeCpp::RolledBack
    );
    assert_eq!(
        reconcile_battle_pet_purchase_charge_like_cpp(None, &expected),
        BattlePetPurchaseChargeOutcomeLikeCpp::RolledBack
    );
}

#[test]
fn battle_pet_purchase_mark_reconciliation_distinguishes_retry_and_conflict() {
    let pending = battle_pet_purchase_command_like_cpp([2; 16]);
    assert!(matches!(
        reconcile_battle_pet_purchase_mark_like_cpp(
            Some(&pending),
            BattlePetPurchaseStatusLikeCpp::PendingApplication,
            BattlePetPurchaseStatusLikeCpp::Completed,
        ),
        Err(BattlePetPurchaseStoreErrorLikeCpp::Retryable(_))
    ));
    let mut completed = pending.clone();
    completed.status = BattlePetPurchaseStatusLikeCpp::Completed;
    assert_eq!(
        reconcile_battle_pet_purchase_mark_like_cpp(
            Some(&completed),
            BattlePetPurchaseStatusLikeCpp::PendingApplication,
            BattlePetPurchaseStatusLikeCpp::Completed,
        ),
        Ok(BattlePetPurchaseMarkOutcomeLikeCpp::AlreadyApplied)
    );
    let mut compensated = pending;
    compensated.status = BattlePetPurchaseStatusLikeCpp::Compensated;
    assert_eq!(
        reconcile_battle_pet_purchase_mark_like_cpp(
            Some(&compensated),
            BattlePetPurchaseStatusLikeCpp::PendingApplication,
            BattlePetPurchaseStatusLikeCpp::Completed,
        ),
        Ok(BattlePetPurchaseMarkOutcomeLikeCpp::ConflictedCompensated)
    );
}

#[test]
fn each_offline_mark_names_its_logical_database_like_cpp() {
    assert_eq!(
        PlayerOfflineMarkLikeCpp::Character { guid_low: 1 }.logical_database(),
        LogicalDatabaseLikeCpp::Characters
    );
    assert_eq!(
        PlayerOfflineMarkLikeCpp::CharacterAccount { account_id: 1 }.logical_database(),
        LogicalDatabaseLikeCpp::Characters
    );
    assert_eq!(
        PlayerOfflineMarkLikeCpp::LoginAccount { account_id: 1 }.logical_database(),
        LogicalDatabaseLikeCpp::Login
    );
}

#[test]
fn every_account_collection_load_names_the_login_database_like_cpp() {
    for request in [
        AccountCollectionLoadRequestLikeCpp::Mounts { bnet_account_id: 1 },
        AccountCollectionLoadRequestLikeCpp::Toys { bnet_account_id: 1 },
        AccountCollectionLoadRequestLikeCpp::Heirlooms { bnet_account_id: 1 },
        AccountCollectionLoadRequestLikeCpp::ItemAppearances { bnet_account_id: 1 },
        AccountCollectionLoadRequestLikeCpp::TransmogIllusions { bnet_account_id: 1 },
    ] {
        assert_eq!(request.logical_database(), LogicalDatabaseLikeCpp::Login);
    }
}

#[test]
fn buyback_clear_names_the_character_database_like_cpp() {
    assert_eq!(
        PlayerBuybackClearRequestLikeCpp {
            player_guid: 1,
            item_db_guids: vec![2],
        }
        .logical_database(),
        LogicalDatabaseLikeCpp::Characters
    );
}

#[test]
fn realm_character_count_refresh_names_both_independent_databases_like_cpp() {
    assert_eq!(
        PlayerRealmCharacterCountRefreshRequestLikeCpp {
            account_id: 1,
            realm_id: 2,
        }
        .logical_databases(),
        [
            LogicalDatabaseLikeCpp::Characters,
            LogicalDatabaseLikeCpp::Login,
        ]
    );
}

#[test]
fn login_transport_load_names_the_world_database_like_cpp() {
    for request in [
        PlayerLoginTransportLoadRequestLikeCpp::All,
        PlayerLoginTransportLoadRequestLikeCpp::ByGuid { guid_low: 7 },
    ] {
        assert_eq!(request.logical_database(), LogicalDatabaseLikeCpp::World);
    }
}

#[test]
fn talent_reset_persistence_names_the_characters_database_like_cpp() {
    let request = PlayerTalentResetPersistenceRequestLikeCpp {
        player_guid: 7,
        money_before: 10,
        money_after: 5,
        reset_cost: 5,
        reset_time_secs: 123,
        retained_talents: Vec::new(),
    };
    assert_eq!(
        request.logical_database(),
        LogicalDatabaseLikeCpp::Characters
    );
}

#[test]
fn xp_persistence_names_the_characters_database_like_cpp() {
    assert_eq!(
        PlayerXpPersistenceRequestLikeCpp {
            player_guid: 7,
            level_changed: false,
            level: 10,
            xp: 42,
            rest: None,
        }
        .logical_database(),
        LogicalDatabaseLikeCpp::Characters
    );
}

#[test]
fn player_online_mark_names_the_characters_database_like_cpp() {
    assert_eq!(
        PlayerOnlineMarkRequestLikeCpp { player_guid: 1 }.logical_database(),
        LogicalDatabaseLikeCpp::Characters
    );
}

#[test]
fn packet_spoof_ban_write_names_the_login_database_like_cpp() {
    assert_eq!(
        PacketSpoofBanWriteRequestLikeCpp {
            target: PacketSpoofBanTargetLikeCpp::Account { account_id: 1 },
            duration_secs: 60,
            author: "author".to_string(),
            reason: "reason".to_string(),
        }
        .logical_database(),
        LogicalDatabaseLikeCpp::Login
    );
}

#[test]
fn void_storage_writes_name_the_characters_database_like_cpp() {
    assert_eq!(
        VoidStorageUnlockWriteRequestLikeCpp {
            player_guid: 1,
            money_before: 2,
            money_after: 1,
            player_flags_after: 4,
        }
        .logical_database(),
        LogicalDatabaseLikeCpp::Characters
    );
    assert_eq!(
        VoidStorageSwapWriteRequestLikeCpp {
            player_guid: 1,
            money_before: 2,
            money_after: 2,
            old_slot: 0,
            new_slot: 1,
            source_item: VoidStorageItemWriteLikeCpp {
                item_id: 3,
                item_entry: 4,
                creator_guid: 5,
                fixed_scaling_level: 6,
                random_properties_id: -7,
                random_properties_seed: 8,
                context: 9,
            },
            destination_item: None,
        }
        .logical_database(),
        LogicalDatabaseLikeCpp::Characters
    );
}

#[test]
fn character_base_load_names_the_characters_database_like_cpp() {
    assert_eq!(
        PlayerCharacterBaseLoadRequestLikeCpp { player_guid: 7 }.logical_database(),
        LogicalDatabaseLikeCpp::Characters
    );
}

#[test]
fn every_player_login_auxiliary_load_names_the_characters_database_like_cpp() {
    for request in [
        PlayerLoginAuxiliaryLoadRequestLikeCpp::Customizations { player_guid: 1 },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::CompletedAchievements { player_guid: 1 },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::InstanceTimeRestrictions { account_id: 2 },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellCooldowns { player_guid: 1 },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellCharges { player_guid: 1 },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::TraitEntries { player_guid: 1 },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::TraitConfigs { player_guid: 1 },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::PetStable { player_guid: 1 },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::PetAuras { pet_number: 2 },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::PetAuraEffects { pet_number: 2 },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::PetSpells { pet_number: 2 },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::PetSpellCooldowns { pet_number: 2 },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::PetSpellCharges { pet_number: 2 },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::PetDeclinedNames {
            player_guid: 1,
            pet_number: 2,
        },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::GroupMembership { player_guid: 1 },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::EquipmentSets { player_guid: 1 },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::TransmogOutfits { player_guid: 1 },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::CufProfiles { player_guid: 1 },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::Currencies { player_guid: 1 },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::Spells { player_guid: 1 },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellFavorites { player_guid: 1 },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::Skills { player_guid: 1 },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::Talents { player_guid: 1 },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::Glyphs { player_guid: 1 },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::ActionButtons {
            player_guid: 1,
            active_spec: 0,
            trait_config_id: 0,
        },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::Reputation { player_guid: 1 },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::CharacterAuras { player_guid: 1 },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::CharacterAuraEffects { player_guid: 1 },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::EquipmentInventory { player_guid: 1 },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::BagInventory { player_guid: 1 },
        PlayerLoginAuxiliaryLoadRequestLikeCpp::VoidStorage { player_guid: 1 },
    ] {
        assert_eq!(
            request.logical_database(),
            LogicalDatabaseLikeCpp::Characters
        );
    }
}

#[test]
fn pet_login_rows_keep_success_empty_and_failure_distinct_like_cpp() {
    let loaded = PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
        PlayerLoginAuxiliaryLoadedLikeCpp::PetSpells(vec![PlayerPetSpellLoadRowLikeCpp {
            spell_id: 17253,
            active: 1,
        }]),
    );
    let empty = PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
        PlayerLoginAuxiliaryLoadedLikeCpp::PetSpells(Vec::new()),
    );
    let failed = PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed {
        reason: "pet query failed".to_owned(),
    };

    assert_ne!(loaded, empty);
    assert_ne!(empty, failed);
    assert_ne!(loaded, failed);
}

#[test]
fn group_login_rows_keep_loaded_empty_and_failure_distinct_like_cpp() {
    let loaded = PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
        PlayerLoginAuxiliaryLoadedLikeCpp::GroupMembership(vec![77]),
    );
    let empty = PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
        PlayerLoginAuxiliaryLoadedLikeCpp::GroupMembership(Vec::new()),
    );
    let failed = PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed {
        reason: "group query failed".to_owned(),
    };

    assert_ne!(loaded, empty);
    assert_ne!(empty, failed);
    assert_ne!(loaded, failed);
}

#[test]
fn profile_login_rows_keep_loaded_empty_and_failure_distinct_like_cpp() {
    let loaded = PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
        PlayerLoginAuxiliaryLoadedLikeCpp::Currencies(vec![PlayerCurrencyLoadRowLikeCpp {
            currency_id: 1,
            quantity: 2,
            weekly_quantity: 3,
            tracked_quantity: 4,
            increased_cap_quantity: 5,
            earned_quantity: 6,
            flags: 7,
        }]),
    );
    let empty = PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
        PlayerLoginAuxiliaryLoadedLikeCpp::Currencies(Vec::new()),
    );
    let failed = PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed {
        reason: "profile query failed".to_owned(),
    };

    assert_ne!(loaded, empty);
    assert_ne!(empty, failed);
    assert_ne!(loaded, failed);
}

#[test]
fn progression_login_rows_keep_loaded_empty_and_failure_distinct_like_cpp() {
    let loaded = PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
        PlayerLoginAuxiliaryLoadedLikeCpp::Spells(vec![PlayerSpellLoadRowLikeCpp {
            spell_id: 133,
            active: 1,
            disabled: 0,
        }]),
    );
    let empty = PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
        PlayerLoginAuxiliaryLoadedLikeCpp::Spells(Vec::new()),
    );
    let failed = PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed {
        reason: "progression query failed".to_owned(),
    };

    assert_ne!(loaded, empty);
    assert_ne!(empty, failed);
    assert_ne!(loaded, failed);
}

#[test]
fn character_aura_rows_keep_loaded_empty_and_failure_distinct_like_cpp() {
    let loaded = PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
        PlayerLoginAuxiliaryLoadedLikeCpp::CharacterAuras(vec![
            PlayerCharacterAuraLoadRowLikeCpp {
                caster_guid_binary: vec![1],
                spell_id: 133,
                effect_mask: 1,
                recalculate_mask: 0,
                difficulty: 0,
                stack_count: 1,
                max_duration_ms: 10_000,
                remain_time_ms: 5_000,
                remain_charges: 0,
            },
        ]),
    );
    let empty = PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
        PlayerLoginAuxiliaryLoadedLikeCpp::CharacterAuras(Vec::new()),
    );
    let failed = PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed {
        reason: "aura query failed".to_owned(),
    };

    assert_ne!(loaded, empty);
    assert_ne!(empty, failed);
    assert_ne!(loaded, failed);
}

#[test]
fn inventory_login_rows_keep_loaded_empty_and_failure_distinct_like_cpp() {
    let loaded = PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
        PlayerLoginAuxiliaryLoadedLikeCpp::VoidStorage(vec![PlayerVoidStorageLoadRowLikeCpp {
            item_id: 1,
            item_entry: 2,
            slot: 3,
            creator_guid: 4,
            fixed_scaling_level: 5,
            random_properties_id: 6,
            random_properties_seed: 7,
            context: 8,
        }]),
    );
    let empty = PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
        PlayerLoginAuxiliaryLoadedLikeCpp::VoidStorage(Vec::new()),
    );
    let failed = PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed {
        reason: "inventory query failed".to_owned(),
    };

    assert_ne!(loaded, empty);
    assert_ne!(empty, failed);
    assert_ne!(loaded, failed);
}

#[test]
fn every_player_login_admission_load_names_the_characters_database_like_cpp() {
    for request in [
        PlayerLoginAdmissionLoadRequestLikeCpp::BattlegroundLocation { player_guid: 1 },
        PlayerLoginAdmissionLoadRequestLikeCpp::HomebindLocation { player_guid: 1 },
        PlayerLoginAdmissionLoadRequestLikeCpp::GuildMembership { player_guid: 1 },
    ] {
        assert_eq!(
            request.logical_database(),
            LogicalDatabaseLikeCpp::Characters
        );
    }
}

#[test]
fn map_corpse_hydration_names_the_characters_database_like_cpp() {
    assert_eq!(
        MapCorpseLoadRequestLikeCpp {
            map_id: 571,
            instance_id: 9,
        }
        .logical_database(),
        LogicalDatabaseLikeCpp::Characters
    );
}

#[test]
fn every_session_account_data_scope_names_the_characters_database_like_cpp() {
    for scope in [
        SessionAccountDataScopeLikeCpp::Global { account_id: 1 },
        SessionAccountDataScopeLikeCpp::Character { guid_low: 2 },
    ] {
        assert_eq!(scope.logical_database(), LogicalDatabaseLikeCpp::Characters);
    }
}

#[test]
fn an_unknown_outcome_is_neither_applied_nor_a_plain_failure_like_cpp() {
    let unknown = PersistenceOutcomeLikeCpp::Unknown {
        reason: "connection lost after COMMIT was sent".to_owned(),
    };
    assert!(!unknown.is_applied());
    assert!(unknown.is_indeterminate());

    let failed = PersistenceOutcomeLikeCpp::Failed {
        reason: "constraint violation".to_owned(),
    };
    assert!(!failed.is_applied());
    assert!(
        !failed.is_indeterminate(),
        "a definite rollback must not fence"
    );

    assert!(PersistenceOutcomeLikeCpp::Applied { rows: 1 }.is_applied());
}

#[test]
fn stored_item_money_reconciliation_requires_joint_money_and_source_evidence_like_cpp() {
    let outcome = StoredItemMoneyPersistenceOutcomeLikeCpp {
        before: 100,
        after: 107,
        applied_delta: 7,
        notified_amount: 7,
    };
    assert_eq!(
        classify_stored_item_money_reconciliation_like_cpp(outcome, 100, Some(7)),
        StoredItemMoneyReconciliationLikeCpp::RolledBack
    );
    assert_eq!(
        classify_stored_item_money_reconciliation_like_cpp(outcome, 107, None),
        StoredItemMoneyReconciliationLikeCpp::Committed
    );
    assert_eq!(
        classify_stored_item_money_reconciliation_like_cpp(outcome, 100, None),
        StoredItemMoneyReconciliationLikeCpp::Indeterminate { reason: None }
    );
}

#[test]
fn zero_cached_stored_item_money_is_the_only_absent_source_noop_like_cpp() {
    assert_eq!(
        stored_item_money_zero_without_source_outcome_like_cpp(41, 0),
        Some(StoredItemMoneyPersistenceOutcomeLikeCpp {
            before: 41,
            after: 41,
            applied_delta: 0,
            notified_amount: 0,
        })
    );
    assert!(stored_item_money_zero_without_source_outcome_like_cpp(41, 1).is_none());
}

#[test]
fn group_loot_money_reconciliation_requires_one_coherent_side_like_cpp() {
    let outcomes = vec![
        GroupLootMoneyPersistenceOutcomeLikeCpp {
            recipient_guid: 1,
            before: 10,
            after: 15,
            applied_delta: 5,
        },
        GroupLootMoneyPersistenceOutcomeLikeCpp {
            recipient_guid: 2,
            before: 20,
            after: 27,
            applied_delta: 7,
        },
    ];
    assert_eq!(
        classify_group_loot_money_reconciliation_like_cpp(
            &outcomes,
            &[(1, Some(10)), (2, Some(20))]
        ),
        GroupLootMoneyReconciliationLikeCpp::RolledBack
    );
    assert_eq!(
        classify_group_loot_money_reconciliation_like_cpp(
            &outcomes,
            &[(1, Some(15)), (2, Some(27))]
        ),
        GroupLootMoneyReconciliationLikeCpp::CommittedOrCapOnlyNoop
    );
    assert_eq!(
        classify_group_loot_money_reconciliation_like_cpp(
            &outcomes,
            &[(1, Some(10)), (2, Some(27))]
        ),
        GroupLootMoneyReconciliationLikeCpp::Indeterminate { reason: None }
    );
}

#[test]
fn group_loot_money_cap_only_noop_needs_no_commit_evidence_like_cpp() {
    let outcomes = [GroupLootMoneyPersistenceOutcomeLikeCpp {
        recipient_guid: 1,
        before: 100,
        after: 100,
        applied_delta: 0,
    }];
    assert_eq!(
        classify_group_loot_money_reconciliation_like_cpp(&outcomes, &[]),
        GroupLootMoneyReconciliationLikeCpp::CommittedOrCapOnlyNoop
    );
}

#[test]
fn standalone_durability_repair_names_the_character_database_like_cpp() {
    assert_eq!(
        PlayerDurabilityRepairSaveLikeCpp {
            item_db_guid: 7,
            durability: 80,
        }
        .logical_database(),
        LogicalDatabaseLikeCpp::Characters
    );
}

#[test]
fn support_bug_report_names_the_character_database_like_cpp() {
    assert_eq!(
        SupportBugReportWriteRequestLikeCpp {
            text: "bug".to_owned(),
            diagnostic_info: "diag".to_owned(),
        }
        .logical_database(),
        LogicalDatabaseLikeCpp::Characters
    );
}

#[test]
fn bank_slot_purchase_names_the_character_database_like_cpp() {
    assert_eq!(
        PlayerBankSlotPurchaseRequestLikeCpp {
            player_guid: 17,
            money_after: 900,
            bank_slot_count: 3,
        }
        .logical_database(),
        LogicalDatabaseLikeCpp::Characters
    );
}

#[test]
fn uncage_item_state_read_names_characters_and_keeps_absence_distinct_from_failure() {
    let request = PlayerUncageItemStateRequestLikeCpp {
        player_guid: 17,
        item_guid: 91,
    };
    assert_eq!(
        request.logical_database(),
        LogicalDatabaseLikeCpp::Characters
    );
    assert_ne!(
        PlayerUncageItemStateLoadOutcomeLikeCpp::Loaded(PlayerUncageItemStateLikeCpp {
            owner_guid: None,
            inventory_linked: false,
        }),
        PlayerUncageItemStateLoadOutcomeLikeCpp::Failed {
            reason: "read failed".to_owned(),
        }
    );
}

#[test]
fn represented_group_request_names_characters_and_failure_keeps_applied_prefix() {
    let request = RepresentedGroupPersistenceRequestLikeCpp {
        commands: vec![RepresentedGroupPersistenceCommandLikeCpp::DeleteMember { member_guid: 17 }],
        mode: RepresentedGroupPersistenceModeLikeCpp::Sequential,
    };
    assert_eq!(
        request.logical_database(),
        LogicalDatabaseLikeCpp::Characters
    );
    assert_ne!(
        RepresentedGroupPersistenceOutcomeLikeCpp::FailedAfterPrefix {
            applied: 1,
            reason: "second command failed".to_owned(),
        },
        RepresentedGroupPersistenceOutcomeLikeCpp::DefinitelyRolledBack {
            reason: "transaction failed".to_owned(),
        }
    );
}
