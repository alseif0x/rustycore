use super::super::collections::account_collection_load_statements_like_cpp;
use super::super::login_reads::nonnegative_i32_to_u32_like_cpp;
use super::super::login_reads::nonnegative_i64_to_u64_like_cpp;
use super::super::login_reads::player_character_base_load_statement_like_cpp;
use super::super::login_reads::player_login_admission_load_statement_like_cpp;
use super::super::login_reads::player_login_auxiliary_load_statement_like_cpp;
use crate::params::PreparedStatement;
use crate::statements::StatementDef;
use crate::statements::{CharStatements, LoginStatements};
use wow_persistence::{
    AccountCollectionLoadRequestLikeCpp, PlayerCharacterBaseLoadRequestLikeCpp,
    PlayerLoginAdmissionLoadRequestLikeCpp, PlayerLoginAuxiliaryLoadRequestLikeCpp,
};

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
            PlayerLoginAuxiliaryLoadRequestLikeCpp::Mail { player_guid: 77 },
            CharStatements::SEL_MAIL.sql(),
            vec![crate::SqlParam::U64(77)],
        ),
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
        (
            PlayerLoginAuxiliaryLoadRequestLikeCpp::GroupMembership { player_guid: 77 },
            CharStatements::SEL_GROUP_MEMBER.sql(),
            vec![crate::SqlParam::U64(77)],
        ),
        (
            PlayerLoginAuxiliaryLoadRequestLikeCpp::EquipmentSets { player_guid: 77 },
            CharStatements::SEL_CHARACTER_EQUIPMENTSETS.sql(),
            vec![crate::SqlParam::U64(77)],
        ),
        (
            PlayerLoginAuxiliaryLoadRequestLikeCpp::TransmogOutfits { player_guid: 77 },
            CharStatements::SEL_CHARACTER_TRANSMOG_OUTFITS.sql(),
            vec![crate::SqlParam::U64(77)],
        ),
        (
            PlayerLoginAuxiliaryLoadRequestLikeCpp::CufProfiles { player_guid: 77 },
            CharStatements::SEL_CHAR_CUF_PROFILES.sql(),
            vec![crate::SqlParam::U64(77)],
        ),
        (
            PlayerLoginAuxiliaryLoadRequestLikeCpp::Currencies { player_guid: 77 },
            CharStatements::SEL_PLAYER_CURRENCY.sql(),
            vec![crate::SqlParam::U64(77)],
        ),
        (
            PlayerLoginAuxiliaryLoadRequestLikeCpp::Spells { player_guid: 77 },
            CharStatements::SEL_CHARACTER_SPELL.sql(),
            vec![crate::SqlParam::U64(77)],
        ),
        (
            PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellFavorites { player_guid: 77 },
            CharStatements::SEL_CHARACTER_SPELL_FAVORITES.sql(),
            vec![crate::SqlParam::U64(77)],
        ),
        (
            PlayerLoginAuxiliaryLoadRequestLikeCpp::Skills { player_guid: 77 },
            CharStatements::SEL_CHARACTER_SKILLS.sql(),
            vec![crate::SqlParam::U64(77)],
        ),
        (
            PlayerLoginAuxiliaryLoadRequestLikeCpp::Talents { player_guid: 77 },
            CharStatements::SEL_CHARACTER_TALENTS.sql(),
            vec![crate::SqlParam::U64(77)],
        ),
        (
            PlayerLoginAuxiliaryLoadRequestLikeCpp::Glyphs { player_guid: 77 },
            CharStatements::SEL_CHARACTER_GLYPHS.sql(),
            vec![crate::SqlParam::U64(77)],
        ),
        (
            PlayerLoginAuxiliaryLoadRequestLikeCpp::ActionButtons {
                player_guid: 77,
                active_spec: 2,
                trait_config_id: -3,
            },
            CharStatements::SEL_CHARACTER_ACTIONS_SPEC.sql(),
            vec![
                crate::SqlParam::U64(77),
                crate::SqlParam::U8(2),
                crate::SqlParam::I32(-3),
            ],
        ),
        (
            PlayerLoginAuxiliaryLoadRequestLikeCpp::Reputation { player_guid: 77 },
            CharStatements::SEL_CHARACTER_REPUTATION.sql(),
            vec![crate::SqlParam::U64(77)],
        ),
        (
            PlayerLoginAuxiliaryLoadRequestLikeCpp::CharacterAuras { player_guid: 77 },
            CharStatements::SEL_CHARACTER_AURAS.sql(),
            vec![crate::SqlParam::U64(77)],
        ),
        (
            PlayerLoginAuxiliaryLoadRequestLikeCpp::CharacterAuraEffects { player_guid: 77 },
            CharStatements::SEL_CHARACTER_AURA_EFFECTS.sql(),
            vec![crate::SqlParam::U64(77)],
        ),
        (
            PlayerLoginAuxiliaryLoadRequestLikeCpp::EquipmentInventory { player_guid: 77 },
            CharStatements::SEL_CHAR_EQUIPMENT.sql(),
            vec![crate::SqlParam::U64(77)],
        ),
        (
            PlayerLoginAuxiliaryLoadRequestLikeCpp::BagInventory { player_guid: 77 },
            CharStatements::SEL_CHAR_BAG_CONTENTS.sql(),
            vec![crate::SqlParam::U64(77)],
        ),
        (
            PlayerLoginAuxiliaryLoadRequestLikeCpp::VoidStorage { player_guid: 77 },
            CharStatements::SEL_CHAR_VOID_STORAGE.sql(),
            vec![crate::SqlParam::U64(77)],
        ),
    ];

    for (request, expected_sql, expected_params) in cases {
        let statement = player_login_auxiliary_load_statement_like_cpp(request);
        assert_eq!(statement.sql(), expected_sql);
        assert_eq!(statement.params(), expected_params);
    }
}

#[test]
fn transmog_signed_schema_values_decode_as_cpp_unsigned_fields() {
    assert_eq!(nonnegative_i64_to_u64_like_cpp(3), Some(3));
    assert_eq!(nonnegative_i32_to_u32_like_cpp(0x7_FFFF), Some(0x7_FFFF));
    assert_eq!(nonnegative_i64_to_u64_like_cpp(-1), None);
    assert_eq!(nonnegative_i32_to_u32_like_cpp(-1), None);
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
