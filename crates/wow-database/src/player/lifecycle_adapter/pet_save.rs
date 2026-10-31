// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! SQL plans for the two independent transactions in C++ `Pet::SavePetToDB`.

use crate::params::PreparedStatement;
use crate::statements::CharStatements;
use wow_persistence::{
    PlayerPetAuraEffectLoadRowLikeCpp, PlayerPetAuraLoadRowLikeCpp,
    PlayerPetSaveTransactionLikeCpp, PlayerPetSpellSaveOperationLikeCpp,
};

pub(super) fn player_pet_save_statements_like_cpp(
    request: PlayerPetSaveTransactionLikeCpp,
) -> Vec<PreparedStatement> {
    match request {
        PlayerPetSaveTransactionLikeCpp::AurasSpellsAndHistory {
            pet_number,
            auras,
            aura_effects,
            spells,
            cooldowns,
            charges,
        } => {
            let mut statements = Vec::new();

            // Pet::_SaveAuras deletes dependent effect rows before aura rows.
            statements.push(pet_number_statement(
                CharStatements::DEL_PET_AURA_EFFECTS,
                pet_number,
            ));
            statements.push(pet_number_statement(
                CharStatements::DEL_PET_AURAS,
                pet_number,
            ));

            // The C++ source inserts each aura followed by its effects. The
            // request retains the source ordering within both selected lists;
            // matching the shared aura key restores their interleaving while
            // still inserting every supplied effect row.
            let mut inserted_effects = vec![false; aura_effects.len()];
            for aura in &auras {
                statements.push(insert_pet_aura(pet_number, aura));
                for (index, effect) in aura_effects.iter().enumerate() {
                    if !inserted_effects[index] && same_pet_aura_key(aura, effect) {
                        statements.push(insert_pet_aura_effect(pet_number, effect));
                        inserted_effects[index] = true;
                    }
                }
            }
            // Keep the port a literal persistence adapter: malformed or
            // unmatched supplied rows are not filtered out here.
            for (index, effect) in aura_effects.iter().enumerate() {
                if !inserted_effects[index] {
                    statements.push(insert_pet_aura_effect(pet_number, effect));
                }
            }

            for spell in spells {
                match spell {
                    PlayerPetSpellSaveOperationLikeCpp::Delete { spell_id } => {
                        let mut statement = PreparedStatement::for_statement(
                            CharStatements::DEL_PET_SPELL_BY_SPELL,
                        );
                        statement.set_u32(0, pet_number);
                        statement.set_u32(1, spell_id);
                        statements.push(statement);
                    }
                    PlayerPetSpellSaveOperationLikeCpp::Insert { spell_id, active } => {
                        let mut statement =
                            PreparedStatement::for_statement(CharStatements::INS_PET_SPELL);
                        statement.set_u32(0, pet_number);
                        statement.set_u32(1, spell_id);
                        statement.set_u8(2, active);
                        statements.push(statement);
                    }
                }
            }

            // SpellHistory::SaveToDB<Pet>: cooldown delete/inserts precede
            // charge delete/inserts. Times are Unix seconds, matching
            // Clock::to_time_t and the existing load DTOs.
            statements.push(pet_number_statement(
                CharStatements::DEL_PET_SPELL_COOLDOWNS,
                pet_number,
            ));
            for cooldown in cooldowns {
                let mut statement =
                    PreparedStatement::for_statement(CharStatements::INS_PET_SPELL_COOLDOWN);
                statement.set_u32(0, pet_number);
                statement.set_u32(1, cooldown.spell_id);
                statement.set_i64(2, cooldown.cooldown_end_unix_secs);
                statement.set_u32(3, cooldown.category_id);
                statement.set_i64(4, cooldown.category_end_unix_secs);
                statements.push(statement);
            }

            statements.push(pet_number_statement(
                CharStatements::DEL_PET_SPELL_CHARGES,
                pet_number,
            ));
            for charge in charges {
                let mut statement =
                    PreparedStatement::for_statement(CharStatements::INS_PET_SPELL_CHARGES);
                statement.set_u32(0, pet_number);
                statement.set_u32(1, charge.category_id);
                statement.set_i64(2, charge.recharge_start_unix_secs);
                statement.set_i64(3, charge.recharge_end_unix_secs);
                statements.push(statement);
            }

            statements
        }
        PlayerPetSaveTransactionLikeCpp::PetRow { owner_guid, row } => {
            let mut statements = Vec::with_capacity(2);
            statements.push(pet_number_statement(
                CharStatements::DEL_CHAR_PET_BY_ID,
                row.pet_number,
            ));

            let mut insert = PreparedStatement::for_statement(CharStatements::INS_PET);
            insert.set_u32(0, row.pet_number);
            insert.set_u32(1, row.creature_id);
            insert.set_u64(2, owner_guid);
            insert.set_u32(3, row.display_id);
            insert.set_u8(4, row.level);
            insert.set_u32(5, row.experience);
            insert.set_u8(6, row.react_state);
            insert.set_i16(7, row.slot);
            insert.set_string(8, row.name);
            insert.set_u8(9, u8::from(row.was_renamed));
            insert.set_u32(10, row.health);
            insert.set_u32(11, row.mana);
            insert.set_string(12, row.action_bar);
            insert.set_u32(13, row.last_save_time);
            insert.set_u32(14, row.created_by_spell_id);
            insert.set_u8(15, row.pet_type);
            insert.set_u16(16, row.specialization_id);
            statements.push(insert);
            statements
        }
    }
}

fn pet_number_statement(statement: CharStatements, pet_number: u32) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(statement);
    statement.set_u32(0, pet_number);
    statement
}

fn insert_pet_aura(pet_number: u32, aura: &PlayerPetAuraLoadRowLikeCpp) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(CharStatements::INS_PET_AURA);
    statement.set_u32(0, pet_number);
    statement.set_bytes(1, aura.caster_guid_binary.clone());
    statement.set_u32(2, aura.spell_id);
    statement.set_u32(3, aura.effect_mask);
    statement.set_u32(4, aura.recalculate_mask);
    statement.set_u8(5, aura.difficulty);
    statement.set_u8(6, aura.stack_count);
    statement.set_i32(7, aura.max_duration_ms);
    statement.set_i32(8, aura.remain_time_ms);
    statement.set_u8(9, aura.remain_charges);
    statement
}

fn insert_pet_aura_effect(
    pet_number: u32,
    effect: &PlayerPetAuraEffectLoadRowLikeCpp,
) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(CharStatements::INS_PET_AURA_EFFECT);
    statement.set_u32(0, pet_number);
    statement.set_bytes(1, effect.caster_guid_binary.clone());
    statement.set_u32(2, effect.spell_id);
    statement.set_u32(3, effect.effect_mask);
    statement.set_u8(4, effect.effect_index);
    statement.set_i32(5, effect.amount);
    statement.set_i32(6, effect.base_amount);
    statement
}

fn same_pet_aura_key(
    aura: &PlayerPetAuraLoadRowLikeCpp,
    effect: &PlayerPetAuraEffectLoadRowLikeCpp,
) -> bool {
    aura.caster_guid_binary == effect.caster_guid_binary
        && aura.spell_id == effect.spell_id
        && aura.effect_mask == effect.effect_mask
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::SqlParam;
    use crate::statements::StatementDef;
    use wow_persistence::{
        PlayerPetSpellChargeLoadRowLikeCpp, PlayerPetSpellCooldownLoadRowLikeCpp,
        PlayerPetStableLoadRowLikeCpp,
    };

    fn aura(
        caster_guid_binary: Vec<u8>,
        spell_id: u32,
        effect_mask: u32,
    ) -> PlayerPetAuraLoadRowLikeCpp {
        PlayerPetAuraLoadRowLikeCpp {
            caster_guid_binary,
            spell_id,
            effect_mask,
            recalculate_mask: 0x3344,
            difficulty: 5,
            stack_count: 6,
            max_duration_ms: -7,
            remain_time_ms: 8,
            remain_charges: 9,
        }
    }

    fn effect(
        caster_guid_binary: Vec<u8>,
        spell_id: u32,
        effect_mask: u32,
        effect_index: u8,
    ) -> PlayerPetAuraEffectLoadRowLikeCpp {
        PlayerPetAuraEffectLoadRowLikeCpp {
            caster_guid_binary,
            spell_id,
            effect_mask,
            effect_index,
            amount: -10,
            base_amount: 11,
        }
    }

    #[test]
    fn pet_aura_spell_and_history_plan_preserves_cpp_order_and_bindings() {
        let caster_a = (0_u8..16).collect::<Vec<_>>();
        let caster_b = (16_u8..32).collect::<Vec<_>>();
        let statements = player_pet_save_statements_like_cpp(
            PlayerPetSaveTransactionLikeCpp::AurasSpellsAndHistory {
                pet_number: 0x1122_3344,
                auras: vec![
                    aura(caster_a.clone(), 100, 0x10),
                    aura(caster_b.clone(), 200, 0x20),
                ],
                // Deliberately place aura B first; output follows aura source
                // order while retaining effect source order within each aura.
                aura_effects: vec![
                    effect(caster_b.clone(), 200, 0x20, 3),
                    effect(caster_a.clone(), 100, 0x10, 1),
                    effect(caster_a.clone(), 100, 0x10, 2),
                ],
                spells: vec![
                    PlayerPetSpellSaveOperationLikeCpp::Delete { spell_id: 300 },
                    PlayerPetSpellSaveOperationLikeCpp::Insert {
                        spell_id: 301,
                        active: 2,
                    },
                ],
                cooldowns: vec![PlayerPetSpellCooldownLoadRowLikeCpp {
                    spell_id: 400,
                    cooldown_end_unix_secs: 1_700_000_111,
                    category_id: 0x5566,
                    category_end_unix_secs: 1_700_000_222,
                }],
                charges: vec![PlayerPetSpellChargeLoadRowLikeCpp {
                    category_id: 0x7788,
                    recharge_start_unix_secs: 1_700_000_333,
                    recharge_end_unix_secs: 1_700_000_444,
                }],
            },
        );

        let sql = statements
            .iter()
            .map(PreparedStatement::sql)
            .collect::<Vec<_>>();
        assert_eq!(
            sql,
            vec![
                CharStatements::DEL_PET_AURA_EFFECTS.sql(),
                CharStatements::DEL_PET_AURAS.sql(),
                CharStatements::INS_PET_AURA.sql(),
                CharStatements::INS_PET_AURA_EFFECT.sql(),
                CharStatements::INS_PET_AURA_EFFECT.sql(),
                CharStatements::INS_PET_AURA.sql(),
                CharStatements::INS_PET_AURA_EFFECT.sql(),
                CharStatements::DEL_PET_SPELL_BY_SPELL.sql(),
                CharStatements::INS_PET_SPELL.sql(),
                CharStatements::DEL_PET_SPELL_COOLDOWNS.sql(),
                CharStatements::INS_PET_SPELL_COOLDOWN.sql(),
                CharStatements::DEL_PET_SPELL_CHARGES.sql(),
                CharStatements::INS_PET_SPELL_CHARGES.sql(),
            ]
        );
        assert!(sql.iter().all(|sql| !sql.contains("character_pet")));

        assert_eq!(statements[0].params(), &[SqlParam::U32(0x1122_3344)]);
        assert_eq!(statements[1].params(), &[SqlParam::U32(0x1122_3344)]);
        assert_eq!(
            statements[2].params(),
            &[
                SqlParam::U32(0x1122_3344),
                SqlParam::Bytes(caster_a.clone()),
                SqlParam::U32(100),
                SqlParam::U32(0x10),
                SqlParam::U32(0x3344),
                SqlParam::U8(5),
                SqlParam::U8(6),
                SqlParam::I32(-7),
                SqlParam::I32(8),
                SqlParam::U8(9),
            ]
        );
        assert_eq!(
            statements[3].params(),
            &[
                SqlParam::U32(0x1122_3344),
                SqlParam::Bytes(caster_a.clone()),
                SqlParam::U32(100),
                SqlParam::U32(0x10),
                SqlParam::U8(1),
                SqlParam::I32(-10),
                SqlParam::I32(11),
            ]
        );
        assert_eq!(statements[4].params()[4], SqlParam::U8(2));
        assert_eq!(statements[6].params()[1], SqlParam::Bytes(caster_b));
        assert_eq!(
            statements[7].params(),
            &[SqlParam::U32(0x1122_3344), SqlParam::U32(300)]
        );
        assert_eq!(
            statements[8].params(),
            &[
                SqlParam::U32(0x1122_3344),
                SqlParam::U32(301),
                SqlParam::U8(2),
            ]
        );
        assert_eq!(
            statements[10].params(),
            &[
                SqlParam::U32(0x1122_3344),
                SqlParam::U32(400),
                SqlParam::I64(1_700_000_111),
                SqlParam::U32(0x5566),
                SqlParam::I64(1_700_000_222),
            ]
        );
        assert_eq!(
            statements[12].params(),
            &[
                SqlParam::U32(0x1122_3344),
                SqlParam::U32(0x7788),
                SqlParam::I64(1_700_000_333),
                SqlParam::I64(1_700_000_444),
            ]
        );
    }

    #[test]
    fn pet_row_plan_is_separate_and_binds_all_seventeen_cpp_columns() {
        let statements =
            player_pet_save_statements_like_cpp(PlayerPetSaveTransactionLikeCpp::PetRow {
                owner_guid: 0x0102_0304_0506_0708,
                row: PlayerPetStableLoadRowLikeCpp {
                    pet_number: 42,
                    creature_id: 123,
                    display_id: 456,
                    level: 7,
                    experience: 890,
                    react_state: 2,
                    slot: -2,
                    name: "Milo".to_owned(),
                    was_renamed: true,
                    health: 9_876,
                    mana: 5_432,
                    action_bar: "spell 1".to_owned(),
                    last_save_time: 1_700_000_555,
                    created_by_spell_id: 987,
                    pet_type: 3,
                    specialization_id: 0x1234,
                },
            });

        assert_eq!(
            statements
                .iter()
                .map(PreparedStatement::sql)
                .collect::<Vec<_>>(),
            vec![
                CharStatements::DEL_CHAR_PET_BY_ID.sql(),
                CharStatements::INS_PET.sql(),
            ]
        );
        assert_eq!(statements[0].params(), &[SqlParam::U32(42)]);
        assert_eq!(
            statements[1].params(),
            &[
                SqlParam::U32(42),
                SqlParam::U32(123),
                SqlParam::U64(0x0102_0304_0506_0708),
                SqlParam::U32(456),
                SqlParam::U8(7),
                SqlParam::U32(890),
                SqlParam::U8(2),
                SqlParam::I16(-2),
                SqlParam::String("Milo".to_owned()),
                SqlParam::U8(1),
                SqlParam::U32(9_876),
                SqlParam::U32(5_432),
                SqlParam::String("spell 1".to_owned()),
                SqlParam::U32(1_700_000_555),
                SqlParam::U32(987),
                SqlParam::U8(3),
                SqlParam::U16(0x1234),
            ]
        );
        assert_eq!(statements[1].params().len(), 17);
        assert!(!statements[1].sql().contains("WHERE owner"));
        assert!(statements.iter().all(|statement| {
            !statement.sql().contains("pet_aura") && !statement.sql().contains("pet_spell")
        }));
    }
}
