//! World-statement regressions.
//!
//! Moved out of the world.rs root under #652; every test is unchanged.

use super::*;

#[test]
fn player_createinfo_startup_statement_keeps_cpp_position_and_transport_keys() {
    let sql = WorldStatements::SEL_PLAYER_CREATEINFO.sql();

    assert!(sql.starts_with("SELECT p.race, p.class, p.map"));
    assert!(sql.ends_with("FROM playercreateinfo p"));
    assert!(sql.contains("t.guid = p.npe_transport_guid LIMIT 1"));
    assert_eq!(sql.matches('?').count(), 0);
}

#[test]
fn gossip_menu_options_select_keeps_cpp_load_column_order_like_cpp() {
    let sql = WorldStatements::SEL_GOSSIP_MENU_OPTIONS.sql();

    assert!(sql.contains(
        "SELECT MenuID, GossipOptionID, OptionID, OptionNpc, OptionText, OptionBroadcastTextID, Language, Flags, ActionMenuID, ActionPoiID, GossipNpcOptionID, BoxCoded, BoxMoney, BoxText, BoxBroadcastTextID, SpellID, OverrideIconID"
    ));
    assert!(sql.contains("FROM gossip_menu_option WHERE MenuID = ?"));
    assert!(sql.contains("ORDER BY MenuID, OptionID"));
}

#[test]
fn trainer_by_creature_gossip_option_matches_cpp_lookup_key() {
    assert_eq!(
        WorldStatements::SEL_TRAINER_BY_CREATURE.sql(),
        "SELECT TrainerID FROM creature_trainer WHERE CreatureID = ? AND MenuID = 0 AND OptionID = 0"
    );
    assert_eq!(
        WorldStatements::SEL_TRAINER_BY_CREATURE_GOSSIP_OPTION.sql(),
        "SELECT TrainerID FROM creature_trainer WHERE CreatureID = ? AND MenuID = ? AND OptionID = ?"
    );
}

#[test]
fn creatures_in_range_selects_addon_path_and_effective_movement_like_cpp() {
    let sql = WorldStatements::SEL_CREATURES_IN_RANGE.sql();

    assert!(sql.contains("LEFT JOIN creature_addon ca ON ca.guid = c.guid"));
    assert!(sql.contains("LEFT JOIN creature_template_addon cta ON cta.entry = c.id"));
    assert!(
        sql.contains("c.wander_distance"),
        "C++ Creature::LoadFromDB copies CreatureData::wander_distance into m_wanderDistance"
    );
    assert!(
        sql.contains(
            "CASE WHEN ca.guid IS NOT NULL AND ca.PathId = 0 AND c.MovementType = 2 THEN 0 ELSE c.MovementType END"
        ),
        "C++ ObjectMgr::LoadCreatureAddons downgrades spawn waypoint movement when a spawn addon has PathId=0"
    );
    assert!(sql.contains("COALESCE(ca.PathId, cta.PathId, 0)"));
    assert!(
        !sql.contains("COALESCE(ca.mount, cta.mount, 0)"),
        "addon create fields must come from CreatureAddonStoreLikeCpp, which mirrors ObjectMgr load-time validation"
    );
    assert!(
        sql.contains("c.equipment_id"),
        "C++ Creature::InitEntry reads CreatureData::equipmentId from creature; VirtualItems are resolved from ObjectMgr::LoadEquipmentTemplates"
    );
    assert!(
        sql.contains("c.spawntimesecs"),
        "C++ Creature::LoadFromDB copies CreatureData::spawntimesecs into m_respawnDelay"
    );
    assert!(
        sql.contains("c.spawnDifficulties"),
        "C++ ObjectMgr::LoadCreatures parses CreatureData::spawnDifficulties and grid load filters by the map spawn mode"
    );
    assert!(
        sql.contains("c.ScriptName"),
        "C++ ObjectMgr::LoadCreatures stores creature.ScriptName on CreatureData"
    );
    assert!(
        sql.contains("c.StringId"),
        "C++ Creature::LoadFromDB copies CreatureData::StringId into m_stringIds[1]"
    );
    assert!(
        sql.contains("ct.VehicleId"),
        "C++ Creature::CreateFromProto chooses HighGuid::Vehicle and writes CreateObjectBits::Vehicle from creature_template.VehicleId"
    );
    assert!(
        !sql.contains("creature_equip_template cet"),
        "creature_equip_template is loaded once into CreatureEquipmentStoreLikeCpp, not joined per visibility query"
    );
    assert_eq!(sql.matches('?').count(), 5);
}

#[test]
fn world_state_load_statement_matches_cpp_sql_exactly() {
    let sql = WorldStatements::SEL_WORLD_STATES.sql();
    assert_eq!(
        sql,
        "SELECT ID, DefaultValue, MapIDs, AreaIDs, ScriptName FROM world_state"
    );
    assert_eq!(sql.matches('?').count(), 0);
    assert_eq!(
        WorldStatements::SEL_WORLD_STATE_IDS.sql(),
        "SELECT ID FROM world_state"
    );
}

#[test]
fn login_transport_materialization_statements_keep_join_and_guid_filter_shape() {
    let all = WorldStatements::SEL_LOGIN_TRANSPORTS.sql();
    let one = WorldStatements::SEL_LOGIN_TRANSPORT_BY_GUID.sql();

    assert!(all.starts_with("SELECT t.guid, t.entry, t.phaseUseFlags, t.phaseid, t.phasegroup, "));
    assert!(all.contains("JOIN gameobject_template gt ON gt.entry = t.entry"));
    assert!(all.contains("LEFT JOIN gameobject_template_addon gta ON gta.entry = t.entry"));
    assert!(all.contains("LEFT JOIN gameobject_overrides goo ON goo.spawnId = t.guid"));
    assert!(all.ends_with("WHERE gt.type = 15 ORDER BY t.guid"));
    assert_eq!(all.matches('?').count(), 0);

    assert_eq!(
        one,
        all.replace(" ORDER BY t.guid", " AND t.guid = ? LIMIT 1")
    );
    assert_eq!(one.matches('?').count(), 1);
}

#[test]
fn world_db_version_statement_matches_cpp_sql_exactly() {
    let sql = WorldStatements::SEL_WORLD_DB_VERSION.sql();

    assert_eq!(sql, "SELECT db_version, cache_id FROM version LIMIT 1");
    assert_eq!(sql.matches('?').count(), 0);
}

#[test]
fn game_event_condition_statement_matches_cpp_sql_exactly() {
    let sql = WorldStatements::SEL_GAME_EVENT_CONDITIONS.sql();
    assert_eq!(
        sql,
        "SELECT eventEntry, condition_id, req_num, max_world_state_field, done_world_state_field FROM game_event_condition"
    );
    assert_eq!(sql.matches('?').count(), 0);
}

#[test]
fn reputation_reward_rate_statement_matches_cpp_sql_exactly() {
    let sql = WorldStatements::SEL_REPUTATION_REWARD_RATE.sql();
    assert_eq!(
        sql,
        "SELECT faction, quest_rate, quest_daily_rate, quest_weekly_rate, quest_monthly_rate, quest_repeatable_rate, creature_rate, spell_rate FROM reputation_reward_rate"
    );
    assert_eq!(sql.matches('?').count(), 0);
}

#[test]
fn creature_onkill_reputation_statement_matches_cpp_sql_exactly() {
    let sql = WorldStatements::SEL_CREATURE_ONKILL_REPUTATION.sql();
    assert_eq!(
        sql,
        "SELECT creature_id, RewOnKillRepFaction1, RewOnKillRepFaction2, IsTeamAward1, MaxStanding1, RewOnKillRepValue1, IsTeamAward2, MaxStanding2, RewOnKillRepValue2, TeamDependent FROM creature_onkill_reputation"
    );
    assert_eq!(sql.matches('?').count(), 0);
}

#[test]
fn reputation_spillover_template_statement_matches_cpp_sql_exactly() {
    let sql = WorldStatements::SEL_REPUTATION_SPILLOVER_TEMPLATE.sql();
    assert_eq!(
        sql,
        "SELECT faction, faction1, rate_1, rank_1, faction2, rate_2, rank_2, faction3, rate_3, rank_3, faction4, rate_4, rank_4, faction5, rate_5, rank_5 FROM reputation_spillover_template"
    );
    assert_eq!(sql.matches('?').count(), 0);
}

#[test]
fn game_event_quest_condition_statement_matches_cpp_sql_exactly() {
    let sql = WorldStatements::SEL_GAME_EVENT_QUEST_CONDITIONS.sql();
    assert_eq!(
        sql,
        "SELECT quest, eventEntry, condition_id, num FROM game_event_quest_condition"
    );
    assert_eq!(sql.matches('?').count(), 0);
}

#[test]
fn game_event_seasonal_questrelation_statement_matches_cpp_sql_exactly() {
    let sql = WorldStatements::SEL_GAME_EVENT_SEASONAL_QUEST_RELATIONS.sql();
    assert_eq!(
        sql,
        "SELECT questId, eventEntry FROM game_event_seasonal_questrelation"
    );
    assert_eq!(sql.matches('?').count(), 0);
}

#[test]
fn npc_spellclick_spells_statement_matches_cpp_sql_exactly() {
    let sql = WorldStatements::SEL_NPC_SPELLCLICK_SPELLS.sql();
    assert_eq!(
        sql,
        "SELECT npc_entry, spell_id, cast_flags, user_type FROM npc_spellclick_spells"
    );
    assert_eq!(sql.matches('?').count(), 0);
}

#[test]
fn quest_template_statement_loads_reward_choice_item_types_like_cpp() {
    let sql = WorldStatements::SEL_QUEST_TEMPLATE.sql();

    assert!(sql.contains("qt.QuestPackageID"));
    assert!(sql.contains("qt.AllowableRaces AS AllowableRaces"));
    assert!(
        sql.contains("CAST(COALESCE(qta.AllowableClasses, 0) AS UNSIGNED) AS AllowableClasses")
    );
    assert!(sql.contains("CAST(COALESCE(qrci.Type1, 0) AS UNSIGNED) AS RewardChoiceItemType1"));
    assert!(sql.contains("CAST(COALESCE(qrci.Type6, 0) AS UNSIGNED) AS RewardChoiceItemType6"));
    assert!(sql.contains("LEFT JOIN quest_reward_choice_items qrci ON qt.ID = qrci.QuestID"));
    assert!(sql.contains("qt.RewardCurrencyID1, qt.RewardCurrencyQty1"));
    assert!(sql.contains("qt.RewardCurrencyID4, qt.RewardCurrencyQty4"));
    assert!(sql.contains("qt.RewardSkillLineID, qt.RewardNumSkillUps, qt.RewardTitle"));
    assert!(sql.contains(
        "CAST(COALESCE(qta.RewardMailTemplateID, 0) AS UNSIGNED) AS RewardMailTemplateID"
    ));
    assert!(sql.contains("CAST(COALESCE(qta.RewardMailDelay, 0) AS UNSIGNED) AS RewardMailDelay"));
    assert!(sql.contains(
        "CAST(COALESCE(qms.RewardMailSenderEntry, 0) AS UNSIGNED) AS RewardMailSenderEntry"
    ));
    assert!(sql.contains("LEFT JOIN quest_mail_sender qms ON qt.ID = qms.QuestId"));
    assert!(sql.contains("qt.RewardFactionID1, qt.RewardFactionValue1"));
    assert!(sql.contains("qt.RewardFactionID5, qt.RewardFactionValue5"));
    assert!(sql.contains("qt.RewardFactionFlags"));
    assert!(sql.contains("CAST(COALESCE(qta.RequiredSkillID, 0) AS UNSIGNED) AS RequiredSkillID"));
    assert!(
        sql.contains(
            "CAST(COALESCE(qta.RequiredSkillPoints, 0) AS UNSIGNED) AS RequiredSkillPoints"
        )
    );
}

#[test]
fn gameobject_quest_relation_statements_match_cpp_sql_exactly() {
    let starter_sql = WorldStatements::SEL_GAMEOBJECT_QUEST_STARTERS.sql();
    let ender_sql = WorldStatements::SEL_GAMEOBJECT_QUEST_ENDERS.sql();

    assert_eq!(starter_sql, "SELECT id, quest FROM gameobject_queststarter");
    assert_eq!(ender_sql, "SELECT id, quest FROM gameobject_questender");
    assert_eq!(starter_sql.matches('?').count(), 0);
    assert_eq!(ender_sql.matches('?').count(), 0);
}

#[test]
fn spell_pet_auras_statement_matches_cpp_sql_exactly() {
    let sql = WorldStatements::SEL_SPELL_PET_AURAS.sql();

    assert_eq!(
        sql,
        "SELECT spell, effectId, pet, aura FROM spell_pet_auras"
    );
    assert_eq!(sql.matches('?').count(), 0);
}

#[test]
fn trainer_cast_audit_statements_cover_every_world_table_blocker() {
    assert_eq!(
        WorldStatements::SEL_TRAINER_CAST_SCRIPT_BINDING_IDS.sql(),
        "SELECT DISTINCT spell_id FROM spell_script_names"
    );
    assert_eq!(
        WorldStatements::SEL_TRAINER_CAST_LEGACY_SCRIPT_IDS.sql(),
        "SELECT DISTINCT (id & 16777215) FROM spell_scripts"
    );
    assert_eq!(
        WorldStatements::SEL_TRAINER_CAST_CONDITION_IDS.sql(),
        "SELECT DISTINCT SourceEntry FROM conditions WHERE SourceTypeOrReferenceId IN (13, 17)"
    );
}

#[test]
fn spell_threats_statement_matches_cpp_sql_exactly() {
    let sql = WorldStatements::SEL_SPELL_THREATS.sql();

    assert_eq!(
        sql,
        "SELECT entry, flatMod, pctMod, apPctMod FROM spell_threat"
    );
    assert_eq!(sql.matches('?').count(), 0);
}

#[test]
fn spell_enchant_proc_data_statement_matches_cpp_sql_exactly() {
    let sql = WorldStatements::SEL_SPELL_ENCHANT_PROC_DATA.sql();

    assert_eq!(
        sql,
        "SELECT EnchantID, Chance, ProcsPerMinute, HitMask, AttributesMask FROM spell_enchant_proc_data"
    );
    assert_eq!(sql.matches('?').count(), 0);
}

#[test]
fn spell_linked_statement_matches_cpp_sql_exactly() {
    let sql = WorldStatements::SEL_SPELL_LINKED.sql();

    assert_eq!(
        sql,
        "SELECT spell_trigger, spell_effect, type FROM spell_linked_spell"
    );
    assert_eq!(sql.matches('?').count(), 0);
}

#[test]
fn spell_totem_model_statement_matches_cpp_sql_exactly() {
    let sql = WorldStatements::SEL_SPELL_TOTEM_MODEL.sql();

    assert_eq!(
        sql,
        "SELECT SpellID, RaceID, DisplayID from spell_totem_model"
    );
    assert_eq!(sql.matches('?').count(), 0);
}

#[test]
fn spell_required_statement_matches_cpp_sql_exactly() {
    let sql = WorldStatements::SEL_SPELL_REQUIRED.sql();

    assert_eq!(sql, "SELECT spell_id, req_spell from spell_required");
    assert_eq!(sql.matches('?').count(), 0);
}

#[test]
fn spell_learn_spell_statement_matches_cpp_sql_exactly() {
    let sql = WorldStatements::SEL_SPELL_LEARN_SPELL.sql();

    assert_eq!(sql, "SELECT entry, SpellID, Active FROM spell_learn_spell");
    assert_eq!(sql.matches('?').count(), 0);
}

#[test]
fn spell_target_position_statement_matches_cpp_sql_exactly() {
    let sql = WorldStatements::SEL_SPELL_TARGET_POSITION.sql();

    assert_eq!(
        sql,
        "SELECT ID, EffectIndex, MapID, PositionX, PositionY, PositionZ, Orientation FROM spell_target_position"
    );
    assert_eq!(sql.matches('?').count(), 0);
}

#[test]
fn spell_group_statement_matches_cpp_sql_exactly() {
    let sql = WorldStatements::SEL_SPELL_GROUP.sql();

    assert_eq!(sql, "SELECT id, spell_id FROM spell_group");
    assert_eq!(sql.matches('?').count(), 0);
}

#[test]
fn spell_group_stack_rules_statement_matches_cpp_sql_exactly() {
    let sql = WorldStatements::SEL_SPELL_GROUP_STACK_RULES.sql();

    assert_eq!(
        sql,
        "SELECT group_id, stack_rule FROM spell_group_stack_rules"
    );
    assert_eq!(sql.matches('?').count(), 0);
}

#[test]
fn spell_proc_statement_matches_cpp_sql_exactly() {
    let sql = WorldStatements::SEL_SPELL_PROC.sql();

    assert_eq!(
        sql,
        "SELECT SpellId, SchoolMask, SpellFamilyName, SpellFamilyMask0, SpellFamilyMask1, SpellFamilyMask2, SpellFamilyMask3, ProcFlags, ProcFlags2, SpellTypeMask, SpellPhaseMask, HitMask, AttributesMask, DisableEffectsMask, ProcsPerMinute, Chance, Cooldown, Charges FROM spell_proc"
    );
    assert_eq!(sql.matches('?').count(), 0);
}

#[test]
fn spell_area_statement_matches_cpp_sql_exactly() {
    let sql = WorldStatements::SEL_SPELL_AREA.sql();

    assert_eq!(
        sql,
        "SELECT spell, area, quest_start, quest_start_status, quest_end_status, quest_end, aura_spell, racemask, gender, flags FROM spell_area"
    );
    assert_eq!(sql.matches('?').count(), 0);
}

#[test]
fn spell_custom_attr_statement_matches_cpp_sql_exactly() {
    let sql = WorldStatements::SEL_SPELL_CUSTOM_ATTR.sql();

    assert_eq!(sql, "SELECT entry, attributes FROM spell_custom_attr");
    assert_eq!(sql.matches('?').count(), 0);
}

#[test]
fn serverside_spell_effect_statement_matches_cpp_sql_exactly() {
    let sql = WorldStatements::SEL_SERVERSIDE_SPELL_EFFECT.sql();

    assert_eq!(
        sql,
        concat!(
            "SELECT SpellID, EffectIndex, DifficultyID, Effect, EffectAura, EffectAmplitude, EffectAttributes, ",
            "EffectAuraPeriod, EffectBonusCoefficient, EffectChainAmplitude, EffectChainTargets, EffectItemType, EffectMechanic, EffectPointsPerResource, ",
            "EffectPosFacing, EffectRealPointsPerLevel, EffectTriggerSpell, BonusCoefficientFromAP, PvpMultiplier, Coefficient, Variance, ",
            "ResourceCoefficient, GroupSizeBasePointsCoefficient, EffectBasePoints, EffectMiscValue1, EffectMiscValue2, EffectRadiusIndex1, ",
            "EffectRadiusIndex2, EffectSpellClassMask1, EffectSpellClassMask2, EffectSpellClassMask3, EffectSpellClassMask4, ImplicitTarget1, ",
            "ImplicitTarget2 FROM serverside_spell_effect"
        )
    );
    assert_eq!(sql.matches('?').count(), 0);
}

#[test]
fn serverside_spell_statement_matches_cpp_sql_exactly() {
    let sql = WorldStatements::SEL_SERVERSIDE_SPELL.sql();

    assert_eq!(
        sql,
        concat!(
            "SELECT Id, DifficultyID, CategoryId, Dispel, Mechanic, Attributes, AttributesEx, AttributesEx2, AttributesEx3, ",
            "AttributesEx4, AttributesEx5, AttributesEx6, AttributesEx7, AttributesEx8, AttributesEx9, AttributesEx10, AttributesEx11, AttributesEx12, AttributesEx13, ",
            "AttributesEx14, Stances, StancesNot, Targets, TargetCreatureType, RequiresSpellFocus, FacingCasterFlags, CasterAuraState, TargetAuraState, ",
            "ExcludeCasterAuraState, ExcludeTargetAuraState, CasterAuraSpell, TargetAuraSpell, ExcludeCasterAuraSpell, ExcludeTargetAuraSpell, ",
            "CasterAuraType, TargetAuraType, ExcludeCasterAuraType, ExcludeTargetAuraType, CastingTimeIndex, ",
            "RecoveryTime, CategoryRecoveryTime, StartRecoveryCategory, StartRecoveryTime, InterruptFlags, AuraInterruptFlags1, AuraInterruptFlags2, ",
            "ChannelInterruptFlags1, ChannelInterruptFlags2, ProcFlags, ProcFlags2, ProcChance, ProcCharges, ProcCooldown, ProcBasePPM, MaxLevel, BaseLevel, SpellLevel, ",
            "DurationIndex, RangeIndex, Speed, LaunchDelay, StackAmount, EquippedItemClass, EquippedItemSubClassMask, EquippedItemInventoryTypeMask, ContentTuningId, ",
            "SpellName, ConeAngle, ConeWidth, MaxTargetLevel, MaxAffectedTargets, SpellFamilyName, SpellFamilyFlags1, SpellFamilyFlags2, SpellFamilyFlags3, SpellFamilyFlags4, ",
            "DmgClass, PreventionType, AreaGroupId, SchoolMask, ChargeCategoryId FROM serverside_spell"
        )
    );
    assert_eq!(sql.matches('?').count(), 0);
}

#[test]
fn faction_change_statements_match_cpp_sql_exactly() {
    assert_eq!(
        WorldStatements::SEL_FACTION_CHANGE_ACHIEVEMENTS.sql(),
        "SELECT alliance_id, horde_id FROM player_factionchange_achievement"
    );
    assert_eq!(
        WorldStatements::SEL_FACTION_CHANGE_QUESTS.sql(),
        "SELECT alliance_id, horde_id FROM player_factionchange_quests"
    );
    assert_eq!(
        WorldStatements::SEL_FACTION_CHANGE_REPUTATIONS.sql(),
        "SELECT alliance_id, horde_id FROM player_factionchange_reputations"
    );
    assert_eq!(
        WorldStatements::SEL_FACTION_CHANGE_SPELLS.sql(),
        "SELECT alliance_id, horde_id FROM player_factionchange_spells"
    );
    assert_eq!(
        WorldStatements::SEL_FACTION_CHANGE_TITLES.sql(),
        "SELECT alliance_id, horde_id FROM player_factionchange_titles"
    );
    assert_eq!(
        WorldStatements::SEL_FACTION_CHANGE_ACHIEVEMENTS
            .sql()
            .matches('?')
            .count()
            + WorldStatements::SEL_FACTION_CHANGE_QUESTS
                .sql()
                .matches('?')
                .count()
            + WorldStatements::SEL_FACTION_CHANGE_REPUTATIONS
                .sql()
                .matches('?')
                .count()
            + WorldStatements::SEL_FACTION_CHANGE_SPELLS
                .sql()
                .matches('?')
                .count()
            + WorldStatements::SEL_FACTION_CHANGE_TITLES
                .sql()
                .matches('?')
                .count(),
        0
    );
}

#[test]
fn scene_templates_statement_matches_cpp_sql_exactly() {
    let sql = WorldStatements::SEL_SCENE_TEMPLATES.sql();

    assert_eq!(
        sql,
        "SELECT SceneId, Flags, ScriptPackageID, Encrypted, ScriptName FROM scene_template"
    );
    assert_eq!(sql.matches('?').count(), 0);
}

#[test]
fn player_choice_statements_match_cpp_sql_exactly() {
    let choices_sql = WorldStatements::SEL_PLAYER_CHOICES.sql();
    let responses_sql = WorldStatements::SEL_PLAYER_CHOICE_RESPONSES.sql();
    let rewards_sql = WorldStatements::SEL_PLAYER_CHOICE_RESPONSE_REWARDS.sql();
    let reward_items_sql = WorldStatements::SEL_PLAYER_CHOICE_RESPONSE_REWARD_ITEMS.sql();
    let reward_currencies_sql = WorldStatements::SEL_PLAYER_CHOICE_RESPONSE_REWARD_CURRENCIES.sql();
    let reward_factions_sql = WorldStatements::SEL_PLAYER_CHOICE_RESPONSE_REWARD_FACTIONS.sql();
    let reward_item_choices_sql =
        WorldStatements::SEL_PLAYER_CHOICE_RESPONSE_REWARD_ITEM_CHOICES.sql();
    let maw_powers_sql = WorldStatements::SEL_PLAYER_CHOICE_RESPONSE_MAW_POWERS.sql();
    let choice_locales_sql = WorldStatements::SEL_PLAYER_CHOICE_LOCALES.sql();
    let response_locales_sql = WorldStatements::SEL_PLAYER_CHOICE_RESPONSE_LOCALES.sql();

    assert_eq!(
        choices_sql,
        "SELECT ChoiceId, UiTextureKitId, SoundKitId, CloseSoundKitId, Duration, Question, PendingChoiceText, HideWarboardHeader, KeepOpenAfterChoice FROM playerchoice"
    );
    assert_eq!(
        responses_sql,
        "SELECT ChoiceId, ResponseId, ResponseIdentifier, ChoiceArtFileId, Flags, WidgetSetID, UiTextureAtlasElementID, SoundKitID, GroupID, UiTextureKitID, Answer, Header, SubHeader, ButtonTooltip, Description, Confirmation, RewardQuestID FROM playerchoice_response ORDER BY `Index` ASC"
    );
    assert_eq!(
        rewards_sql,
        "SELECT ChoiceId, ResponseId, TitleId, PackageId, SkillLineId, SkillPointCount, ArenaPointCount, HonorPointCount, Money, Xp FROM playerchoice_response_reward"
    );
    assert_eq!(
        reward_items_sql,
        "SELECT ChoiceId, ResponseId, ItemId, BonusListIDs, Quantity FROM playerchoice_response_reward_item ORDER BY `Index` ASC"
    );
    assert_eq!(
        reward_currencies_sql,
        "SELECT ChoiceId, ResponseId, CurrencyId, Quantity FROM playerchoice_response_reward_currency ORDER BY `Index` ASC"
    );
    assert_eq!(
        reward_factions_sql,
        "SELECT ChoiceId, ResponseId, FactionId, Quantity FROM playerchoice_response_reward_faction ORDER BY `Index` ASC"
    );
    assert_eq!(
        reward_item_choices_sql,
        "SELECT ChoiceId, ResponseId, ItemId, BonusListIDs, Quantity FROM playerchoice_response_reward_item_choice ORDER BY `Index` ASC"
    );
    assert_eq!(
        maw_powers_sql,
        "SELECT ChoiceId, ResponseId, TypeArtFileID, Rarity, RarityColor, SpellID, MaxStacks FROM playerchoice_response_maw_power"
    );
    assert_eq!(
        choice_locales_sql,
        "SELECT ChoiceId, locale, Question FROM playerchoice_locale"
    );
    assert_eq!(
        response_locales_sql,
        "SELECT ChoiceID, ResponseID, locale, Answer, Header, SubHeader, ButtonTooltip, Description, Confirmation FROM playerchoice_response_locale"
    );
    assert_eq!(choices_sql.matches('?').count(), 0);
    assert_eq!(responses_sql.matches('?').count(), 0);
    assert_eq!(rewards_sql.matches('?').count(), 0);
    assert_eq!(reward_items_sql.matches('?').count(), 0);
    assert_eq!(reward_currencies_sql.matches('?').count(), 0);
    assert_eq!(reward_factions_sql.matches('?').count(), 0);
    assert_eq!(reward_item_choices_sql.matches('?').count(), 0);
    assert_eq!(maw_powers_sql.matches('?').count(), 0);
    assert_eq!(choice_locales_sql.matches('?').count(), 0);
    assert_eq!(response_locales_sql.matches('?').count(), 0);
}

#[test]
fn jump_charge_params_statement_matches_cpp_sql_exactly() {
    let sql = WorldStatements::SEL_JUMP_CHARGE_PARAMS.sql();

    assert_eq!(
        sql,
        "SELECT id, speed, treatSpeedAsMoveTimeSeconds, jumpGravity, spellVisualId, progressCurveId, parabolicCurveId FROM jump_charge_params"
    );
    assert_eq!(sql.matches('?').count(), 0);
}

#[test]
fn quest_item_loader_statements_match_cpp_sql_exactly() {
    let gameobject_sql = WorldStatements::SEL_GAMEOBJECT_QUEST_ITEM_ROWS.sql();
    let creature_sql = WorldStatements::SEL_CREATURE_QUEST_ITEM_ROWS.sql();

    assert_eq!(
        gameobject_sql,
        "SELECT GameObjectEntry, ItemId, Idx FROM gameobject_questitem ORDER BY Idx ASC"
    );
    assert_eq!(
        creature_sql,
        "SELECT CreatureEntry, DifficultyID, ItemId, Idx FROM creature_questitem ORDER BY Idx ASC"
    );
    assert_eq!(gameobject_sql.matches('?').count(), 0);
    assert_eq!(creature_sql.matches('?').count(), 0);
}

#[test]
fn areatrigger_template_loader_statements_match_cpp_sql_exactly() {
    let actions_sql = WorldStatements::SEL_AREATRIGGER_TEMPLATE_ACTIONS.sql();
    let polygon_vertices_sql =
        WorldStatements::SEL_AREATRIGGER_CREATE_PROPERTIES_POLYGON_VERTICES.sql();
    let spline_points_sql = WorldStatements::SEL_AREATRIGGER_CREATE_PROPERTIES_SPLINE_POINTS.sql();
    let create_properties_sql = WorldStatements::SEL_AREATRIGGER_CREATE_PROPERTIES.sql();
    let orbit_sql = WorldStatements::SEL_AREATRIGGER_CREATE_PROPERTIES_ORBIT.sql();
    let templates_sql = WorldStatements::SEL_AREATRIGGER_TEMPLATES.sql();

    assert_eq!(
        actions_sql,
        "SELECT AreaTriggerId, IsCustom, ActionType, ActionParam, TargetType FROM `areatrigger_template_actions`"
    );
    assert_eq!(
        polygon_vertices_sql,
        "SELECT AreaTriggerCreatePropertiesId, IsCustom, Idx, VerticeX, VerticeY, VerticeTargetX, VerticeTargetY FROM `areatrigger_create_properties_polygon_vertex` ORDER BY `AreaTriggerCreatePropertiesId`, `IsCustom`, `Idx`"
    );
    assert_eq!(
        spline_points_sql,
        "SELECT AreaTriggerCreatePropertiesId, IsCustom, X, Y, Z FROM `areatrigger_create_properties_spline_point` ORDER BY `AreaTriggerCreatePropertiesId`, `IsCustom`, `Idx`"
    );
    assert_eq!(
        create_properties_sql,
        "SELECT Id, IsCustom, AreaTriggerId, IsAreatriggerCustom, Flags, MoveCurveId, ScaleCurveId, MorphCurveId, FacingCurveId, AnimId, AnimKitId, DecalPropertiesId, TimeToTarget, TimeToTargetScale, Shape, ShapeData0, ShapeData1, ShapeData2, ShapeData3, ShapeData4, ShapeData5, ShapeData6, ShapeData7, ScriptName FROM `areatrigger_create_properties`"
    );
    assert_eq!(
        orbit_sql,
        "SELECT AreaTriggerCreatePropertiesId, IsCustom, StartDelay, CircleRadius, BlendFromRadius, InitialAngle, ZOffset, CounterClockwise, CanLoop FROM `areatrigger_create_properties_orbit`"
    );
    assert_eq!(
        templates_sql,
        "SELECT Id, IsCustom, Flags FROM `areatrigger_template`"
    );
    assert_eq!(actions_sql.matches('?').count(), 0);
    assert_eq!(polygon_vertices_sql.matches('?').count(), 0);
    assert_eq!(spline_points_sql.matches('?').count(), 0);
    assert_eq!(create_properties_sql.matches('?').count(), 0);
    assert_eq!(orbit_sql.matches('?').count(), 0);
    assert_eq!(templates_sql.matches('?').count(), 0);
}
