//! Character login statement selection and row decoding.
//! Private MariaDB implementation; no semantic port or transaction changes.

use crate::statements::StatementDef;

use crate::params::PreparedStatement;
use crate::statements::CharStatements;
use wow_persistence::{
    PlayerCharacterBaseLoadRequestLikeCpp, PlayerCharacterBaseLoadRowLikeCpp,
    PlayerInventoryItemLoadRowLikeCpp, PlayerLoginAdmissionLoadRequestLikeCpp,
    PlayerLoginAuxiliaryLoadRequestLikeCpp,
};

pub(super) fn player_login_admission_load_statement_like_cpp(
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

pub(super) fn player_login_auxiliary_load_statement_like_cpp(
    request: PlayerLoginAuxiliaryLoadRequestLikeCpp,
) -> PreparedStatement {
    match request {
        PlayerLoginAuxiliaryLoadRequestLikeCpp::Mail { player_guid } => {
            let mut statement = PreparedStatement::for_statement(CharStatements::SEL_MAIL);
            statement.set_u64(0, player_guid);
            statement
        }
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
        PlayerLoginAuxiliaryLoadRequestLikeCpp::GroupMembership { player_guid } => {
            let mut statement = PreparedStatement::for_statement(CharStatements::SEL_GROUP_MEMBER);
            statement.set_u64(0, player_guid);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::EquipmentSets { player_guid } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::SEL_CHARACTER_EQUIPMENTSETS);
            statement.set_u64(0, player_guid);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::TransmogOutfits { player_guid } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::SEL_CHARACTER_TRANSMOG_OUTFITS);
            statement.set_u64(0, player_guid);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::CufProfiles { player_guid } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::SEL_CHAR_CUF_PROFILES);
            statement.set_u64(0, player_guid);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::Currencies { player_guid } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::SEL_PLAYER_CURRENCY);
            statement.set_u64(0, player_guid);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::Spells { player_guid } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::SEL_CHARACTER_SPELL);
            statement.set_u64(0, player_guid);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellFavorites { player_guid } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::SEL_CHARACTER_SPELL_FAVORITES);
            statement.set_u64(0, player_guid);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::Skills { player_guid } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::SEL_CHARACTER_SKILLS);
            statement.set_u64(0, player_guid);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::Talents { player_guid } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::SEL_CHARACTER_TALENTS);
            statement.set_u64(0, player_guid);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::Glyphs { player_guid } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::SEL_CHARACTER_GLYPHS);
            statement.set_u64(0, player_guid);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::ActionButtons {
            player_guid,
            active_spec,
            trait_config_id,
        } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::SEL_CHARACTER_ACTIONS_SPEC);
            statement.set_u64(0, player_guid);
            statement.set_u8(1, active_spec);
            statement.set_i32(2, trait_config_id);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::Reputation { player_guid } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::SEL_CHARACTER_REPUTATION);
            statement.set_u64(0, player_guid);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::CharacterAuras { player_guid } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::SEL_CHARACTER_AURAS);
            statement.set_u64(0, player_guid);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::CharacterAuraEffects { player_guid } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::SEL_CHARACTER_AURA_EFFECTS);
            statement.set_u64(0, player_guid);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::EquipmentInventory { player_guid } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::SEL_CHAR_EQUIPMENT);
            statement.set_u64(0, player_guid);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::BagInventory { player_guid } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::SEL_CHAR_BAG_CONTENTS);
            statement.set_u64(0, player_guid);
            statement
        }
        PlayerLoginAuxiliaryLoadRequestLikeCpp::VoidStorage { player_guid } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::SEL_CHAR_VOID_STORAGE);
            statement.set_u64(0, player_guid);
            statement
        }
    }
}

pub(super) fn player_inventory_item_load_row_like_cpp(
    result: &crate::SqlResult,
    first_column: usize,
) -> PlayerInventoryItemLoadRowLikeCpp {
    PlayerInventoryItemLoadRowLikeCpp {
        item_entry: result.try_read::<u32>(first_column).unwrap_or(0),
        item_db_guid: result.try_read::<u64>(first_column + 1).unwrap_or(0),
        count: result.try_read::<u32>(first_column + 2).unwrap_or(1),
        durability: result.try_read::<u32>(first_column + 3).unwrap_or(0),
        context: result.try_read::<u8>(first_column + 4).unwrap_or(0),
        flags: result.try_read::<u32>(first_column + 5).unwrap_or(0),
        played_time: result.try_read::<u32>(first_column + 6).unwrap_or(0),
        enchantments: result
            .try_read::<String>(first_column + 7)
            .unwrap_or_default(),
        random_properties_id: result.try_read::<i32>(first_column + 8).unwrap_or(0),
        random_properties_seed: result.try_read::<i32>(first_column + 9).unwrap_or(0),
        gems: [
            (
                result.try_read::<i32>(first_column + 10).unwrap_or(0),
                result
                    .try_read::<String>(first_column + 11)
                    .unwrap_or_default(),
                result.try_read::<u8>(first_column + 12).unwrap_or(0),
            ),
            (
                result.try_read::<i32>(first_column + 13).unwrap_or(0),
                result
                    .try_read::<String>(first_column + 14)
                    .unwrap_or_default(),
                result.try_read::<u8>(first_column + 15).unwrap_or(0),
            ),
            (
                result.try_read::<i32>(first_column + 16).unwrap_or(0),
                result
                    .try_read::<String>(first_column + 17)
                    .unwrap_or_default(),
                result.try_read::<u8>(first_column + 18).unwrap_or(0),
            ),
        ],
        paid_money: result.try_read::<u64>(first_column + 19),
        paid_extended_cost: result.try_read::<u16>(first_column + 20),
        expiration: result.try_read::<u32>(first_column + 21).unwrap_or(0),
        spell_charges: result
            .try_read::<String>(first_column + 22)
            .unwrap_or_default(),
    }
}

pub(super) fn nonnegative_i64_to_u64_like_cpp(value: i64) -> Option<u64> {
    u64::try_from(value).ok()
}

pub(super) fn nonnegative_i32_to_u32_like_cpp(value: i32) -> Option<u32> {
    u32::try_from(value).ok()
}

pub(super) fn player_character_base_load_statement_like_cpp(
    request: PlayerCharacterBaseLoadRequestLikeCpp,
) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(CharStatements::SEL_CHARACTER);
    statement.set_u64(0, request.player_guid);
    statement
}

pub(super) fn player_character_base_load_row_like_cpp(
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
