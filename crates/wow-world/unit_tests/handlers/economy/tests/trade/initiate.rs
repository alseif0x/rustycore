//! `CMSG_INITIATE_TRADE` (#38): C++ `HandleInitiateTradeOpcode`
//! (`TradeHandler.cpp:582-701`) against two sessions sharing one canonical
//! map manager.

use super::*;
use crate::session::WorldSession;
use wow_packet::packets::misc::{
    TRADE_STATUS_DEAD_LIKE_CPP, TRADE_STATUS_FAILED_LIKE_CPP, TRADE_STATUS_NO_TARGET_LIKE_CPP,
    TRADE_STATUS_PROPOSED_LIKE_CPP, TRADE_STATUS_TARGET_STUNNED_LIKE_CPP,
    TRADE_STATUS_TOO_FAR_AWAY_LIKE_CPP, TRADE_STATUS_WRONG_FACTION_LIKE_CPP,
};
use wow_world_social::TradePolicyLikeCpp;

pub(super) struct TradePairLikeCpp {
    pub(super) source: WorldSession,
    pub(super) source_rx: flume::Receiver<Vec<u8>>,
    pub(super) partner: WorldSession,
    pub(super) partner_rx: flume::Receiver<Vec<u8>>,
    pub(super) source_guid: ObjectGuid,
    pub(super) partner_guid: ObjectGuid,
    pub(super) canonical: crate::session::SharedCanonicalMapManager,
}

fn add_trade_player(
    canonical: &crate::session::SharedCanonicalMapManager,
    guid: ObjectGuid,
    position: Position,
    race: u8,
    level: u8,
) {
    let mut player = wow_entities::Player::new(Some(1), false);
    player.unit_mut().world_mut().object_mut().create(guid);
    player.unit_mut().world_mut().set_name("Trader");
    player.unit_mut().world_mut().set_map(571, 0).unwrap();
    player.unit_mut().world_mut().relocate(position);
    player.unit_mut().set_max_health(100);
    player.unit_mut().set_health(100);
    player.unit_mut().set_race(race);
    player.unit_mut().set_level(level);
    player.unit_mut().world_mut().object_mut().add_to_world();
    canonical
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
        .unwrap();
}

fn bind_session(
    session: &mut WorldSession,
    guid: ObjectGuid,
    canonical: &crate::session::SharedCanonicalMapManager,
) {
    session.set_player_guid(Some(guid));
    session.set_canonical_map_manager(Arc::clone(canonical));
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
}

pub(super) fn trade_pair_like_cpp(
    partner_position: Position,
    partner_race: u8,
    partner_level: u8,
    same_bnet: bool,
) -> TradePairLikeCpp {
    let (mut source, source_rx) = make_session();
    let (mut partner, partner_rx) = make_session();
    let source_guid = ObjectGuid::create_player(1, 77);
    let partner_guid = ObjectGuid::create_player(1, 88);
    let canonical = shared_canonical_map_manager_for_misc_test();
    add_trade_player(
        &canonical,
        source_guid,
        Position::new(1.0, 1.0, 0.0, 0.0),
        1,
        10,
    );
    add_trade_player(
        &canonical,
        partner_guid,
        partner_position,
        partner_race,
        partner_level,
    );
    bind_session(&mut source, source_guid, &canonical);
    bind_session(&mut partner, partner_guid, &canonical);

    let registry = Arc::new(PlayerRegistry::default());
    let mut source_info = broadcast_info_with_command_tx(source.session_command_tx());
    let mut partner_info = broadcast_info_with_command_tx(partner.session_command_tx());
    source_info.identity.battlenet_account_id = source.core.battlenet_account_id();
    partner_info.identity.battlenet_account_id = if same_bnet {
        source.core.battlenet_account_id()
    } else {
        source.core.battlenet_account_id().wrapping_add(1)
    };
    registry.register_or_replace(source_guid, source_info, Default::default());
    registry.register_or_replace(partner_guid, partner_info, Default::default());
    source.set_player_registry(Arc::clone(&registry));
    partner.set_player_registry(registry);
    TradePairLikeCpp {
        source,
        source_rx,
        partner,
        partner_rx,
        source_guid,
        partner_guid,
        canonical,
    }
}

pub(super) fn initiate_trade_packet(guid: ObjectGuid) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&guid);
    pkt.reset_read();
    pkt
}

fn trade_status_of(bytes: &[u8]) -> (bool, u8) {
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::TradeStatus as u16
    );
    (bytes[2] & 0x80 != 0, (bytes[2] >> 2) & 0x1F)
}

pub(super) fn with_canonical_player_mut<R>(
    canonical: &crate::session::SharedCanonicalMapManager,
    guid: ObjectGuid,
    f: impl FnOnce(&mut wow_entities::Player) -> R,
) -> R {
    let mut manager = canonical.lock().unwrap();
    let player = manager
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .get_typed_player_mut(guid)
        .unwrap();
    f(player)
}

fn trade_partner(session: &WorldSession) -> Option<ObjectGuid> {
    wow_world_social::player_trade_state_snapshot_like_cpp(&session.core, &session.social)
        .flatten()
        .map(|trade| trade.partner_guid)
}

#[tokio::test]
async fn initiate_trade_opens_both_trades_and_proposes_to_target_like_cpp() {
    let mut pair = trade_pair_like_cpp(Position::new(3.0, 1.0, 0.0, 0.0), 1, 10, false);

    pair.source
        .handle_initiate_trade_with_policy(
            initiate_trade_packet(pair.partner_guid),
            TradePolicyLikeCpp::default(),
        )
        .await;
    pair.partner
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(trade_partner(&pair.source), Some(pair.partner_guid));
    assert_eq!(trade_partner(&pair.partner), Some(pair.source_guid));
    assert!(
        pair.source_rx.try_recv().is_err(),
        "C++ answers only the target"
    );
    let bytes = pair.partner_rx.try_recv().expect("PROPOSED");
    assert_eq!(
        trade_status_of(&bytes),
        (false, TRADE_STATUS_PROPOSED_LIKE_CPP)
    );
    let mut payload = WorldPacket::from_bytes(&bytes[3..]);
    assert_eq!(payload.read_packed_guid().unwrap(), pair.source_guid);
    assert_eq!(payload.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
}

#[tokio::test]
async fn initiate_trade_marks_same_bnet_account_like_cpp() {
    let mut pair = trade_pair_like_cpp(Position::new(3.0, 1.0, 0.0, 0.0), 1, 10, true);
    pair.source
        .handle_initiate_trade_with_policy(
            initiate_trade_packet(pair.partner_guid),
            TradePolicyLikeCpp::default(),
        )
        .await;
    pair.partner
        .process_represented_session_commands_like_cpp()
        .await;
    let bytes = pair.partner_rx.try_recv().expect("PROPOSED");
    assert_eq!(
        trade_status_of(&bytes),
        (true, TRADE_STATUS_PROPOSED_LIKE_CPP)
    );
}

#[tokio::test]
async fn initiate_trade_refusals_follow_cpp_order_like_cpp() {
    async fn refused(
        partner_position: Position,
        partner_race: u8,
        partner_level: u8,
        policy: TradePolicyLikeCpp,
        prepare: impl FnOnce(&mut TradePairLikeCpp),
        target: Option<ObjectGuid>,
    ) -> (u8, Vec<Vec<u8>>, bool) {
        let mut pair = trade_pair_like_cpp(partner_position, partner_race, partner_level, false);
        prepare(&mut pair);
        let target = target.unwrap_or(pair.partner_guid);
        pair.source
            .handle_initiate_trade_with_policy(initiate_trade_packet(target), policy)
            .await;
        let bytes: Vec<Vec<u8>> = pair.source_rx.try_iter().collect();
        let status = trade_status_of(bytes.last().expect("refusal status")).1;
        (status, bytes, trade_partner(&pair.partner).is_some())
    }
    let near = Position::new(3.0, 1.0, 0.0, 0.0);

    // TRADE_DISTANCE 11.11 + both combat reaches; 40 yards is too far.
    let (status, _, opened) = refused(
        Position::new(40.0, 1.0, 0.0, 0.0),
        1,
        10,
        TradePolicyLikeCpp::default(),
        |_| {},
        None,
    )
    .await;
    assert_eq!(status, TRADE_STATUS_TOO_FAR_AWAY_LIKE_CPP);
    assert!(!opened);

    let (status, _, _) = refused(
        near,
        1,
        10,
        TradePolicyLikeCpp::default(),
        |_| {},
        Some(ObjectGuid::create_player(1, 999)),
    )
    .await;
    assert_eq!(status, TRADE_STATUS_NO_TARGET_LIKE_CPP);

    // Alliance human against a Horde orc, without AllowTwoSide.Trade.
    let (status, _, opened) =
        refused(near, 2, 10, TradePolicyLikeCpp::default(), |_| {}, None).await;
    assert_eq!(status, TRADE_STATUS_WRONG_FACTION_LIKE_CPP);
    assert!(!opened);

    let (status, bytes, _) = refused(
        near,
        1,
        10,
        TradePolicyLikeCpp {
            trade_level_req: 20,
            allow_two_side_trade: false,
        },
        |_| {},
        None,
    )
    .await;
    assert_eq!(status, TRADE_STATUS_FAILED_LIKE_CPP);
    assert_eq!(bytes.len(), 2, "notification then TRADE_STATUS_FAILED");

    let (status, _, _) = refused(
        near,
        1,
        10,
        TradePolicyLikeCpp::default(),
        |pair| {
            with_canonical_player_mut(&pair.canonical, pair.partner_guid, |player| {
                player
                    .unit_mut()
                    .add_unit_state(wow_constants::unit::UnitState::STUNNED.bits())
            });
        },
        None,
    )
    .await;
    assert_eq!(status, TRADE_STATUS_TARGET_STUNNED_LIKE_CPP);

    let (status, _, _) = refused(
        near,
        1,
        10,
        TradePolicyLikeCpp::default(),
        |pair| {
            pair.source
                .core
                .with_owned_player_mut_like_cpp(|player| {
                    player
                        .unit_mut()
                        .set_death_state(wow_constants::unit::DeathState::Dead)
                })
                .unwrap();
        },
        None,
    )
    .await;
    assert_eq!(status, TRADE_STATUS_DEAD_LIKE_CPP);

    // A target that already trades answers PLAYER_BUSY.
    let (status, _, _) = refused(
        near,
        1,
        10,
        TradePolicyLikeCpp::default(),
        |pair| {
            with_canonical_player_mut(&pair.canonical, pair.partner_guid, |player| {
                player.open_trade_like_cpp(ObjectGuid::create_player(1, 5))
            });
        },
        None,
    )
    .await;
    assert_eq!(status, TRADE_STATUS_PLAYER_BUSY_LIKE_CPP);
}

#[tokio::test]
async fn initiate_trade_allows_two_side_trade_by_config_like_cpp() {
    let mut pair = trade_pair_like_cpp(Position::new(3.0, 1.0, 0.0, 0.0), 2, 10, false);
    pair.source
        .handle_initiate_trade_with_policy(
            initiate_trade_packet(pair.partner_guid),
            TradePolicyLikeCpp {
                trade_level_req: 1,
                allow_two_side_trade: true,
            },
        )
        .await;
    assert_eq!(trade_partner(&pair.partner), Some(pair.source_guid));
}

#[tokio::test]
async fn initiate_trade_while_trading_is_silent_like_cpp() {
    let mut pair = trade_pair_like_cpp(Position::new(3.0, 1.0, 0.0, 0.0), 1, 10, false);
    let other = ObjectGuid::create_player(1, 5);
    pair.source
        .core
        .with_owned_player_mut_like_cpp(|player| player.open_trade_like_cpp(other))
        .unwrap();
    pair.source
        .handle_initiate_trade_with_policy(
            initiate_trade_packet(pair.partner_guid),
            TradePolicyLikeCpp::default(),
        )
        .await;
    assert!(pair.source_rx.try_recv().is_err());
    assert_eq!(trade_partner(&pair.source), Some(other));
    assert!(trade_partner(&pair.partner).is_none());
    let _ = &pair.canonical;
}

#[tokio::test]
async fn logout_cancel_clears_partner_trade_like_cpp() {
    let mut pair = trade_pair_like_cpp(Position::new(3.0, 1.0, 0.0, 0.0), 1, 10, false);
    pair.source
        .handle_initiate_trade_with_policy(
            initiate_trade_packet(pair.partner_guid),
            TradePolicyLikeCpp::default(),
        )
        .await;
    pair.partner
        .process_represented_session_commands_like_cpp()
        .await;
    let _ = pair.partner_rx.try_iter().count();

    wow_world_application::SessionLifecycleHostLikeCpp::cancel_trade_before_logout_like_cpp(
        &mut pair.source,
    );
    pair.partner
        .process_represented_session_commands_like_cpp()
        .await;

    assert!(trade_partner(&pair.source).is_none());
    assert!(trade_partner(&pair.partner).is_none());
    assert!(
        pair.source_rx.try_recv().is_err(),
        "TradeCancel(false) sends nothing back"
    );
    let bytes = pair.partner_rx.try_recv().expect("partner cancel");
    assert_eq!(trade_status_of(&bytes).1, TRADE_STATUS_CANCELLED_LIKE_CPP);
}
