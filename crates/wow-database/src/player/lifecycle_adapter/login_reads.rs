//! Character login statement selection and row decoding.
//! Private MariaDB implementation; no semantic port or transaction changes.

use crate::statements::StatementDef;

use crate::field_like_cpp::{FieldNumberLikeCpp, FieldRowLikeCpp, NativeFieldLikeCpp};
use crate::params::PreparedStatement;
use crate::statements::CharStatements;
use wow_persistence::{
    PlayerCharacterBaseLoadRequestLikeCpp, PlayerCharacterBaseLoadRowLikeCpp,
    PlayerInventoryItemLoadPayload, PlayerInventoryItemLoadRowLikeCpp,
    PlayerLoginAdmissionLoadRequestLikeCpp, PlayerLoginAuxiliaryLoadRequestLikeCpp,
};

const INVENTORY_ITEM_LEGACY_FIELD_COUNT: usize = 23;
const INVENTORY_ITEM_PAYLOAD_FIELD_COUNT: usize = 28;
const TRANSMOG_COLUMNS: [&str; 18] = [
    "itemModifiedAppearanceAllSpecs",
    "itemModifiedAppearanceSpec1",
    "itemModifiedAppearanceSpec2",
    "itemModifiedAppearanceSpec3",
    "itemModifiedAppearanceSpec4",
    "itemModifiedAppearanceSpec5",
    "spellItemEnchantmentAllSpecs",
    "spellItemEnchantmentSpec1",
    "spellItemEnchantmentSpec2",
    "spellItemEnchantmentSpec3",
    "spellItemEnchantmentSpec4",
    "spellItemEnchantmentSpec5",
    "secondaryItemModifiedAppearanceAllSpecs",
    "secondaryItemModifiedAppearanceSpec1",
    "secondaryItemModifiedAppearanceSpec2",
    "secondaryItemModifiedAppearanceSpec3",
    "secondaryItemModifiedAppearanceSpec4",
    "secondaryItemModifiedAppearanceSpec5",
];

fn required_number<T: FieldNumberLikeCpp>(
    row: &[NativeFieldLikeCpp],
    column: usize,
    field: &str,
) -> Option<T> {
    if matches!(row.get(column)?, NativeFieldLikeCpp::Null) {
        return None;
    }
    row.get_like_cpp::<T>(column, field).ok()
}

fn nullable_number<T: FieldNumberLikeCpp>(
    row: &[NativeFieldLikeCpp],
    column: usize,
    field: &str,
) -> Option<Option<T>> {
    row.get_nullable_like_cpp::<T>(column, field).ok()
}

fn required_text(row: &[NativeFieldLikeCpp], column: usize) -> Option<String> {
    if matches!(row.get(column)?, NativeFieldLikeCpp::Null) {
        return None;
    }
    row.get_string_like_cpp(column, "item_instance.enchantments")
        .ok()
}

fn optional_text(row: &[NativeFieldLikeCpp], column: usize) -> Option<Option<String>> {
    match row.get(column)? {
        NativeFieldLikeCpp::Null => Some(None),
        NativeFieldLikeCpp::Text(text) => Some(Some(text.clone())),
        NativeFieldLikeCpp::Int8(_)
        | NativeFieldLikeCpp::UInt8(_)
        | NativeFieldLikeCpp::Int16(_)
        | NativeFieldLikeCpp::UInt16(_)
        | NativeFieldLikeCpp::Int32(_)
        | NativeFieldLikeCpp::UInt32(_)
        | NativeFieldLikeCpp::Int64(_)
        | NativeFieldLikeCpp::UInt64(_)
        | NativeFieldLikeCpp::Float(_)
        | NativeFieldLikeCpp::Double(_)
        | NativeFieldLikeCpp::Unsupported(_) => None,
    }
}

fn legacy_inventory_projection_guid(
    row: &[NativeFieldLikeCpp],
    first_column: usize,
) -> Option<u64> {
    if row.len() < first_column.checked_add(INVENTORY_ITEM_LEGACY_FIELD_COUNT)? {
        return None;
    }

    required_number::<u32>(row, first_column, "item_instance.itemEntry")?;
    let item_guid = required_number::<u64>(row, first_column + 1, "character_inventory.item")?;
    required_number::<u32>(row, first_column + 2, "item_instance.count")?;
    required_number::<u16>(row, first_column + 3, "item_instance.durability")?;
    required_number::<u8>(row, first_column + 4, "item_instance.context")?;
    required_number::<u32>(row, first_column + 5, "item_instance.flags")?;
    required_number::<u32>(row, first_column + 6, "item_instance.playedTime")?;
    required_text(row, first_column + 7)?;
    required_number::<i32>(row, first_column + 8, "item_instance.randomPropertiesId")?;
    required_number::<i32>(row, first_column + 9, "item_instance.randomPropertiesSeed")?;

    let gem_numeric_present = [
        nullable_number::<i32>(row, first_column + 10, "item_instance_gems.gemItemId1")?.is_some(),
        nullable_number::<u8>(row, first_column + 12, "item_instance_gems.gemContext1")?.is_some(),
        nullable_number::<i32>(row, first_column + 13, "item_instance_gems.gemItemId2")?.is_some(),
        nullable_number::<u8>(row, first_column + 15, "item_instance_gems.gemContext2")?.is_some(),
        nullable_number::<i32>(row, first_column + 16, "item_instance_gems.gemItemId3")?.is_some(),
        nullable_number::<u8>(row, first_column + 18, "item_instance_gems.gemContext3")?.is_some(),
    ];
    for column in [11, 14, 17] {
        optional_text(row, first_column + column)?;
    }
    let present_gem_columns = gem_numeric_present.iter().filter(|&&present| present).count();
    if present_gem_columns != 0 && present_gem_columns != gem_numeric_present.len() {
        return None;
    }
    if present_gem_columns == 0
        && [11, 14, 17]
            .iter()
            .any(|&column| optional_text(row, first_column + column) != Some(None))
    {
        return None;
    }

    let paid_money = nullable_number::<u64>(row, first_column + 19, "item_refund_instance.paidMoney")?;
    let paid_cost = nullable_number::<u16>(
        row,
        first_column + 20,
        "item_refund_instance.paidExtendedCost",
    )?;
    if paid_money.is_some() != paid_cost.is_some() {
        return None;
    }
    required_number::<u32>(row, first_column + 21, "item_instance.duration")?;
    optional_text(row, first_column + 22)?;

    Some(item_guid)
}

fn decode_inventory_item_payload(
    row: &[NativeFieldLikeCpp],
    first_column: usize,
) -> Option<PlayerInventoryItemLoadPayload> {
    let legacy_end = first_column.checked_add(INVENTORY_ITEM_LEGACY_FIELD_COUNT)?;
    let payload_end = legacy_end.checked_add(INVENTORY_ITEM_PAYLOAD_FIELD_COUNT)?;
    if row.len() < payload_end {
        return None;
    }
    let item_guid = legacy_inventory_projection_guid(row, first_column)?;

    let creator_guid = required_number::<u64>(row, legacy_end, "item_instance.creatorGuid")?;
    let gift_creator_guid =
        required_number::<u64>(row, legacy_end + 1, "item_instance.giftCreatorGuid")?;
    let text = optional_text(row, legacy_end + 2)?;
    let battle_pet_species_id =
        required_number::<u32>(row, legacy_end + 3, "item_instance.battlePetSpeciesId")?;
    let battle_pet_breed_data =
        required_number::<u32>(row, legacy_end + 4, "item_instance.battlePetBreedData")?;
    let battle_pet_level =
        required_number::<u16>(row, legacy_end + 5, "item_instance.battlePetLevel")?;
    let battle_pet_display_id =
        required_number::<u32>(row, legacy_end + 6, "item_instance.battlePetDisplayId")?;

    let transmog_row_guid = nullable_number::<u64>(row, legacy_end + 7, "transmogItemGuid")?;
    let mut transmog_modifiers = [0; 18];
    match transmog_row_guid {
        None => {
            if TRANSMOG_COLUMNS
                .iter()
                .enumerate()
                .any(|(index, _)| row[legacy_end + 8 + index] != NativeFieldLikeCpp::Null)
            {
                return None;
            }
        }
        Some(guid) if guid == item_guid => {
            for (index, name) in TRANSMOG_COLUMNS.iter().enumerate() {
                transmog_modifiers[index] = required_number::<u32>(
                    row,
                    legacy_end + 8 + index,
                    name,
                )?;
            }
        }
        Some(_) => return None,
    }

    let trade_row_guid = nullable_number::<u64>(row, legacy_end + 26, "soulboundTradeItemGuid")?;
    let allowed_players = optional_text(row, legacy_end + 27)?;
    let soulbound_trade_allowed_players = match (trade_row_guid, allowed_players) {
        (None, None) => None,
        (Some(guid), Some(value)) if guid == item_guid => Some(value),
        _ => return None,
    };

    Some(PlayerInventoryItemLoadPayload {
        creator_guid,
        gift_creator_guid,
        text,
        battle_pet_species_id,
        battle_pet_breed_data,
        battle_pet_level,
        battle_pet_display_id,
        transmog_appearance: transmog_modifiers[0..6].try_into().ok()?,
        transmog_enchant_illusion: transmog_modifiers[6..12].try_into().ok()?,
        transmog_secondary_appearance: transmog_modifiers[12..18].try_into().ok()?,
        soulbound_trade_allowed_players,
    })
}

fn number_or_default<T: FieldNumberLikeCpp>(
    row: &[NativeFieldLikeCpp],
    column: usize,
    field: &str,
) -> T {
    row.get_like_cpp::<T>(column, field).unwrap_or_default()
}

fn inventory_item_row_from_native(
    row: &[NativeFieldLikeCpp],
    first_column: usize,
) -> PlayerInventoryItemLoadRowLikeCpp {
    let nullable_i32 = |column, field| {
        row.get_nullable_like_cpp::<i32>(column, field)
            .ok()
            .flatten()
            .unwrap_or(0)
    };
    let nullable_u8 = |column, field| {
        row.get_nullable_like_cpp::<u8>(column, field)
            .ok()
            .flatten()
            .unwrap_or(0)
    };
    let string = |column, field| row.get_string_like_cpp(column, field).unwrap_or_default();

    PlayerInventoryItemLoadRowLikeCpp {
        item_entry: number_or_default(row, first_column, "item_instance.itemEntry"),
        item_db_guid: number_or_default(row, first_column + 1, "character_inventory.item"),
        count: match row.get(first_column + 2) {
            None | Some(NativeFieldLikeCpp::Null) => 1,
            Some(_) => row
                .get_like_cpp::<u32>(first_column + 2, "item_instance.count")
                .unwrap_or(1),
        },
        durability: u32::from(number_or_default::<u16>(
            row,
            first_column + 3,
            "item_instance.durability",
        )),
        context: number_or_default(row, first_column + 4, "item_instance.context"),
        flags: number_or_default(row, first_column + 5, "item_instance.flags"),
        played_time: number_or_default(row, first_column + 6, "item_instance.playedTime"),
        enchantments: string(first_column + 7, "item_instance.enchantments"),
        random_properties_id: number_or_default(
            row,
            first_column + 8,
            "item_instance.randomPropertiesId",
        ),
        random_properties_seed: number_or_default(
            row,
            first_column + 9,
            "item_instance.randomPropertiesSeed",
        ),
        gems: [
            (
                nullable_i32(first_column + 10, "item_instance_gems.gemItemId1"),
                string(first_column + 11, "item_instance_gems.gemBonuses1"),
                nullable_u8(first_column + 12, "item_instance_gems.gemContext1"),
            ),
            (
                nullable_i32(first_column + 13, "item_instance_gems.gemItemId2"),
                string(first_column + 14, "item_instance_gems.gemBonuses2"),
                nullable_u8(first_column + 15, "item_instance_gems.gemContext2"),
            ),
            (
                nullable_i32(first_column + 16, "item_instance_gems.gemItemId3"),
                string(first_column + 17, "item_instance_gems.gemBonuses3"),
                nullable_u8(first_column + 18, "item_instance_gems.gemContext3"),
            ),
        ],
        paid_money: row
            .get_nullable_like_cpp::<u64>(first_column + 19, "item_refund_instance.paidMoney")
            .ok()
            .flatten(),
        paid_extended_cost: row
            .get_nullable_like_cpp::<u16>(
                first_column + 20,
                "item_refund_instance.paidExtendedCost",
            )
            .ok()
            .flatten(),
        expiration: number_or_default(row, first_column + 21, "item_instance.duration"),
        spell_charges: string(first_column + 22, "item_instance.charges"),
        complete_payload: decode_inventory_item_payload(row, first_column),
    }
}

pub(super) fn player_inventory_item_load_row_like_cpp(
    result: &crate::SqlResult,
    first_column: usize,
) -> PlayerInventoryItemLoadRowLikeCpp {
    match result.native_row_like_cpp() {
        Ok(row) => inventory_item_row_from_native(&row, first_column),
        Err(_) => PlayerInventoryItemLoadRowLikeCpp {
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
            complete_payload: None,
        },
    }
}

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
        PlayerLoginAuxiliaryLoadRequestLikeCpp::CorpseLocation { player_guid } => {
            let mut statement =
                PreparedStatement::for_statement(CharStatements::SEL_CORPSE_LOCATION);
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
        death_expire_time: result.try_read(42),
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

#[cfg(test)]
mod inventory_payload_tests {
    use super::*;
    use super::NativeFieldLikeCpp as Native;

    fn inventory_row() -> Vec<NativeFieldLikeCpp> {
        let mut row = vec![
            Native::UInt32(2121),
            Native::UInt64(9301),
            Native::UInt32(3),
            Native::UInt16(57),
            Native::UInt8(2),
            Native::UInt32(0x20),
            Native::UInt32(1234),
            Native::Text("1 2 3".into()),
            Native::Int32(-17),
            Native::Int32(19),
            Native::UInt32(5001),
            Native::Text("101 102".into()),
            Native::UInt8(4),
            Native::UInt32(0),
            Native::Text(String::new()),
            Native::UInt8(0),
            Native::UInt32(0),
            Native::Text(String::new()),
            Native::UInt8(0),
            Native::UInt64(765432),
            Native::UInt16(81),
            Native::Int32(3600),
            Native::Text("2 -1".into()),
            Native::UInt64(9302),
            Native::UInt64(9303),
            Native::Text("wrapped item".into()),
            Native::UInt32(733),
            Native::UInt32(0x0102_0304),
            Native::UInt16(25),
            Native::UInt32(98765),
            Native::UInt64(9301),
        ];
        row.extend((1000..1018).map(Native::Int32));
        row.extend([
            Native::UInt64(9301),
            Native::Text("42 77 9001".into()),
        ]);
        row
    }

    #[test]
    fn inventory_item_payload_decodes_populated_values() {
        let row = inventory_row();
        let decoded = inventory_item_row_from_native(&row, 0);
        let payload = decoded.complete_payload.expect("complete item payload");

        assert_eq!(decoded.item_db_guid, 9301);
        assert_eq!(decoded.durability, 57);
        assert_eq!(decoded.expiration, 3600);
        assert_eq!(decoded.random_properties_id, -17);
        assert_eq!(payload.creator_guid, 9302);
        assert_eq!(payload.gift_creator_guid, 9303);
        assert_eq!(payload.text.as_deref(), Some("wrapped item"));
        assert_eq!(payload.battle_pet_species_id, 733);
        assert_eq!(payload.battle_pet_breed_data, 0x0102_0304);
        assert_eq!(payload.battle_pet_level, 25);
        assert_eq!(payload.battle_pet_display_id, 98765);
        assert_eq!(payload.transmog_appearance, [1000, 1001, 1002, 1003, 1004, 1005]);
        assert_eq!(
            payload.transmog_enchant_illusion,
            [1006, 1007, 1008, 1009, 1010, 1011]
        );
        assert_eq!(
            payload.transmog_secondary_appearance,
            [1012, 1013, 1014, 1015, 1016, 1017]
        );
        assert_eq!(
            payload.soulbound_trade_allowed_players.as_deref(),
            Some("42 77 9001")
        );
    }

    #[test]
    fn inventory_item_payload_maps_absent_join_rows_to_native_empty_values() {
        let mut row = inventory_row();
        for index in [10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 22] {
            row[index] = Native::Null;
        }
        row[25] = Native::Null;
        row[30] = Native::Null;
        row[31..49].fill(Native::Null);
        row[49] = Native::Null;
        row[50] = Native::Null;

        let payload = decode_inventory_item_payload(&row, 0).expect("complete empty joins");
        assert_eq!(payload.text, None);
        assert_eq!(payload.transmog_appearance, [0; 6]);
        assert_eq!(payload.transmog_enchant_illusion, [0; 6]);
        assert_eq!(payload.transmog_secondary_appearance, [0; 6]);
        assert_eq!(payload.soulbound_trade_allowed_players, None);
    }

    #[test]
    fn inventory_item_payload_is_unknown_when_any_required_projection_is_incomplete() {
        let mut missing_appended_column = inventory_row();
        missing_appended_column.pop();
        assert!(decode_inventory_item_payload(&missing_appended_column, 0).is_none());

        let mut undecodable_legacy_column = inventory_row();
        undecodable_legacy_column[9] = Native::Text("bad seed".into());
        assert!(decode_inventory_item_payload(&undecodable_legacy_column, 0).is_none());

        let mut malformed_transmog_row = inventory_row();
        malformed_transmog_row[31] = Native::Null;
        assert!(decode_inventory_item_payload(&malformed_transmog_row, 0).is_none());

        let mut malformed_trade_row = inventory_row();
        malformed_trade_row[50] = Native::Null;
        assert!(decode_inventory_item_payload(&malformed_trade_row, 0).is_none());
    }
}
