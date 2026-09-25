use super::*;

#[test]
fn player_spell_hit_source_authority_requires_locked_complete_battle_pet_slots_like_cpp() {
    let mut session = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    assert!(session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());

    assert!(
        !session.complete_represented_battle_pet_slot_authority_load_like_cpp([
            (0, None, true),
            (1, None, true),
        ])
    );
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "an incomplete battle_pet_slots query cannot exclude C++ login spell 125610"
    );

    assert!(
        session.complete_represented_battle_pet_slot_authority_load_like_cpp([
            (0, None, false),
            (1, None, true),
            (2, None, true),
        ])
    );
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "C++ learns spell 125610 when battle-pet slot zero is unlocked"
    );

    assert!(
        session.complete_represented_battle_pet_slot_authority_load_like_cpp([
            (0, None, true),
            (1, None, true),
            (2, None, true),
        ])
    );
    assert!(session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());
}

#[test]
fn player_spell_hit_source_authority_requires_complete_empty_character_pets_like_cpp() {
    let mut session = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    assert!(session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());

    session.begin_represented_character_pet_authority_load_like_cpp();
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "an incomplete character_pet query cannot exclude pet-to-owner aura casts"
    );

    assert_eq!(
        session.load_represented_pet_stable_rows_like_cpp(
            0,
            [character_pet_stable_row_like_cpp(
                42,
                PetSaveMode::active_slot(0),
                1,
            )],
        ),
        1,
    );
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "any persisted character_pet row must fail closed"
    );

    assert_eq!(
        session.load_represented_pet_stable_rows_like_cpp(
            0,
            std::iter::empty::<CharacterPetStableRowLikeCpp>(),
        ),
        0,
    );
    assert!(session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());

    let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 530, 0, 500, 42);
    session.set_represented_pet_mode_state_like_cpp(
        Some(pet_guid),
        wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP,
        wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP,
    );
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "an active represented pet revokes the empty lifetime proof"
    );
}

#[test]
fn player_spell_hit_source_authority_rejects_instance_map_login_hooks_like_cpp() {
    let mut session = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    assert!(session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());

    session.set_map_store(Arc::new(MapStore::from_entries([wow_data::MapEntry {
        id: 530,
        instance_type: wow_data::map::MAP_INSTANCE,
        expansion_id: 1,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }])));
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "unrepresented InstanceScript::OnPlayerEnter can cast auras during AddPlayerToMap"
    );
}

#[test]
fn player_spell_hit_source_first_login_tombstone_survives_area_change_like_cpp() {
    let mut session = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    assert!(session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());

    session.tombstone_player_spell_hit_aura_authority_like_cpp();
    assert!(!session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());
    assert!(session.update_area_represented_like_cpp(2));
    session.set_area_table_store(Arc::new(AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 2,
            continent_id: 530,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
    ])));
    session.set_player_zone_area_authority_complete_like_cpp(true);
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "an unknown FIRST-login cast can leave a permanent aura across later area changes"
    );
}

#[test]
fn player_spell_hit_source_authority_requires_login_skill_guild_and_quest_sources_like_cpp() {
    let mut session = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    assert!(session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());

    session.set_represented_guild_id_like_cpp(7);
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "C++ Guild::SendLoginInfo can learn guild perk spells"
    );
    session.set_represented_guild_id_like_cpp(0);

    session.set_player_skill_records_like_cpp(HashMap::new());
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "trait and quest conditions require a complete player skill source"
    );
    assert!(session.set_complete_player_skill_records_like_cpp(HashMap::new(), 0));

    let mut auto_push = test_quest_template(10_044);
    auto_push.flags_ex |= 0x0400_0000;
    auto_push.source_spell_id = 33_795;
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [auto_push],
    )));
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "C++ PushQuests can AddQuest and cast an AUTO_PUSH template's SourceSpellID"
    );

    let mut rewarded = test_quest_template(10_046);
    rewarded.reward_spell = 90_046;
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [rewarded],
    )));
    session.begin_player_quest_status_authority_load_like_cpp();
    session.record_represented_rewarded_quest_row_like_cpp(10_046);
    session.complete_player_quest_status_authority_load_like_cpp();
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "C++ _LoadQuestStatusRewarded learns each row's reward spell"
    );

    let mut recast = test_quest_template(10_047);
    recast.flags |= QUEST_FLAGS_PLAYER_CAST_ACCEPT_LIKE_CPP;
    recast.flags_ex |= QUEST_FLAGS_EX_RECAST_ACCEPT_SPELL_ON_LOGIN_LIKE_CPP;
    recast.source_spell_id = 33_795;
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [recast.clone()],
    )));
    session.begin_player_quest_status_authority_load_like_cpp();
    session.quest_test_fixture_like_cpp.player_quests.clear();
    session.quest_test_fixture_like_cpp.player_quests.insert(
        recast.id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id: recast.id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );
    session.complete_player_quest_status_authority_load_like_cpp();
    assert!(
        session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "an accept-spell recast is admissible only after its exact spell is proven hit-inert"
    );

    recast.source_spell_id = 90_047;
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [recast],
    )));
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "an unrepresented active-quest login cast must fail closed"
    );
}
