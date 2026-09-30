//! Instance and map-lock catalog fixtures.
//!
//! These builders retain the original session-test behavior and are
//! visible only within the parent `session::tests` subtree.

use super::*;

pub(in crate::session::tests) fn install_create_map_difficulty_stores_like_cpp(
    session: &mut WorldSession,
    map_id: u32,
    default_difficulty_id: u8,
    default_difficulty_flags: DifficultyFlags,
) {
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([
        difficulty_entry(
            2,
            MAP_INSTANCE_LIKE_CPP,
            DifficultyFlags::CAN_SELECT | DifficultyFlags::DEFAULT,
        ),
        difficulty_entry(
            3,
            MAP_RAID_LIKE_CPP,
            DifficultyFlags::CAN_SELECT | DifficultyFlags::LEGACY,
        ),
        difficulty_entry(
            4,
            MAP_RAID_LIKE_CPP,
            DifficultyFlags::CAN_SELECT | DifficultyFlags::LEGACY,
        ),
        difficulty_entry(15, MAP_RAID_LIKE_CPP, DifficultyFlags::CAN_SELECT),
        difficulty_entry(
            u32::from(default_difficulty_id),
            MAP_RAID_LIKE_CPP,
            default_difficulty_flags,
        ),
    ])));
    session.set_map_difficulty_store(Arc::new(MapDifficultyStore::from_entries([
        MapDifficultyEntry {
            id: 1,
            message: String::new(),
            map_id,
            difficulty_id: default_difficulty_id,
            lock_id: 0,
            reset_interval: 0,
            max_players: 0,
            flags: 0,
        },
    ])));
}

pub(in crate::session::tests) fn represented_map_entry_for_create_map_context_like_cpp(
    map_id: u32,
    instance_type: i8,
) -> wow_data::map::MapEntry {
    wow_data::map::MapEntry {
        id: map_id,
        instance_type,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }
}

pub(in crate::session::tests) fn install_create_map_active_lock_stores_like_cpp(
    session: &mut WorldSession,
    map_id: u32,
    difficulty_id: u8,
    lock_id: u8,
    reset_interval: u8,
) {
    install_create_map_active_lock_stores_with_max_players_like_cpp(
        session,
        map_id,
        difficulty_id,
        lock_id,
        reset_interval,
        10,
    );
}

pub(in crate::session::tests) fn install_create_map_active_lock_stores_with_max_players_like_cpp(
    session: &mut WorldSession,
    map_id: u32,
    difficulty_id: u8,
    lock_id: u8,
    reset_interval: u8,
    max_players: u32,
) {
    install_create_map_active_lock_stores_with_expansion_and_max_players_like_cpp(
        session,
        map_id,
        difficulty_id,
        lock_id,
        reset_interval,
        0,
        max_players,
    );
}

pub(in crate::session::tests) fn install_create_map_active_lock_stores_with_expansion_and_max_players_like_cpp(
    session: &mut WorldSession,
    map_id: u32,
    difficulty_id: u8,
    lock_id: u8,
    reset_interval: u8,
    expansion_id: u8,
    max_players: u32,
) {
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: map_id,
            instance_type: wow_data::map::MAP_RAID,
            expansion_id,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([DifficultyEntry {
        id: u32::from(difficulty_id),
        instance_type: MAP_RAID_LIKE_CPP,
        flags: DifficultyFlags::CAN_SELECT.bits(),
        fallback_difficulty_id: 0,
        toggle_difficulty_id: 0,
    }])));
    session.set_map_difficulty_store(Arc::new(MapDifficultyStore::from_entries([
        MapDifficultyEntry {
            id: 900,
            message: String::new(),
            map_id,
            difficulty_id,
            lock_id,
            reset_interval,
            max_players,
            flags: 0,
        },
    ])));
}

pub(in crate::session::tests) fn install_access_requirement_store_like_cpp(
    session: &mut WorldSession,
    requirement: wow_data::AccessRequirementLikeCpp,
) {
    session.set_access_requirement_store(Arc::new(
        wow_data::AccessRequirementStoreLikeCpp::from_entries_like_cpp([requirement]),
    ));
}

pub(in crate::session::tests) fn access_requirement_like_cpp(
    map_id: u32,
    difficulty: u8,
) -> wow_data::AccessRequirementLikeCpp {
    wow_data::AccessRequirementLikeCpp {
        map_id,
        difficulty,
        level_min: 0,
        level_max: 0,
        item: 0,
        item2: 0,
        quest_done_a: 0,
        quest_done_h: 0,
        completed_achievement: 0,
        quest_failed_text: String::new(),
    }
}

pub(in crate::session::tests) fn trinity_string_entry_like_cpp(
    entry: u32,
    content_default: &str,
) -> wow_data::TrinityStringEntryLikeCpp {
    wow_data::TrinityStringEntryLikeCpp {
        entry,
        content: std::array::from_fn(|idx| {
            if idx == 0 {
                content_default.to_string()
            } else {
                String::new()
            }
        }),
    }
}

pub(in crate::session::tests) fn install_access_notification_stores_like_cpp(
    session: &mut WorldSession,
) {
    session.set_trinity_string_store(Arc::new(
        wow_data::TrinityStringStoreLikeCpp::from_entries_like_cpp([
            trinity_string_entry_like_cpp(
                wow_data::LANG_LEVEL_MINREQUIRED_LIKE_CPP,
                "You must be at least level %u to enter.",
            ),
            trinity_string_entry_like_cpp(
                wow_data::LANG_LEVEL_MINREQUIRED_AND_ITEM_LIKE_CPP,
                "You must be at least level %u and have %s to enter.",
            ),
        ]),
    ));
    session.set_item_search_name_store(Arc::new(ItemSearchNameStore::from_entries([
        ItemSearchNameEntry {
            id: 700,
            allowable_race: 0,
            display: "The Workshop Key".to_string(),
            overall_quality_id: 1,
            expansion_id: 0,
            min_faction_id: 0,
            min_reputation: 0,
            allowable_class: 0,
            required_level: 0,
            required_skill: 0,
            required_skill_rank: 0,
            required_ability: 0,
            item_level: 1,
            flags: [0; 4],
        },
        ItemSearchNameEntry {
            id: 701,
            allowable_race: 0,
            display: "The Scarlet Key".to_string(),
            overall_quality_id: 1,
            expansion_id: 0,
            min_faction_id: 0,
            min_reputation: 0,
            allowable_class: 0,
            required_level: 0,
            required_skill: 0,
            required_skill_rank: 0,
            required_ability: 0,
            item_level: 1,
            flags: [0; 4],
        },
    ])));
}

pub(in crate::session::tests) fn install_create_map_encounter_lock_stores_like_cpp(
    session: &mut WorldSession,
    map_id: u32,
    difficulty_id: u8,
    lock_id: u8,
    reset_interval: u8,
) {
    install_create_map_active_lock_stores_like_cpp(
        session,
        map_id,
        difficulty_id,
        lock_id,
        reset_interval,
    );
    session.set_map_difficulty_store(Arc::new(MapDifficultyStore::from_entries([
        MapDifficultyEntry {
            id: 901,
            message: String::new(),
            map_id,
            difficulty_id,
            lock_id,
            reset_interval,
            max_players: 10,
            flags: wow_data::map::MAP_DIFFICULTY_FLAG_USE_LOOT_BASED_LOCK,
        },
    ])));
}

pub(in crate::session::tests) fn install_active_instance_lock_mgr_like_cpp(
    session: &mut WorldSession,
    owner_guid: ObjectGuid,
    map_id: u32,
    difficulty_id: u8,
    instance_id: u32,
) -> u64 {
    let entries = session
        .create_map_db2_entries_like_cpp(map_id, difficulty_id)
        .unwrap();
    let now = u64::try_from(unix_now()).unwrap_or(0);
    let mut mgr = wow_instances::InstanceLockMgr::default();
    let lock = mgr
        .create_instance_lock_for_new_instance_at(
            owner_guid,
            &entries,
            instance_id,
            wow_instances::ResetSchedule::default(),
            now,
        )
        .unwrap();
    let expected_token = create_map_instance_lock_token_like_cpp(owner_guid, &entries, lock);
    session.set_instance_lock_mgr(Arc::new(std::sync::RwLock::new(mgr)));
    expected_token
}
