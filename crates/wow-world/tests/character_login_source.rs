#[test]
fn continue_login_no_longer_names_the_core_character_statement() {
    let handler = include_str!("../src/handlers/character/world_entry/login.rs");
    assert!(handler.contains("load_character_base_like_cpp"));
    assert!(handler.contains("PlayerCharacterBaseLoadOutcomeLikeCpp::Loaded(Some(row))"));
    assert!(handler.contains("PlayerCharacterBaseLoadOutcomeLikeCpp::Loaded(None)"));
    assert!(handler.contains("PlayerCharacterBaseLoadOutcomeLikeCpp::Failed { reason }"));
    assert!(!handler.contains("prepare(CharStatements::SEL_CHARACTER)"));
}

#[test]
fn persisted_transport_restore_stays_between_identity_and_reputation_loading_like_cpp() {
    let login = include_str!("../src/handlers/character/world_entry/login.rs");
    let transport_restore = include_str!("../src/handlers/character/world_entry/login/transport_restore.rs");
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
    let login = include_str!("../src/handlers/character/world_entry/login.rs");
    let reputation_loading = include_str!("../src/handlers/character/world_entry/login/reputation_loading.rs");
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
    let login = include_str!("../src/handlers/character/world_entry/login.rs");
    let talent_loading = include_str!("../src/handlers/character/world_entry/login/talent_loading.rs");
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
    let login = include_str!("../src/handlers/character/world_entry/login.rs");
    let skill_loading = include_str!("../src/handlers/character/world_entry/login/skill_loading.rs");
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
    let login = include_str!("../src/handlers/character/world_entry/login.rs");
    let spell_loading = include_str!("../src/handlers/character/world_entry/login/spell_loading.rs");
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
    let login = include_str!("../src/handlers/character/world_entry/login.rs");
    let default_skills = include_str!("../src/handlers/character/world_entry/login/default_skills.rs");
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
    let login = include_str!("../src/handlers/character/world_entry/login.rs");
    let aura_loading = include_str!("../src/handlers/character/world_entry/login/aura_loading.rs");
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
    let login = include_str!("../src/handlers/character/world_entry/login.rs");
    let finalization = include_str!("../src/handlers/character/world_entry/login/spell_map_finalization.rs");
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
    let login = include_str!("../src/handlers/character/world_entry/login.rs");
    let group_loading = include_str!("../src/handlers/character/world_entry/login/group_loading.rs");
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
    let login = include_str!("../src/handlers/character/world_entry/login.rs");
    let mail_loading = include_str!("../src/handlers/character/world_entry/login/mail_loading.rs");
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
    let login = include_str!("../src/handlers/character/world_entry/login.rs");
    let cuf_profiles = include_str!("../src/handlers/character/world_entry/login/cuf_profiles.rs");
    let currency_loading = include_str!("../src/handlers/character/world_entry/login/currency_loading.rs");
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
    let login = include_str!("../src/handlers/character/world_entry/login.rs");
    let currency_loading = include_str!("../src/handlers/character/world_entry/login/currency_loading.rs");
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
    let login = include_str!("../src/handlers/character/world_entry/login.rs");
    let glyph_loading = include_str!("../src/handlers/character/world_entry/login/glyph_loading.rs");
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
    let login = include_str!("../src/handlers/character/world_entry/login.rs");
    let action_buttons = include_str!("../src/handlers/character/world_entry/login/action_buttons.rs");
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
