//! Login scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn continue_login_no_longer_names_the_core_character_statement() {
    let handler = include_str!("../character/world_entry/login.rs");
    assert!(handler.contains("load_character_base_like_cpp"));
    assert!(handler.contains("PlayerCharacterBaseLoadOutcomeLikeCpp::Loaded(Some(row))"));
    assert!(handler.contains("PlayerCharacterBaseLoadOutcomeLikeCpp::Loaded(None)"));
    assert!(handler.contains("PlayerCharacterBaseLoadOutcomeLikeCpp::Failed { reason }"));
    assert!(!handler.contains("prepare(CharStatements::SEL_CHARACTER)"));
}

#[test]
fn persisted_transport_restore_stays_between_identity_and_reputation_loading_like_cpp() {
    let login = include_str!("../character/world_entry/login.rs");
    let transport_restore = include_str!("../character/world_entry/login/transport_restore.rs");
    let identity_offset = login
        .rfind("set_loaded_player_identity_like_cpp(map_id as u16, race, class, level, gender);")
        .expect("login stores the loaded identity before restoring persisted transport");
    let restore_offset = login
        .find("restore_persisted_transport_for_login_like_cpp(")
        .expect("login runs its transport-restore phase");
    let reputation_offset = login
        .find("load_character_reputation_for_login_like_cpp")
        .expect("login loads reputation after transport restoration");

    assert!(identity_offset < restore_offset);
    assert!(restore_offset < reputation_offset);
    for marker in [
        "saved_transport_guid_low != 0 && !saved_character_map_is_battleground",
        ".resolve_persisted_transport_login_like_cpp(",
        "set_player_transport_guid_like_cpp(Some(transport.guid))",
        "set_player_transport_guid_like_cpp(None)",
        "seed_login_location_zone_area_like_cpp(zone, login_homebind)",
    ] {
        assert!(
            transport_restore.contains(marker),
            "transport restore phase lost `{marker}`"
        );
    }
}

#[test]
fn reputation_login_phase_returns_completion_for_first_login_reputation() {
    let login = include_str!("../character/world_entry/login.rs");
    let reputation_loading = include_str!("../character/world_entry/login/reputation_loading.rs");
    let restore_offset = login
        .find("restore_persisted_transport_for_login_like_cpp(")
        .expect("transport restoration precedes reputation loading");
    let reputation_offset = login
        .find("let reputation_rows_complete_like_cpp = self")
        .expect("login keeps the reputation completion result");
    let health_offset = login
        .find("let saved_health = base_row.health;")
        .expect("saved health follows reputation loading");
    let consumer_offset = login
        .find("&& reputation_rows_complete_like_cpp")
        .expect("later reputation phase consumes the completion result");
    assert!(restore_offset < reputation_offset);
    assert!(reputation_offset < health_offset);
    assert!(health_offset < consumer_offset);
    assert!(!login.contains("PlayerLoginAuxiliaryLoadRequestLikeCpp::Reputation"));
    assert!(!login.contains("let mut reputation_rows_complete_like_cpp"));

    for marker in [
        "PlayerLoginAuxiliaryLoadRequestLikeCpp::Reputation",
        "CharacterReputationRowLikeCpp",
        "load_character_reputation_rows_like_cpp(rows)",
        "rows_complete = true;",
        "missing Faction.db2 store",
        "PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason }",
        "rows_complete\n    }",
    ] {
        assert!(
            reputation_loading.contains(marker),
            "reputation phase lost `{marker}`"
        );
    }
}

#[test]
fn talent_login_phase_keeps_spell_side_effects_and_completion_result() {
    let login = include_str!("../character/world_entry/login.rs");
    let talent_loading = include_str!("../character/world_entry/login/talent_loading.rs");
    let skills_offset = login
        .find("// ── C++ Player::_LoadSkills ──")
        .expect("skill loading precedes talent hydration");
    let talent_offset = login
        .find("let talent_rows_complete_like_cpp = self")
        .expect("login keeps the talent completion result");
    let custom_spells_offset = login
        .find("apply_represented_start_all_spells_with_catalogs_like_cpp")
        .expect("custom spells follow talent hydration");
    let consumer_offset = login
        .find("&& talent_rows_complete_like_cpp")
        .expect("complete-spell-rows gate consumes the talent result");
    assert!(skills_offset < talent_offset);
    assert!(talent_offset < custom_spells_offset);
    assert!(custom_spells_offset < consumer_offset);
    assert!(!login.contains("PlayerLoginAuxiliaryLoadRequestLikeCpp::Talents"));
    assert!(!login.contains("let mut talent_rows_complete_like_cpp"));

    for marker in [
        "reset_represented_talents_like_cpp",
        "PlayerLoginAuxiliaryLoadRequestLikeCpp::Talents",
        "load_represented_talent_row_with_spell_side_effects_like_cpp",
        "player_bootstrap.talent_tabs.as_ref()",
        "known_spells,",
        "skill_rewarded_dependent_spells,",
        "mark_represented_talents_loaded_like_cpp",
        "rows_complete = true;",
        "PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason }",
        "rows_complete\n    }",
    ] {
        assert!(
            talent_loading.contains(marker),
            "talent phase lost `{marker}`"
        );
    }
}

#[test]
fn skill_row_login_phase_returns_rows_for_coordinator_normalization() {
    let login = include_str!("../character/world_entry/login.rs");
    let skill_loading = include_str!("../character/world_entry/login/skill_loading.rs");
    let favorites_offset = login
        .find("load_character_favorite_spells_for_login_like_cpp")
        .expect("favorite spells load before skill rows");
    let skill_offset = login
        .find("let (mut skill_records, loaded_skill_records_like_cpp) = self")
        .expect("login keeps the skill rows and completion result");
    let normalize_offset = login
        .find("loaded_skill_info_like_cpp(")
        .expect("coordinator normalizes persisted skill rows");
    let replace_offset = login
        .find("if loaded_skill_records_like_cpp")
        .expect("canonical skill replacement is gated on completion");
    assert!(favorites_offset < skill_offset);
    assert!(skill_offset < normalize_offset);
    assert!(normalize_offset < replace_offset);
    assert!(!login.contains("PlayerLoginAuxiliaryLoadRequestLikeCpp::Skills"));
    assert!(!login.contains("let mut loaded_skill_records_like_cpp"));

    for marker in [
        "PlayerLoginAuxiliaryLoadRequestLikeCpp::Skills",
        "loaded = true;",
        "row.skill_id > 0",
        "RepresentedPlayerSkillStateLikeCpp::Unchanged",
        "profession_slot: row.profession_slot",
        "PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason }",
        "(skill_records, loaded)",
    ] {
        assert!(
            skill_loading.contains(marker),
            "skill-row phase lost `{marker}`"
        );
    }
}

#[test]
fn spell_login_phase_returns_projected_rows_and_favorites() {
    let login = include_str!("../character/world_entry/login.rs");
    let spell_loading = include_str!("../character/world_entry/login/spell_loading.rs");
    let currency_offset = login
        .find("load_character_currencies_for_login_like_cpp")
        .expect("currency hydration precedes spell loading");
    let spell_offset = login
        .find("load_character_spell_rows_for_login_like_cpp")
        .expect("login delegates spell-row loading");
    let favorite_offset = login
        .find("load_character_favorite_spells_for_login_like_cpp")
        .expect("favorite spells follow spell rows");
    let skill_offset = login
        .find("load_character_skill_rows_for_login_like_cpp")
        .expect("skill rows follow favorite spells");
    let consumer_offset = login
        .find("let login_spell_map_authority_complete_like_cpp = loaded_player_spell_rows_complete_like_cpp")
        .expect("complete-spell-rows gate consumes the spell result");
    assert!(currency_offset < spell_offset);
    assert!(spell_offset < favorite_offset);
    assert!(favorite_offset < skill_offset);
    assert!(skill_offset < consumer_offset);
    assert!(!login.contains("PlayerLoginAuxiliaryLoadRequestLikeCpp::Spells"));
    assert!(!login.contains("PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellFavorites"));
    assert!(!login.contains("let mut loaded_player_spell_rows_complete_like_cpp"));
    assert!(!login.contains("let mut favorite_spell_rows_complete_like_cpp"));

    for marker in [
        "PlayerLoginAuxiliaryLoadRequestLikeCpp::Spells",
        "i32::try_from(row.spell_id)",
        "spell_id > 0",
        "RepresentedPlayerSpellStateLikeCpp::Unchanged",
        "loaded_spell_for_add_spell_side_effects_like_cpp(row.spell_id, row.disabled)",
        "active_known_spell_for_send_like_cpp(row.spell_id, row.active, row.disabled)",
        "rows.complete = true;",
        "PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellFavorites",
        "favorites.insert(spell_id)",
        "complete = true;",
        "(favorites, complete)",
        "PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason }",
    ] {
        assert!(
            spell_loading.contains(marker),
            "spell phase lost `{marker}`"
        );
    }
}

#[test]
fn default_skill_login_phase_follows_quests_and_returns_learned_entries() {
    let login = include_str!("../character/world_entry/login.rs");
    let default_skills = include_str!("../character/world_entry/login/default_skills.rs");
    let quests_offset = login
        .find("self.load_player_quests().await;")
        .expect("quest loading precedes default skills");
    let count_offset = login
        .find("let persisted_skill_count = skill_info_by_id.len();")
        .expect("persisted skill count is captured before default skills");
    let default_offset = login
        .find("let Some(default_skill_entries) = self.apply_default_skills_for_login_like_cpp(")
        .expect("login delegates the default-skill pass");
    let rewarded_offset = login
        .find("for entry in &default_skill_entries {")
        .expect("skill-rewarded spells follow default skills");
    assert!(quests_offset < count_offset);
    assert!(count_offset < default_offset);
    assert!(default_offset < rewarded_offset);
    assert!(!login.contains("default_starting_skill_info_like_cpp"));
    assert!(!login.contains("let mut default_skill_entries"));

    for marker in [
        "default_starting_skill_info_like_cpp(",
        ".is_some_and(|skill| skill.value > 0)",
        "skill_info_by_id.len() >= 256",
        "RepresentedPlayerSkillStateLikeCpp::Deleted",
        "RepresentedPlayerSkillStateLikeCpp::Changed",
        "RepresentedPlayerSkillStateLikeCpp::New",
        "default_skill_entries.push(entry)",
        "replace_player_skill_records_like_cpp(skill_records.clone(), true, false)",
        "canonical Player skill owner unavailable while applying default skills",
        "return None;",
        "Some(default_skill_entries)",
    ] {
        assert!(
            default_skills.contains(marker),
            "default-skill phase lost `{marker}`"
        );
    }
}

#[test]
fn aura_login_phase_precedes_initial_item_mods_and_sets_authority() {
    let login = include_str!("../character/world_entry/login.rs");
    let aura_loading = include_str!("../character/world_entry/login/aura_loading.rs");
    let account_data_offset = login
        .find("self.load_player_account_data_like_cpp(guid).await;")
        .expect("account data loads before character auras");
    let aura_offset = login
        .find("load_character_auras_for_login_like_cpp")
        .expect("login delegates character-aura loading");
    let item_mods_offset = login
        .find("apply_initial_loaded_item_mods_like_cpp")
        .expect("initial item mods follow character auras");
    assert!(account_data_offset < aura_offset);
    assert!(aura_offset < item_mods_offset);
    assert!(!login.contains("PlayerLoginAuxiliaryLoadRequestLikeCpp::CharacterAuras"));
    assert!(!login.contains("PlayerLoginAuxiliaryLoadRequestLikeCpp::CharacterAuraEffects"));

    let reset_offset = aura_loading
        .find("set_player_aura_authority_complete_like_cpp(false)")
        .expect("aura authority is reset before loading");
    let load_offset = aura_loading
        .find("load_represented_character_auras_like_cpp(aura_rows, aura_effect_rows, 0)")
        .expect("aura rows are applied after both queries");
    let authority_offset = aura_loading
        .find("aura_rows_complete && aura_effect_rows_complete")
        .expect("aura authority requires both queries");
    assert!(reset_offset < load_offset);
    assert!(load_offset < authority_offset);
    for marker in [
        "PlayerLoginAuxiliaryLoadRequestLikeCpp::CharacterAuras",
        "PlayerLoginAuxiliaryLoadRequestLikeCpp::CharacterAuraEffects",
        "object_guid_from_db_binary_like_cpp",
        "CharacterAuraRowLikeCpp",
        "CharacterAuraEffectRowLikeCpp",
        "PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason }",
    ] {
        assert!(aura_loading.contains(marker), "aura phase lost `{marker}`");
    }
}

#[test]
fn player_spell_map_finalization_keeps_merge_rules_and_authority_gate() {
    let login = include_str!("../character/world_entry/login.rs");
    let finalization = include_str!("../character/world_entry/login/spell_map_finalization.rs");
    let mount_offset = login
        .find("self.promote_loaded_character_mount_spells_like_cpp(&known_spells);")
        .expect("mount promotion precedes PlayerSpellMap finalization");
    let gate_offset = login
        .find("let login_spell_map_authority_complete_like_cpp")
        .expect("login computes the PlayerSpellMap authority gate");
    let finalize_offset = login
        .find("finalize_player_spell_map_for_login_like_cpp(")
        .expect("login delegates PlayerSpellMap finalization");
    let summary_offset = login
        .find("\"Applied C++ LearnDefaultSkills and LearnSkillRewardedSpells\"")
        .expect("default-skill summary follows finalization");
    assert!(mount_offset < gate_offset);
    assert!(gate_offset < finalize_offset);
    assert!(finalize_offset < summary_offset);
    for flag in [
        "&& favorite_spell_rows_complete_like_cpp",
        "&& talent_rows_complete_like_cpp",
        "&& account_mount_rows_complete_like_cpp",
        "&& reputation_rows_complete_like_cpp",
    ] {
        assert!(login.contains(flag), "authority gate lost `{flag}`");
    }
    assert!(!login.contains("set_complete_represented_player_spell_rows_like_cpp"));

    for marker in [
        "self.known_spells_like_cpp().to_vec()",
        "represented_dependent_known_spells_like_cpp()",
        "skill_rewarded_removed_spells.contains(&row.spell_id)",
        "RepresentedPlayerSpellStateLikeCpp::Removed",
        "RepresentedPlayerSpellStateLikeCpp::Unchanged",
        "if login_authority_complete {",
        "set_complete_represented_player_spell_rows_like_cpp(",
        "mark_represented_spell_acquisition_snapshot_complete_like_cpp",
        "Keeping represented PlayerSpellMap incomplete after incomplete login authority",
    ] {
        assert!(
            finalization.contains(marker),
            "PlayerSpellMap finalization lost `{marker}`"
        );
    }
}

#[test]
fn group_membership_login_phase_resets_then_restores_represented_group() {
    let login = include_str!("../character/world_entry/login.rs");
    let group_loading = include_str!("../character/world_entry/login/group_loading.rs");
    let pet_offset = login
        .find("load_represented_login_pet_state_like_cpp(")
        .expect("pet state loads before group membership");
    let group_offset = login
        .find("load_group_membership_for_login_like_cpp")
        .expect("login delegates group-membership loading");
    let xp_offset = login
        .find("self.clamp_loaded_player_xp_to_next_level_like_cpp();")
        .expect("xp clamp follows group membership");
    assert!(pet_offset < group_offset);
    assert!(group_offset < xp_offset);
    assert!(!login.contains("PlayerLoginAuxiliaryLoadRequestLikeCpp::GroupMembership"));
    assert!(!login.contains("set_owned_player_group_like_cpp(None)"));

    let reset_offset = group_loading
        .find("set_owned_player_group_like_cpp(None)")
        .expect("group owner is cleared before loading");
    let request_offset = group_loading
        .find("PlayerLoginAuxiliaryLoadRequestLikeCpp::GroupMembership")
        .expect("group membership is requested");
    assert!(reset_offset < request_offset);
    for marker in [
        "rows.into_iter().next()",
        "load_represented_group_by_db_store_id_like_cpp(db_store_id)",
        "reset_group_update_sequence_if_needed_like_cpp()",
        "failed to load represented group membership",
    ] {
        assert!(
            group_loading.contains(marker),
            "group-membership phase lost `{marker}`"
        );
    }
}

#[test]
fn mail_login_phase_follows_controller_and_aborts_on_failure() {
    let login = include_str!("../character/world_entry/login.rs");
    let mail_loading = include_str!("../character/world_entry/login/mail_loading.rs");
    let controller_offset = login
        .find("ensure_login_player_controller_like_cpp(")
        .expect("login establishes the Player controller before mail");
    let mail_offset = login
        .find("load_character_mail_for_login_like_cpp")
        .expect("login delegates mail hydration");
    let money_offset = login
        .find("set_player_gold_like_cpp(")
        .expect("money hydration follows mail");
    assert!(controller_offset < mail_offset);
    assert!(mail_offset < money_offset);
    assert!(!login.contains("PlayerLoginAuxiliaryLoadRequestLikeCpp::Mail"));
    assert!(!login.contains("replace_owned_player_mails_like_cpp"));

    for marker in [
        "PlayerLoginAuxiliaryLoadRequestLikeCpp::Mail",
        "Player mail hydration failed",
        "invalid Player mail hydration outcome",
        "wow_entities::PlayerMailRecord",
        "(row.template_id != 0).then_some(row.template_id)",
        "replace_owned_player_mails_like_cpp(mails)",
        "canonical Player mail owner disappeared",
        "return false;",
    ] {
        assert!(mail_loading.contains(marker), "mail phase lost `{marker}`");
    }
}

#[test]
fn cuf_profiles_load_after_inventory_and_before_currencies() {
    let login = include_str!("../character/world_entry/login.rs");
    let cuf_profiles = include_str!("../character/world_entry/login/cuf_profiles.rs");
    let currency_loading = include_str!("../character/world_entry/login/currency_loading.rs");
    let inventory_offset = login
        .find("load_inventory_for_login_like_cpp")
        .expect("login loads inventory before CUF profiles");
    let cuf_profiles_offset = login
        .find("load_cuf_profiles_for_login_like_cpp")
        .expect("login delegates CUF profile hydration");
    let currencies_offset = login
        .find("load_character_currencies_for_login_like_cpp")
        .expect("login loads currencies after CUF profiles");

    assert!(inventory_offset < cuf_profiles_offset);
    assert!(cuf_profiles_offset < currencies_offset);
    let clear_offset = cuf_profiles
        .find("clear_represented_cuf_profiles_like_cpp")
        .expect("stale represented profiles are cleared first");
    let request_offset = cuf_profiles
        .find("PlayerLoginAuxiliaryLoadRequestLikeCpp::CufProfiles")
        .expect("CUF rows are loaded through the lifecycle port");
    let mark_loaded_offset = cuf_profiles
        .find("mark_represented_cuf_profiles_loaded_like_cpp")
        .expect("a completed row family is marked loaded");
    assert!(clear_offset < request_offset);
    assert!(request_offset < mark_loaded_offset);
    assert!(currency_loading.contains("PlayerLoginAuxiliaryLoadRequestLikeCpp::Currencies"));
}

#[test]
fn currency_hydration_remains_before_spell_loading_and_requires_canonical_player_state() {
    let login = include_str!("../character/world_entry/login.rs");
    let currency_loading = include_str!("../character/world_entry/login/currency_loading.rs");
    let currency_call_offset = login
        .find("load_character_currencies_for_login_like_cpp")
        .expect("login delegates currency hydration");
    let spell_request_offset = login
        .find("load_character_spell_rows_for_login_like_cpp")
        .expect("spell loading follows currency hydration");
    assert!(currency_call_offset < spell_request_offset);

    for marker in [
        "PlayerLoginAuxiliaryLoadRequestLikeCpp::Currencies",
        "self.player_currencies_like_cpp()",
        "currency_types_store()",
        "or_insert_with",
        "set_player_currencies_like_cpp(currencies)",
        "PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason }",
        "return false",
    ] {
        assert!(
            currency_loading.contains(marker),
            "currency hydration lost `{marker}`"
        );
    }
}

#[test]
fn glyph_login_phase_keeps_catalog_filter_and_loaded_marking() {
    let login = include_str!("../character/world_entry/login.rs");
    let glyph_loading = include_str!("../character/world_entry/login/glyph_loading.rs");
    let spell_offset = login
        .find("promote_loaded_character_mount_spells_like_cpp")
        .expect("spell loading precedes glyph hydration");
    let glyph_offset = login
        .find("load_character_glyphs_for_login_like_cpp")
        .expect("login delegates glyph hydration");
    let action_button_offset = login
        .find("load_action_buttons_for_login_like_cpp")
        .expect("action-button hydration follows glyphs");
    assert!(spell_offset < glyph_offset);
    assert!(glyph_offset < action_button_offset);
    assert!(!login.contains("PlayerLoginAuxiliaryLoadRequestLikeCpp::Glyphs"));

    for marker in [
        "reset_represented_glyphs_like_cpp",
        "PlayerLoginAuxiliaryLoadRequestLikeCpp::Glyphs",
        "load_represented_glyph_row_like_cpp",
        "player_bootstrap.glyph_properties.as_ref()",
        "mark_represented_glyphs_loaded_like_cpp",
        "PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason }",
    ] {
        assert!(
            glyph_loading.contains(marker),
            "glyph phase lost `{marker}`"
        );
    }
}

#[test]
fn action_button_login_phase_keeps_active_configuration_and_packet_projection() {
    let login = include_str!("../character/world_entry/login.rs");
    let action_buttons = include_str!("../character/world_entry/login/action_buttons.rs");
    let glyph_offset = login
        .find("load_character_glyphs_for_login_like_cpp")
        .expect("glyph loading precedes action buttons");
    let action_button_offset = login
        .find("load_action_buttons_for_login_like_cpp")
        .expect("login delegates action-button hydration");
    let identity_offset = login
        .rfind("set_loaded_player_identity_like_cpp(map_id as u16, race, class, level, gender);")
        .expect("login stores identity after action-button hydration");
    assert!(glyph_offset < action_button_offset);
    assert!(action_button_offset < identity_offset);

    for marker in [
        "reset_represented_action_buttons_like_cpp",
        "represented_action_button_db_context_like_cpp()",
        "PlayerLoginAuxiliaryLoadRequestLikeCpp::ActionButtons",
        "active_spec",
        "trait_config_id",
        "(row.button as usize) < 180 && row.action > 0",
        "record_loaded_action_button_like_cpp",
        "mark_represented_action_buttons_loaded_like_cpp",
        "UpdateActionButtons::pack_button",
        "return None",
        "Some(action_buttons)",
    ] {
        assert!(
            action_buttons.contains(marker),
            "action-button phase lost `{marker}`"
        );
    }
}
#[tokio::test]
async fn account_collection_empty_and_adapter_failure_clear_represented_rows_like_cpp() {
    let port = CollectionLoadPortLikeCpp::new([
        AccountCollectionLoadOutcomeLikeCpp::Loaded(AccountCollectionLoadedLikeCpp::Toys(
            Vec::new(),
        )),
        AccountCollectionLoadOutcomeLikeCpp::Failed {
            reason: "heirloom read failed".to_owned(),
        },
    ]);
    let (mut session, _) = make_session_with_send_capacity(1);
    session.set_battlenet_account_id(77);
    session.load_represented_account_toys_like_cpp([(42, true, false)]);
    session.load_represented_account_heirlooms_like_cpp([(43, 2)]);
    session.set_player_lifecycle_port_like_cpp(port.clone());

    session.load_account_toys_like_cpp().await;
    session.load_account_heirlooms_like_cpp().await;

    assert!(session.account_toy_rows_like_cpp().is_empty());
    assert!(session.account_heirloom_rows_like_cpp().is_empty());
    assert_eq!(
        port.requests(),
        vec![
            AccountCollectionLoadRequestLikeCpp::Toys {
                bnet_account_id: 77
            },
            AccountCollectionLoadRequestLikeCpp::Heirlooms {
                bnet_account_id: 77
            },
        ]
    );
}
#[test]
fn void_storage_login_context_preserves_cpp_field_five_bug() {
    let selected_context_column = ItemContext::Timewalking as u8;

    assert_eq!(
        void_storage_login_context_like_cpp(29, selected_context_column),
        29
    );
    assert_ne!(29, selected_context_column);
}
#[tokio::test]
async fn handle_player_login_prelude_resends_account_state_and_orders_packets_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(32);
    let guid = ObjectGuid::create_player(1, 42);
    let tutorials = [10, 20, 30, 40, 50, 60, 70, 80];
    let mounts = [
        AccountMount {
            spell_id: 100,
            flags: 1,
        },
        AccountMount {
            spell_id: 200,
            flags: 2,
        },
    ];
    session.set_player_guid(Some(guid));
    session.load_tutorials_data_values_like_cpp(Some(tutorials));
    let generators = session.id_generators_for_test_like_cpp();
    let feature_policy = session.support_feature_policy_for_test_like_cpp();
    assert!(
        session
            .send_handle_player_login_packets_like_cpp(
                generators.item.as_ref(),
                &feature_policy,
                guid,
                &Position::new(1.0, 2.0, 3.0, 4.0),
                571,
                &mounts,
                "first@second",
            )
            .await
    );

    let packets = send_rx.try_iter().collect::<Vec<_>>();
    let opcodes = packets
        .iter()
        .filter_map(|bytes| WorldPacket::from_bytes(bytes).server_opcode())
        .collect::<Vec<_>>();
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::AccountMountUpdate,
            ServerOpcodes::AccountMountUpdate,
            ServerOpcodes::AccountDataTimes,
            ServerOpcodes::TutorialFlags,
            ServerOpcodes::SetDungeonDifficulty,
            ServerOpcodes::LoginVerifyWorld,
            ServerOpcodes::AccountDataTimes,
            ServerOpcodes::FeatureSystemStatus,
            ServerOpcodes::ChatServerMessage,
            ServerOpcodes::ChatServerMessage,
            ServerOpcodes::SetTimeZoneInformation,
            ServerOpcodes::BattlePetJournalLockAcquired,
        ]
    );

    for (packet, expected_mount) in packets[..2].iter().zip(mounts) {
        let mut body = WorldPacket::from_bytes(&packet[2..]);
        assert!(!body.read_bit().unwrap());
        assert_eq!(body.read_int32().unwrap(), 1);
        assert_eq!(body.read_int32().unwrap(), expected_mount.spell_id);
        assert_eq!(body.read_bits(4).unwrap(), u32::from(expected_mount.flags));
        assert_eq!(body.remaining(), 0);
    }

    let mut global_account_data = WorldPacket::from_bytes(&packets[2][2..]);
    assert_eq!(
        global_account_data.read_packed_guid().unwrap(),
        ObjectGuid::EMPTY
    );
    let mut tutorial_packet = WorldPacket::from_bytes(&packets[3][2..]);
    for expected in tutorials {
        assert_eq!(tutorial_packet.read_uint32().unwrap(), expected);
    }
    assert_eq!(tutorial_packet.remaining(), 0);

    let mut character_account_data = WorldPacket::from_bytes(&packets[6][2..]);
    assert_eq!(character_account_data.read_packed_guid().unwrap(), guid);

    for (packet, expected_line) in packets[8..10].iter().zip(["first", "second"]) {
        let mut body = WorldPacket::from_bytes(&packet[2..]);
        assert_eq!(body.read_int32().unwrap(), 3);
        let string_len = body.read_bits(11).unwrap() as usize;
        assert_eq!(body.read_string(string_len).unwrap(), expected_line);
        assert_eq!(body.remaining(), 0);
    }

    assert!(session.has_represented_battle_pet_journal_lock_like_cpp());
}
#[test]
fn battleground_login_fallback_prefers_valid_entry_point_then_homebind_like_cpp() {
    let map_store = wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 1,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ]);
    let entry_point = CharacterLoginLocationLikeCpp {
        map_id: 1,
        bind_area_id: None,
        position: Position::new(10.0, 20.0, 30.0, 1.0),
    };
    let homebind = CharacterLoginLocationLikeCpp {
        map_id: 0,
        bind_area_id: Some(12),
        position: Position::new(-1.0, -2.0, 3.0, 0.0),
    };
    let bg_data = CharacterBattlegroundLoginDataLikeCpp { entry_point };

    assert!(usable_character_homebind_like_cpp(
        homebind,
        Some(&map_store),
        2,
    ));
    assert!(!usable_character_homebind_like_cpp(
        entry_point,
        Some(&map_store),
        2,
    ));

    assert_eq!(
        login_location_zone_area_like_cpp(entry_point, |map_id, position| {
            assert_eq!(map_id, 1);
            assert_eq!(position, entry_point.position);
            Ok((34, 56))
        })
        .unwrap(),
        (34, 56)
    );
    assert_eq!(
        login_location_zone_area_like_cpp(homebind, |map_id, position| {
            assert_eq!(map_id, 0);
            assert_eq!(position, homebind.position);
            Ok((78, 90))
        })
        .unwrap(),
        (78, 90)
    );
    let bind_update = login_bind_point_update_like_cpp(homebind);
    assert_eq!(bind_update.x, homebind.position.x);
    assert_eq!(bind_update.y, homebind.position.y);
    assert_eq!(bind_update.z, homebind.position.z);
    assert_eq!(bind_update.map_id, homebind.map_id);
    assert_eq!(bind_update.area_id, 12);

    assert_eq!(
        battleground_login_fallback_location_like_cpp(
            Some(bg_data),
            Some(homebind),
            Some(&map_store),
        ),
        Some(entry_point)
    );
    assert_eq!(
        battleground_login_fallback_location_like_cpp(
            Some(CharacterBattlegroundLoginDataLikeCpp {
                entry_point: CharacterLoginLocationLikeCpp {
                    map_id: u32::from(u16::MAX),
                    bind_area_id: None,
                    position: Position::ZERO,
                },
                ..bg_data
            }),
            Some(homebind),
            Some(&map_store),
        ),
        Some(homebind)
    );
    assert_eq!(
        battleground_login_fallback_location_like_cpp(
            None,
            Some(CharacterLoginLocationLikeCpp {
                position: Position::new(f32::NAN, 0.0, 0.0, 0.0),
                ..homebind
            }),
            Some(&map_store),
        ),
        None
    );
}
#[test]
fn rejected_instance_login_retries_valid_homebind_before_disconnect_like_cpp() {
    run_login_grid_cleanup_test(|| {
        let (mut session, _send_rx) = make_session_with_send_capacity(2);
        let guid = ObjectGuid::create_player(1, 45);
        let saved_position = Position::new(1.0, 2.0, 3.0, 0.0);
        let homebind_position = Position::new(10.0, 20.0, 30.0, 1.0);
        let canonical: crate::session::SharedCanonicalMapManager =
            Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 33,
                instance_type: wow_data::map::MAP_INSTANCE,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
            wow_data::MapEntry {
                id: 1,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
        ])));
        assert!(session.ensure_login_player_controller_like_cpp(
            guid,
            "InstanceFallback".to_string(),
            saved_position,
            33,
            1,
            1,
            10,
            0,
        ));
        assert!(matches!(
            session.ensure_canonical_world_map_for_current_player_like_cpp(),
            Some(wow_map::CreateMapDecision::Reject { .. })
        ));
        assert!(
            session
                .current_canonical_player_map_key_like_cpp()
                .is_none()
        );

        let mut map_id = 33;
        let mut zone_id = 999;
        let mut position = saved_position;
        assert!(session.retry_login_at_homebind_like_cpp(
            &mut map_id,
            &mut zone_id,
            &mut position,
            CharacterLoginLocationLikeCpp {
                map_id: 1,
                bind_area_id: Some(12),
                position: homebind_position,
            },
        ));

        assert_eq!(map_id, 1);
        assert_eq!(zone_id, 12);
        assert_eq!(session.player_zone_area_like_cpp(), Some((12, 12)));
        assert_eq!(position, homebind_position);
        assert_eq!(
            session.current_canonical_player_map_key_like_cpp(),
            Some(wow_map::MapKey::new(1, 0))
        );
    });
}
#[test]
fn garrison_login_uses_create_map_world_branch_like_cpp() {
    run_login_grid_cleanup_test(|| {
        let (mut session, _send_rx) = make_session_with_send_capacity(1);
        let guid = ObjectGuid::create_player(1, 47);
        let canonical: crate::session::SharedCanonicalMapManager =
            Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 1_151,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 2,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: wow_data::map::MAP_FLAG_GARRISON,
                flags2: 0,
            },
        ])));
        assert!(session.ensure_login_player_controller_like_cpp(
            guid,
            "GarrisonLogin".to_string(),
            Position::ZERO,
            1_151,
            1,
            1,
            10,
            0,
        ));

        assert!(matches!(
            session.ensure_canonical_world_map_for_current_player_like_cpp(),
            Some(wow_map::CreateMapDecision::Create {
                key,
                kind: wow_map::ManagedMapKind::World,
                ..
            }) if key == wow_map::MapKey::new(1_151, 0)
        ));
        assert_eq!(
            session.current_canonical_player_map_key_like_cpp(),
            Some(wow_map::MapKey::new(1_151, 0))
        );
    });
}
#[test]
fn garrison_login_rejects_unsupported_expansion_and_retries_homebind_like_cpp() {
    run_login_grid_cleanup_test(|| {
        let (mut session, _send_rx) = make_session_with_send_capacity(1);
        let guid = ObjectGuid::create_player(1, 48);
        let canonical: crate::session::SharedCanonicalMapManager =
            Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 1,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
            wow_data::MapEntry {
                id: 1_151,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 3,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: wow_data::map::MAP_FLAG_GARRISON,
                flags2: 0,
            },
        ])));
        assert!(session.ensure_login_player_controller_like_cpp(
            guid,
            "UnsupportedGarrisonLogin".to_string(),
            Position::ZERO,
            1_151,
            1,
            1,
            10,
            0,
        ));

        assert!(matches!(
            session.ensure_canonical_world_map_for_current_player_like_cpp(),
            Some(wow_map::CreateMapDecision::Reject { .. })
        ));
        assert!(
            session
                .current_canonical_player_map_key_like_cpp()
                .is_none()
        );
        assert!(canonical.lock().unwrap().find_map(1_151, 0).is_none());

        let mut map_id = 1_151;
        let mut zone_id = 999;
        let mut position = Position::ZERO;
        let homebind_position = Position::new(10.0, 20.0, 30.0, 1.0);
        assert!(session.retry_login_at_homebind_like_cpp(
            &mut map_id,
            &mut zone_id,
            &mut position,
            CharacterLoginLocationLikeCpp {
                map_id: 1,
                bind_area_id: Some(12),
                position: homebind_position,
            },
        ));
        assert_eq!(map_id, 1);
        assert_eq!(zone_id, 12);
        assert_eq!(session.player_zone_area_like_cpp(), Some((12, 12)));
        assert_eq!(position, homebind_position);
        assert_eq!(
            session.current_canonical_player_map_key_like_cpp(),
            Some(wow_map::MapKey::new(1, 0))
        );
    });
}
#[test]
fn unavailable_login_grid_cleans_partial_player_and_kicks_without_failure_packet_like_cpp() {
    run_login_grid_cleanup_test(|| {
        let (mut session, send_rx) = make_session_with_send_capacity(1);
        let guid = ObjectGuid::create_player(1, 42);
        let canonical: crate::session::SharedCanonicalMapManager =
            Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
        let registry = Arc::new(crate::session::directory::PlayerRegistry::default());
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 33,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
        ])));
        session.set_player_registry(Arc::clone(&registry));
        assert!(session.ensure_login_player_controller_like_cpp(
            guid,
            "GridFailure".to_string(),
            Position::ZERO,
            33,
            1,
            1,
            10,
            0,
        ));
        let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
        session.register_in_player_registry();
        assert!(
            session
                .current_canonical_player_map_key_like_cpp()
                .is_some()
        );
        assert!(registry.runtime_recipient(guid).is_some());

        assert!(!session.continue_login_after_grid_load_like_cpp(
            guid,
            33,
            0,
            Some(crate::session::PlayerGridLoadOutcomeLikeCpp {
                map_unavailable: true,
                ..Default::default()
            }),
        ));

        assert_eq!(session.state(), crate::session::SessionState::Disconnecting);
        assert!(session.player_guid().is_none());
        assert!(
            canonical
                .lock()
                .unwrap()
                .find_map(33, 0)
                .unwrap()
                .map()
                .get_typed_player(guid)
                .is_none()
        );
        assert!(registry.runtime_recipient(guid).is_none());
        assert!(send_rx.try_recv().is_err());
    });
}
#[test]
fn login_identity_hydrates_race_faction_into_registry_and_canonical_player_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let guid = ObjectGuid::create_player(1, 42_001);
    let canonical: crate::session::SharedCanonicalMapManager =
        Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
    let registry = Arc::new(crate::session::directory::PlayerRegistry::default());
    let mut race_entry = chr_race_entry(1, 0);
    race_entry.faction_id = 1;

    session.set_chr_races_store(Arc::new(ChrRacesStore::from_entries([race_entry])));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_registry(Arc::clone(&registry));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));

    // Mirror the real LoadFromDB order: identity is loaded from the
    // character row before the controller/map/registry publication.
    session.set_player_guid(Some(guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 10, 0);
    assert!(session.ensure_login_player_controller_like_cpp(
        guid,
        "FactionLogin".to_string(),
        Position::ZERO,
        571,
        1,
        1,
        10,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session.register_in_player_registry();

    assert_eq!(registry.legacy_aggro_candidates()[0].faction_template_id, 1);
    let manager = canonical.lock().unwrap();
    let player = manager
        .find_map(571, 0)
        .expect("login map")
        .map()
        .get_typed_player(guid)
        .expect("canonical login player");
    assert_eq!(player.unit().data().faction_template, 1);
}
#[test]
fn login_passive_parry_and_block_capabilities_feed_first_stat_projection_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 86);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_stats(Arc::new(PlayerStatsStore::from_entries([(
        (1, 1, 80),
        PlayerLevelStats {
            strength: 50,
            agility: 30,
            stamina: 40,
            intellect: 10,
            spirit: 20,
            base_mana: 0,
        },
    )])));
    session.set_chr_classes_store(Arc::new(ChrClassesStore::from_entries([chr_class_entry(
        1, 0,
    )])));
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 0, 0, 100, 220);
    let parry_spell_id = 90_087;
    let block_spell_id = 90_088;
    session.set_spell_store(Arc::new(passive_combat_capability_spell_store_like_cpp(
        parry_spell_id,
        block_spell_id,
    )));

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
            &RepresentedPlayerGearStatsLikeCpp::default(),
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
            &RepresentedPlayerGearStatsLikeCpp::default(),
        )
        .expect("warrior stat projection with limits");
    assert_eq!(limited.parry_pct, 1.0);
    assert_eq!(limited.block_pct, 1.0);
    assert!(limited.crit_pct <= 1.0);
    assert!(limited.ranged_crit_pct <= 1.0);
    assert!(limited.offhand_crit_pct <= 1.0);
}
