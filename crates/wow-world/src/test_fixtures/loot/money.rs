//! Complete money setup over one shared canonical manager and the existing loot authority.

use super::*;

pub use crate::handlers::loot::{
    consume_stored_money_with_port_for_test,
    allow_money_looter_for_test,
    consume_personal_money_loot_for_test,
    open_personal_money_loot_for_test,
    open_money_loot_normally_for_test,
    LootMoneyApplication,
    LootMoneyDelivery,
    LootMoneyFanout,
    LootMoneyTracker,
    LootMoneyWorker,
    LootMoneyWorkerError,
    accepted_money_delta_for_test,
    active_money_generation_for_test,
    apply_money_command_for_test,
    attach_money_player_controller_for_test,
    clear_money_persistence_outcome_for_test,
    ensure_money_player_map_for_test,
    generate_chest_money_loot_for_test,
    generate_creature_money_loot_for_test,
    money_instance_for_test,
    money_map_for_test,
    money_recipients_for_test,
    money_tracker_for_test,
    notify_money_removed_for_test,
    set_group_money_port_for_test,
    source_money_delivery_for_test,
    spawn_group_money_worker_for_test,
    sync_creature_loot_fixture_for_test,
};

fn prepare_money_player_in_manager(
    session: &mut WorldSession,
    manager: Arc<std::sync::Mutex<wow_map::MapManager>>,
) {
    let guid = session.player_guid().expect("money fixture Player GUID");
    let position = session.player_position_like_cpp().unwrap_or(Position::ZERO);
    let map_id = u32::from(session.player_map_id_like_cpp());
    let money = session.player_gold_like_cpp();
    let name = session.player_name_like_cpp().unwrap_or_default();
    let race = session.player_race_like_cpp();
    let class = session.player_class_like_cpp();
    let level = session.player_level_like_cpp();
    let gender = session.player_gender_like_cpp();
    session.set_canonical_map_manager(Arc::clone(&manager));
    {
        let mut manager = manager.lock().expect("money fixture canonical map lock");
        let map = manager.create_world_map(map_id, 0).map_mut();
        if map.get_typed_player(guid).is_none() {
            let mut player = wow_entities::Player::new(Some(u64::from(session.account_id)), false);
            player.unit_mut().world_mut().object_mut().create(guid);
            player.unit_mut().world_mut().set_name(name);
            player.unit_mut().world_mut().set_map(map_id, 0).unwrap();
            player.unit_mut().world_mut().relocate(position);
            player.unit_mut().world_mut().object_mut().add_to_world();
            player.set_race_class_gender(race, class, match gender {
                1 => wow_constants::Gender::Female,
                2 => wow_constants::Gender::None,
                _ => wow_constants::Gender::Male,
            });
            player.unit_mut().set_level(level);
            player.unit_mut().set_max_health(100);
            player.unit_mut().set_death_state(wow_constants::DeathState::Alive);
            player.unit_mut().set_health(100);
            player.set_money(money);
            map.insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
                .unwrap();
        }
    }
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
}

pub fn prepare_money_player_residence_for_test(session: &mut WorldSession) {
    let manager = session.canonical_map_manager.as_ref().map(Arc::clone)
        .unwrap_or_else(|| Arc::new(std::sync::Mutex::new(wow_map::MapManager::default())));
    prepare_money_player_in_manager(session, manager);
}

pub fn two_sessions_with_money_loot_for_test(
    loot: CreatureLoot,
) -> (
    WorldSession, flume::Receiver<Vec<u8>>, WorldSession, flume::Receiver<Vec<u8>>,
    ObjectGuid, ObjectGuid, ObjectGuid,
) {
    let (mut first, first_rx, mut second, second_rx, owner, first_guid, second_guid) =
        two_sessions_with_recovery_loot_for_test(loot);
    let manager = first.canonical_map_manager.as_ref().map(Arc::clone)
        .unwrap_or_else(|| Arc::new(std::sync::Mutex::new(wow_map::MapManager::default())));
    prepare_money_player_in_manager(&mut first, Arc::clone(&manager));
    prepare_money_player_in_manager(&mut second, manager);
    (first, first_rx, second, second_rx, owner, first_guid, second_guid)
}
