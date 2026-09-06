use wow_persistence::*;

fn equipment_row() -> PlayerEquipmentSetSaveLikeCpp {
    PlayerEquipmentSetSaveLikeCpp {
        set_guid: 2,
        set_id: 3,
        set_type: PlayerEquipmentSetTypeLikeCpp::Equipment,
        state: PlayerEquipmentSetStateLikeCpp::New,
        name: "set".to_owned(),
        icon: "icon".to_owned(),
        ignore_mask: 4,
        assigned_spec_index: 5,
        pieces: vec![0; 19],
        appearances: vec![0; 19],
        enchants: [0; 2],
    }
}

fn minimal_character_request() -> PlayerCharacterSaveRequestLikeCpp {
    PlayerCharacterSaveRequestLikeCpp {
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
            powers: None,
            talent_reset_cost: 0,
            talent_reset_time: 0,
            explored_zones: String::new(),
            dungeon_difficulty: 0,
            raid_difficulty: 0,
            legacy_raid_difficulty: 0,
        },
        spells: None,
        skills: None,
        glyphs: None,
        talents: None,
        spell_cooldowns: None,
        spell_charges: None,
        action_buttons: None,
        equipment_sets: None,
        void_storage: None,
        tutorials: None,
        instance_lock_times: Vec::new(),
        played_time: PlayerPlayedTimeSaveLikeCpp {
            total_time: 11,
            level_time: 5,
        },
        reputations: Vec::new(),
        cuf_profiles: None,
    }
}

mod economy;
mod login;
mod save_order;
mod save_steps;
