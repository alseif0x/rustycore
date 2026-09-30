use super::*;
use wow_world::session::{AuraApplication, RepresentedAuraEffectLikeCpp, WorldSession};
use wow_world::test_fixtures::{
    player_gossip_options_for_test, push_player_gossip_option_for_test,
    set_player_trainer_interaction_for_test,
};
use std::sync::{Arc, Mutex};
use wow_constants::unit::{NPCFlags1, UnitState};
use wow_constants::ServerOpcodes;
use wow_core::guid::HighGuid;
use wow_core::{ObjectGuid, Position};
use wow_entities::{MapObjectRecord, Player};
use wow_packet::WorldPacket;

#[tokio::test]
async fn invalid_gossip_hello_preserves_active_player_menu_state_like_cpp() {
    let (mut session, send_rx, _canonical) = make_bank_slot_session(8);
    let active_source = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9306, 305);
    let invalid_source = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9306, 999);
    set_player_trainer_interaction_for_test(&mut session, active_source, 77);
    push_player_gossip_option_for_test(
        &mut session,
        wow_world::session::GossipOptionInfo {
            gossip_option_id: 31,
            menu_id: 32,
            order_index: 33,
            option_npc: 6,
            action_menu_id: 0,
        },
    );

    session
        .handle_gossip_hello(wow_packet::packets::gossip::Hello {
            unit: invalid_source,
        })
        .await;

    assert!(
        send_rx.try_recv().is_err(),
        "C++ returns before publishing when GetNPCIfCanInteractWith rejects the source"
    );
    assert!(
        gossip_trainer_interaction_matches_for_test(&session, active_source, 77),
        "invalid hello must not replace InteractionData"
    );
    assert_eq!(
        player_gossip_options_for_test(&session).len(),
        1,
        "C++ clears PlayerMenu only after validating the source"
    );
}

#[tokio::test]
async fn gossip_select_requires_exact_active_menu_id_like_cpp() {
    const FEIGN_SLOT: u8 = 22;
    let (mut session, send_rx, canonical) = make_bank_slot_session(2);
    insert_admission_test_player_in_world(&session, &canonical);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 15_513, 523);
    insert_binder_creature(
        &canonical,
        banker,
        NPCFlags1::GOSSIP.bits() | NPCFlags1::BANKER.bits(),
    );
    set_player_trainer_interaction_for_test(&mut session, banker, 77);
    push_player_gossip_option_for_test(
        &mut session,
        wow_world::session::GossipOptionInfo {
            gossip_option_id: 61,
            menu_id: 62,
            order_index: 63,
            option_npc: 6,
            action_menu_id: 0,
        },
    );
    seed_admission_test_feign_death(&mut session, FEIGN_SLOT);

    session
        .handle_gossip_select_option(wow_packet::packets::gossip::GossipSelectOption {
            gossip_unit: banker,
            gossip_id: 999,
            gossip_option_id: 61,
            promotion_code: String::new(),
        })
        .await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::AuraUpdate],
        "C++ removes fake death before Player::OnGossipSelect rejects a mismatched GossipID"
    );
    assert!(
        gossip_trainer_interaction_matches_for_test(&session, banker, 77),
        "a mismatched packet GossipID must not route or replace InteractionData"
    );
    assert_eq!(player_gossip_options_for_test(&session).len(), 1);
    assert!(!has_gossip_visible_aura_for_test(&session, FEIGN_SLOT));
    assert!(!canonical_player_has_died_state_like_cpp(&mut session));
}

fn insert_admission_test_player_in_world(
    session: &WorldSession,
    canonical: &Arc<Mutex<wow_map::MapManager>>,
) {
    let player_guid = session.player_guid().expect("player guid");
    let mut player = Player::new(Some(1), false);
    player.unit_mut().world_mut().object_mut().create(player_guid);
    player.unit_mut().world_mut().set_map(571, 0).unwrap();
    player
        .unit_mut()
        .world_mut()
        .relocate(Position::new(0.0, 0.0, 0.0, 0.0));
    player.unit_mut().world_mut().object_mut().add_to_world();
    canonical
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
}

fn seed_admission_test_feign_death(session: &mut WorldSession, slot: u8) {
    let player_guid = session.player_guid().expect("player guid");
    mutate_canonical_player_for_test(&*session, |player| {
            player.unit_mut().add_unit_state(UnitState::DIED.bits());
        })
        .expect("canonical player");
    insert_gossip_visible_aura_for_test(
        session,
        slot,
        AuraApplication {
            spell_id: 5384,
            difficulty_id: 0,
            caster_guid: player_guid,
            slot,
            duration_total: 0,
            duration_remaining: 0,
            stack_count: 1,
            aura_flags: 0,
            effect_mask: 1,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: Some(RepresentedAuraEffectLikeCpp::FeignDeath),
            represented_amount: 0,
            represented_effect_amounts: Vec::new(),
            represented_misc_value: None,
            represented_multiplier: 1.0,
            applied_at: std::time::Instant::now(),
        },
    );
}

fn canonical_player_has_died_state_like_cpp(session: &mut WorldSession) -> bool {
    mutate_canonical_player_for_test(&*session, |player| {
            player.unit().has_unit_state(UnitState::DIED.bits())
        })
        .expect("canonical player")
}

fn drain_server_opcodes(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<ServerOpcodes> {
    let mut opcodes = Vec::new();
    while let Ok(bytes) = send_rx.try_recv() {
        let packet = WorldPacket::from_bytes(&bytes);
        if let Some(opcode) = packet.server_opcode() {
            opcodes.push(opcode);
        }
    }
    opcodes
}
