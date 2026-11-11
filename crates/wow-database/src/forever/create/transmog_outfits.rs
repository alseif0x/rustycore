//! Player::_SaveTransmogOutfits projection at 02245dcd, keyed by dirty IDs.
use super::{PreparedStatement, SqlTransaction};
use std::collections::BTreeMap;
use wow_persistence::forever::creation::save::{
    NewTransmogOutfit, NewTransmogOutfitSituation, NewTransmogOutfitSlot,
};

#[cfg(test)]
#[path = "../../../unit_tests/forever/creation_transmog_outfits.rs"]
mod tests;

const SLOT_DELETE: &str =
    "DELETE FROM character_transmog_outfit_slot WHERE guid = ? AND transmogOutfitId = ?";
const SITUATION_DELETE: &str =
    "DELETE FROM character_transmog_outfit_situation WHERE guid = ? AND transmogOutfitId = ?";
const OUTFIT_DELETE: &str =
    "DELETE FROM character_transmog_outfit WHERE guid = ? AND transmogOutfitId = ?";
const OUTFIT_INSERT: &str = "INSERT INTO character_transmog_outfit (guid, transmogOutfitId, name, icon, situationsEnabled) VALUES (?, ?, ?, ?, ?)";
const SITUATION_INSERT: &str = "INSERT INTO character_transmog_outfit_situation (guid, transmogOutfitId, situationID, specID, loadoutID, equipmentSetID) VALUES (?, ?, ?, ?, ?, ?)";
const SLOT_INSERT: &str = "INSERT INTO character_transmog_outfit_slot (guid, transmogOutfitId, slot, slotOption, sheatheCategory, itemModifiedAppearanceID, appearanceDisplayType, spellItemEnchantmentID, illusionDisplayType, flags) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";

pub(super) fn append(
    transaction: &mut SqlTransaction,
    guid: u64,
    outfits: &BTreeMap<u32, Option<NewTransmogOutfit>>,
) {
    for row in statements(guid, outfits) {
        transaction.append(row);
    }
}

fn statements(
    guid: u64,
    outfits: &BTreeMap<u32, Option<NewTransmogOutfit>>,
) -> impl Iterator<Item = PreparedStatement> + '_ {
    outfits
        .iter()
        .flat_map(move |(id, outfit)| outfit_statements(guid, *id, outfit.as_ref()))
}

fn outfit_statements(
    guid: u64,
    id: u32,
    outfit: Option<&NewTransmogOutfit>,
) -> Vec<PreparedStatement> {
    let mut rows = vec![
        scoped_statement(SLOT_DELETE, guid, id),
        scoped_statement(SITUATION_DELETE, guid, id),
        scoped_statement(OUTFIT_DELETE, guid, id),
    ];
    if let Some(outfit) = outfit {
        rows.push(outfit_statement(guid, id, outfit));
        rows.extend(
            outfit
                .situations
                .iter()
                .map(|situation| situation_statement(guid, id, situation)),
        );
        rows.extend(
            outfit
                .slots
                .iter()
                .map(|slot| slot_statement(guid, id, slot)),
        );
    }
    rows
}

fn scoped_statement(sql: &'static str, guid: u64, id: u32) -> PreparedStatement {
    let mut row = PreparedStatement::new(sql);
    row.set_u64(0, guid);
    row.set_u32(1, id);
    row
}

fn outfit_statement(guid: u64, id: u32, outfit: &NewTransmogOutfit) -> PreparedStatement {
    let mut row = scoped_statement(OUTFIT_INSERT, guid, id);
    row.set_string(2, outfit.name.clone());
    row.set_u32(3, outfit.icon);
    row.set_bool(4, outfit.situations_enabled);
    row
}

fn situation_statement(
    guid: u64,
    id: u32,
    situation: &NewTransmogOutfitSituation,
) -> PreparedStatement {
    let mut row = scoped_statement(SITUATION_INSERT, guid, id);
    row.set_u32(2, situation.situation_id);
    row.set_u32(3, situation.spec_id);
    row.set_u32(4, situation.loadout_id);
    row.set_u32(5, situation.equipment_set_id);
    row
}

fn slot_statement(guid: u64, id: u32, slot: &NewTransmogOutfitSlot) -> PreparedStatement {
    let mut row = scoped_statement(SLOT_INSERT, guid, id);
    row.set_i8(2, slot.slot);
    row.set_u8(3, slot.slot_option);
    row.set_u8(4, slot.sheathe_category);
    row.set_u32(5, slot.item_modified_appearance_id);
    row.set_u8(6, slot.appearance_display_type);
    row.set_u32(7, slot.spell_item_enchantment_id);
    row.set_u8(8, slot.illusion_display_type);
    row.set_u32(9, slot.flags);
    row
}
