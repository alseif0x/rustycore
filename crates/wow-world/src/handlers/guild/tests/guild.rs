// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! guild capability handler tests.

use super::*;
use wow_packet::packets::misc::GuildCommandResult;

#[tokio::test]
async fn decline_guild_invites_sets_and_clears_auto_decline_flag_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 9011);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_position_like_cpp(Position::new(1.0, 2.0, 3.0, 0.0));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        0,
    );

    let mut enable = WorldPacket::new_empty();
    enable.write_bit(true);
    enable.flush_bits();
    enable.reset_read();
    session.handle_decline_guild_invites(enable).await;
    assert!(session.represented_auto_decline_guild_invites_like_cpp());

    let mut disable = WorldPacket::new_empty();
    disable.write_bit(false);
    disable.flush_bits();
    disable.reset_read();
    session.handle_decline_guild_invites(disable).await;
    assert!(!session.represented_auto_decline_guild_invites_like_cpp());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn decline_guild_invites_short_packet_does_not_change_flag_like_cpp() {
    let (mut session, _send_rx) = make_session();
    session
        .handle_decline_guild_invites(WorldPacket::from_bytes(&[]))
        .await;

    assert!(!session.represented_auto_decline_guild_invites_like_cpp());
}

#[tokio::test]
async fn guild_decline_invitation_clears_pending_invite_when_unguilded_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_represented_guild_id_like_cpp(0);
    session.set_represented_guild_id_invited_like_cpp(7_001);

    session
        .handle_guild_decline_invitation(WorldPacket::new_empty())
        .await;

    assert_eq!(session.represented_guild_id_invited_like_cpp(), 0);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn guild_decline_invitation_preserves_pending_invite_when_already_guilded_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_represented_guild_id_like_cpp(42);
    session.set_represented_guild_id_invited_like_cpp(7_001);

    session
        .handle_guild_decline_invitation(WorldPacket::new_empty())
        .await;

    assert_eq!(session.represented_guild_id_invited_like_cpp(), 7_001);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn accept_guild_invite_records_invited_guild_when_unguilded_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_represented_guild_id_like_cpp(0);
    session.set_represented_guild_id_invited_like_cpp(7_001);

    session
        .handle_accept_guild_invite(WorldPacket::new_empty())
        .await;

    assert_eq!(
        session.represented_guild_accept_invites_like_cpp(),
        &[7_001]
    );
    assert_eq!(session.represented_guild_id_invited_like_cpp(), 7_001);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn accept_guild_invite_ignores_guilded_player_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_represented_guild_id_like_cpp(42);
    session.set_represented_guild_id_invited_like_cpp(7_001);

    session
        .handle_accept_guild_invite(WorldPacket::new_empty())
        .await;

    assert!(
        session
            .represented_guild_accept_invites_like_cpp()
            .is_empty()
    );
    assert_eq!(session.represented_guild_id_invited_like_cpp(), 7_001);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn accept_guild_invite_ignores_missing_invited_guild_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_represented_guild_id_like_cpp(0);
    session.set_represented_guild_id_invited_like_cpp(0);

    session
        .handle_accept_guild_invite(WorldPacket::new_empty())
        .await;

    assert!(
        session
            .represented_guild_accept_invites_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn guild_bank_remaining_withdraw_money_without_guild_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_guild_bank_remaining_withdraw_money_query(WorldPacket::new_empty())
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn auto_guild_bank_item_without_guild_or_banker_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let banker = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 11);

    session
        .handle_auto_guild_bank_item(auto_guild_bank_item_packet(banker, 1, 2, 20, Some(255)))
        .await;
    session
        .handle_auto_store_guild_bank_item(auto_store_guild_bank_item_packet(banker, 1, 2))
        .await;

    assert!(
        session
            .represented_guild_bank_inventory_moves_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn guild_bank_activate_without_banker_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let banker = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 14);

    session
        .handle_guild_bank_activate(guild_bank_activate_packet(banker, true))
        .await;

    assert!(
        session
            .represented_guild_bank_list_requests_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn guild_bank_activate_without_guild_sends_view_tab_error_like_cpp() {
    let (mut session, send_rx) = make_session();
    let banker = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 15);
    install_represented_guild_bank_like_cpp(&mut session, banker, 0);

    session
        .handle_guild_bank_activate(guild_bank_activate_packet(banker, true))
        .await;

    assert!(
        session
            .represented_guild_bank_list_requests_like_cpp()
            .is_empty()
    );
    let bytes = send_rx.try_recv().unwrap();
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::GuildCommandResult as u16
    );
    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(
        pkt.read_int32().unwrap(),
        GuildCommandResult::ERR_PLAYER_NOT_IN_GUILD_LIKE_CPP
    );
    assert_eq!(
        pkt.read_int32().unwrap(),
        GuildCommandResult::COMMAND_VIEW_TAB_LIKE_CPP
    );
    assert_eq!(pkt.read_bits(8).unwrap(), 0);
    assert_eq!(pkt.remaining(), 0);
}

#[tokio::test]
async fn guild_bank_activate_records_represented_bank_list_like_cpp() {
    let (mut session, send_rx) = make_session();
    let banker = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 16);
    install_represented_guild_bank_like_cpp(&mut session, banker, 42);

    session
        .handle_guild_bank_activate(guild_bank_activate_packet(banker, true))
        .await;

    assert_eq!(
        session.represented_guild_bank_list_requests_like_cpp(),
        &[crate::session::RepresentedGuildBankListRequestLikeCpp {
            banker,
            guild_id: 42,
            tab: 0,
            full_update: true,
        }]
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn guild_bank_query_tab_without_banker_or_guild_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let banker = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 17);

    session
        .handle_guild_bank_query_tab(guild_bank_query_tab_packet(banker, 4, true))
        .await;

    assert!(
        session
            .represented_guild_bank_list_requests_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());

    install_represented_guild_bank_like_cpp(&mut session, banker, 0);
    session
        .handle_guild_bank_query_tab(guild_bank_query_tab_packet(banker, 4, true))
        .await;

    assert!(
        session
            .represented_guild_bank_list_requests_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn guild_bank_query_tab_records_bank_list_with_forced_full_update_like_cpp() {
    let (mut session, send_rx) = make_session();
    let banker = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 18);
    install_represented_guild_bank_like_cpp(&mut session, banker, 42);

    session
        .handle_guild_bank_query_tab(guild_bank_query_tab_packet(banker, 4, false))
        .await;

    assert_eq!(
        session.represented_guild_bank_list_requests_like_cpp(),
        &[crate::session::RepresentedGuildBankListRequestLikeCpp {
            banker,
            guild_id: 42,
            tab: 4,
            full_update: true,
        }]
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn guild_bank_money_without_banker_guild_or_money_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let banker = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 19);

    session
        .handle_guild_bank_deposit_money(guild_bank_money_packet(banker, 25))
        .await;
    session
        .handle_guild_bank_withdraw_money(guild_bank_money_packet(banker, 25))
        .await;
    assert!(
        session
            .represented_guild_bank_money_moves_like_cpp()
            .is_empty()
    );

    install_represented_guild_bank_like_cpp(&mut session, banker, 0);
    session
        .handle_guild_bank_deposit_money(guild_bank_money_packet(banker, 25))
        .await;
    session
        .handle_guild_bank_withdraw_money(guild_bank_money_packet(banker, 25))
        .await;
    session
        .handle_guild_bank_deposit_money(guild_bank_money_packet(banker, 0))
        .await;
    session
        .handle_guild_bank_withdraw_money(guild_bank_money_packet(banker, 0))
        .await;

    assert!(
        session
            .represented_guild_bank_money_moves_like_cpp()
            .is_empty()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn guild_bank_deposit_money_requires_player_money_like_cpp() {
    let (mut session, send_rx) = make_session();
    let banker = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 20);
    install_represented_guild_bank_like_cpp(&mut session, banker, 42);
    session.set_player_gold_like_cpp(10);

    session
        .handle_guild_bank_deposit_money(guild_bank_money_packet(banker, 25))
        .await;
    assert!(
        session
            .represented_guild_bank_money_moves_like_cpp()
            .is_empty()
    );

    session.set_player_gold_like_cpp(100);
    session
        .handle_guild_bank_deposit_money(guild_bank_money_packet(banker, 25))
        .await;

    assert_eq!(
        session.represented_guild_bank_money_moves_like_cpp(),
        &[crate::session::RepresentedGuildBankMoneyMoveLikeCpp {
            banker,
            guild_id: 42,
            deposit: true,
            money: 25,
        }]
    );
    assert_eq!(session.player_gold_like_cpp(), 100);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn guild_bank_withdraw_money_records_without_player_money_check_like_cpp() {
    let (mut session, send_rx) = make_session();
    let banker = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 21);
    install_represented_guild_bank_like_cpp(&mut session, banker, 42);
    session.set_player_gold_like_cpp(0);

    session
        .handle_guild_bank_withdraw_money(guild_bank_money_packet(banker, 30))
        .await;

    assert_eq!(
        session.represented_guild_bank_money_moves_like_cpp(),
        &[crate::session::RepresentedGuildBankMoneyMoveLikeCpp {
            banker,
            guild_id: 42,
            deposit: false,
            money: 30,
        }]
    );
    assert_eq!(session.player_gold_like_cpp(), 0);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn guild_bank_buy_tab_accepts_empty_banker_but_requires_guild_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_guild_bank_buy_tab(guild_bank_buy_tab_packet(ObjectGuid::EMPTY, 1))
        .await;
    assert!(
        session
            .represented_guild_bank_tab_actions_like_cpp()
            .is_empty()
    );

    session.set_represented_guild_id_like_cpp(42);
    session
        .handle_guild_bank_buy_tab(guild_bank_buy_tab_packet(ObjectGuid::EMPTY, 1))
        .await;

    assert_eq!(
        session.represented_guild_bank_tab_actions_like_cpp(),
        &[crate::session::RepresentedGuildBankTabActionLikeCpp {
            banker: None,
            guild_id: 42,
            tab: 1,
            action: crate::session::RepresentedGuildBankTabActionKindLikeCpp::Buy,
        }]
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn guild_bank_buy_tab_requires_interactable_nonempty_banker_like_cpp() {
    let (mut session, send_rx) = make_session();
    let banker = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 22);
    session.set_represented_guild_id_like_cpp(42);

    session
        .handle_guild_bank_buy_tab(guild_bank_buy_tab_packet(banker, 2))
        .await;
    assert!(
        session
            .represented_guild_bank_tab_actions_like_cpp()
            .is_empty()
    );

    install_represented_guild_bank_like_cpp(&mut session, banker, 42);
    session
        .handle_guild_bank_buy_tab(guild_bank_buy_tab_packet(banker, 2))
        .await;

    assert_eq!(
        session.represented_guild_bank_tab_actions_like_cpp(),
        &[crate::session::RepresentedGuildBankTabActionLikeCpp {
            banker: Some(banker),
            guild_id: 42,
            tab: 2,
            action: crate::session::RepresentedGuildBankTabActionKindLikeCpp::Buy,
        }]
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn guild_bank_update_tab_requires_name_icon_banker_and_guild_like_cpp() {
    let (mut session, send_rx) = make_session();
    let banker = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 23);

    session
        .handle_guild_bank_update_tab(guild_bank_update_tab_packet(
            banker,
            3,
            "Raid",
            "inv_misc_bag",
        ))
        .await;
    assert!(
        session
            .represented_guild_bank_tab_actions_like_cpp()
            .is_empty()
    );

    install_represented_guild_bank_like_cpp(&mut session, banker, 42);
    session
        .handle_guild_bank_update_tab(guild_bank_update_tab_packet(banker, 3, "", "icon"))
        .await;
    session
        .handle_guild_bank_update_tab(guild_bank_update_tab_packet(banker, 3, "Raid", ""))
        .await;
    assert!(
        session
            .represented_guild_bank_tab_actions_like_cpp()
            .is_empty()
    );

    session
        .handle_guild_bank_update_tab(guild_bank_update_tab_packet(
            banker,
            3,
            "Raid",
            "inv_misc_bag",
        ))
        .await;

    assert_eq!(
        session.represented_guild_bank_tab_actions_like_cpp(),
        &[crate::session::RepresentedGuildBankTabActionLikeCpp {
            banker: Some(banker),
            guild_id: 42,
            tab: 3,
            action: crate::session::RepresentedGuildBankTabActionKindLikeCpp::Update {
                name: "Raid".to_string(),
                icon: "inv_misc_bag".to_string(),
            },
        }]
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn guild_bank_log_text_and_set_text_require_guild_only_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_guild_bank_log_query(guild_bank_tab_query_packet(7))
        .await;
    session
        .handle_guild_bank_text_query(guild_bank_tab_query_packet(2))
        .await;
    session
        .handle_guild_bank_set_tab_text(guild_bank_set_tab_text_packet(2, "hello"))
        .await;
    assert!(
        session
            .represented_guild_bank_tab_actions_like_cpp()
            .is_empty()
    );

    session.set_represented_guild_id_like_cpp(42);
    session
        .handle_guild_bank_log_query(guild_bank_tab_query_packet(7))
        .await;
    session
        .handle_guild_bank_text_query(guild_bank_tab_query_packet(2))
        .await;
    session
        .handle_guild_bank_set_tab_text(guild_bank_set_tab_text_packet(2, "hello"))
        .await;

    assert_eq!(
        session.represented_guild_bank_tab_actions_like_cpp(),
        &[
            crate::session::RepresentedGuildBankTabActionLikeCpp {
                banker: None,
                guild_id: 42,
                tab: 7,
                action: crate::session::RepresentedGuildBankTabActionKindLikeCpp::LogQuery,
            },
            crate::session::RepresentedGuildBankTabActionLikeCpp {
                banker: None,
                guild_id: 42,
                tab: 2,
                action: crate::session::RepresentedGuildBankTabActionKindLikeCpp::TextQuery,
            },
            crate::session::RepresentedGuildBankTabActionLikeCpp {
                banker: None,
                guild_id: 42,
                tab: 2,
                action: crate::session::RepresentedGuildBankTabActionKindLikeCpp::SetText {
                    text: "hello".to_string(),
                },
            },
        ]
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn auto_guild_bank_item_records_represented_swap_with_inventory_like_cpp() {
    let (mut session, send_rx) = make_session();
    let banker = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 12);
    install_represented_guild_bank_like_cpp(&mut session, banker, 42);

    session
        .handle_auto_guild_bank_item(auto_guild_bank_item_packet(
            banker,
            2,
            14,
            wow_entities::INVENTORY_SLOT_ITEM_START,
            None,
        ))
        .await;

    assert_eq!(
        session.represented_guild_bank_inventory_moves_like_cpp(),
        &[crate::session::RepresentedGuildBankInventoryMoveLikeCpp {
            banker,
            guild_id: 42,
            to_char: false,
            bank_tab: 2,
            bank_slot: 14,
            player_bag: wow_entities::INVENTORY_SLOT_BAG_0,
            player_slot: wow_entities::INVENTORY_SLOT_ITEM_START,
            stack_count: 0,
        }]
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn auto_store_guild_bank_item_records_null_slot_to_char_like_cpp() {
    let (mut session, send_rx) = make_session();
    let banker = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 13);
    install_represented_guild_bank_like_cpp(&mut session, banker, 42);

    session
        .handle_auto_store_guild_bank_item(auto_store_guild_bank_item_packet(banker, 3, 19))
        .await;

    assert_eq!(
        session.represented_guild_bank_inventory_moves_like_cpp(),
        &[crate::session::RepresentedGuildBankInventoryMoveLikeCpp {
            banker,
            guild_id: 42,
            to_char: true,
            bank_tab: 3,
            bank_slot: 19,
            player_bag: wow_entities::INVENTORY_SLOT_BAG_0,
            player_slot: wow_entities::NULL_SLOT,
            stack_count: 0,
        }]
    );
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn guild_bank_inventory_move_handler_metadata_matches_cpp() {
    let activate = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::GuildBankActivate)
        .expect("GuildBankActivate handler entry");
    assert_eq!(activate.status, SessionStatus::LoggedIn);
    assert_eq!(activate.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(activate.handler_name, "handle_guild_bank_activate");

    let query_tab = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::GuildBankQueryTab)
        .expect("GuildBankQueryTab handler entry");
    assert_eq!(query_tab.status, SessionStatus::LoggedIn);
    assert_eq!(query_tab.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(query_tab.handler_name, "handle_guild_bank_query_tab");

    let buy_tab = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::GuildBankBuyTab)
        .expect("GuildBankBuyTab handler entry");
    assert_eq!(buy_tab.status, SessionStatus::LoggedIn);
    assert_eq!(buy_tab.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(buy_tab.handler_name, "handle_guild_bank_buy_tab");

    let update_tab = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::GuildBankUpdateTab)
        .expect("GuildBankUpdateTab handler entry");
    assert_eq!(update_tab.status, SessionStatus::LoggedIn);
    assert_eq!(update_tab.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(update_tab.handler_name, "handle_guild_bank_update_tab");

    let deposit_money = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::GuildBankDepositMoney)
        .expect("GuildBankDepositMoney handler entry");
    assert_eq!(deposit_money.status, SessionStatus::LoggedIn);
    assert_eq!(deposit_money.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(
        deposit_money.handler_name,
        "handle_guild_bank_deposit_money"
    );

    let withdraw_money = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::GuildBankWithdrawMoney)
        .expect("GuildBankWithdrawMoney handler entry");
    assert_eq!(withdraw_money.status, SessionStatus::LoggedIn);
    assert_eq!(withdraw_money.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(
        withdraw_money.handler_name,
        "handle_guild_bank_withdraw_money"
    );

    let log_query = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::GuildBankLogQuery)
        .expect("GuildBankLogQuery handler entry");
    assert_eq!(log_query.status, SessionStatus::LoggedIn);
    assert_eq!(log_query.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(log_query.handler_name, "handle_guild_bank_log_query");

    let text_query = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::GuildBankTextQuery)
        .expect("GuildBankTextQuery handler entry");
    assert_eq!(text_query.status, SessionStatus::LoggedIn);
    assert_eq!(text_query.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(text_query.handler_name, "handle_guild_bank_text_query");

    let set_tab_text = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::GuildBankSetTabText)
        .expect("GuildBankSetTabText handler entry");
    assert_eq!(set_tab_text.status, SessionStatus::LoggedIn);
    assert_eq!(set_tab_text.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(set_tab_text.handler_name, "handle_guild_bank_set_tab_text");

    let auto_guild = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::AutoGuildBankItem)
        .expect("AutoGuildBankItem handler entry");
    assert_eq!(auto_guild.status, SessionStatus::LoggedIn);
    assert_eq!(auto_guild.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(auto_guild.handler_name, "handle_auto_guild_bank_item");

    let auto_store = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::AutoStoreGuildBankItem)
        .expect("AutoStoreGuildBankItem handler entry");
    assert_eq!(auto_store.status, SessionStatus::LoggedIn);
    assert_eq!(auto_store.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(auto_store.handler_name, "handle_auto_store_guild_bank_item");
}

#[tokio::test]
async fn guild_set_achievement_tracking_without_guild_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(2);
    pkt.write_uint32(100);
    pkt.write_uint32(200);
    pkt.reset_read();

    session.handle_guild_set_achievement_tracking(pkt).await;

    assert!(send_rx.try_recv().is_err());
}
