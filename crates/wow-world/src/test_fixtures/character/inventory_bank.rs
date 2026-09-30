//! Bank fixture composition uses the existing detached controller installer.
use crate::session::WorldSession;
use wow_constants::PowerType;
use wow_core::ObjectGuid;

pub fn make_inventory_bank_session_for_test(capacity: usize) -> (
    WorldSession, flume::Receiver<Vec<u8>>, std::sync::Arc<std::sync::Mutex<wow_map::MapManager>>,
) {
    crate::handlers::character::inventory_fixture_access::make_bank_fixture_for_test(capacity)
}

pub fn inventory_can_use_bank_for_test(session: &WorldSession) -> bool {
    session.represented_can_use_current_bank_like_cpp()
}

pub fn configure_inventory_player_vitals_for_test(
    session: &WorldSession, guid: ObjectGuid,
    vitals: (u32, u32, PowerType, i32, i32, i32),
) -> bool {
    if session.player_guid() != Some(guid) { return false; }
    let (health, max_health, power, current_power, max_power, base_mana) = vitals;
    session.mutate_canonical_player_like_cpp(|player| {
        player.unit_mut().set_max_health(u64::from(max_health));
        player.unit_mut().set_health(u64::from(health));
        player.unit_mut().set_create_mana_like_cpp(base_mana);
        player.unit_mut().set_display_power(power);
        player.set_power_index(power, Some(0));
        player.unit_mut().set_max_power(power, max_power);
        player.unit_mut().set_power(power, current_power);
    }).is_some()
}
