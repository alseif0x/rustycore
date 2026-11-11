//! Item.cpp::SaveToDB base row, auxiliary groups, and Azerite subtype saves,
//! plus Player::_SaveInventory link at 02245dcd. Other template-dependent
//! subtype state remains outside this snapshot.
use super::{CreateError, PreparedStatement, SqlTransaction, decimal_words};
use std::{collections::BTreeSet, fmt::Write};
use wow_persistence::forever::creation::save::{NewItem, NewItemCreator};

mod auxiliary;
mod subtype;

#[cfg(test)]
#[path = "../../../unit_tests/forever/creation_items.rs"]
pub(super) mod tests;

const ITEM_INSERT: &str = "REPLACE INTO item_instance (itemEntry, owner_guid, creatorGuid, giftCreatorGuid, count, duration, charges, flags, enchantments, randomBonusListId, durability, playedTime, createTime, text, battlePetSpeciesId, battlePetBreedData, battlePetLevel, battlePetDisplayId, context, bonusListIDs, guid) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)";
const INVENTORY_INSERT: &str =
    "REPLACE INTO character_inventory (guid, bag, slot, item) VALUES (?,?,?,?)";

/// Player::_SaveMail calls all three static item deletion routines for each
/// deleted attachment. These global SQL keys do not establish authorization.
pub(super) fn append_mail_attachment_delete(transaction: &mut SqlTransaction, item_guid: u64) {
    for sql in [
        "DELETE FROM item_instance WHERE guid = ?",
        auxiliary::DELETE_GEMS,
        auxiliary::DELETE_TRANSMOG,
        auxiliary::DELETE_ARTIFACT,
        auxiliary::DELETE_ARTIFACT_POWERS,
        auxiliary::DELETE_MODIFIERS,
        "DELETE FROM character_gifts WHERE item_guid = ?",
        subtype::DELETE_AZERITE,
        subtype::DELETE_AZERITE_MILESTONES,
        subtype::DELETE_AZERITE_ESSENCES,
        subtype::DELETE_AZERITE_EMPOWERED,
    ] {
        let mut row = PreparedStatement::new(sql);
        row.set_u64(0, item_guid);
        transaction.append(row);
    }
}

pub(super) fn validate(items: &[NewItem]) -> Result<(), CreateError> {
    let mut positions = BTreeSet::new();
    let mut identities = BTreeSet::new();
    for (index, item) in items.iter().enumerate() {
        if item.entry == 0
            || item.count == 0
            || item.guid == 0
            || !identities.insert(item.guid)
            || !positions.insert((item.bag, item.slot))
        {
            return Err(CreateError::InvalidRow);
        }
        auxiliary::validate(item)?;
        subtype::validate(item)?;
        // References identify members of the snapshot and resolve to their
        // already reserved identities, without allocating another GUID.
        // Neither containment nor child-creator chains may dangle or cycle.
        validate_chain(items, index, |item| item.bag)?;
        validate_chain(items, index, |item| match item.creator {
            NewItemCreator::Item(parent) => Some(parent),
            _ => None,
        })?;
    }
    Ok(())
}

fn validate_chain(
    items: &[NewItem],
    mut index: usize,
    parent: impl Fn(&NewItem) -> Option<usize>,
) -> Result<(), CreateError> {
    for _ in 0..items.len() {
        let item = items.get(index).ok_or(CreateError::InvalidRow)?;
        match parent(item) {
            None => return Ok(()),
            Some(next) => index = next,
        }
    }
    Err(CreateError::InvalidRow)
}

pub(super) fn append(transaction: &mut SqlTransaction, character: u64, items: &[NewItem]) {
    // These are identities already used by construction, never late allocations.
    let guids: Vec<_> = items.iter().map(|item| item.guid).collect();
    for item in items {
        // Player::_SaveInventory appends the position before Item::SaveToDB.
        transaction.append(inventory_statement(character, item, &guids));
        subtype::append(transaction, item);
        transaction.append(item_statement(character, item, &guids));
        auxiliary::append(transaction, item);
    }
}

fn item_statement(character: u64, item: &NewItem, guids: &[u64]) -> PreparedStatement {
    let creator = match item.creator {
        NewItemCreator::None => 0,
        NewItemCreator::Character => character,
        NewItemCreator::Item(index) => guids[index],
    };
    let mut enchantments = String::new();
    for enchantment in item.enchantments {
        let words = enchantment.map_or([0; 3], |value| [value.id, value.duration, value.charges]);
        enchantments.push_str(&decimal_words(words));
    }
    let mut statement = PreparedStatement::new(ITEM_INSERT);
    statement.set_u32(0, item.entry);
    statement.set_u64(1, character);
    statement.set_u64(2, creator);
    statement.set_u64(3, item.gift_creator_guid);
    statement.set_u32(4, item.count);
    statement.set_u32(5, item.expiration);
    statement.set_string(6, signed_words(&item.charges));
    statement.set_u32(7, item.dynamic_flags);
    statement.set_string(8, enchantments);
    statement.set_u32(9, item.random_bonus_list);
    statement.set_u16(10, item.durability);
    statement.set_u32(11, item.played_time);
    statement.set_i64(12, item.create_time);
    statement.set_string(13, item.text.clone());
    for (index, value) in item.battle_pet_modifiers.into_iter().enumerate() {
        statement.set_u32(14 + index, value);
    }
    statement.set_u8(18, item.context);
    statement.set_string(19, signed_words(&item.bonus_lists));
    statement.set_u64(20, item.guid);
    statement
}

fn inventory_statement(character: u64, item: &NewItem, guids: &[u64]) -> PreparedStatement {
    let mut statement = PreparedStatement::new(INVENTORY_INSERT);
    statement.set_u64(0, character);
    statement.set_u64(1, item.bag.map_or(0, |index| guids[index]));
    statement.set_u8(2, item.slot);
    statement.set_u64(3, item.guid);
    statement
}

fn signed_words(values: &[i32]) -> String {
    let mut output = String::new();
    for value in values {
        write!(output, "{value} ").expect("String formatting");
    }
    output
}
