use super::*;
use wow_packet::packets::loot::LootRoll;

pub async fn handle_loot_unit_for_test(session: &mut WorldSession, pkt: WorldPacket) {
    let item_valuation = session.item_valuation_catalogs_for_test_like_cpp();
    session.handle_loot_unit_with_catalogs_like_cpp(&item_valuation, pkt)
        .await;
}

pub async fn handle_loot_item_for_test(session: &mut WorldSession, pkt: WorldPacket) {
    let generators = session.id_generators_for_test_like_cpp();
    session.handle_loot_item_with_generator_like_cpp(generators.item.as_ref(), pkt)
        .await;
}

pub async fn handle_loot_money_for_test(session: &mut WorldSession, pkt: WorldPacket) {
    let generators = session.id_generators_for_test_like_cpp();
    session.handle_loot_money_with_generator_like_cpp(generators.item.as_ref(), pkt)
        .await;
}

pub async fn handle_loot_roll_for_test(session: &mut WorldSession, roll: LootRoll) {
    let generators = session.id_generators_for_test_like_cpp();
    let item_valuation = session.item_valuation_catalogs_for_test_like_cpp();
    session.handle_loot_roll_with_generator_like_cpp(
        generators.item.as_ref(),
        &item_valuation,
        roll,
    )
    .await;
}

/// Drive the original master-loot admission cases through the production handler.
pub async fn handle_master_loot_item_for_test(
    session: &mut WorldSession,
    master_loot_item: wow_packet::packets::loot::MasterLootItem,
) {
    let generators = session.id_generators_for_test_like_cpp();
    session.handle_master_loot_item_with_generator_like_cpp(
        generators.item.as_ref(),
        master_loot_item,
    )
    .await;
}

pub fn set_loot_for_test(
    session: &mut WorldSession,
    owner_guid: ObjectGuid,
    loot: CreatureLoot,
) {
    session.loot_table.insert(owner_guid, loot);
}

pub fn loot_for_test(session: &WorldSession, owner_guid: ObjectGuid) -> Option<CreatureLoot> {
    session.loot_table.get(&owner_guid).cloned()
}

pub fn has_loot_for_test(session: &WorldSession, owner_guid: ObjectGuid) -> bool {
    session.loot_table.contains_key(&owner_guid)
}

pub fn active_loot_guid_for_test(session: &WorldSession) -> ObjectGuid {
    session.loot_views.primary_guid()
}

pub fn active_loot_view_owners_for_test(
    session: &WorldSession,
) -> std::collections::HashSet<ObjectGuid> {
    session.loot_views.owners_snapshot()
}

pub fn set_active_loot_guid_for_test(session: &mut WorldSession, guid: ObjectGuid) {
    session.set_active_loot_guid(guid);
}

pub fn add_active_loot_view_owner_for_test(session: &mut WorldSession, guid: ObjectGuid) {
    session.add_active_loot_view_owner_like_cpp(guid);
}

pub fn is_active_loot_guid_for_test(session: &WorldSession, guid: ObjectGuid) -> bool {
    session.is_active_loot_guid(guid)
}

pub fn insert_allowed_coin_loot_for_test(
    session: &mut WorldSession,
    owner_guid: ObjectGuid,
    player_guid: ObjectGuid,
    coins: u32,
) {
    set_loot_for_test(
        session,
        owner_guid,
        CreatureLoot {
            loot_guid: represented_loot_object_guid_for_test(owner_guid),
            coins,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: Vec::new(),
            looted_by_player: false,
        },
    );
    if session
        .represented_owned_loot_authority_like_cpp(owner_guid)
        .is_some()
    {
        install_cached_test_creature_loot_authority_for_test(session, owner_guid, player_guid);
    }
}

pub async fn open_test_ae_pair_for_loot(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    primary_guid: ObjectGuid,
    secondary_guid: ObjectGuid,
) -> OwnedLootAuthority {
    session.set_player_guid(Some(player_guid));
    session.set_enable_ae_loot_like_cpp(true);
    session.set_player_position_like_cpp(Position::ZERO);
    register_test_creature_for_loot(session, test_creature_for_loot(primary_guid, false));
    register_test_creature_for_loot(session, test_creature_for_loot(secondary_guid, false));
    insert_allowed_coin_loot_for_test(session, primary_guid, player_guid, 7);
    insert_allowed_coin_loot_for_test(session, secondary_guid, player_guid, 7);
    handle_loot_unit_for_test(session, loot_unit_packet(primary_guid)).await;

    assert!(is_active_loot_guid_for_test(session, primary_guid));
    assert!(active_loot_view_owners_for_test(session).contains(&secondary_guid));
    let authority = session
        .represented_owned_loot_authority_like_cpp(secondary_guid)
        .expect("the secondary AE owner must expose its object-owned authority");
    assert!(
        authority
            .snapshot_for_player_like_cpp(player_guid)
            .unwrap()
            .loot
            .players_looting
            .contains(&player_guid)
    );
    authority
}

pub fn loot_specialization_for_test(session: &WorldSession) -> Option<u32> {
    session.loot_specialization_id_like_cpp()
}

pub fn set_player_position_for_loot_test(session: &mut WorldSession, position: Position) {
    session.set_player_position_like_cpp(position);
}

pub fn set_player_map_position_for_loot_test(
    session: &mut WorldSession,
    map_id: u16,
    position: Position,
) {
    session.set_player_map_position_like_cpp(map_id, position);
}

pub fn reconcile_loot_cache_for_test(
    session: &mut WorldSession,
    owner_guid: ObjectGuid,
    player_guid: ObjectGuid,
) -> bool {
    reconcile_loot_recovery_cache_for_test(session, owner_guid, player_guid)
}

pub async fn process_pending_for_loot_test(session: &mut WorldSession) {
    let catalogs = session.session_handler_catalogs_for_test_like_cpp();
    session
        .process_pending_with_catalogs_like_cpp(&catalogs)
        .await;
}

pub fn set_personal_loot_for_loot_test(
    session: &mut WorldSession,
    owner_guid: ObjectGuid,
    player_guid: ObjectGuid,
    money: u32,
) {
    session.represented_personal_loot_owners.insert(owner_guid);
    session
        .represented_personal_loot_money
        .insert((owner_guid, player_guid), money);
}

pub fn personal_loot_marker_for_test(
    session: &WorldSession,
    owner_guid: ObjectGuid,
    player_guid: ObjectGuid,
) -> (bool, Option<u32>) {
    (
        session.represented_personal_loot_owners.contains(&owner_guid),
        session
            .represented_personal_loot_money
            .get(&(owner_guid, player_guid))
            .copied(),
    )
}

pub fn set_session_logged_in_for_loot_test(session: &mut WorldSession) {
    session.set_state(SessionState::LoggedIn);
}
