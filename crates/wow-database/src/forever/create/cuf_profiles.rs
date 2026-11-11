//! Player::_SaveCUFProfiles five-slot projection at 02245dcd.
use super::{PreparedStatement, SqlTransaction};
use wow_persistence::forever::creation::save::NewCufProfile;

#[cfg(test)]
#[path = "../../../unit_tests/forever/creation_cuf_profiles.rs"]
mod tests;

const DELETE: &str = "DELETE FROM character_cuf_profiles WHERE guid = ? AND id = ?";
const REPLACE: &str = "REPLACE INTO character_cuf_profiles (guid, id, name, frameHeight, frameWidth, sortBy, healthText, boolOptions, topPoint, bottomPoint, leftPoint, topOffset, bottomOffset, leftOffset) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";

pub(super) fn append(
    transaction: &mut SqlTransaction,
    guid: u64,
    profiles: &[Option<NewCufProfile>; 5],
) {
    for row in statements(guid, profiles) {
        transaction.append(row);
    }
}

fn statements<'a>(
    guid: u64,
    profiles: &'a [Option<NewCufProfile>; 5],
) -> impl Iterator<Item = PreparedStatement> + 'a {
    profiles
        .iter()
        .enumerate()
        .map(move |(index, profile)| profile_statement(guid, index as u8, profile.as_ref()))
}

fn profile_statement(guid: u64, index: u8, profile: Option<&NewCufProfile>) -> PreparedStatement {
    let mut row = PreparedStatement::new(if profile.is_some() { REPLACE } else { DELETE });
    row.set_u64(0, guid);
    row.set_u8(1, index);
    if let Some(profile) = profile {
        row.set_string(2, profile.name.clone());
        row.set_u16(3, profile.frame_height);
        row.set_u16(4, profile.frame_width);
        row.set_u8(5, profile.sort_by);
        row.set_u8(6, profile.health_text);
        let mask = profile
            .bool_options
            .iter()
            .enumerate()
            .fold(0u32, |mask, (bit, enabled)| {
                mask | (u32::from(*enabled) << bit)
            });
        row.set_u32(7, mask);
        row.set_u8(8, profile.top_point);
        row.set_u8(9, profile.bottom_point);
        row.set_u8(10, profile.left_point);
        row.set_u16(11, profile.top_offset);
        row.set_u16(12, profile.bottom_offset);
        row.set_u16(13, profile.left_offset);
    }
    row
}
