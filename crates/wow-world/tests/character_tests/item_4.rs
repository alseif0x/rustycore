//! Item scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn gossip_banker_selection_replaces_trainer_provenance_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let banker = creature_guid(15_513, 522);
    attach_legacy_creature(
        &mut session,
        banker,
        15_513,
        NPCFlags1::GOSSIP.bits() | NPCFlags1::BANKER.bits(),
    );
    set_player_trainer_interaction_for_test(&mut session, banker, 77);
    push_player_gossip_option_for_test(
        &mut session,
        wow_world::session::GossipOptionInfo {
            gossip_option_id: 51,
            menu_id: 52,
            order_index: 53,
            option_npc: 6,
            action_menu_id: 0,
        },
    );

    session
        .handle_gossip_select_option(wow_packet::packets::gossip::GossipSelectOption {
            gossip_unit: banker,
            gossip_id: 52,
            gossip_option_id: 51,
            promotion_code: String::new(),
        })
        .await;

    assert_eq!(
        WorldPacket::from_bytes(&send_rx.try_recv().unwrap()).server_opcode(),
        Some(ServerOpcodes::NpcInteractionOpenResult)
    );
    assert_eq!(
        player_interaction_source_guid_for_test(&session),
        Some(banker)
    );
    assert_eq!(player_interaction_trainer_id_for_test(&session), 0);
}
