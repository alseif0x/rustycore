//! Session construction and canonical-player test fixtures.
//!
//! These builders retain the original session-test behavior and are
//! visible only within the parent `session::tests` subtree.

use super::*;

pub(in crate::session::tests) const BATTLEGROUND_AB_LIKE_CPP: u32 = 3;
pub(in crate::session::tests) const XP_HOOK_DOUBLE_PLAYER_COUNTER: i64 = 0xE1D0;
pub(in crate::session::tests) const XP_HOOK_ZERO_PLAYER_COUNTER: i64 = 0xE1D1;
pub(in crate::session::tests) const XP_HOOK_MAX_PLAYER_COUNTER: i64 = 0xE1D2;
pub(in crate::session::tests) const XP_HOOK_GUARD_PLAYER_COUNTER: i64 = 0xE1D3;

pub(in crate::session::tests) static XP_HOOK_DOUBLE_CALLS: AtomicUsize = AtomicUsize::new(0);
pub(in crate::session::tests) static XP_HOOK_ZERO_CALLS: AtomicUsize = AtomicUsize::new(0);
pub(in crate::session::tests) static XP_HOOK_MAX_CALLS: AtomicUsize = AtomicUsize::new(0);
pub(in crate::session::tests) static XP_HOOK_GUARD_CALLS: AtomicUsize = AtomicUsize::new(0);

pub(in crate::session::tests) fn represented_test_give_player_xp_hook_like_cpp(
    context: wow_script::player::GivePlayerXpContextLikeCpp,
    amount: &mut u32,
) {
    match context.player_guid.counter() {
        XP_HOOK_DOUBLE_PLAYER_COUNTER => {
            assert_eq!(context.victim_guid.counter(), 0xE1D0);
            XP_HOOK_DOUBLE_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
            *amount = amount.saturating_mul(2);
        }
        XP_HOOK_ZERO_PLAYER_COUNTER => {
            assert_eq!(context.victim_guid.counter(), 0xE1D1);
            XP_HOOK_ZERO_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
            *amount = 0;
        }
        XP_HOOK_MAX_PLAYER_COUNTER => {
            assert_eq!(context.victim_guid.counter(), 0xE1D2);
            XP_HOOK_MAX_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
        }
        XP_HOOK_GUARD_PLAYER_COUNTER => {
            XP_HOOK_GUARD_CALLS.fetch_add(1, AtomicOrdering::SeqCst);
        }
        _ => {}
    }
}

/// Exact propagated Map-tick time needed for a scheduled assistance call.
/// There is no scheduler margin after #371 because wall time cannot advance
/// the creature between scheduling and the explicit logical-clock step.
pub(in crate::session::tests) const ASSISTANCE_DELAY_ELAPSED_LIKE_CPP: Duration =
    Duration::from_millis(wow_movement::CREATURE_FAMILY_ASSISTANCE_DELAY_MS_LIKE_CPP as u64);

pub(in crate::session::tests) fn make_session() -> (
    WorldSession,
    flume::Sender<WorldPacket>,
    flume::Receiver<Vec<u8>>,
) {
    let (pkt_tx, pkt_rx) = flume::bounded(100);
    let (send_tx, send_rx) = flume::unbounded();

    let mut session = WorldSession::new(
        1,
        "TestAccount".into(),
        0,
        2,
        9, // account_expansion (raw from DB)
        54261,
        vec![0u8; 40],
        "esES".into(),
        pkt_rx,
        send_tx,
    );
    session.set_active_player_local_flags_like_cpp(
        PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP,
    );

    (session, pkt_tx, send_rx)
}

pub(in crate::session::tests) fn make_session_with_give_player_xp_hook() -> (
    WorldSession,
    flume::Sender<WorldPacket>,
    flume::Receiver<Vec<u8>>,
) {
    let (mut session, pkt_tx, send_rx) = make_session();
    session.set_give_player_xp_script_dispatcher_like_cpp(Arc::new(
        represented_test_give_player_xp_hook_like_cpp,
    ));
    (session, pkt_tx, send_rx)
}

pub(in crate::session::tests) fn run_canonical_player_owner_test(
    test: impl FnOnce() + Send + 'static,
) {
    std::thread::Builder::new()
        .name("canonical-player-owner".into())
        .stack_size(8 * 1024 * 1024)
        .spawn(test)
        .unwrap()
        .join()
        .unwrap();
}

pub(in crate::session::tests) fn insert_session_player_into_canonical_map_like_cpp(
    session: &WorldSession,
    canonical: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
) {
    if let Some(handle) = session.player_handle_like_cpp {
        let position = session
            .player_position_like_cpp()
            .expect("canonical Player fixture position");
        let key = wow_map::MapKey::new(map_id, instance_id);
        let mut manager = canonical.lock().unwrap();
        manager.create_world_map(map_id, instance_id);
        match manager.player_residence_like_cpp(handle) {
            Some(wow_map::PlayerResidenceLikeCpp::Detached) => manager
                .attach_player_like_cpp(handle, key, position)
                .expect("attach detached canonical Player fixture"),
            Some(wow_map::PlayerResidenceLikeCpp::Active(current)) if current == key => {}
            Some(wow_map::PlayerResidenceLikeCpp::Active(_)) => {
                manager
                    .detach_player_like_cpp(handle)
                    .expect("detach canonical Player fixture");
                manager
                    .attach_player_like_cpp(handle, key, position)
                    .expect("reattach canonical Player fixture");
            }
            None => panic!("stale canonical Player fixture handle"),
        }
        return;
    }

    let player = session
        .build_initial_player_for_owner_like_cpp(wow_map::MapKey::new(map_id, instance_id), None)
        .expect("complete canonical player fixture");
    let record =
        wow_entities::MapObjectRecord::new_player(player).expect("canonical Player fixture record");
    let mut manager = canonical.lock().unwrap();
    manager
        .create_world_map(map_id, instance_id)
        .map_mut()
        .insert_map_object_record(record)
        .expect("insert canonical Player fixture");
}

pub(in crate::session::tests) fn session_with_canonical_player_for_away_like_cpp()
-> (WorldSession, SharedCanonicalMapManager, ObjectGuid) {
    let (session, _, canonical, player_guid) =
        session_with_canonical_player_for_away_like_cpp_with_packet_tx();
    (session, canonical, player_guid)
}

pub(in crate::session::tests) fn session_with_canonical_player_for_away_like_cpp_with_packet_tx()
-> (
    WorldSession,
    flume::Sender<WorldPacket>,
    SharedCanonicalMapManager,
    ObjectGuid,
) {
    let (mut session, pkt_tx, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 0xAFD0);
    session.ensure_login_player_controller_like_cpp(
        player_guid,
        "AwayTester".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    );
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 571, 0);
    (session, pkt_tx, canonical, player_guid)
}

pub(in crate::session::tests) fn set_canonical_player_farsight_object_like_cpp(
    canonical: &SharedCanonicalMapManager,
    player_guid: ObjectGuid,
    farsight_object: ObjectGuid,
) {
    set_canonical_player_farsight_object_on_map_like_cpp(
        canonical,
        player_guid,
        farsight_object,
        571,
        0,
    );
}

pub(in crate::session::tests) fn set_canonical_player_farsight_object_on_map_like_cpp(
    canonical: &SharedCanonicalMapManager,
    player_guid: ObjectGuid,
    farsight_object: ObjectGuid,
    map_id: u32,
    instance_id: u32,
) {
    canonical
        .lock()
        .unwrap()
        .find_map_mut(map_id, instance_id)
        .unwrap()
        .map_mut()
        .get_typed_player_mut(player_guid)
        .unwrap()
        .set_farsight_object_like_cpp(farsight_object);
}

pub(in crate::session::tests) fn faction_template_entry(
    id: u32,
    faction: u16,
    faction_group: u8,
    friend_group: u8,
    enemy: u16,
) -> wow_data::progression_rewards::FactionTemplateEntry {
    let mut enemies = [0; 8];
    enemies[0] = enemy;
    wow_data::progression_rewards::FactionTemplateEntry {
        id,
        faction,
        flags: 0,
        faction_group,
        friend_group,
        enemy_group: 0,
        enemies,
        friend: [0; 8],
    }
}

pub(in crate::session::tests) fn represented_get_reaction_input_like_cpp()
-> RepresentedGetReactionInputLikeCpp {
    RepresentedGetReactionInputLikeCpp {
        self_faction_template_id: 1,
        target_faction_template_id: 2,
        same_object: false,
        attackable_by_summoner: false,
        same_charmer_or_owner_or_self: false,
        self_has_player_owner: true,
        target_has_player_owner: true,
        target_player_owner_is_current_session: true,
        target_owner_forced_rank_for_self: None,
        same_player_owner: false,
        duel_in_progress: false,
        same_raid: false,
        self_unit_player_controlled: true,
        target_unit_player_controlled: true,
        self_ffa_pvp: false,
        target_ffa_pvp: false,
        self_ignores_reputation: false,
        target_ignores_reputation: false,
        target_is_unit: true,
        target_player_contested_pvp: false,
    }
}
