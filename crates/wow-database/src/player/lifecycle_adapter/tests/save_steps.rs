use super::super::save_steps::PlayerCharacterSaveStepLikeCpp;
use super::super::save_steps::player_character_save_statement_like_cpp;
use super::equipment_row;
use crate::statements::CharStatements;
use crate::statements::StatementDef;
use wow_persistence::{PlayerCufProfileSaveLikeCpp, PlayerVoidStorageSaveLikeCpp};

#[test]
fn every_private_character_save_operation_maps_to_the_existing_mariadb_statement_like_cpp() {
    use PlayerCharacterSaveStepLikeCpp as Step;

    let cuf = PlayerCufProfileSaveLikeCpp {
        profile_name: "profile".to_owned(),
        frame_height: 1,
        frame_width: 1,
        sort_by: 0,
        health_text: 0,
        bool_options: 0,
        top_point: 0,
        bottom_point: 0,
        left_point: 0,
        top_offset: 0,
        bottom_offset: 0,
        left_offset: 0,
    };
    let void_item = PlayerVoidStorageSaveLikeCpp {
        item_id: 1,
        item_entry: 3,
        creator_guid: 5,
        fixed_scaling_level: 6,
        random_properties_id: 7,
        random_properties_seed: 8,
        context: 9,
    };
    let cases = vec![
        (
            Step::Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                orientation: 0.0,
                map_id: 0,
                instance_id: 0,
                zone_id: 0,
                guid: 1,
            },
            CharStatements::UPD_CHARACTER_POSITION_PRESERVE_TRAVEL,
        ),
        (
            Step::LevelXp {
                level: 1,
                xp: 2,
                guid: 3,
            },
            CharStatements::UPD_CHAR_LEVEL,
        ),
        (
            Step::Money { money: 1, guid: 2 },
            CharStatements::UPD_CHAR_MONEY,
        ),
        (
            Step::RestState {
                rest_state: 0,
                player_flags: 0,
                rest_bonus: 0.0,
                logout_time: 0,
                is_logout_resting: false,
                guid: 1,
            },
            CharStatements::UPD_CHAR_REST_STATE,
        ),
        (
            Step::Health { health: 1, guid: 2 },
            CharStatements::UPD_CHAR_HEALTH,
        ),
        (
            Step::Powers {
                powers: [0; 10],
                guid: 1,
            },
            CharStatements::UPD_CHAR_POWERS,
        ),
        (
            Step::TalentReset {
                reset_cost: 0,
                reset_time: 0,
                guid: 1,
            },
            CharStatements::UPD_CHAR_TALENT_RESET_STATE,
        ),
        (
            Step::ExploredZones {
                explored_zones: String::new(),
                guid: 1,
            },
            CharStatements::UPD_CHAR_EXPLORED_ZONES,
        ),
        (
            Step::DeleteSpell {
                spell_id: 1,
                guid: 2,
            },
            CharStatements::DEL_CHAR_SPELL_BY_SPELL,
        ),
        (
            Step::InsertSpell {
                guid: 1,
                spell_id: 2,
                active: true,
                disabled: false,
            },
            CharStatements::INS_CHAR_SPELL,
        ),
        (
            Step::UpsertFallbackSpell {
                guid: 1,
                spell_id: 2,
                active: true,
            },
            CharStatements::UPSERT_CHAR_SPELL_LEARN_FALLBACK,
        ),
        (
            Step::DeleteFavoriteSpell {
                guid: 1,
                spell_id: 2,
            },
            CharStatements::DEL_CHAR_SPELL_FAVORITE,
        ),
        (
            Step::InsertFavoriteSpell {
                guid: 1,
                spell_id: 2,
            },
            CharStatements::INS_CHAR_SPELL_FAVORITE,
        ),
        (
            Step::DeleteSkills { guid: 1 },
            CharStatements::DEL_CHAR_SKILLS,
        ),
        (
            Step::InsertSkill {
                guid: 1,
                skill_id: 2,
                value: 3,
                max: 4,
                profession_slot: -1,
            },
            CharStatements::INS_CHAR_SKILLS,
        ),
        (
            Step::Difficulties {
                dungeon: 1,
                raid: 2,
                legacy_raid: 3,
                guid: 4,
            },
            CharStatements::UPD_CHAR_DIFFICULTIES,
        ),
        (
            Step::DeleteGlyphs { guid: 1 },
            CharStatements::DEL_CHAR_GLYPHS,
        ),
        (
            Step::InsertGlyph {
                guid: 1,
                talent_group: 0,
                glyph_slot: 0,
                glyph_id: 2,
            },
            CharStatements::INS_CHAR_GLYPHS,
        ),
        (
            Step::DeleteTalents { guid: 1 },
            CharStatements::DEL_CHAR_TALENT,
        ),
        (
            Step::InsertTalent {
                guid: 1,
                talent_id: 2,
                rank: 3,
                talent_group: 0,
            },
            CharStatements::INS_CHAR_TALENT,
        ),
        (
            Step::DeleteSpellCooldowns { guid: 1 },
            CharStatements::DEL_CHAR_SPELL_COOLDOWNS,
        ),
        (
            Step::InsertSpellCooldown {
                guid: 1,
                spell_id: 2,
                item_id: 3,
                cooldown_end: 4,
                category_id: 5,
                category_end: 6,
            },
            CharStatements::INS_CHAR_SPELL_COOLDOWN,
        ),
        (
            Step::DeleteSpellCharges { guid: 1 },
            CharStatements::DEL_CHAR_SPELL_CHARGES,
        ),
        (
            Step::InsertSpellCharge {
                guid: 1,
                category_id: 2,
                recharge_start: 3,
                recharge_end: 4,
            },
            CharStatements::INS_CHAR_SPELL_CHARGES,
        ),
        (
            Step::DeleteActions {
                guid: 1,
                spec: 0,
                trait_config_id: 2,
            },
            CharStatements::DEL_CHAR_ACTION_BY_SPEC,
        ),
        (
            Step::InsertAction {
                guid: 1,
                spec: 0,
                trait_config_id: 2,
                button: 3,
                action: 4,
                action_type: 5,
            },
            CharStatements::INS_CHAR_ACTION,
        ),
        (
            Step::InsertEquipmentSet {
                player_guid: 1,
                row: equipment_row(),
            },
            CharStatements::INS_EQUIP_SET,
        ),
        (
            Step::UpdateEquipmentSet {
                player_guid: 1,
                row: equipment_row(),
            },
            CharStatements::UPD_EQUIP_SET,
        ),
        (
            Step::DeleteEquipmentSet { set_guid: 1 },
            CharStatements::DEL_EQUIP_SET,
        ),
        (
            Step::InsertTransmogOutfit {
                player_guid: 1,
                row: equipment_row(),
            },
            CharStatements::INS_TRANSMOG_OUTFIT,
        ),
        (
            Step::UpdateTransmogOutfit {
                player_guid: 1,
                row: equipment_row(),
            },
            CharStatements::UPD_TRANSMOG_OUTFIT,
        ),
        (
            Step::DeleteTransmogOutfit { set_guid: 1 },
            CharStatements::DEL_TRANSMOG_OUTFIT,
        ),
        (
            Step::ReplaceVoidStorageItem {
                player_guid: 2,
                slot: 4,
                row: void_item,
            },
            CharStatements::REP_CHAR_VOID_STORAGE_ITEM,
        ),
        (
            Step::DeleteVoidStorageSlot {
                player_guid: 1,
                slot: 2,
            },
            CharStatements::DEL_CHAR_VOID_STORAGE_ITEM_BY_SLOT,
        ),
        (
            Step::InsertTutorials {
                account_id: 1,
                tutorials: [0; 8],
            },
            CharStatements::INS_TUTORIALS,
        ),
        (
            Step::UpdateTutorials {
                account_id: 1,
                tutorials: [0; 8],
            },
            CharStatements::UPD_TUTORIALS,
        ),
        (
            Step::DeleteInstanceLockTimes { account_id: 1 },
            CharStatements::DEL_ACCOUNT_INSTANCE_LOCK_TIMES,
        ),
        (
            Step::InsertInstanceLockTime {
                account_id: 1,
                instance_id: 2,
                release_time: 3,
            },
            CharStatements::INS_ACCOUNT_INSTANCE_LOCK_TIMES,
        ),
        (
            Step::PlayedTime {
                total_time: 1,
                level_time: 2,
                guid: 3,
            },
            CharStatements::UPD_CHAR_PLAYED_TIME,
        ),
        (
            Step::DeleteReputation {
                guid: 1,
                faction_id: 2,
            },
            CharStatements::DEL_CHAR_REPUTATION_BY_FACTION,
        ),
        (
            Step::InsertReputation {
                guid: 1,
                faction_id: 2,
                standing: 3,
                flags: 4,
            },
            CharStatements::INS_CHAR_REPUTATION_BY_FACTION,
        ),
        (
            Step::ReplaceCufProfile {
                guid: 1,
                profile_id: 0,
                row: cuf,
            },
            CharStatements::REP_CHAR_CUF_PROFILES,
        ),
        (
            Step::DeleteCufProfile {
                guid: 1,
                profile_id: 0,
            },
            CharStatements::DEL_CHAR_CUF_PROFILES_BY_ID,
        ),
    ];

    for (step, expected) in cases {
        assert_eq!(
            player_character_save_statement_like_cpp(&step).sql(),
            expected.sql(),
            "{step:?} must retain its MariaDB statement",
        );
    }
}
