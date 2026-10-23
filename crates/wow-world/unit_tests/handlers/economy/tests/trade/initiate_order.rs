//! `HandleInitiateTradeOpcode` branch coverage and order (#38 review): every
//! C++ refusal, and pairs of conditions that prove which check runs first.

use super::initiate::{
    TradePairLikeCpp, initiate_trade_packet, trade_pair_like_cpp, with_canonical_player_mut,
};
use super::*;
use crate::session::WorldSession;
use wow_constants::unit::{DeathState, UnitState};
use wow_packet::packets::misc::{
    TRADE_STATUS_DEAD_LIKE_CPP, TRADE_STATUS_FAILED_LIKE_CPP, TRADE_STATUS_LOGGING_OUT_LIKE_CPP,
    TRADE_STATUS_PROPOSED_LIKE_CPP, TRADE_STATUS_STUNNED_LIKE_CPP,
    TRADE_STATUS_TARGET_DEAD_LIKE_CPP, TRADE_STATUS_TARGET_LOGGING_OUT_LIKE_CPP,
    TRADE_STATUS_TARGET_STUNNED_LIKE_CPP, TRADE_STATUS_TOO_FAR_AWAY_LIKE_CPP,
    TRADE_STATUS_WRONG_FACTION_LIKE_CPP,
};
use wow_world_social::TradePolicyLikeCpp;

#[allow(non_snake_case)]
fn NEAR() -> Position {
    Position::new(3.0, 1.0, 0.0, 0.0)
}

#[allow(non_snake_case)]
fn FAR() -> Position {
    Position::new(40.0, 1.0, 0.0, 0.0)
}

fn status_of(bytes: &[u8]) -> u8 {
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::TradeStatus as u16
    );
    (bytes[2] >> 2) & 0x1F
}

fn opened(session: &WorldSession) -> bool {
    wow_world_social::player_trade_state_snapshot_like_cpp(&session.core, &session.social)
        .flatten()
        .is_some()
}

fn own(pair: &mut TradePairLikeCpp, f: impl FnOnce(&mut wow_entities::Player)) {
    pair.source.core.with_owned_player_mut_like_cpp(f).unwrap();
}

fn target(pair: &mut TradePairLikeCpp, f: impl FnOnce(&mut wow_entities::Player)) {
    with_canonical_player_mut(&pair.canonical, pair.partner_guid, f);
}

/// Run one initiation; return the source's packets and whether a trade opened.
async fn run(
    position: Position,
    race: u8,
    level: u8,
    policy: TradePolicyLikeCpp,
    prepare: impl FnOnce(&mut TradePairLikeCpp),
    self_target: bool,
) -> (Vec<Vec<u8>>, bool, Vec<Vec<u8>>) {
    let mut pair = trade_pair_like_cpp(position, race, level, false);
    prepare(&mut pair);
    let target = if self_target {
        pair.source_guid
    } else {
        pair.partner_guid
    };
    pair.source
        .handle_initiate_trade_with_policy(initiate_trade_packet(target), policy)
        .await;
    pair.partner
        .process_represented_session_commands_like_cpp()
        .await;
    let source: Vec<Vec<u8>> = pair.source_rx.try_iter().collect();
    let partner: Vec<Vec<u8>> = pair.partner_rx.try_iter().collect();
    (
        source,
        opened(&pair.partner) || opened(&pair.source),
        partner,
    )
}

async fn refused_with(
    position: Position,
    race: u8,
    level: u8,
    policy: TradePolicyLikeCpp,
    prepare: impl FnOnce(&mut TradePairLikeCpp),
) -> u8 {
    let (source, opened, _) = run(position, race, level, policy, prepare, false).await;
    assert!(!opened, "a refusal opens no trade");
    status_of(source.last().expect("refusal status"))
}

fn stun(player: &mut wow_entities::Player) {
    player.unit_mut().add_unit_state(UnitState::STUNNED.bits());
}

fn fly(player: &mut wow_entities::Player) {
    player
        .unit_mut()
        .add_unit_state(UnitState::IN_FLIGHT.bits());
}

fn kill(player: &mut wow_entities::Player) {
    player.unit_mut().set_death_state(DeathState::Dead);
}

#[tokio::test]
async fn own_state_refusals_like_cpp() {
    let default = TradePolicyLikeCpp::default;
    assert_eq!(
        refused_with(NEAR(), 1, 10, default(), |p| own(p, stun)).await,
        TRADE_STATUS_STUNNED_LIKE_CPP
    );
    assert_eq!(
        refused_with(NEAR(), 1, 10, default(), |p| own(p, fly)).await,
        TRADE_STATUS_TOO_FAR_AWAY_LIKE_CPP
    );
    // `isLogingOut()` of the initiator: `m_playerLogout` during finalization.
    let (source, opened, _) = {
        let mut pair = trade_pair_like_cpp(NEAR(), 1, 10, false);
        pair.source.lifecycle.set_player_logout_like_cpp(true);
        let target = pair.partner_guid;
        pair.source
            .handle_initiate_trade_with_policy(initiate_trade_packet(target), default())
            .await;
        let source: Vec<Vec<u8>> = pair.source_rx.try_iter().collect();
        (source, opened(&pair.partner), ())
    };
    assert!(!opened);
    assert_eq!(status_of(&source[0]), TRADE_STATUS_LOGGING_OUT_LIKE_CPP);
}

#[tokio::test]
async fn target_state_refusals_like_cpp() {
    let default = TradePolicyLikeCpp::default;
    assert_eq!(
        refused_with(NEAR(), 1, 10, default(), |p| target(p, kill)).await,
        TRADE_STATUS_TARGET_DEAD_LIKE_CPP
    );
    assert_eq!(
        refused_with(NEAR(), 1, 10, default(), |p| target(p, fly)).await,
        TRADE_STATUS_TOO_FAR_AWAY_LIKE_CPP
    );
    assert_eq!(
        refused_with(NEAR(), 1, 10, default(), |p| target(p, |t| {
            t.set_trade_session_logging_out_like_cpp(true)
        }))
        .await,
        TRADE_STATUS_TARGET_LOGGING_OUT_LIKE_CPP
    );
    // Trading with oneself is PLAYER_BUSY (`pOther == GetPlayer()`).
    let (source, opened, _) = run(NEAR(), 1, 10, default(), |_| {}, true).await;
    assert!(!opened);
    assert_eq!(
        status_of(&source[0]),
        crate::session::TRADE_STATUS_PLAYER_BUSY_LIKE_CPP
    );
    // The target's level gate: LANG_TRADE_OTHER_REQ notification, then FAILED.
    let (source, opened, _) = run(
        NEAR(),
        1,
        5,
        TradePolicyLikeCpp {
            trade_level_req: 8,
            allow_two_side_trade: false,
        },
        |_| {},
        false,
    )
    .await;
    assert!(!opened);
    assert_eq!(source.len(), 2);
    assert_eq!(
        u16::from_le_bytes([source[0][0], source[0][1]]),
        ServerOpcodes::PrintNotification as u16
    );
    assert_eq!(status_of(&source[1]), TRADE_STATUS_FAILED_LIKE_CPP);
}

#[tokio::test]
async fn refusals_keep_the_cpp_check_order_like_cpp() {
    let default = TradePolicyLikeCpp::default;
    // Own death is checked before everything about the target.
    assert_eq!(
        refused_with(FAR(), 2, 10, default(), |p| {
            own(p, kill);
            target(p, kill);
        })
        .await,
        TRADE_STATUS_DEAD_LIKE_CPP
    );
    // Own stun before own flight.
    assert_eq!(
        refused_with(NEAR(), 1, 10, default(), |p| own(p, |o| {
            stun(o);
            fly(o);
        }))
        .await,
        TRADE_STATUS_STUNNED_LIKE_CPP
    );
    // Target dead before target in flight and stunned.
    assert_eq!(
        refused_with(NEAR(), 1, 10, default(), |p| target(p, |t| {
            kill(t);
            fly(t);
            stun(t);
        }))
        .await,
        TRADE_STATUS_TARGET_DEAD_LIKE_CPP
    );
    // Target in flight before target stunned.
    assert_eq!(
        refused_with(NEAR(), 1, 10, default(), |p| target(p, |t| {
            fly(t);
            stun(t);
        }))
        .await,
        TRADE_STATUS_TOO_FAR_AWAY_LIKE_CPP
    );
    // Target stunned before the faction check.
    assert_eq!(
        refused_with(NEAR(), 2, 10, default(), |p| target(p, stun)).await,
        TRADE_STATUS_TARGET_STUNNED_LIKE_CPP
    );
    // Faction before distance.
    assert_eq!(
        refused_with(FAR(), 2, 10, default(), |_| {}).await,
        TRADE_STATUS_WRONG_FACTION_LIKE_CPP
    );
    // Distance before the target's level.
    assert_eq!(
        refused_with(
            FAR(),
            1,
            5,
            TradePolicyLikeCpp {
                trade_level_req: 8,
                allow_two_side_trade: false,
            },
            |_| {}
        )
        .await,
        TRADE_STATUS_TOO_FAR_AWAY_LIKE_CPP
    );
}

#[tokio::test]
async fn game_master_overrides_the_faction_check_like_cpp() {
    // `HasPermission(RBAC_PERM_ALLOW_TWO_SIDE_TRADE)`, represented by GM mode.
    let (_, opened, partner) = run(
        NEAR(),
        2,
        10,
        TradePolicyLikeCpp::default(),
        |p| own(p, |o| o.set_game_master_like_cpp(true)),
        false,
    )
    .await;
    assert!(opened);
    assert_eq!(status_of(&partner[0]), TRADE_STATUS_PROPOSED_LIKE_CPP);
}

#[tokio::test]
async fn level_gate_notification_goes_to_the_realm_connection_like_cpp() {
    let mut pair = trade_pair_like_cpp(NEAR(), 1, 10, false);
    let (realm_tx, realm_rx) = flume::bounded(8);
    pair.source.install_realm_send_channel_for_test(realm_tx);
    let target = pair.partner_guid;
    pair.source
        .handle_initiate_trade_with_policy(
            initiate_trade_packet(target),
            TradePolicyLikeCpp {
                trade_level_req: 20,
                allow_two_side_trade: false,
            },
        )
        .await;
    let realm: Vec<Vec<u8>> = realm_rx.try_iter().collect();
    let instance: Vec<Vec<u8>> = pair.source_rx.try_iter().collect();
    assert_eq!(realm.len(), 1);
    assert_eq!(
        u16::from_le_bytes([realm[0][0], realm[0][1]]),
        ServerOpcodes::PrintNotification as u16
    );
    assert_eq!(instance.len(), 1);
    assert_eq!(status_of(&instance[0]), TRADE_STATUS_FAILED_LIKE_CPP);
}

#[tokio::test]
async fn logout_retirement_cancels_the_open_trade_and_refuses_new_ones_like_cpp() {
    let mut pair = trade_pair_like_cpp(NEAR(), 1, 10, false);
    let target_guid = pair.partner_guid;
    pair.source
        .handle_initiate_trade_with_policy(
            initiate_trade_packet(target_guid),
            TradePolicyLikeCpp::default(),
        )
        .await;
    pair.partner
        .process_represented_session_commands_like_cpp()
        .await;
    let _ = pair.partner_rx.try_iter().count();

    // The finalization ledger's `Retirement` step, as logout runs it.
    let generator = wow_core::ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);
    let _ = wow_world_application::execute_finalization_step(
        &mut pair.source,
        wow_world_lifecycle::FinalizationStep::Retirement,
        wow_world_lifecycle::FinalizationMode::Disconnect,
        &generator,
    )
    .await;
    pair.partner
        .process_represented_session_commands_like_cpp()
        .await;
    assert!(!opened(&pair.partner), "the partner's m_trade is gone");
    let partner: Vec<Vec<u8>> = pair.partner_rx.try_iter().collect();
    assert_eq!(status_of(&partner[0]), TRADE_STATUS_CANCELLED_LIKE_CPP);
}

/// `isLogingOut()` covers the whole 20 s logout countdown (#1476): a target
/// in it is refused with TARGET_LOGGING_OUT; after its cancel it is proposed.
#[tokio::test]
async fn target_in_logout_countdown_is_logging_out_until_it_cancels_like_cpp() {
    use wow_world_lifecycle::AccountDataHandlerHostLikeCpp;
    let mut pair = trade_pair_like_cpp(NEAR(), 1, 10, false);
    pair.partner
        .handle_logout_request(wow_packet::packets::misc::LogoutRequest { idle_logout: false })
        .await;
    assert!(
        pair.partner.lifecycle.is_logging_out_like_cpp(),
        "countdown started"
    );
    let _ = pair.partner_rx.try_iter().count();

    let target_guid = pair.partner_guid;
    pair.source
        .handle_initiate_trade_with_policy(
            initiate_trade_packet(target_guid),
            TradePolicyLikeCpp::default(),
        )
        .await;
    let source: Vec<Vec<u8>> = pair.source_rx.try_iter().collect();
    assert_eq!(
        status_of(&source[0]),
        TRADE_STATUS_TARGET_LOGGING_OUT_LIKE_CPP
    );
    assert!(!opened(&pair.partner));

    let catalogs = crate::session::SessionHandlerCatalogsLikeCpp::default();
    assert!(
        pair.partner
            .account_data_handler_cx_like_cpp(&catalogs)
            .handle_logout_cancel()
            .await
    );
    pair.source
        .handle_initiate_trade_with_policy(
            initiate_trade_packet(target_guid),
            TradePolicyLikeCpp::default(),
        )
        .await;
    pair.partner
        .process_represented_session_commands_like_cpp()
        .await;
    let partner: Vec<Vec<u8>> = pair.partner_rx.try_iter().collect();
    assert!(opened(&pair.partner));
    assert!(
        partner
            .iter()
            .any(|bytes| u16::from_le_bytes([bytes[0], bytes[1]])
                == ServerOpcodes::TradeStatus as u16
                && status_of(bytes) == TRADE_STATUS_PROPOSED_LIKE_CPP)
    );
}
