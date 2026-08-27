// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! MariaDB adapter for the Player lifecycle port.
//!
//! `wow-persistence` says *what* to persist and how the outcome is classified;
//! this is the only place that knows the statement, the pool and the driver
//! error. Keeping the mapping here is the point of the split: the Session sees
//! `Applied` / `Failed` / `Unknown` and never a `sqlx::Error`.

use std::sync::Arc;

use wow_persistence::{
    AccountCollectionLoadOutcomeLikeCpp, AccountCollectionLoadRequestLikeCpp,
    AccountCollectionLoadedLikeCpp, AccountCollectionRowsLikeCpp, AccountCollectionSaveLikeCpp,
    AccountHeirloomLoadRowLikeCpp, AccountMaskBlockLikeCpp, AccountMountLoadRowLikeCpp,
    AccountToyLoadRowLikeCpp, PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp,
    PlayerBattlegroundLocationLoadRowLikeCpp, PlayerBuybackClearRequestLikeCpp,
    PlayerCharacterBaseLoadOutcomeLikeCpp, PlayerCharacterBaseLoadRequestLikeCpp,
    PlayerCharacterBaseLoadRowLikeCpp, PlayerCharacterSaveRequestLikeCpp,
    PlayerCharacterSaveResultLikeCpp, PlayerCufProfileSaveLikeCpp,
    PlayerCustomizationLoadRowLikeCpp, PlayerEquipmentSetSaveLikeCpp,
    PlayerEquipmentSetStateLikeCpp, PlayerEquipmentSetTypeLikeCpp,
    PlayerGuildMembershipLoadRowLikeCpp, PlayerHomebindLocationLoadRowLikeCpp,
    PlayerHomebindPersistenceRequestLikeCpp, PlayerInitialWorldStateRowsLikeCpp,
    PlayerInitialWorldStateTemplateRowLikeCpp, PlayerInitialWorldStateValueRowLikeCpp,
    PlayerInitialWorldStatesLoadOutcomeLikeCpp, PlayerInstanceTimeRestrictionLoadRowLikeCpp,
    PlayerLifecyclePortLikeCpp, PlayerLoginAdmissionLoadOutcomeLikeCpp,
    PlayerLoginAdmissionLoadRequestLikeCpp, PlayerLoginAdmissionLoadedLikeCpp,
    PlayerLoginAuxiliaryLoadOutcomeLikeCpp, PlayerLoginAuxiliaryLoadRequestLikeCpp,
    PlayerLoginAuxiliaryLoadedLikeCpp, PlayerLoginTransportLoadOutcomeLikeCpp,
    PlayerLoginTransportLoadRequestLikeCpp, PlayerLoginTransportLoadRowLikeCpp,
    PlayerOfflineMarkLikeCpp, PlayerPetAuraEffectLoadRowLikeCpp, PlayerPetAuraLoadRowLikeCpp,
    PlayerPetDeclinedNamesLoadRowLikeCpp, PlayerPetSpellChargeLoadRowLikeCpp,
    PlayerPetSpellCooldownLoadRowLikeCpp, PlayerPetSpellLoadRowLikeCpp,
    PlayerPetStableLoadRowLikeCpp, PlayerRealmCharacterCountRefreshRequestLikeCpp,
    PlayerSpellChargeLoadRowLikeCpp, PlayerSpellCooldownLoadRowLikeCpp,
    PlayerSpellSaveGroupLikeCpp, PlayerSpellStateLikeCpp,
    PlayerTalentResetPersistenceRequestLikeCpp, PlayerTraitConfigLoadRowLikeCpp,
    PlayerTraitEntryLoadRowLikeCpp, PlayerVoidStorageSaveLikeCpp,
    PlayerXpPersistenceRequestLikeCpp,
};

use crate::params::PreparedStatement;
use crate::statements::{CharStatements, LoginStatements, StatementDef, WorldStatements};
use crate::transaction::SqlTransaction;
use crate::{CharacterDatabase, LoginDatabase, SqlTransactionCommitError, WorldDatabase};

/// Private statement decomposition for the MariaDB adapter.
///
/// This must not cross into `wow-persistence`: the port carries semantic
/// Player groups, while this adapter remains free to change their SQL shape.
#[derive(Debug, Clone, PartialEq)]
enum PlayerCharacterSaveStepLikeCpp {
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

fn player_character_save_statement_like_cpp(
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

fn player_character_save_statements_like_cpp(
    request: &PlayerCharacterSaveRequestLikeCpp,
) -> Vec<PreparedStatement> {
    use PlayerCharacterSaveStepLikeCpp as Step;

    let guid = request.player_guid;
    let account_id = request.account_id;
    let character = &request.character;
    let mut steps = vec![
        Step::Position {
            x: character.position.x,
            y: character.position.y,
            z: character.position.z,
            orientation: character.position.orientation,
            map_id: character.position.map_id,
            instance_id: character.position.instance_id,
            zone_id: character.position.zone_id,
            guid,
        },
        Step::LevelXp {
            level: character.level,
            xp: character.xp,
            guid,
        },
        Step::Money {
            money: character.money,
            guid,
        },
        Step::RestState {
            rest_state: character.rest_state,
            player_flags: character.player_flags,
            rest_bonus: character.rest_bonus,
            logout_time: character.logout_time,
            is_logout_resting: character.is_logout_resting,
            guid,
        },
        Step::Health {
            health: character.health,
            guid,
        },
    ];
    if let Some(powers) = character.powers {
        steps.push(Step::Powers { powers, guid });
    }
    steps.extend([
        Step::TalentReset {
            reset_cost: character.talent_reset_cost,
            reset_time: character.talent_reset_time,
            guid,
        },
        Step::ExploredZones {
            explored_zones: character.explored_zones.clone(),
            guid,
        },
    ]);

    match &request.spells {
        Some(PlayerSpellSaveGroupLikeCpp::Complete { rows, .. }) => {
            let mut rows = rows.clone();
            rows.sort_by_key(|spell| spell.spell_id);
            for spell in rows {
                if matches!(
                    spell.state,
                    PlayerSpellStateLikeCpp::Removed | PlayerSpellStateLikeCpp::Changed
                ) {
                    steps.push(Step::DeleteSpell {
                        spell_id: spell.spell_id,
                        guid,
                    });
                }
                if matches!(
                    spell.state,
                    PlayerSpellStateLikeCpp::New | PlayerSpellStateLikeCpp::Changed
                ) {
                    if !spell.dependent {
                        steps.push(Step::InsertSpell {
                            guid,
                            spell_id: spell.spell_id,
                            active: spell.active,
                            disabled: spell.disabled,
                        });
                    }
                    steps.push(Step::DeleteFavoriteSpell {
                        guid,
                        spell_id: spell.spell_id,
                    });
                    if spell.favorite {
                        steps.push(Step::InsertFavoriteSpell {
                            guid,
                            spell_id: spell.spell_id,
                        });
                    }
                }
            }
        }
        Some(PlayerSpellSaveGroupLikeCpp::Fallback { rows }) => {
            let mut rows = rows.clone();
            rows.sort_by_key(|spell| spell.spell_id);
            for spell in rows {
                steps.push(if spell.dependent {
                    Step::DeleteSpell {
                        spell_id: spell.spell_id,
                        guid,
                    }
                } else {
                    Step::UpsertFallbackSpell {
                        guid,
                        spell_id: spell.spell_id,
                        active: spell.active,
                    }
                });
            }
        }
        None => {}
    }

    if let Some(skills) = &request.skills {
        steps.push(Step::DeleteSkills { guid });
        let mut skills = skills.clone();
        skills.sort_by_key(|skill| skill.skill_id);
        steps.extend(skills.into_iter().map(|skill| Step::InsertSkill {
            guid,
            skill_id: skill.skill_id,
            value: skill.value,
            max: skill.max,
            profession_slot: skill.profession_slot,
        }));
    }

    steps.push(Step::Difficulties {
        dungeon: character.dungeon_difficulty,
        raid: character.raid_difficulty,
        legacy_raid: character.legacy_raid_difficulty,
        guid,
    });

    if let Some(glyphs) = &request.glyphs {
        steps.push(Step::DeleteGlyphs { guid });
        steps.extend(glyphs.iter().map(|glyph| Step::InsertGlyph {
            guid,
            talent_group: glyph.talent_group,
            glyph_slot: glyph.glyph_slot,
            glyph_id: glyph.glyph_id,
        }));
    }
    if let Some(talents) = &request.talents {
        steps.push(Step::DeleteTalents { guid });
        steps.extend(talents.iter().map(|talent| Step::InsertTalent {
            guid,
            talent_id: talent.talent_id,
            rank: talent.rank,
            talent_group: talent.talent_group,
        }));
    }
    if let Some(cooldowns) = &request.spell_cooldowns {
        steps.push(Step::DeleteSpellCooldowns { guid });
        let mut cooldowns = cooldowns
            .iter()
            .copied()
            .filter(|cooldown| {
                cooldown.cooldown_end_unix_secs > request.wall_clock_unix_secs
                    || cooldown.category_end_unix_secs > request.wall_clock_unix_secs
            })
            .collect::<Vec<_>>();
        cooldowns.sort_by_key(|cooldown| cooldown.spell_id);
        steps.extend(
            cooldowns
                .into_iter()
                .map(|cooldown| Step::InsertSpellCooldown {
                    guid,
                    spell_id: cooldown.spell_id,
                    item_id: cooldown.item_id,
                    cooldown_end: cooldown.cooldown_end_unix_secs,
                    category_id: cooldown.category_id,
                    category_end: cooldown.category_end_unix_secs,
                }),
        );
    }
    if let Some(charges) = &request.spell_charges {
        steps.push(Step::DeleteSpellCharges { guid });
        steps.extend(
            charges
                .iter()
                .copied()
                .filter(|charge| charge.recharge_end_unix_secs > request.wall_clock_unix_secs)
                .map(|charge| Step::InsertSpellCharge {
                    guid,
                    category_id: charge.category_id,
                    recharge_start: charge.recharge_start_unix_secs,
                    recharge_end: charge.recharge_end_unix_secs,
                }),
        );
    }
    if let Some(actions) = &request.action_buttons {
        steps.push(Step::DeleteActions {
            guid,
            spec: actions.spec,
            trait_config_id: actions.trait_config_id,
        });
        steps.extend(actions.rows.iter().map(|button| Step::InsertAction {
            guid,
            spec: actions.spec,
            trait_config_id: actions.trait_config_id,
            button: button.button,
            action: button.packed_action & 0x00FF_FFFF,
            action_type: (button.packed_action >> 24) as u8,
        }));
    }
    if let Some(equipment_sets) = &request.equipment_sets {
        for row in equipment_sets {
            let step = match (row.state, row.set_type) {
                (PlayerEquipmentSetStateLikeCpp::Unchanged, _) => None,
                (
                    PlayerEquipmentSetStateLikeCpp::Deleted,
                    PlayerEquipmentSetTypeLikeCpp::Equipment,
                ) => Some(Step::DeleteEquipmentSet {
                    set_guid: row.set_guid,
                }),
                (
                    PlayerEquipmentSetStateLikeCpp::Deleted,
                    PlayerEquipmentSetTypeLikeCpp::Transmog,
                ) => Some(Step::DeleteTransmogOutfit {
                    set_guid: row.set_guid,
                }),
                (PlayerEquipmentSetStateLikeCpp::New, PlayerEquipmentSetTypeLikeCpp::Equipment) => {
                    Some(Step::InsertEquipmentSet {
                        player_guid: guid,
                        row: row.clone(),
                    })
                }
                (
                    PlayerEquipmentSetStateLikeCpp::Changed,
                    PlayerEquipmentSetTypeLikeCpp::Equipment,
                ) => Some(Step::UpdateEquipmentSet {
                    player_guid: guid,
                    row: row.clone(),
                }),
                (PlayerEquipmentSetStateLikeCpp::New, PlayerEquipmentSetTypeLikeCpp::Transmog) => {
                    Some(Step::InsertTransmogOutfit {
                        player_guid: guid,
                        row: row.clone(),
                    })
                }
                (
                    PlayerEquipmentSetStateLikeCpp::Changed,
                    PlayerEquipmentSetTypeLikeCpp::Transmog,
                ) => Some(Step::UpdateTransmogOutfit {
                    player_guid: guid,
                    row: row.clone(),
                }),
            };
            if let Some(step) = step {
                steps.push(step);
            }
        }
    }
    if let Some(slots) = &request.void_storage {
        steps.extend(slots.iter().map(|slot| match &slot.item {
            Some(row) => Step::ReplaceVoidStorageItem {
                player_guid: guid,
                slot: slot.slot,
                row: row.clone(),
            },
            None => Step::DeleteVoidStorageSlot {
                player_guid: guid,
                slot: slot.slot,
            },
        }));
    }
    if let Some(tutorials) = &request.tutorials {
        steps.push(if tutorials.already_persisted {
            Step::UpdateTutorials {
                account_id,
                tutorials: tutorials.tutorials,
            }
        } else {
            Step::InsertTutorials {
                account_id,
                tutorials: tutorials.tutorials,
            }
        });
    }
    if !request.instance_lock_times.is_empty() {
        steps.push(Step::DeleteInstanceLockTimes { account_id });
        steps.extend(
            request
                .instance_lock_times
                .iter()
                .map(|lock| Step::InsertInstanceLockTime {
                    account_id,
                    instance_id: lock.instance_id,
                    release_time: lock.release_time,
                }),
        );
    }
    steps.push(Step::PlayedTime {
        total_time: request.played_time.total_time,
        level_time: request.played_time.level_time,
        guid,
    });
    for reputation in &request.reputations {
        steps.push(Step::DeleteReputation {
            guid,
            faction_id: reputation.faction_id,
        });
        steps.push(Step::InsertReputation {
            guid,
            faction_id: reputation.faction_id,
            standing: reputation.standing,
            flags: reputation.flags,
        });
    }
    if let Some(profiles) = &request.cuf_profiles {
        steps.extend(profiles.iter().map(|slot| match &slot.profile {
            Some(row) => Step::ReplaceCufProfile {
                guid,
                profile_id: slot.profile_id,
                row: row.clone(),
            },
            None => Step::DeleteCufProfile {
                guid,
                profile_id: slot.profile_id,
            },
        }));
    }

    steps
        .iter()
        .map(player_character_save_statement_like_cpp)
        .collect()
}

/// Build the tutorials statement for one account.
///
/// Shared rather than duplicated: both the standalone SaveTutorialsData path
/// and the #286 Player full-save adapter append this same row, and two
/// independent copies of the column order would be free to drift.
pub fn build_tutorials_save_statement_like_cpp(
    account_id: u32,
    tutorials: &[u32],
    already_persisted: bool,
) -> PreparedStatement {
    let mut stmt = PreparedStatement::new(if already_persisted {
        CharStatements::UPD_TUTORIALS.sql()
    } else {
        CharStatements::INS_TUTORIALS.sql()
    });
    for (index, value) in tutorials.iter().copied().enumerate() {
        stmt.set_u32(index, value);
    }
    stmt.set_u32(tutorials.len(), account_id);
    stmt
}

fn account_collection_load_statements_like_cpp(
    request: AccountCollectionLoadRequestLikeCpp,
) -> Vec<PreparedStatement> {
    let (bnet_account_id, statements) = match request {
        AccountCollectionLoadRequestLikeCpp::Mounts { bnet_account_id } => {
            (bnet_account_id, vec![LoginStatements::SEL_ACCOUNT_MOUNTS])
        }
        AccountCollectionLoadRequestLikeCpp::Toys { bnet_account_id } => {
            (bnet_account_id, vec![LoginStatements::SEL_ACCOUNT_TOYS])
        }
        AccountCollectionLoadRequestLikeCpp::Heirlooms { bnet_account_id } => (
            bnet_account_id,
            vec![LoginStatements::SEL_ACCOUNT_HEIRLOOMS],
        ),
        AccountCollectionLoadRequestLikeCpp::ItemAppearances { bnet_account_id } => (
            bnet_account_id,
            vec![
                LoginStatements::SEL_BNET_ITEM_APPEARANCES,
                LoginStatements::SEL_BNET_ITEM_FAVORITE_APPEARANCES,
            ],
        ),
        AccountCollectionLoadRequestLikeCpp::TransmogIllusions { bnet_account_id } => (
            bnet_account_id,
            vec![LoginStatements::SEL_BNET_TRANSMOG_ILLUSIONS],
        ),
    };

    statements
        .into_iter()
        .map(|statement| {
            let mut prepared = PreparedStatement::new(statement.sql());
            prepared.set_u32(0, bnet_account_id);
            prepared
        })
        .collect()
}

fn player_login_admission_load_statement_like_cpp(
    request: PlayerLoginAdmissionLoadRequestLikeCpp,
) -> PreparedStatement {
    match request {
        PlayerLoginAdmissionLoadRequestLikeCpp::BattlegroundLocation { player_guid } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::SEL_CHARACTER_BGDATA);
            statement.set_u64(0, player_guid);
            statement
        }
        PlayerLoginAdmissionLoadRequestLikeCpp::HomebindLocation { player_guid } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::SEL_CHARACTER_HOMEBIND);
            statement.set_u64(0, player_guid);
            statement
        }
        PlayerLoginAdmissionLoadRequestLikeCpp::GuildMembership { player_guid } => {
            let mut statement = PreparedStatement::for_statement(CharStatements::SEL_GUILD_MEMBER);
            statement.set_u64(0, player_guid);
            statement
        }
    }
}

fn player_login_auxiliary_load_statement_like_cpp(
    request: PlayerLoginAuxiliaryLoadRequestLikeCpp,
) -> PreparedStatement {
    match request {
        PlayerLoginAuxiliaryLoadRequestLikeCpp::Customizations { player_guid } => {
            let mut statement =
                PreparedStatement::new(CharStatements::SEL_CHARACTER_CUSTOMIZATIONS.sql());
            statement.set_u64(0, player_guid);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::CompletedAchievements { player_guid } => {
            let mut statement =
                PreparedStatement::new(CharStatements::SEL_CHARACTER_ACHIEVEMENTS.sql());
            statement.set_u64(0, player_guid);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::InstanceTimeRestrictions { account_id } => {
            let mut statement =
                PreparedStatement::new(CharStatements::SEL_ACCOUNT_INSTANCELOCKTIMES.sql());
            statement.set_u32(0, account_id);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellCooldowns { player_guid } => {
            let mut statement =
                PreparedStatement::new(CharStatements::SEL_CHARACTER_SPELLCOOLDOWNS.sql());
            statement.set_u64(0, player_guid);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellCharges { player_guid } => {
            let mut statement =
                PreparedStatement::new(CharStatements::SEL_CHARACTER_SPELL_CHARGES.sql());
            statement.set_u64(0, player_guid);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::TraitEntries { player_guid } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::SEL_CHAR_TRAIT_ENTRIES);
            statement.set_u64(0, player_guid);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::TraitConfigs { player_guid } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::SEL_CHAR_TRAIT_CONFIGS);
            statement.set_u64(0, player_guid);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::PetStable { player_guid } => {
            let mut statement = PreparedStatement::for_statement(CharStatements::SEL_CHAR_PETS);
            statement.set_u64(0, player_guid);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::PetAuras { pet_number } => {
            let mut statement = PreparedStatement::for_statement(CharStatements::SEL_PET_AURA);
            statement.set_u32(0, pet_number);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::PetAuraEffects { pet_number } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::SEL_PET_AURA_EFFECT);
            statement.set_u32(0, pet_number);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::PetSpells { pet_number } => {
            let mut statement = PreparedStatement::for_statement(CharStatements::SEL_PET_SPELL);
            statement.set_u32(0, pet_number);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::PetSpellCooldowns { pet_number } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::SEL_PET_SPELL_COOLDOWN);
            statement.set_u32(0, pet_number);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::PetSpellCharges { pet_number } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::SEL_PET_SPELL_CHARGES);
            statement.set_u32(0, pet_number);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::PetDeclinedNames {
            player_guid,
            pet_number,
        } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::SEL_PET_DECLINED_NAME);
            statement.set_u64(0, player_guid);
            statement.set_u32(1, pet_number);
            statement
        }
    }
}

fn player_character_base_load_statement_like_cpp(
    request: PlayerCharacterBaseLoadRequestLikeCpp,
) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(CharStatements::SEL_CHARACTER);
    statement.set_u64(0, request.player_guid);
    statement
}

fn player_character_base_load_row_like_cpp(
    result: &crate::SqlResult,
) -> PlayerCharacterBaseLoadRowLikeCpp {
    PlayerCharacterBaseLoadRowLikeCpp {
        name: result.read_string(2),
        race: result.read(3),
        class: result.read(4),
        gender: result.read(5),
        level: result.read(6),
        xp: result.try_read(7),
        money: result.try_read(8),
        inventory_slots: result.try_read(9),
        bank_slots: result.try_read(10),
        rest_state: result.try_read(11),
        player_flags: result.try_read(12),
        player_flags_ex: result.try_read(13),
        position_x: result.try_read(14),
        position_y: result.try_read(15),
        position_z: result.try_read(16),
        map_id: result.try_read(17),
        orientation: result.try_read(18),
        create_mode: result.try_read(21),
        total_played_time: result.try_read(23),
        level_played_time: result.try_read(24),
        rest_bonus: result.try_read(25),
        logout_time_secs: result
            .try_read::<u64>(26)
            .or_else(|| result.try_read::<i64>(26).map(|value| value.max(0) as u64)),
        logout_was_resting: result.try_read(27),
        talent_reset_cost: result.try_read(28),
        talent_reset_time_secs: result.try_read(29),
        active_talent_group: result.try_read(30),
        bonus_talent_groups: result.try_read(31),
        transport_x: result.try_read(32),
        transport_y: result.try_read(33),
        transport_z: result.try_read(34),
        transport_orientation: result.try_read(35),
        transport_guid_low: result
            .try_read::<u64>(36)
            .or_else(|| result.try_read::<i64>(36).map(|value| value.max(0) as u64)),
        summoned_pet_number: result.try_read(38),
        at_login_flags: result.try_read(39),
        zone_id: result.try_read(40),
        dungeon_difficulty: result.try_read(44),
        chosen_title: result.try_read(48),
        health: result.try_read(51),
        powers: std::array::from_fn(|index| result.try_read(52 + index)),
        explored_zones: result.read_string(64),
        known_titles: result.try_read(65),
        raid_difficulty: result.try_read(67),
        legacy_raid_difficulty: result.try_read(68),
    }
}

fn player_homebind_persistence_statement_like_cpp(
    request: PlayerHomebindPersistenceRequestLikeCpp,
) -> PreparedStatement {
    match request {
        PlayerHomebindPersistenceRequestLikeCpp::DeleteInvalid { player_guid } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::DEL_PLAYER_HOMEBIND);
            statement.set_u64(0, player_guid);
            statement
        }
        PlayerHomebindPersistenceRequestLikeCpp::InsertRepaired {
            player_guid,
            map_id,
            area_id,
            x,
            y,
            z,
            orientation,
        } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::INS_PLAYER_HOMEBIND);
            statement.set_u64(0, player_guid);
            statement.set_u16(1, map_id);
            statement.set_u16(2, area_id);
            statement.set_f32(3, x);
            statement.set_f32(4, y);
            statement.set_f32(5, z);
            statement.set_f32(6, orientation);
            statement
        }
        PlayerHomebindPersistenceRequestLikeCpp::UpdateLive {
            player_guid,
            map_id,
            area_id,
            x,
            y,
            z,
            orientation,
        } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::UPD_PLAYER_HOMEBIND);
            // C++ PreparedStatement::setUInt16 narrows these uint32 values at
            // this concrete adapter boundary.
            statement.set_u16(0, map_id as u16);
            statement.set_u16(1, area_id as u16);
            statement.set_f32(2, x);
            statement.set_f32(3, y);
            statement.set_f32(4, z);
            statement.set_f32(5, orientation);
            statement.set_u64(6, player_guid);
            statement
        }
    }
}

fn player_buyback_clear_statements_like_cpp(
    request: &PlayerBuybackClearRequestLikeCpp,
) -> Vec<PreparedStatement> {
    let mut statements = Vec::with_capacity(request.item_db_guids.len().saturating_mul(2));
    for &item_db_guid in &request.item_db_guids {
        let mut delete_inventory =
            PreparedStatement::for_statement(CharStatements::DEL_CHAR_INVENTORY_ITEM);
        delete_inventory.set_u64(0, request.player_guid);
        delete_inventory.set_u64(1, item_db_guid);
        statements.push(delete_inventory);

        let mut delete_item = PreparedStatement::for_statement(CharStatements::DEL_ITEM_INSTANCE);
        delete_item.set_u64(0, item_db_guid);
        statements.push(delete_item);
    }
    statements
}

fn player_talent_reset_statements_like_cpp(
    request: &PlayerTalentResetPersistenceRequestLikeCpp,
) -> Vec<PreparedStatement> {
    let mut statements = Vec::with_capacity(3 + request.retained_talents.len());

    let mut money = PreparedStatement::for_statement(CharStatements::UPD_CHAR_MONEY);
    money.set_u64(0, request.money_after);
    money.set_u64(1, request.player_guid);
    statements.push(money);

    let mut metadata =
        PreparedStatement::for_statement(CharStatements::UPD_CHAR_TALENT_RESET_STATE);
    metadata.set_u32(0, request.reset_cost);
    metadata.set_u64(1, request.reset_time_secs);
    metadata.set_u64(2, request.player_guid);
    statements.push(metadata);

    let mut delete = PreparedStatement::for_statement(CharStatements::DEL_CHAR_TALENT);
    delete.set_u64(0, request.player_guid);
    statements.push(delete);

    for row in &request.retained_talents {
        let mut insert = PreparedStatement::for_statement(CharStatements::INS_CHAR_TALENT);
        insert.set_u64(0, request.player_guid);
        insert.set_u32(1, row.talent_id);
        insert.set_u8(2, row.rank);
        insert.set_u8(3, row.talent_group);
        statements.push(insert);
    }
    statements
}

fn player_xp_persistence_statements_like_cpp(
    request: &PlayerXpPersistenceRequestLikeCpp,
) -> Vec<PreparedStatement> {
    let mut statements = Vec::with_capacity(if request.rest.is_some() { 2 } else { 1 });
    if request.level_changed {
        let mut statement = PreparedStatement::for_statement(CharStatements::UPD_CHAR_LEVEL);
        statement.set_u8(0, request.level);
        statement.set_u32(1, request.xp);
        statement.set_u64(2, request.player_guid);
        statements.push(statement);
    } else {
        let mut statement = PreparedStatement::for_statement(CharStatements::UPD_CHAR_XP);
        statement.set_u32(0, request.xp);
        statement.set_u64(1, request.player_guid);
        statements.push(statement);
    }

    if let Some(rest) = request.rest {
        let mut statement =
            PreparedStatement::for_statement(CharStatements::UPD_CHAR_ONLINE_REST_STATE);
        statement.set_u8(0, rest.rest_state);
        statement.set_u32(1, rest.player_flags);
        statement.set_f32(2, rest.rest_bonus);
        statement.set_u64(3, request.player_guid);
        statements.push(statement);
    }
    statements
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayerTalentResetCommitReconciliationLikeCpp {
    Applied,
    Failed,
    Unknown,
}

fn reconcile_player_talent_reset_commit_like_cpp(
    money_before: u64,
    money_after: u64,
    observed_money: Option<u64>,
) -> PlayerTalentResetCommitReconciliationLikeCpp {
    if money_before == money_after {
        return PlayerTalentResetCommitReconciliationLikeCpp::Unknown;
    }
    match observed_money {
        Some(observed) if observed == money_after => {
            PlayerTalentResetCommitReconciliationLikeCpp::Applied
        }
        Some(observed) if observed == money_before => {
            PlayerTalentResetCommitReconciliationLikeCpp::Failed
        }
        Some(_) | None => PlayerTalentResetCommitReconciliationLikeCpp::Unknown,
    }
}

fn player_realm_character_count_statements_like_cpp(
    request: PlayerRealmCharacterCountRefreshRequestLikeCpp,
    num_chars: u8,
) -> (PreparedStatement, PreparedStatement) {
    let mut count = PreparedStatement::for_statement(CharStatements::SEL_SUM_CHARS);
    count.set_u32(0, request.account_id);

    let mut replace = PreparedStatement::for_statement(LoginStatements::REP_REALM_CHARACTERS);
    replace.set_u8(0, num_chars);
    replace.set_u32(1, request.account_id);
    replace.set_u32(2, request.realm_id);
    (count, replace)
}

fn player_login_transport_load_statement_like_cpp(
    request: PlayerLoginTransportLoadRequestLikeCpp,
) -> PreparedStatement {
    match request {
        PlayerLoginTransportLoadRequestLikeCpp::All => {
            PreparedStatement::for_statement(WorldStatements::SEL_LOGIN_TRANSPORTS)
        }
        PlayerLoginTransportLoadRequestLikeCpp::ByGuid { guid_low } => {
            let mut statement =
                PreparedStatement::for_statement(WorldStatements::SEL_LOGIN_TRANSPORT_BY_GUID);
            statement.set_u64(0, guid_low);
            statement
        }
    }
}

fn player_login_transport_load_rows_like_cpp(
    mut result: crate::SqlResult,
) -> Vec<PlayerLoginTransportLoadRowLikeCpp> {
    let mut rows = Vec::new();
    if result.is_empty() {
        return rows;
    }
    loop {
        rows.push(PlayerLoginTransportLoadRowLikeCpp {
            guid_low: result
                .try_read::<i64>(0)
                .map(|value| value.max(0) as u32)
                .or_else(|| result.try_read::<u32>(0))
                .unwrap_or(0),
            entry: result
                .try_read::<i32>(1)
                .map(|value| value.max(0) as u32)
                .or_else(|| result.try_read::<u32>(1))
                .unwrap_or(0),
            phase_use_flags: result
                .try_read::<u8>(2)
                .or_else(|| result.try_read::<i16>(2).map(|value| value.max(0) as u8))
                .unwrap_or(0),
            phase_id: result
                .try_read::<u16>(3)
                .or_else(|| result.try_read::<i32>(3).map(|value| value.max(0) as u16))
                .unwrap_or(0),
            phase_group_id: result
                .try_read::<u32>(4)
                .or_else(|| result.try_read::<i32>(4).map(|value| value.max(0) as u32))
                .unwrap_or(0),
            display_id: result
                .try_read::<i32>(5)
                .map(|value| value.max(0) as u32)
                .or_else(|| result.try_read::<u32>(5))
                .unwrap_or(0),
            scale: result.try_read::<f32>(6).unwrap_or(1.0),
            taxi_path_id: result
                .try_read::<i32>(7)
                .map(|value| value.max(0) as u16)
                .or_else(|| result.try_read::<u16>(7))
                .unwrap_or(0),
            move_speed: result
                .try_read::<i32>(8)
                .map(|value| value.max(1) as u32)
                .or_else(|| result.try_read::<u32>(8))
                .unwrap_or(1),
            accel_rate: result
                .try_read::<i32>(9)
                .map(|value| value.max(1) as u32)
                .or_else(|| result.try_read::<u32>(9))
                .unwrap_or(1),
            allow_stopping: result
                .try_read::<i32>(10)
                .map(|value| value != 0)
                .or_else(|| result.try_read::<u8>(10).map(|value| value != 0))
                .unwrap_or(false),
            gameobject_flags: result
                .try_read::<i64>(11)
                .map(|value| value.max(0) as u32)
                .or_else(|| result.try_read::<u32>(11))
                .unwrap_or(0),
            faction_template: result
                .try_read::<i64>(12)
                .map(|value| value as i32)
                .or_else(|| result.try_read::<i32>(12))
                .unwrap_or(0),
        });
        if !result.next_row() {
            break;
        }
    }
    rows
}

/// Binds the lifecycle port to the Characters, Login and World adapters its
/// semantic requests address.
pub struct MariaDbPlayerLifecycleAdapterLikeCpp {
    character_db: Arc<CharacterDatabase>,
    login_db: Arc<LoginDatabase>,
    world_db: Arc<WorldDatabase>,
}

impl MariaDbPlayerLifecycleAdapterLikeCpp {
    pub fn new(
        character_db: Arc<CharacterDatabase>,
        login_db: Arc<LoginDatabase>,
        world_db: Arc<WorldDatabase>,
    ) -> Self {
        Self {
            character_db,
            login_db,
            world_db,
        }
    }
}

impl PlayerLifecyclePortLikeCpp for MariaDbPlayerLifecycleAdapterLikeCpp {
    fn mark_offline_like_cpp<'a>(
        &'a self,
        mark: PlayerOfflineMarkLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let result = match mark {
                PlayerOfflineMarkLikeCpp::Character { guid_low } => {
                    let mut stmt = self.character_db.prepare(CharStatements::UPD_CHAR_OFFLINE);
                    stmt.set_u32(0, guid_low);
                    self.character_db.execute(&stmt).await
                }
                PlayerOfflineMarkLikeCpp::CharacterAccount { account_id } => {
                    let mut stmt = self
                        .character_db
                        .prepare(CharStatements::UPD_ACCOUNT_ONLINE);
                    stmt.set_u32(0, account_id);
                    self.character_db.execute(&stmt).await
                }
                PlayerOfflineMarkLikeCpp::LoginAccount { account_id } => {
                    let mut stmt = self.login_db.prepare(LoginStatements::UPD_ACCOUNT_OFFLINE);
                    stmt.set_u32(0, account_id);
                    self.login_db.execute(&stmt).await
                }
            };
            match result {
                Ok(rows) => PersistenceOutcomeLikeCpp::Applied { rows },
                // A single-statement write outside a transaction either applied
                // or it did not; there is no COMMIT whose outcome could be
                // indeterminate. `Unknown` is reserved for the transactional
                // paths #200 migrates next, so do not manufacture it here.
                Err(error) => PersistenceOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn persist_homebind_like_cpp<'a>(
        &'a self,
        request: PlayerHomebindPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let statement = player_homebind_persistence_statement_like_cpp(request);
            match self.character_db.execute(&statement).await {
                Ok(rows) => PersistenceOutcomeLikeCpp::Applied { rows },
                Err(error) => PersistenceOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn clear_buyback_like_cpp<'a>(
        &'a self,
        request: PlayerBuybackClearRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let item_count = request.item_db_guids.len() as u64;
            let mut transaction = SqlTransaction::new();
            for statement in player_buyback_clear_statements_like_cpp(&request) {
                transaction.append(statement);
            }
            match transaction
                .commit_with_outcome_like_cpp(self.character_db.pool())
                .await
            {
                Ok(()) => PersistenceOutcomeLikeCpp::Applied { rows: item_count },
                Err(SqlTransactionCommitError::DefinitelyRolledBack(error)) => {
                    PersistenceOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    }
                }
                Err(SqlTransactionCommitError::CommitOutcomeUnknown(error)) => {
                    PersistenceOutcomeLikeCpp::Unknown {
                        reason: error.to_string(),
                    }
                }
            }
        })
    }

    fn persist_talent_reset_like_cpp<'a>(
        &'a self,
        request: PlayerTalentResetPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let statements = player_talent_reset_statements_like_cpp(&request);
            let rows = statements.len() as u64;
            let mut transaction = SqlTransaction::new();
            for statement in statements {
                transaction.append(statement);
            }

            match transaction
                .commit_with_outcome_like_cpp(self.character_db.pool())
                .await
            {
                Ok(()) => PersistenceOutcomeLikeCpp::Applied { rows },
                Err(SqlTransactionCommitError::DefinitelyRolledBack(error)) => {
                    PersistenceOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    }
                }
                Err(SqlTransactionCommitError::CommitOutcomeUnknown(error)) => {
                    let observed_money = if request.money_before == request.money_after {
                        None
                    } else {
                        let mut observed =
                            self.character_db.prepare(CharStatements::SEL_CHAR_MONEY);
                        observed.set_u64(0, request.player_guid);
                        self.character_db
                            .query(&observed)
                            .await
                            .ok()
                            .filter(|result| !result.is_empty())
                            .and_then(|result| result.try_read::<u64>(0))
                    };

                    match reconcile_player_talent_reset_commit_like_cpp(
                        request.money_before,
                        request.money_after,
                        observed_money,
                    ) {
                        PlayerTalentResetCommitReconciliationLikeCpp::Applied => {
                            PersistenceOutcomeLikeCpp::Applied { rows }
                        }
                        PlayerTalentResetCommitReconciliationLikeCpp::Failed => {
                            PersistenceOutcomeLikeCpp::Failed {
                                reason: error.to_string(),
                            }
                        }
                        PlayerTalentResetCommitReconciliationLikeCpp::Unknown => {
                            PersistenceOutcomeLikeCpp::Unknown {
                                reason: error.to_string(),
                            }
                        }
                    }
                }
            }
        })
    }

    fn persist_xp_like_cpp<'a>(
        &'a self,
        request: PlayerXpPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let statements = player_xp_persistence_statements_like_cpp(&request);
            let rows = statements.len() as u64;
            let mut transaction = SqlTransaction::new();
            for statement in statements {
                transaction.append(statement);
            }
            match transaction
                .commit_with_outcome_like_cpp(self.character_db.pool())
                .await
            {
                Ok(()) => PersistenceOutcomeLikeCpp::Applied { rows },
                Err(SqlTransactionCommitError::DefinitelyRolledBack(error)) => {
                    PersistenceOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    }
                }
                Err(SqlTransactionCommitError::CommitOutcomeUnknown(error)) => {
                    PersistenceOutcomeLikeCpp::Unknown {
                        reason: error.to_string(),
                    }
                }
            }
        })
    }

    fn refresh_realm_character_count_like_cpp<'a>(
        &'a self,
        request: PlayerRealmCharacterCountRefreshRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let (count_statement, _) = player_realm_character_count_statements_like_cpp(request, 0);
            let num_chars = match self.character_db.query(&count_statement).await {
                Ok(result) if !result.is_empty() => result.try_read::<i64>(0).unwrap_or(0) as u8,
                Ok(_) => 0,
                Err(error) => {
                    return PersistenceOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };
            let (_, replace_statement) =
                player_realm_character_count_statements_like_cpp(request, num_chars);
            match self.login_db.execute(&replace_statement).await {
                Ok(rows) => PersistenceOutcomeLikeCpp::Applied { rows },
                Err(error) => PersistenceOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn load_initial_world_states_like_cpp<'a>(
        &'a self,
    ) -> PersistenceFutureLikeCpp<'a, PlayerInitialWorldStatesLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let templates = {
                let statement = self.world_db.prepare(WorldStatements::SEL_WORLD_STATES);
                match self.world_db.query(&statement).await {
                    Ok(mut result) => {
                        let mut rows = Vec::new();
                        if !result.is_empty() {
                            loop {
                                rows.push(PlayerInitialWorldStateTemplateRowLikeCpp {
                                    id: result.read(0),
                                    default_value: result.read(1),
                                    map_ids_csv: result.try_read(2).unwrap_or_default(),
                                    area_ids_csv: result.try_read(3).unwrap_or_default(),
                                });
                                if !result.next_row() {
                                    break;
                                }
                            }
                        }
                        PlayerInitialWorldStateRowsLikeCpp::Loaded(rows)
                    }
                    Err(error) => PlayerInitialWorldStateRowsLikeCpp::Failed {
                        reason: error.to_string(),
                    },
                }
            };

            // Preserve C++ and the existing Rust order even when the template
            // read failed: this is a second logical database, not one ACID unit.
            let saved_values = {
                let statement = self
                    .character_db
                    .prepare(CharStatements::SEL_WORLD_STATE_VALUES);
                match self.character_db.query(&statement).await {
                    Ok(mut result) => {
                        let mut rows = Vec::new();
                        if !result.is_empty() {
                            loop {
                                rows.push(PlayerInitialWorldStateValueRowLikeCpp {
                                    id: result.read(0),
                                    value: result.read(1),
                                });
                                if !result.next_row() {
                                    break;
                                }
                            }
                        }
                        PlayerInitialWorldStateRowsLikeCpp::Loaded(rows)
                    }
                    Err(error) => PlayerInitialWorldStateRowsLikeCpp::Failed {
                        reason: error.to_string(),
                    },
                }
            };

            PlayerInitialWorldStatesLoadOutcomeLikeCpp {
                templates,
                saved_values,
            }
        })
    }

    fn load_login_transports_like_cpp<'a>(
        &'a self,
        request: PlayerLoginTransportLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginTransportLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let statement = player_login_transport_load_statement_like_cpp(request);
            match self.world_db.query(&statement).await {
                Ok(result) => PlayerLoginTransportLoadOutcomeLikeCpp::Loaded(
                    player_login_transport_load_rows_like_cpp(result),
                ),
                Err(error) => PlayerLoginTransportLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn load_character_base_like_cpp<'a>(
        &'a self,
        request: PlayerCharacterBaseLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerCharacterBaseLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let statement = player_character_base_load_statement_like_cpp(request);
            match self.character_db.query(&statement).await {
                Ok(result) if result.is_empty() => {
                    PlayerCharacterBaseLoadOutcomeLikeCpp::Loaded(None)
                }
                Ok(result) => PlayerCharacterBaseLoadOutcomeLikeCpp::Loaded(Some(
                    player_character_base_load_row_like_cpp(&result),
                )),
                Err(error) => PlayerCharacterBaseLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn load_account_collection_like_cpp<'a>(
        &'a self,
        request: AccountCollectionLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, AccountCollectionLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let statements = account_collection_load_statements_like_cpp(request);
            match request {
                AccountCollectionLoadRequestLikeCpp::Mounts { .. } => {
                    match self.login_db.query(&statements[0]).await {
                        Ok(mut result) => {
                            let mut rows = Vec::new();
                            if !result.is_empty() {
                                loop {
                                    rows.push(AccountMountLoadRowLikeCpp {
                                        mount_spell_id: result.try_read::<i32>(0).unwrap_or(0),
                                        flags: result.try_read::<u8>(1).unwrap_or(0),
                                    });
                                    if !result.next_row() {
                                        break;
                                    }
                                }
                            }
                            AccountCollectionLoadOutcomeLikeCpp::Loaded(
                                AccountCollectionLoadedLikeCpp::Mounts(rows),
                            )
                        }
                        Err(error) => AccountCollectionLoadOutcomeLikeCpp::Failed {
                            reason: error.to_string(),
                        },
                    }
                }
                AccountCollectionLoadRequestLikeCpp::Toys { .. } => {
                    match self.login_db.query(&statements[0]).await {
                        Ok(mut result) => {
                            let mut rows = Vec::new();
                            if !result.is_empty() {
                                loop {
                                    rows.push(AccountToyLoadRowLikeCpp {
                                        item_id: result.try_read::<i32>(0).unwrap_or(0),
                                        is_favorite: result.try_read::<bool>(1).unwrap_or(false),
                                        has_fanfare: result.try_read::<bool>(2).unwrap_or(false),
                                    });
                                    if !result.next_row() {
                                        break;
                                    }
                                }
                            }
                            AccountCollectionLoadOutcomeLikeCpp::Loaded(
                                AccountCollectionLoadedLikeCpp::Toys(rows),
                            )
                        }
                        Err(error) => AccountCollectionLoadOutcomeLikeCpp::Failed {
                            reason: error.to_string(),
                        },
                    }
                }
                AccountCollectionLoadRequestLikeCpp::Heirlooms { .. } => {
                    match self.login_db.query(&statements[0]).await {
                        Ok(mut result) => {
                            let mut rows = Vec::new();
                            if !result.is_empty() {
                                loop {
                                    rows.push(AccountHeirloomLoadRowLikeCpp {
                                        item_id: result.try_read::<i32>(0).unwrap_or(0),
                                        flags: result.try_read::<u32>(1).unwrap_or(0),
                                    });
                                    if !result.next_row() {
                                        break;
                                    }
                                }
                            }
                            AccountCollectionLoadOutcomeLikeCpp::Loaded(
                                AccountCollectionLoadedLikeCpp::Heirlooms(rows),
                            )
                        }
                        Err(error) => AccountCollectionLoadOutcomeLikeCpp::Failed {
                            reason: error.to_string(),
                        },
                    }
                }
                AccountCollectionLoadRequestLikeCpp::ItemAppearances { .. } => {
                    let appearance_blocks = match self.login_db.query(&statements[0]).await {
                        Ok(mut result) => {
                            let mut rows = Vec::new();
                            if !result.is_empty() {
                                loop {
                                    rows.push(AccountMaskBlockLikeCpp {
                                        block_index: result.try_read::<u32>(0).unwrap_or(0),
                                        mask: result.try_read::<u32>(1).unwrap_or(0),
                                    });
                                    if !result.next_row() {
                                        break;
                                    }
                                }
                            }
                            AccountCollectionRowsLikeCpp::Loaded(rows)
                        }
                        Err(error) => AccountCollectionRowsLikeCpp::Failed {
                            reason: error.to_string(),
                        },
                    };
                    let favorite_appearance_ids = match self.login_db.query(&statements[1]).await {
                        Ok(mut result) => {
                            let mut rows = Vec::new();
                            if !result.is_empty() {
                                loop {
                                    rows.push(result.try_read::<u32>(0).unwrap_or(0));
                                    if !result.next_row() {
                                        break;
                                    }
                                }
                            }
                            AccountCollectionRowsLikeCpp::Loaded(rows)
                        }
                        Err(error) => AccountCollectionRowsLikeCpp::Failed {
                            reason: error.to_string(),
                        },
                    };
                    AccountCollectionLoadOutcomeLikeCpp::Loaded(
                        AccountCollectionLoadedLikeCpp::ItemAppearances {
                            appearance_blocks,
                            favorite_appearance_ids,
                        },
                    )
                }
                AccountCollectionLoadRequestLikeCpp::TransmogIllusions { .. } => {
                    match self.login_db.query(&statements[0]).await {
                        Ok(mut result) => {
                            let mut rows = Vec::new();
                            if !result.is_empty() {
                                loop {
                                    rows.push(AccountMaskBlockLikeCpp {
                                        block_index: result.try_read::<u32>(0).unwrap_or(0),
                                        mask: result.try_read::<u32>(1).unwrap_or(0),
                                    });
                                    if !result.next_row() {
                                        break;
                                    }
                                }
                            }
                            AccountCollectionLoadOutcomeLikeCpp::Loaded(
                                AccountCollectionLoadedLikeCpp::TransmogIllusions {
                                    illusion_blocks: rows,
                                },
                            )
                        }
                        Err(error) => AccountCollectionLoadOutcomeLikeCpp::Failed {
                            reason: error.to_string(),
                        },
                    }
                }
            }
        })
    }

    fn load_login_admission_like_cpp<'a>(
        &'a self,
        request: PlayerLoginAdmissionLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginAdmissionLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let statement = player_login_admission_load_statement_like_cpp(request);
            let mut result = match self.character_db.query(&statement).await {
                Ok(result) => result,
                Err(error) => {
                    return PlayerLoginAdmissionLoadOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };

            let loaded = match request {
                PlayerLoginAdmissionLoadRequestLikeCpp::BattlegroundLocation { .. } => {
                    let row =
                        (!result.is_empty()).then(|| PlayerBattlegroundLocationLoadRowLikeCpp {
                            x: result.try_read(2),
                            y: result.try_read(3),
                            z: result.try_read(4),
                            orientation: result.try_read(5),
                            map_id: result.try_read(6),
                        });
                    PlayerLoginAdmissionLoadedLikeCpp::BattlegroundLocation(row)
                }
                PlayerLoginAdmissionLoadRequestLikeCpp::HomebindLocation { .. } => {
                    let row = (!result.is_empty()).then(|| PlayerHomebindLocationLoadRowLikeCpp {
                        map_id: result.try_read(0),
                        area_id: result.try_read(1),
                        x: result.try_read(2),
                        y: result.try_read(3),
                        z: result.try_read(4),
                        orientation: result.try_read(5),
                    });
                    PlayerLoginAdmissionLoadedLikeCpp::HomebindLocation(row)
                }
                PlayerLoginAdmissionLoadRequestLikeCpp::GuildMembership { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerGuildMembershipLoadRowLikeCpp {
                                guild_id: result.try_read(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAdmissionLoadedLikeCpp::GuildMembership(rows)
                }
            };
            PlayerLoginAdmissionLoadOutcomeLikeCpp::Loaded(loaded)
        })
    }

    fn load_login_auxiliary_like_cpp<'a>(
        &'a self,
        request: PlayerLoginAuxiliaryLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginAuxiliaryLoadOutcomeLikeCpp> {
        Box::pin(async move {
            let statement = player_login_auxiliary_load_statement_like_cpp(request);
            let mut result = match self.character_db.query(&statement).await {
                Ok(result) => result,
                Err(error) => {
                    return PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    };
                }
            };

            let loaded = match request {
                PlayerLoginAuxiliaryLoadRequestLikeCpp::Customizations { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerCustomizationLoadRowLikeCpp {
                                option_id: result.try_read::<u32>(0).unwrap_or(0),
                                choice_id: result.try_read::<u32>(1).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::Customizations(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::CompletedAchievements { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(result.try_read::<u32>(0).unwrap_or(0));
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::CompletedAchievements(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::InstanceTimeRestrictions { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerInstanceTimeRestrictionLoadRowLikeCpp {
                                instance_id: result.try_read::<u32>(0).unwrap_or(0),
                                release_time: result.try_read::<u64>(1).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::InstanceTimeRestrictions(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellCooldowns { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerSpellCooldownLoadRowLikeCpp {
                                spell_id: result.try_read::<u32>(0).unwrap_or(0),
                                item_id: result.try_read::<u32>(1).unwrap_or(0),
                                cooldown_end: result.try_read::<i64>(2).unwrap_or(0),
                                category_id: result.try_read::<u32>(3).unwrap_or(0),
                                category_end: result.try_read::<i64>(4).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::SpellCooldowns(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellCharges { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerSpellChargeLoadRowLikeCpp {
                                category_id: result.try_read::<u32>(0).unwrap_or(0),
                                recharge_start: result.try_read::<i64>(1).unwrap_or(0),
                                recharge_end: result.try_read::<i64>(2).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::SpellCharges(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::TraitEntries { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerTraitEntryLoadRowLikeCpp {
                                trait_config_id: result.try_read::<i32>(0),
                                trait_node_id: result.try_read::<i32>(1),
                                trait_node_entry_id: result.try_read::<i32>(2),
                                rank: result.try_read::<i32>(3),
                                granted_ranks: result.try_read::<i32>(4),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::TraitEntries(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::TraitConfigs { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerTraitConfigLoadRowLikeCpp {
                                id: result.try_read::<i32>(0),
                                config_type: result.try_read::<i32>(1),
                                chr_specialization_id: result.try_read::<i32>(2),
                                combat_config_flags: result.try_read::<i32>(3),
                                local_identifier: result.try_read::<i32>(4),
                                skill_line_id: result.try_read::<i32>(5),
                                trait_system_id: result.try_read::<i32>(6),
                                name: result.try_read::<String>(7),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::TraitConfigs(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::PetStable { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerPetStableLoadRowLikeCpp {
                                pet_number: result.try_read::<u32>(0).unwrap_or(0),
                                creature_id: result.try_read::<u32>(1).unwrap_or(0),
                                display_id: result.try_read::<u32>(2).unwrap_or(0),
                                level: result.try_read::<u8>(3).unwrap_or(1),
                                experience: result.try_read::<u32>(4).unwrap_or(0),
                                react_state: result.try_read::<u8>(5).unwrap_or(0),
                                slot: result.try_read::<i16>(6).unwrap_or(-1),
                                name: result.read_string(7),
                                was_renamed: result.try_read::<bool>(8).unwrap_or(false),
                                health: result.try_read::<u32>(9).unwrap_or(1),
                                mana: result.try_read::<u32>(10).unwrap_or(0),
                                action_bar: result.try_read::<String>(11).unwrap_or_default(),
                                last_save_time: result.try_read::<u32>(12).unwrap_or(0),
                                created_by_spell_id: result.try_read::<u32>(13).unwrap_or(0),
                                pet_type: result.try_read::<u8>(14).unwrap_or(0),
                                specialization_id: result.try_read::<u16>(15).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::PetStable(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::PetAuras { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerPetAuraLoadRowLikeCpp {
                                caster_guid_binary: result
                                    .try_read::<Vec<u8>>(0)
                                    .unwrap_or_default(),
                                spell_id: result.try_read::<u32>(1).unwrap_or(0),
                                effect_mask: result.try_read::<u32>(2).unwrap_or(0),
                                recalculate_mask: result.try_read::<u32>(3).unwrap_or(0),
                                difficulty: result.try_read::<u8>(4).unwrap_or(0),
                                stack_count: result.try_read::<u8>(5).unwrap_or(0),
                                max_duration_ms: result.try_read::<i32>(6).unwrap_or(0),
                                remain_time_ms: result.try_read::<i32>(7).unwrap_or(0),
                                remain_charges: result.try_read::<u8>(8).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::PetAuras(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::PetAuraEffects { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerPetAuraEffectLoadRowLikeCpp {
                                caster_guid_binary: result
                                    .try_read::<Vec<u8>>(0)
                                    .unwrap_or_default(),
                                spell_id: result.try_read::<u32>(1).unwrap_or(0),
                                effect_mask: result.try_read::<u32>(2).unwrap_or(0),
                                effect_index: result.try_read::<u8>(3).unwrap_or(0),
                                amount: result.try_read::<i32>(4).unwrap_or(0),
                                base_amount: result.try_read::<i32>(5).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::PetAuraEffects(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::PetSpells { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerPetSpellLoadRowLikeCpp {
                                spell_id: result.try_read::<u32>(0).unwrap_or(0),
                                active: result.try_read::<u8>(1).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::PetSpells(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::PetSpellCooldowns { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerPetSpellCooldownLoadRowLikeCpp {
                                spell_id: result.try_read::<u32>(0).unwrap_or(0),
                                cooldown_end_unix_secs: result.try_read::<i64>(1).unwrap_or(0),
                                category_id: result.try_read::<u32>(2).unwrap_or(0),
                                category_end_unix_secs: result.try_read::<i64>(3).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::PetSpellCooldowns(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::PetSpellCharges { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        loop {
                            rows.push(PlayerPetSpellChargeLoadRowLikeCpp {
                                category_id: result.try_read::<u32>(0).unwrap_or(0),
                                recharge_start_unix_secs: result.try_read::<i64>(1).unwrap_or(0),
                                recharge_end_unix_secs: result.try_read::<i64>(2).unwrap_or(0),
                            });
                            if !result.next_row() {
                                break;
                            }
                        }
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::PetSpellCharges(rows)
                }
                PlayerLoginAuxiliaryLoadRequestLikeCpp::PetDeclinedNames { .. } => {
                    let mut rows = Vec::new();
                    if !result.is_empty() {
                        rows.push(PlayerPetDeclinedNamesLoadRowLikeCpp {
                            names: [
                                result.read_string(0),
                                result.read_string(1),
                                result.read_string(2),
                                result.read_string(3),
                                result.read_string(4),
                            ],
                        });
                    }
                    PlayerLoginAuxiliaryLoadedLikeCpp::PetDeclinedNames(rows)
                }
            };
            PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(loaded)
        })
    }

    fn save_account_collection_like_cpp<'a>(
        &'a self,
        save: AccountCollectionSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            let mut tx = SqlTransaction::new();
            let rows = match &save {
                AccountCollectionSaveLikeCpp::Mounts(rows) => {
                    for row in rows {
                        let mut stmt = self.login_db.prepare(LoginStatements::REP_ACCOUNT_MOUNTS);
                        stmt.set_u32(0, row.bnet_account_id);
                        stmt.set_u32(1, row.mount_spell_id);
                        stmt.set_u8(2, row.flags);
                        tx.append(stmt);
                    }
                    rows.len()
                }
                AccountCollectionSaveLikeCpp::Toys(rows) => {
                    for row in rows {
                        let mut stmt = self.login_db.prepare(LoginStatements::REP_ACCOUNT_TOYS);
                        stmt.set_u32(0, row.bnet_account_id);
                        stmt.set_u32(1, row.item_id);
                        stmt.set_bool(2, row.is_favorite);
                        stmt.set_bool(3, row.has_fanfare);
                        tx.append(stmt);
                    }
                    rows.len()
                }
                AccountCollectionSaveLikeCpp::Heirlooms(rows) => {
                    for row in rows {
                        let mut stmt = self
                            .login_db
                            .prepare(LoginStatements::REP_ACCOUNT_HEIRLOOMS);
                        stmt.set_u32(0, row.bnet_account_id);
                        stmt.set_u32(1, row.item_id);
                        stmt.set_u32(2, row.flags);
                        tx.append(stmt);
                    }
                    rows.len()
                }
                AccountCollectionSaveLikeCpp::ItemAppearances {
                    bnet_account_id,
                    appearance_blocks,
                    favorite_inserts,
                    favorite_deletes,
                } => {
                    for block in appearance_blocks {
                        let mut stmt = self
                            .login_db
                            .prepare(LoginStatements::INS_BNET_ITEM_APPEARANCES);
                        stmt.set_u32(0, *bnet_account_id);
                        stmt.set_u32(1, block.block_index);
                        stmt.set_u32(2, block.mask);
                        tx.append(stmt);
                    }
                    // Inserts before deletes, as the Session built them.
                    for id in favorite_inserts {
                        let mut stmt = self
                            .login_db
                            .prepare(LoginStatements::INS_BNET_ITEM_FAVORITE_APPEARANCE);
                        stmt.set_u32(0, *bnet_account_id);
                        stmt.set_u32(1, *id);
                        tx.append(stmt);
                    }
                    for id in favorite_deletes {
                        let mut stmt = self
                            .login_db
                            .prepare(LoginStatements::DEL_BNET_ITEM_FAVORITE_APPEARANCE);
                        stmt.set_u32(0, *bnet_account_id);
                        stmt.set_u32(1, *id);
                        tx.append(stmt);
                    }
                    appearance_blocks.len() + favorite_inserts.len() + favorite_deletes.len()
                }
                AccountCollectionSaveLikeCpp::TransmogIllusions {
                    bnet_account_id,
                    illusion_blocks,
                } => {
                    for block in illusion_blocks {
                        let mut stmt = self
                            .login_db
                            .prepare(LoginStatements::INS_BNET_TRANSMOG_ILLUSIONS);
                        stmt.set_u32(0, *bnet_account_id);
                        stmt.set_u32(1, block.block_index);
                        stmt.set_u32(2, block.mask);
                        tx.append(stmt);
                    }
                    illusion_blocks.len()
                }
            };
            // One collection, one transaction — the shape C++ logout uses and
            // #187 freezes.
            match self.login_db.commit_transaction(tx).await {
                Ok(()) => PersistenceOutcomeLikeCpp::Applied { rows: rows as u64 },
                Err(error) => PersistenceOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }

    fn save_character_like_cpp<'a>(
        &'a self,
        request: PlayerCharacterSaveRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerCharacterSaveResultLikeCpp> {
        Box::pin(async move {
            let committed = request.committed_groups_like_cpp();
            let statements = player_character_save_statements_like_cpp(&request);
            let rows = statements.len() as u64;
            let mut tx = SqlTransaction::new();
            for statement in statements {
                tx.append(statement);
            }
            let outcome = match tx
                .commit_with_outcome_like_cpp(self.character_db.pool())
                .await
            {
                Ok(()) => PersistenceOutcomeLikeCpp::Applied { rows },
                Err(SqlTransactionCommitError::DefinitelyRolledBack(error)) => {
                    PersistenceOutcomeLikeCpp::Failed {
                        reason: error.to_string(),
                    }
                }
                Err(SqlTransactionCommitError::CommitOutcomeUnknown(error)) => {
                    PersistenceOutcomeLikeCpp::Unknown {
                        reason: error.to_string(),
                    }
                }
            };
            PlayerCharacterSaveResultLikeCpp { outcome, committed }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SqlParam;
    use wow_persistence::*;

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

    #[test]
    fn account_collection_load_requests_map_to_exact_login_statements_like_cpp() {
        let cases = [
            (
                AccountCollectionLoadRequestLikeCpp::Toys {
                    bnet_account_id: 77,
                },
                vec![LoginStatements::SEL_ACCOUNT_TOYS.sql()],
            ),
            (
                AccountCollectionLoadRequestLikeCpp::Heirlooms {
                    bnet_account_id: 77,
                },
                vec![LoginStatements::SEL_ACCOUNT_HEIRLOOMS.sql()],
            ),
            (
                AccountCollectionLoadRequestLikeCpp::Mounts {
                    bnet_account_id: 77,
                },
                vec![LoginStatements::SEL_ACCOUNT_MOUNTS.sql()],
            ),
            (
                AccountCollectionLoadRequestLikeCpp::ItemAppearances {
                    bnet_account_id: 77,
                },
                vec![
                    LoginStatements::SEL_BNET_ITEM_APPEARANCES.sql(),
                    LoginStatements::SEL_BNET_ITEM_FAVORITE_APPEARANCES.sql(),
                ],
            ),
            (
                AccountCollectionLoadRequestLikeCpp::TransmogIllusions {
                    bnet_account_id: 77,
                },
                vec![LoginStatements::SEL_BNET_TRANSMOG_ILLUSIONS.sql()],
            ),
        ];

        for (request, expected_sql) in cases {
            let statements = account_collection_load_statements_like_cpp(request);
            assert_eq!(
                statements
                    .iter()
                    .map(PreparedStatement::sql)
                    .collect::<Vec<_>>(),
                expected_sql
            );
            assert!(
                statements
                    .iter()
                    .all(|statement| { statement.params() == [crate::SqlParam::U32(77)] })
            );
        }
    }

    #[test]
    fn player_login_auxiliary_requests_map_to_exact_character_statements_like_cpp() {
        let cases = [
            (
                PlayerLoginAuxiliaryLoadRequestLikeCpp::Customizations { player_guid: 77 },
                CharStatements::SEL_CHARACTER_CUSTOMIZATIONS.sql(),
                vec![crate::SqlParam::U64(77)],
            ),
            (
                PlayerLoginAuxiliaryLoadRequestLikeCpp::CompletedAchievements { player_guid: 77 },
                CharStatements::SEL_CHARACTER_ACHIEVEMENTS.sql(),
                vec![crate::SqlParam::U64(77)],
            ),
            (
                PlayerLoginAuxiliaryLoadRequestLikeCpp::InstanceTimeRestrictions { account_id: 88 },
                CharStatements::SEL_ACCOUNT_INSTANCELOCKTIMES.sql(),
                vec![crate::SqlParam::U32(88)],
            ),
            (
                PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellCooldowns { player_guid: 77 },
                CharStatements::SEL_CHARACTER_SPELLCOOLDOWNS.sql(),
                vec![crate::SqlParam::U64(77)],
            ),
            (
                PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellCharges { player_guid: 77 },
                CharStatements::SEL_CHARACTER_SPELL_CHARGES.sql(),
                vec![crate::SqlParam::U64(77)],
            ),
            (
                PlayerLoginAuxiliaryLoadRequestLikeCpp::TraitEntries { player_guid: 77 },
                CharStatements::SEL_CHAR_TRAIT_ENTRIES.sql(),
                vec![crate::SqlParam::U64(77)],
            ),
            (
                PlayerLoginAuxiliaryLoadRequestLikeCpp::TraitConfigs { player_guid: 77 },
                CharStatements::SEL_CHAR_TRAIT_CONFIGS.sql(),
                vec![crate::SqlParam::U64(77)],
            ),
            (
                PlayerLoginAuxiliaryLoadRequestLikeCpp::PetStable { player_guid: 77 },
                CharStatements::SEL_CHAR_PETS.sql(),
                vec![crate::SqlParam::U64(77)],
            ),
            (
                PlayerLoginAuxiliaryLoadRequestLikeCpp::PetAuras { pet_number: 42 },
                CharStatements::SEL_PET_AURA.sql(),
                vec![crate::SqlParam::U32(42)],
            ),
            (
                PlayerLoginAuxiliaryLoadRequestLikeCpp::PetAuraEffects { pet_number: 42 },
                CharStatements::SEL_PET_AURA_EFFECT.sql(),
                vec![crate::SqlParam::U32(42)],
            ),
            (
                PlayerLoginAuxiliaryLoadRequestLikeCpp::PetSpells { pet_number: 42 },
                CharStatements::SEL_PET_SPELL.sql(),
                vec![crate::SqlParam::U32(42)],
            ),
            (
                PlayerLoginAuxiliaryLoadRequestLikeCpp::PetSpellCooldowns { pet_number: 42 },
                CharStatements::SEL_PET_SPELL_COOLDOWN.sql(),
                vec![crate::SqlParam::U32(42)],
            ),
            (
                PlayerLoginAuxiliaryLoadRequestLikeCpp::PetSpellCharges { pet_number: 42 },
                CharStatements::SEL_PET_SPELL_CHARGES.sql(),
                vec![crate::SqlParam::U32(42)],
            ),
            (
                PlayerLoginAuxiliaryLoadRequestLikeCpp::PetDeclinedNames {
                    player_guid: 77,
                    pet_number: 42,
                },
                CharStatements::SEL_PET_DECLINED_NAME.sql(),
                vec![crate::SqlParam::U64(77), crate::SqlParam::U32(42)],
            ),
        ];

        for (request, expected_sql, expected_params) in cases {
            let statement = player_login_auxiliary_load_statement_like_cpp(request);
            assert_eq!(statement.sql(), expected_sql);
            assert_eq!(statement.params(), expected_params);
        }
    }

    #[test]
    fn player_login_admission_requests_map_to_exact_cpp_statements_and_guid_bind() {
        let cases = [
            (
                PlayerLoginAdmissionLoadRequestLikeCpp::BattlegroundLocation { player_guid: 77 },
                CharStatements::SEL_CHARACTER_BGDATA.sql(),
            ),
            (
                PlayerLoginAdmissionLoadRequestLikeCpp::HomebindLocation { player_guid: 77 },
                CharStatements::SEL_CHARACTER_HOMEBIND.sql(),
            ),
            (
                PlayerLoginAdmissionLoadRequestLikeCpp::GuildMembership { player_guid: 77 },
                CharStatements::SEL_GUILD_MEMBER.sql(),
            ),
        ];

        for (request, expected_sql) in cases {
            let statement = player_login_admission_load_statement_like_cpp(request);
            assert_eq!(statement.sql(), expected_sql);
            assert_eq!(statement.params(), [crate::SqlParam::U64(77)]);
        }
    }

    #[test]
    fn character_base_load_maps_to_exact_cpp_statement_and_guid_bind() {
        let statement =
            player_character_base_load_statement_like_cpp(PlayerCharacterBaseLoadRequestLikeCpp {
                player_guid: 77,
            });
        assert_eq!(statement.sql(), CharStatements::SEL_CHARACTER.sql());
        assert_eq!(statement.params(), [crate::SqlParam::U64(77)]);
    }

    #[test]
    fn buyback_clear_maps_to_one_ordered_character_transaction_plan_like_cpp() {
        let _serialized = crate::persistence_trace::capture_flag_test_lock();
        let _capture = crate::persistence_trace::RecordingGuard::enable();
        let statements =
            player_buyback_clear_statements_like_cpp(&PlayerBuybackClearRequestLikeCpp {
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

        let xp_only =
            player_xp_persistence_statements_like_cpp(&PlayerXpPersistenceRequestLikeCpp {
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

        let all = player_login_transport_load_statement_like_cpp(
            PlayerLoginTransportLoadRequestLikeCpp::All,
        );
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

    #[test]
    fn character_save_adapter_preserves_the_frozen_statement_order_like_cpp() {
        let mut equipment_insert = equipment_row();
        equipment_insert.set_guid = 10;
        let mut equipment_update = equipment_row();
        equipment_update.set_guid = 11;
        equipment_update.state = PlayerEquipmentSetStateLikeCpp::Changed;
        let mut equipment_delete = equipment_row();
        equipment_delete.set_guid = 12;
        equipment_delete.state = PlayerEquipmentSetStateLikeCpp::Deleted;
        let mut transmog_insert = equipment_row();
        transmog_insert.set_guid = 13;
        transmog_insert.set_type = PlayerEquipmentSetTypeLikeCpp::Transmog;
        let mut transmog_update = equipment_row();
        transmog_update.set_guid = 14;
        transmog_update.set_type = PlayerEquipmentSetTypeLikeCpp::Transmog;
        transmog_update.state = PlayerEquipmentSetStateLikeCpp::Changed;
        let mut transmog_delete = equipment_row();
        transmog_delete.set_guid = 15;
        transmog_delete.set_type = PlayerEquipmentSetTypeLikeCpp::Transmog;
        transmog_delete.state = PlayerEquipmentSetStateLikeCpp::Deleted;

        let request = PlayerCharacterSaveRequestLikeCpp {
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
                powers: Some([0; 10]),
                talent_reset_cost: 0,
                talent_reset_time: 0,
                explored_zones: String::new(),
                dungeon_difficulty: 0,
                raid_difficulty: 0,
                legacy_raid_difficulty: 0,
            },
            spells: Some(PlayerSpellSaveGroupLikeCpp::Complete {
                rows: vec![PlayerSpellSaveLikeCpp {
                    spell_id: 100,
                    active: true,
                    disabled: false,
                    dependent: false,
                    favorite: true,
                    state: PlayerSpellStateLikeCpp::Changed,
                }],
                fallback_rows_were_present: false,
            }),
            skills: Some(vec![PlayerSkillSaveLikeCpp {
                skill_id: 6,
                value: 300,
                max: 300,
                profession_slot: -1,
            }]),
            glyphs: Some(
                (0..24)
                    .map(|glyph_slot| PlayerGlyphSaveLikeCpp {
                        talent_group: 0,
                        glyph_slot,
                        glyph_id: 0,
                    })
                    .collect(),
            ),
            talents: Some(vec![PlayerTalentSaveLikeCpp {
                talent_id: 200,
                rank: 1,
                talent_group: 0,
            }]),
            spell_cooldowns: Some(vec![PlayerSpellCooldownSaveLikeCpp {
                spell_id: 300,
                item_id: 0,
                cooldown_end_unix_secs: 1_700_000_010,
                category_id: 30,
                category_end_unix_secs: 1_700_000_020,
            }]),
            spell_charges: Some(vec![PlayerSpellChargeSaveLikeCpp {
                category_id: 31,
                recharge_start_unix_secs: 1_700_000_001,
                recharge_end_unix_secs: 1_700_000_030,
            }]),
            action_buttons: Some(PlayerActionButtonsSaveLikeCpp {
                spec: 0,
                trait_config_id: 0,
                rows: vec![PlayerActionButtonSaveLikeCpp {
                    button: 0,
                    packed_action: 0x0100_0064,
                }],
            }),
            equipment_sets: Some(vec![
                equipment_insert,
                equipment_update,
                equipment_delete,
                transmog_insert,
                transmog_update,
                transmog_delete,
            ]),
            void_storage: Some(vec![
                PlayerVoidStorageSlotSaveLikeCpp {
                    slot: 0,
                    item: Some(PlayerVoidStorageSaveLikeCpp {
                        item_id: 400,
                        item_entry: 401,
                        creator_guid: 402,
                        fixed_scaling_level: 80,
                        random_properties_id: 0,
                        random_properties_seed: 0,
                        context: 0,
                    }),
                },
                PlayerVoidStorageSlotSaveLikeCpp {
                    slot: 1,
                    item: None,
                },
            ]),
            tutorials: Some(PlayerTutorialsSaveLikeCpp {
                tutorials: [0; 8],
                already_persisted: false,
            }),
            instance_lock_times: vec![PlayerInstanceLockTimeSaveLikeCpp {
                instance_id: 500,
                release_time: 1_700_000_100,
            }],
            played_time: PlayerPlayedTimeSaveLikeCpp {
                total_time: 11,
                level_time: 5,
            },
            reputations: vec![PlayerReputationSaveLikeCpp {
                faction_id: 600,
                standing: 1_000,
                flags: 1,
            }],
            cuf_profiles: Some(vec![
                PlayerCufProfileSlotSaveLikeCpp {
                    profile_id: 0,
                    profile: Some(PlayerCufProfileSaveLikeCpp {
                        profile_name: "raid".to_owned(),
                        frame_height: 40,
                        frame_width: 80,
                        sort_by: 0,
                        health_text: 0,
                        bool_options: 0,
                        top_point: 0,
                        bottom_point: 0,
                        left_point: 0,
                        top_offset: 0,
                        bottom_offset: 0,
                        left_offset: 0,
                    }),
                },
                PlayerCufProfileSlotSaveLikeCpp {
                    profile_id: 1,
                    profile: None,
                },
            ]),
        };
        let mut runs: Vec<(String, usize)> = Vec::new();
        for statement in player_character_save_statements_like_cpp(&request) {
            let sql = statement.sql().to_owned();
            match runs.last_mut() {
                Some((previous, count)) if *previous == sql => *count += 1,
                _ => runs.push((sql, 1)),
            }
        }
        let golden: Vec<(String, usize)> = serde_json::from_str(include_str!(
            "../../wow-world/tests/fixtures/player-save-plan-order.json"
        ))
        .expect("frozen order fixture parses");
        assert_eq!(
            runs,
            golden,
            "replace player-save-plan-order.json with:\n{}",
            serde_json::to_string_pretty(&runs).unwrap_or_default()
        );
    }

    #[test]
    fn fallback_spells_expand_inside_the_spell_group_without_a_statement_port_like_cpp() {
        let mut request = minimal_character_request();
        request.spells = Some(PlayerSpellSaveGroupLikeCpp::Fallback {
            rows: vec![
                PlayerFallbackSpellSaveLikeCpp {
                    spell_id: 20,
                    active: true,
                    dependent: false,
                },
                PlayerFallbackSpellSaveLikeCpp {
                    spell_id: 30,
                    active: false,
                    dependent: true,
                },
            ],
        });
        let statements = player_character_save_statements_like_cpp(&request);
        let sql = statements
            .iter()
            .map(PreparedStatement::sql)
            .collect::<Vec<_>>();
        let fallback = sql
            .iter()
            .position(|sql| *sql == CharStatements::UPSERT_CHAR_SPELL_LEARN_FALLBACK.sql())
            .expect("non-dependent fallback spell is upserted");
        assert_eq!(
            sql.get(fallback + 1),
            Some(&CharStatements::DEL_CHAR_SPELL_BY_SPELL.sql()),
            "dependent fallback removal retains its position in the spell group"
        );
    }

    #[test]
    fn cooldown_and_charge_groups_drop_expired_rows_but_keep_the_group_replace_like_cpp() {
        let mut request = minimal_character_request();
        request.spell_cooldowns = Some(vec![
            PlayerSpellCooldownSaveLikeCpp {
                spell_id: 1,
                item_id: 0,
                cooldown_end_unix_secs: 1_699_999_999,
                category_id: 0,
                category_end_unix_secs: 1_699_999_999,
            },
            PlayerSpellCooldownSaveLikeCpp {
                spell_id: 2,
                item_id: 0,
                cooldown_end_unix_secs: 1_700_000_001,
                category_id: 0,
                category_end_unix_secs: 1_699_999_999,
            },
        ]);
        request.spell_charges = Some(vec![
            PlayerSpellChargeSaveLikeCpp {
                category_id: 3,
                recharge_start_unix_secs: 1_699_999_990,
                recharge_end_unix_secs: 1_699_999_999,
            },
            PlayerSpellChargeSaveLikeCpp {
                category_id: 4,
                recharge_start_unix_secs: 1_700_000_000,
                recharge_end_unix_secs: 1_700_000_001,
            },
        ]);
        let statements = player_character_save_statements_like_cpp(&request);
        let sql = statements
            .iter()
            .map(PreparedStatement::sql)
            .collect::<Vec<_>>();
        assert_eq!(
            sql.iter()
                .filter(|sql| **sql == CharStatements::INS_CHAR_SPELL_COOLDOWN.sql())
                .count(),
            1
        );
        assert_eq!(
            sql.iter()
                .filter(|sql| **sql == CharStatements::INS_CHAR_SPELL_CHARGES.sql())
                .count(),
            1
        );
        assert!(sql.contains(&CharStatements::DEL_CHAR_SPELL_COOLDOWNS.sql()));
        assert!(sql.contains(&CharStatements::DEL_CHAR_SPELL_CHARGES.sql()));
    }
}
