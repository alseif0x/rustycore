//! Player::_SaveEquipmentSets equipment/legacy-transmog projection at 02245dcd.
use super::{PreparedStatement, SqlTransaction};
use std::collections::BTreeMap;
use wow_persistence::forever::creation::save::{
    NewSavedEquipmentSet, NewSavedSetKind, NewSavedSetState,
};

#[cfg(test)]
#[path = "../../../unit_tests/forever/creation_equipment_sets.rs"]
mod tests;

const EQUIPMENT_UPDATE: &str = "UPDATE character_equipmentsets SET name=?, iconname=?, ignore_mask=?, AssignedSpecIndex=?, item0=?, item1=?, item2=?, item3=?, item4=?, item5=?, item6=?, item7=?, item8=?, item9=?, item10=?, item11=?, item12=?, item13=?, item14=?, item15=?, item16=?, item17=?, item18=? WHERE guid=? AND setguid=? AND setindex=?";
const EQUIPMENT_INSERT: &str = "INSERT INTO character_equipmentsets (guid, setguid, setindex, name, iconname, ignore_mask, AssignedSpecIndex, item0, item1, item2, item3, item4, item5, item6, item7, item8, item9, item10, item11, item12, item13, item14, item15, item16, item17, item18) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";
const EQUIPMENT_DELETE: &str = "DELETE FROM character_equipmentsets WHERE setguid=?";
const TRANSMOG_UPDATE: &str = "UPDATE character_transmog_outfits SET name=?, iconname=?, ignore_mask=?, appearance0=?, appearance1=?, appearance2=?, appearance3=?, appearance4=?, appearance5=?, appearance6=?, appearance7=?, appearance8=?, appearance9=?, appearance10=?, appearance11=?, appearance12=?, appearance13=?, appearance14=?, appearance15=?, appearance16=?, appearance17=?, appearance18=?, mainHandEnchant=?, offHandEnchant=? WHERE guid=? AND setguid=? AND setindex=?";
const TRANSMOG_INSERT: &str = "INSERT INTO character_transmog_outfits (guid, setguid, setindex, name, iconname, ignore_mask, appearance0, appearance1, appearance2, appearance3, appearance4, appearance5, appearance6, appearance7, appearance8, appearance9, appearance10, appearance11, appearance12, appearance13, appearance14, appearance15, appearance16, appearance17, appearance18, mainHandEnchant, offHandEnchant) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";
const TRANSMOG_DELETE: &str = "DELETE FROM character_transmog_outfits WHERE setguid=?";

pub(super) fn append(
    transaction: &mut SqlTransaction,
    guid: u64,
    sets: &BTreeMap<u64, NewSavedEquipmentSet>,
) {
    for row in statements(guid, sets) {
        transaction.append(row);
    }
}

fn statements(
    guid: u64,
    sets: &BTreeMap<u64, NewSavedEquipmentSet>,
) -> impl Iterator<Item = PreparedStatement> + '_ {
    // Traverse source map keys, while all SQL identity comes from the values.
    sets.values()
        .filter_map(move |set| set_statement(guid, set))
}

fn set_statement(guid: u64, set: &NewSavedEquipmentSet) -> Option<PreparedStatement> {
    let is_new = match set.state {
        NewSavedSetState::Unchanged => return None,
        NewSavedSetState::Deleted => {
            let sql = match &set.kind {
                NewSavedSetKind::Equipment { .. } => EQUIPMENT_DELETE,
                NewSavedSetKind::Transmog { .. } => TRANSMOG_DELETE,
            };
            let mut row = PreparedStatement::new(sql);
            row.set_u64(0, set.guid);
            return Some(row);
        }
        NewSavedSetState::New => true,
        NewSavedSetState::Changed => false,
    };
    let (sql, owner_index) = match (&set.kind, is_new) {
        (NewSavedSetKind::Equipment { .. }, true) => (EQUIPMENT_INSERT, 0),
        (NewSavedSetKind::Equipment { .. }, false) => (EQUIPMENT_UPDATE, 23),
        (NewSavedSetKind::Transmog { .. }, true) => (TRANSMOG_INSERT, 0),
        (NewSavedSetKind::Transmog { .. }, false) => (TRANSMOG_UPDATE, 24),
    };
    let mut row = PreparedStatement::new(sql);
    row.set_u64(owner_index, guid);
    row.set_u64(owner_index + 1, set.guid);
    row.set_u32(owner_index + 2, set.set_index);
    let base = if is_new { 3 } else { 0 };
    row.set_string(base, set.name.clone());
    row.set_string(base + 1, set.icon.clone());
    row.set_u32(base + 2, set.ignore_mask);
    match &set.kind {
        NewSavedSetKind::Equipment {
            assigned_spec,
            items,
        } => {
            row.set_i32(base + 3, assigned_spec.unwrap_or(-1));
            for (index, item) in items.iter().enumerate() {
                row.set_u64(base + 4 + index, *item);
            }
        }
        NewSavedSetKind::Transmog {
            appearances,
            enchants,
        } => {
            for (index, appearance) in appearances.iter().enumerate() {
                row.set_i32(base + 3 + index, *appearance);
            }
            for (index, enchant) in enchants.iter().enumerate() {
                row.set_i32(base + 22 + index, *enchant);
            }
        }
    }
    Some(row)
}
