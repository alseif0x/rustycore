//! Private full-save operations and their exact SQL bindings.
//! Private MariaDB implementation; no semantic port or transaction changes.

use super::save_plan::build_tutorials_save_statement_like_cpp;
use crate::params::PreparedStatement;
use crate::statements::CharStatements;
use wow_persistence::{
    PlayerCufProfileSaveLikeCpp, PlayerEquipmentSetSaveLikeCpp, PlayerVoidStorageSaveLikeCpp,
};

/// Private statement decomposition for the MariaDB adapter.
///
/// This must not cross into `wow-persistence`: the port carries semantic
/// Player groups, while this adapter remains free to change their SQL shape.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum PlayerCharacterSaveStepLikeCpp {
    Position {
        x: f32,
        y: f32,
        z: f32,
        orientation: f32,
        map_id: u16,
        instance_id: u32,
        zone_id: u16,
        guid: u64,
    },
    LevelXp {
        level: u8,
        xp: u32,
        guid: u64,
    },
    Money {
        money: u64,
        guid: u64,
    },
    RestState {
        rest_state: u8,
        player_flags: u32,
        rest_bonus: f32,
        logout_time: u64,
        is_logout_resting: bool,
        guid: u64,
    },
    Health {
        health: u32,
        guid: u64,
    },
    Powers {
        powers: [i32; 10],
        guid: u64,
    },
    TalentReset {
        reset_cost: u32,
        reset_time: u64,
        guid: u64,
    },
    ExploredZones {
        explored_zones: String,
        guid: u64,
    },
    DeleteSpell {
        spell_id: i32,
        guid: u64,
    },
    InsertSpell {
        guid: u64,
        spell_id: i32,
        active: bool,
        disabled: bool,
    },
    UpsertFallbackSpell {
        guid: u64,
        spell_id: i32,
        active: bool,
    },
    DeleteFavoriteSpell {
        guid: u64,
        spell_id: i32,
    },
    InsertFavoriteSpell {
        guid: u64,
        spell_id: i32,
    },
    DeleteSkills {
        guid: u64,
    },
    InsertSkill {
        guid: u64,
        skill_id: u16,
        value: u16,
        max: u16,
        profession_slot: i8,
    },
    Difficulties {
        dungeon: u32,
        raid: u32,
        legacy_raid: u32,
        guid: u64,
    },
    DeleteGlyphs {
        guid: u64,
    },
    InsertGlyph {
        guid: u64,
        talent_group: u8,
        glyph_slot: u8,
        glyph_id: u16,
    },
    DeleteTalents {
        guid: u64,
    },
    InsertTalent {
        guid: u64,
        talent_id: u32,
        rank: u8,
        talent_group: u8,
    },
    DeleteSpellCooldowns {
        guid: u64,
    },
    InsertSpellCooldown {
        guid: u64,
        spell_id: u32,
        item_id: u32,
        cooldown_end: i64,
        category_id: u32,
        category_end: i64,
    },
    DeleteSpellCharges {
        guid: u64,
    },
    InsertSpellCharge {
        guid: u64,
        category_id: u32,
        recharge_start: i64,
        recharge_end: i64,
    },
    DeleteActions {
        guid: u64,
        spec: u8,
        trait_config_id: i32,
    },
    InsertAction {
        guid: u64,
        spec: u8,
        trait_config_id: i32,
        button: u8,
        action: u32,
        action_type: u8,
    },
    InsertEquipmentSet {
        player_guid: u64,
        row: PlayerEquipmentSetSaveLikeCpp,
    },
    UpdateEquipmentSet {
        player_guid: u64,
        row: PlayerEquipmentSetSaveLikeCpp,
    },
    DeleteEquipmentSet {
        set_guid: u64,
    },
    InsertTransmogOutfit {
        player_guid: u64,
        row: PlayerEquipmentSetSaveLikeCpp,
    },
    UpdateTransmogOutfit {
        player_guid: u64,
        row: PlayerEquipmentSetSaveLikeCpp,
    },
    DeleteTransmogOutfit {
        set_guid: u64,
    },
    ReplaceVoidStorageItem {
        player_guid: u64,
        slot: u8,
        row: PlayerVoidStorageSaveLikeCpp,
    },
    DeleteVoidStorageSlot {
        player_guid: u64,
        slot: u8,
    },
    InsertTutorials {
        account_id: u32,
        tutorials: [u32; 8],
    },
    UpdateTutorials {
        account_id: u32,
        tutorials: [u32; 8],
    },
    DeleteInstanceLockTimes {
        account_id: u32,
    },
    InsertInstanceLockTime {
        account_id: u32,
        instance_id: u32,
        release_time: u64,
    },
    PlayedTime {
        total_time: u32,
        level_time: u32,
        guid: u64,
    },
    DeleteReputation {
        guid: u64,
        faction_id: u16,
    },
    InsertReputation {
        guid: u64,
        faction_id: u16,
        standing: i32,
        flags: u16,
    },
    ReplaceCufProfile {
        guid: u64,
        profile_id: u8,
        row: PlayerCufProfileSaveLikeCpp,
    },
    DeleteCufProfile {
        guid: u64,
        profile_id: u8,
    },
}

pub(super) fn player_character_save_statement_like_cpp(
    step: &PlayerCharacterSaveStepLikeCpp,
) -> PreparedStatement {
    use PlayerCharacterSaveStepLikeCpp as Step;

    let statement = match step {
        Step::Position {
            x,
            y,
            z,
            orientation,
            map_id,
            instance_id,
            zone_id,
            guid,
        } => {
            let mut stmt = PreparedStatement::for_statement(
                CharStatements::UPD_CHARACTER_POSITION_PRESERVE_TRAVEL,
            );
            stmt.set_f32(0, *x);
            stmt.set_f32(1, *y);
            stmt.set_f32(2, *z);
            stmt.set_f32(3, *orientation);
            stmt.set_u16(4, *map_id);
            stmt.set_u32(5, *instance_id);
            stmt.set_u16(6, *zone_id);
            stmt.set_u64(7, *guid);
            stmt
        }
        Step::LevelXp { level, xp, guid } => {
            let mut stmt = PreparedStatement::for_statement(CharStatements::UPD_CHAR_LEVEL);
            stmt.set_u8(0, *level);
            stmt.set_u32(1, *xp);
            stmt.set_u64(2, *guid);
            stmt
        }
        Step::Money { money, guid } => {
            let mut stmt = PreparedStatement::for_statement(CharStatements::UPD_CHAR_MONEY);
            stmt.set_u64(0, *money);
            stmt.set_u64(1, *guid);
            stmt
        }
        Step::RestState {
            rest_state,
            player_flags,
            rest_bonus,
            logout_time,
            is_logout_resting,
            guid,
        } => {
            let mut stmt = PreparedStatement::for_statement(CharStatements::UPD_CHAR_REST_STATE);
            stmt.set_u8(0, *rest_state);
            stmt.set_u32(1, *player_flags);
            stmt.set_f32(2, *rest_bonus);
            stmt.set_u64(3, *logout_time);
            stmt.set_bool(4, *is_logout_resting);
            stmt.set_u64(5, *guid);
            stmt
        }
        Step::Health { health, guid } => {
            let mut stmt = PreparedStatement::for_statement(CharStatements::UPD_CHAR_HEALTH);
            stmt.set_u32(0, *health);
            stmt.set_u64(1, *guid);
            stmt
        }
        Step::Powers { powers, guid } => {
            let mut stmt = PreparedStatement::for_statement(CharStatements::UPD_CHAR_POWERS);
            for (index, power) in powers.iter().copied().enumerate() {
                stmt.set_i32(index, power.max(0));
            }
            stmt.set_u64(10, *guid);
            stmt
        }
        Step::TalentReset {
            reset_cost,
            reset_time,
            guid,
        } => {
            let mut stmt =
                PreparedStatement::for_statement(CharStatements::UPD_CHAR_TALENT_RESET_STATE);
            stmt.set_u32(0, *reset_cost);
            stmt.set_u64(1, *reset_time);
            stmt.set_u64(2, *guid);
            stmt
        }
        Step::ExploredZones {
            explored_zones,
            guid,
        } => {
            let mut stmt =
                PreparedStatement::for_statement(CharStatements::UPD_CHAR_EXPLORED_ZONES);
            stmt.set_string(0, explored_zones.clone());
            stmt.set_u64(1, *guid);
            stmt
        }
        Step::DeleteSpell { spell_id, guid } => {
            let mut stmt =
                PreparedStatement::for_statement(CharStatements::DEL_CHAR_SPELL_BY_SPELL);
            stmt.set_i32(0, *spell_id);
            stmt.set_u64(1, *guid);
            stmt
        }
        Step::InsertSpell {
            guid,
            spell_id,
            active,
            disabled,
        } => {
            let mut stmt = PreparedStatement::for_statement(CharStatements::INS_CHAR_SPELL);
            stmt.set_u64(0, *guid);
            stmt.set_i32(1, *spell_id);
            stmt.set_bool(2, *active);
            stmt.set_bool(3, *disabled);
            stmt
        }
        Step::UpsertFallbackSpell {
            guid,
            spell_id,
            active,
        } => {
            let mut stmt =
                PreparedStatement::for_statement(CharStatements::UPSERT_CHAR_SPELL_LEARN_FALLBACK);
            stmt.set_u64(0, *guid);
            stmt.set_i32(1, *spell_id);
            stmt.set_bool(2, *active);
            stmt.set_bool(3, false);
            stmt
        }
        Step::DeleteFavoriteSpell { guid, spell_id } => {
            let mut stmt =
                PreparedStatement::for_statement(CharStatements::DEL_CHAR_SPELL_FAVORITE);
            stmt.set_u64(0, *guid);
            stmt.set_i32(1, *spell_id);
            stmt
        }
        Step::InsertFavoriteSpell { guid, spell_id } => {
            let mut stmt =
                PreparedStatement::for_statement(CharStatements::INS_CHAR_SPELL_FAVORITE);
            stmt.set_u64(0, *guid);
            stmt.set_i32(1, *spell_id);
            stmt
        }
        Step::DeleteSkills { guid } => {
            let mut stmt = PreparedStatement::for_statement(CharStatements::DEL_CHAR_SKILLS);
            stmt.set_u64(0, *guid);
            stmt
        }
        Step::InsertSkill {
            guid,
            skill_id,
            value,
            max,
            profession_slot,
        } => {
            let mut stmt = PreparedStatement::for_statement(CharStatements::INS_CHAR_SKILLS);
            stmt.set_u64(0, *guid);
            stmt.set_u16(1, *skill_id);
            stmt.set_u16(2, *value);
            stmt.set_u16(3, *max);
            stmt.set_i8(4, *profession_slot);
            stmt
        }
        Step::Difficulties {
            dungeon,
            raid,
            legacy_raid,
            guid,
        } => {
            let mut stmt = PreparedStatement::for_statement(CharStatements::UPD_CHAR_DIFFICULTIES);
            stmt.set_u32(0, *dungeon);
            stmt.set_u32(1, *raid);
            stmt.set_u32(2, *legacy_raid);
            stmt.set_u64(3, *guid);
            stmt
        }
        Step::DeleteGlyphs { guid } => {
            let mut stmt = PreparedStatement::for_statement(CharStatements::DEL_CHAR_GLYPHS);
            stmt.set_u64(0, *guid);
            stmt
        }
        Step::InsertGlyph {
            guid,
            talent_group,
            glyph_slot,
            glyph_id,
        } => {
            let mut stmt = PreparedStatement::for_statement(CharStatements::INS_CHAR_GLYPHS);
            stmt.set_u64(0, *guid);
            stmt.set_u8(1, *talent_group);
            stmt.set_u8(2, *glyph_slot);
            stmt.set_u16(3, *glyph_id);
            stmt
        }
        Step::DeleteTalents { guid } => {
            let mut stmt = PreparedStatement::for_statement(CharStatements::DEL_CHAR_TALENT);
            stmt.set_u64(0, *guid);
            stmt
        }
        Step::InsertTalent {
            guid,
            talent_id,
            rank,
            talent_group,
        } => {
            let mut stmt = PreparedStatement::for_statement(CharStatements::INS_CHAR_TALENT);
            stmt.set_u64(0, *guid);
            stmt.set_u32(1, *talent_id);
            stmt.set_u8(2, *rank);
            stmt.set_u8(3, *talent_group);
            stmt
        }
        Step::DeleteSpellCooldowns { guid } => {
            let mut stmt =
                PreparedStatement::for_statement(CharStatements::DEL_CHAR_SPELL_COOLDOWNS);
            stmt.set_u64(0, *guid);
            stmt
        }
        Step::InsertSpellCooldown {
            guid,
            spell_id,
            item_id,
            cooldown_end,
            category_id,
            category_end,
        } => {
            let mut stmt =
                PreparedStatement::for_statement(CharStatements::INS_CHAR_SPELL_COOLDOWN);
            stmt.set_u64(0, *guid);
            stmt.set_u32(1, *spell_id);
            stmt.set_u32(2, *item_id);
            stmt.set_i64(3, *cooldown_end);
            stmt.set_u32(4, *category_id);
            stmt.set_i64(5, *category_end);
            stmt
        }
        Step::DeleteSpellCharges { guid } => {
            let mut stmt = PreparedStatement::for_statement(CharStatements::DEL_CHAR_SPELL_CHARGES);
            stmt.set_u64(0, *guid);
            stmt
        }
        Step::InsertSpellCharge {
            guid,
            category_id,
            recharge_start,
            recharge_end,
        } => {
            let mut stmt = PreparedStatement::for_statement(CharStatements::INS_CHAR_SPELL_CHARGES);
            stmt.set_u64(0, *guid);
            stmt.set_u32(1, *category_id);
            stmt.set_i64(2, *recharge_start);
            stmt.set_i64(3, *recharge_end);
            stmt
        }
        Step::DeleteActions {
            guid,
            spec,
            trait_config_id,
        } => {
            let mut stmt =
                PreparedStatement::for_statement(CharStatements::DEL_CHAR_ACTION_BY_SPEC);
            stmt.set_u64(0, *guid);
            stmt.set_u8(1, *spec);
            stmt.set_i32(2, *trait_config_id);
            stmt
        }
        Step::InsertAction {
            guid,
            spec,
            trait_config_id,
            button,
            action,
            action_type,
        } => {
            let mut stmt = PreparedStatement::for_statement(CharStatements::INS_CHAR_ACTION);
            stmt.set_u64(0, *guid);
            stmt.set_u8(1, *spec);
            stmt.set_i32(2, *trait_config_id);
            stmt.set_u8(3, *button);
            stmt.set_u32(4, *action);
            stmt.set_u8(5, *action_type);
            stmt
        }
        Step::InsertEquipmentSet { player_guid, row } => {
            let mut stmt = PreparedStatement::for_statement(CharStatements::INS_EQUIP_SET);
            stmt.set_u64(0, *player_guid);
            stmt.set_u64(1, row.set_guid);
            stmt.set_u32(2, row.set_id);
            stmt.set_string(3, row.name.clone());
            stmt.set_string(4, row.icon.clone());
            stmt.set_u32(5, row.ignore_mask);
            stmt.set_i32(6, row.assigned_spec_index);
            for (offset, piece) in row.pieces.iter().copied().enumerate() {
                stmt.set_u64(7 + offset, piece);
            }
            stmt
        }
        Step::UpdateEquipmentSet { player_guid, row } => {
            let mut stmt = PreparedStatement::for_statement(CharStatements::UPD_EQUIP_SET);
            stmt.set_string(0, row.name.clone());
            stmt.set_string(1, row.icon.clone());
            stmt.set_u32(2, row.ignore_mask);
            stmt.set_i32(3, row.assigned_spec_index);
            for (offset, piece) in row.pieces.iter().copied().enumerate() {
                stmt.set_u64(4 + offset, piece);
            }
            stmt.set_u64(23, *player_guid);
            stmt.set_u64(24, row.set_guid);
            stmt.set_u32(25, row.set_id);
            stmt
        }
        Step::DeleteEquipmentSet { set_guid } => {
            let mut stmt = PreparedStatement::for_statement(CharStatements::DEL_EQUIP_SET);
            stmt.set_u64(0, *set_guid);
            stmt
        }
        Step::InsertTransmogOutfit { player_guid, row } => {
            let mut stmt = PreparedStatement::for_statement(CharStatements::INS_TRANSMOG_OUTFIT);
            stmt.set_u64(0, *player_guid);
            stmt.set_u64(1, row.set_guid);
            stmt.set_u32(2, row.set_id);
            stmt.set_string(3, row.name.clone());
            stmt.set_string(4, row.icon.clone());
            stmt.set_u32(5, row.ignore_mask);
            for (offset, appearance) in row.appearances.iter().copied().enumerate() {
                stmt.set_i32(6 + offset, appearance);
            }
            stmt.set_i32(25, row.enchants[0]);
            stmt.set_i32(26, row.enchants[1]);
            stmt
        }
        Step::UpdateTransmogOutfit { player_guid, row } => {
            let mut stmt = PreparedStatement::for_statement(CharStatements::UPD_TRANSMOG_OUTFIT);
            stmt.set_string(0, row.name.clone());
            stmt.set_string(1, row.icon.clone());
            stmt.set_u32(2, row.ignore_mask);
            for (offset, appearance) in row.appearances.iter().copied().enumerate() {
                stmt.set_i32(3 + offset, appearance);
            }
            stmt.set_i32(22, row.enchants[0]);
            stmt.set_i32(23, row.enchants[1]);
            stmt.set_u64(24, *player_guid);
            stmt.set_u64(25, row.set_guid);
            stmt.set_u32(26, row.set_id);
            stmt
        }
        Step::DeleteTransmogOutfit { set_guid } => {
            let mut stmt = PreparedStatement::for_statement(CharStatements::DEL_TRANSMOG_OUTFIT);
            stmt.set_u64(0, *set_guid);
            stmt
        }
        Step::ReplaceVoidStorageItem {
            player_guid,
            slot,
            row,
        } => {
            let mut stmt =
                PreparedStatement::for_statement(CharStatements::REP_CHAR_VOID_STORAGE_ITEM);
            stmt.set_u64(0, row.item_id);
            stmt.set_u64(1, *player_guid);
            stmt.set_u32(2, row.item_entry);
            stmt.set_u8(3, *slot);
            stmt.set_u64(4, row.creator_guid);
            stmt.set_u32(5, row.fixed_scaling_level);
            stmt.set_i32(6, row.random_properties_id);
            stmt.set_i32(7, row.random_properties_seed);
            stmt.set_u8(8, row.context);
            stmt
        }
        Step::DeleteVoidStorageSlot { player_guid, slot } => {
            let mut stmt = PreparedStatement::for_statement(
                CharStatements::DEL_CHAR_VOID_STORAGE_ITEM_BY_SLOT,
            );
            stmt.set_u8(0, *slot);
            stmt.set_u64(1, *player_guid);
            stmt
        }
        Step::InsertTutorials {
            account_id,
            tutorials,
        }
        | Step::UpdateTutorials {
            account_id,
            tutorials,
        } => build_tutorials_save_statement_like_cpp(
            *account_id,
            tutorials,
            matches!(step, Step::UpdateTutorials { .. }),
        ),
        Step::DeleteInstanceLockTimes { account_id } => {
            let mut stmt =
                PreparedStatement::for_statement(CharStatements::DEL_ACCOUNT_INSTANCE_LOCK_TIMES);
            stmt.set_u32(0, *account_id);
            stmt
        }
        Step::InsertInstanceLockTime {
            account_id,
            instance_id,
            release_time,
        } => {
            let mut stmt =
                PreparedStatement::for_statement(CharStatements::INS_ACCOUNT_INSTANCE_LOCK_TIMES);
            stmt.set_u32(0, *account_id);
            stmt.set_u32(1, *instance_id);
            stmt.set_u64(2, *release_time);
            stmt
        }
        Step::PlayedTime {
            total_time,
            level_time,
            guid,
        } => {
            let mut stmt = PreparedStatement::for_statement(CharStatements::UPD_CHAR_PLAYED_TIME);
            stmt.set_u32(0, *total_time);
            stmt.set_u32(1, *level_time);
            stmt.set_u64(2, *guid);
            stmt
        }
        Step::DeleteReputation { guid, faction_id } => {
            let mut stmt =
                PreparedStatement::for_statement(CharStatements::DEL_CHAR_REPUTATION_BY_FACTION);
            stmt.set_u64(0, *guid);
            stmt.set_u16(1, *faction_id);
            stmt
        }
        Step::InsertReputation {
            guid,
            faction_id,
            standing,
            flags,
        } => {
            let mut stmt =
                PreparedStatement::for_statement(CharStatements::INS_CHAR_REPUTATION_BY_FACTION);
            stmt.set_u64(0, *guid);
            stmt.set_u16(1, *faction_id);
            stmt.set_i32(2, *standing);
            stmt.set_u16(3, *flags);
            stmt
        }
        Step::ReplaceCufProfile {
            guid,
            profile_id,
            row,
        } => {
            let mut stmt = PreparedStatement::for_statement(CharStatements::REP_CHAR_CUF_PROFILES);
            stmt.set_u64(0, *guid);
            stmt.set_u8(1, *profile_id);
            stmt.set_string(2, row.profile_name.clone());
            stmt.set_u16(3, row.frame_height);
            stmt.set_u16(4, row.frame_width);
            stmt.set_u8(5, row.sort_by);
            stmt.set_u8(6, row.health_text);
            stmt.set_u32(7, row.bool_options);
            stmt.set_u8(8, row.top_point);
            stmt.set_u8(9, row.bottom_point);
            stmt.set_u8(10, row.left_point);
            stmt.set_u16(11, row.top_offset);
            stmt.set_u16(12, row.bottom_offset);
            stmt.set_u16(13, row.left_offset);
            stmt
        }
        Step::DeleteCufProfile { guid, profile_id } => {
            let mut stmt =
                PreparedStatement::for_statement(CharStatements::DEL_CHAR_CUF_PROFILES_BY_ID);
            stmt.set_u64(0, *guid);
            stmt.set_u8(1, *profile_id);
            stmt
        }
    };
    statement
}
