//! Scoped SQL bindings for the selected aura family.
//! C++ insert column order: Player.cpp `Player::_SaveAuras` (19811-19865).

use crate::params::PreparedStatement;
use crate::statements::CharStatements;
use wow_persistence::{PlayerLifecycleAuraEffectSave, PlayerLifecycleAuraSave};

pub(super) fn delete(
    guid: u64,
    caster_guid_binary: [u8; 16],
    item_guid_binary: [u8; 16],
    spell_id: u32,
) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(CharStatements::DEL_CHAR_AURA_BY_KEY);
    bind_key(
        &mut statement,
        guid,
        caster_guid_binary,
        item_guid_binary,
        spell_id,
    );
    statement
}

pub(super) fn delete_effects(
    guid: u64,
    caster_guid_binary: [u8; 16],
    item_guid_binary: [u8; 16],
    spell_id: u32,
) -> PreparedStatement {
    let mut statement =
        PreparedStatement::for_statement(CharStatements::DEL_CHAR_AURA_EFFECT_BY_KEY);
    bind_key(
        &mut statement,
        guid,
        caster_guid_binary,
        item_guid_binary,
        spell_id,
    );
    statement
}

pub(super) fn delete_by_spell(guid: u64, spell_id: u32) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(CharStatements::DEL_CHAR_AURA_BY_SPELL);
    bind_spell_key(&mut statement, guid, spell_id);
    statement
}

pub(super) fn delete_effects_by_spell(guid: u64, spell_id: u32) -> PreparedStatement {
    let mut statement =
        PreparedStatement::for_statement(CharStatements::DEL_CHAR_AURA_EFFECT_BY_SPELL);
    bind_spell_key(&mut statement, guid, spell_id);
    statement
}

pub(super) fn insert_aura(
    guid: u64,
    caster_guid_binary: [u8; 16],
    item_guid_binary: [u8; 16],
    spell_id: u32,
    effect_mask: u32,
    row: &PlayerLifecycleAuraSave,
) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(CharStatements::INS_AURA);
    statement.set_u64(0, guid);
    statement.set_bytes(1, caster_guid_binary.to_vec());
    statement.set_bytes(2, item_guid_binary.to_vec());
    statement.set_u32(3, spell_id);
    statement.set_u32(4, effect_mask);
    statement.set_u32(5, row.recalculate_mask);
    statement.set_u8(6, row.difficulty);
    statement.set_u8(7, row.stack_count);
    statement.set_i32(8, row.max_duration);
    statement.set_i32(9, row.remaining_duration);
    statement.set_u8(10, row.charges);
    statement.set_u32(11, row.cast_item_id);
    statement.set_i32(12, row.cast_item_level);
    statement
}

pub(super) fn insert_effect(
    guid: u64,
    caster_guid_binary: [u8; 16],
    item_guid_binary: [u8; 16],
    spell_id: u32,
    effect_mask: u32,
    row: PlayerLifecycleAuraEffectSave,
) -> PreparedStatement {
    let mut statement = PreparedStatement::for_statement(CharStatements::INS_AURA_EFFECT);
    statement.set_u64(0, guid);
    statement.set_bytes(1, caster_guid_binary.to_vec());
    statement.set_bytes(2, item_guid_binary.to_vec());
    statement.set_u32(3, spell_id);
    statement.set_u32(4, effect_mask);
    statement.set_u8(5, row.index);
    statement.set_i32(6, row.amount);
    statement.set_i32(7, row.base_amount);
    statement
}

fn bind_key(
    statement: &mut PreparedStatement,
    guid: u64,
    caster_guid_binary: [u8; 16],
    item_guid_binary: [u8; 16],
    spell_id: u32,
) {
    statement.set_u64(0, guid);
    statement.set_bytes(1, caster_guid_binary.to_vec());
    statement.set_bytes(2, item_guid_binary.to_vec());
    statement.set_u32(3, spell_id);
}

fn bind_spell_key(statement: &mut PreparedStatement, guid: u64, spell_id: u32) {
    statement.set_u64(0, guid);
    statement.set_u32(1, spell_id);
}
