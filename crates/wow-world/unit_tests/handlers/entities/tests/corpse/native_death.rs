// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;
use wow_constants::DeathState;
use wow_entities::PlayerDeathOperationContext;
use wow_world_entities::PlayerMovementHostLikeCpp;

#[path = "native_death_auras.rs"]
mod native_death_auras;
#[path = "native_death_inventory_guard.rs"]
mod native_death_inventory_guard;

fn live_player(
    counter: i64,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    flume::Receiver<Vec<u8>>,
    ObjectGuid,
) {
    let (mut session, send, realm, guid, _) = dead_player_session_with_graveyard_like_cpp(counter);
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_death_state(DeathState::Alive);
            player.unit_mut().set_max_health(1_000);
            player.unit_mut().set_health(1_000);
        })
        .unwrap();
    crate::native_death_test_fixtures::prepare(&mut session);
    (session, send, realm, guid)
}

fn install_creature_damage_attacker(session: &WorldSession, counter: i64) -> ObjectGuid {
    let map_key = session
        .core
        .current_canonical_player_map_key_like_cpp()
        .expect("native damage Player is resident on its canonical Map");
    let entry = 94_010;
    let attacker = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Creature,
        0,
        1,
        u16::try_from(map_key.map_id).expect("Player Map id fits Creature GUID"),
        0,
        entry,
        counter,
    );
    let mut creature = crate::creature_projection::new_runtime_creature_like_cpp(
        attacker,
        entry,
        Position::ZERO,
        100,
        80,
        1,
        1,
        10.0,
        1,
        14,
        0,
        0,
    );
    creature
        .unit_mut()
        .world_mut()
        .set_map(map_key.map_id, map_key.instance_id)
        .expect("Creature attacker binds to the Player Map");
    let manager = session
        .core
        .canonical_map_manager
        .as_ref()
        .expect("native damage Player has a canonical Map manager")
        .clone();
    manager
        .lock()
        .unwrap()
        .find_map_mut(map_key.map_id, map_key.instance_id)
        .expect("Player Map exists")
        .map_mut()
        .add_map_object_record_to_map_like_cpp(
            wow_entities::MapObjectRecord::new_creature(creature)
                .expect("Creature attacker forms a canonical record"),
        )
        .expect("Creature attacker is admitted to the Player Map");
    attacker
}

#[tokio::test]
async fn native_damage_runs_death_body_before_health_publication_and_can_release() {
    let (mut session, send, realm, guid) = live_player(94_001);
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player.add_self_res_spell(123);
            player.unit_mut().set_emote_state_like_cpp(10);
        })
        .unwrap();
    crate::session::hub_mut(&mut session).set_fall_information_like_cpp(100, 1_000.0);
    let movement = wow_packet::packets::movement::MovementInfo {
        position: Position::xyz(0.0, 0.0, 100.0),
        ..Default::default()
    };
    let _ = sent_opcodes(&send);
    let _ = sent_opcodes(&realm);
    let item_guid_generator = session.id_generators_for_test_like_cpp().item;
    let event = wow_world_entities::handle_fall_like_cpp(
        &mut session,
        &movement,
        &Default::default(),
        item_guid_generator.as_ref(),
    )
    .await
    .unwrap();
    assert_eq!(event.final_damage, 1_000);
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| (
            player.unit().death_state(),
            player.unit().data().health,
            player.data().player_flags & 0x10 != 0,
            player.native_death_operation_snapshot(),
            player.self_res_spells().to_vec(),
            player.unit().emote_state_like_cpp(),
            player.extra_flags() & 0x100 != 0,
        )),
        Some((DeathState::JustDied, 0, false, None, vec![], 0, true))
    );
    let instance_opcodes = sent_opcodes(&send);
    let realm_opcodes = sent_opcodes(&realm);
    for opcode in [
        ServerOpcodes::EnvironmentalDamageLog,
        ServerOpcodes::PowerUpdate,
        ServerOpcodes::StandStateUpdate,
        ServerOpcodes::DurabilityDamageDeath,
    ] {
        assert!(
            realm_opcodes.contains(&(opcode as u16)),
            "{opcode:?} is sent on the C++ realm connection"
        );
        assert!(
            !instance_opcodes.contains(&(opcode as u16)),
            "{opcode:?} is absent from the instance connection"
        );
    }
    assert!(
        !instance_opcodes.contains(&(ServerOpcodes::HealthUpdate as u16))
            && !realm_opcodes.contains(&(ServerOpcodes::HealthUpdate as u16)),
        "Unit::Kill does not call ModifyHealth's immediate HealthUpdate"
    );

    handle_repop_request(&mut session, repop_request_packet(true)).await;
    assert!(
        current_corpse_guid_like_cpp(&session, guid).is_some(),
        "real lethal damage must reach release/corpse persistence"
    );
}

#[tokio::test]
async fn native_damage_creature_kill_wear_uses_realm_and_skips_battleground() {
    for (counter, battleground, expected_realm_durability_packets) in
        [(94_004, false, 1_usize), (94_005, true, 0_usize)]
    {
        let (mut session, send, realm, player_guid) = live_player(counter);
        let attacker = install_creature_damage_attacker(&session, counter + 100);
        let _ = sent_opcodes(&send);
        let _ = sent_opcodes(&realm);
        if battleground {
            session
                .core
                .with_owned_player_mut_like_cpp(|player| {
                    player.set_battleground_context_like_cpp(1, 571);
                    player.set_battleground_instance_id_like_cpp(1);
                })
                .expect("canonical Player exists");
        }
        let item_guid_generator = session.id_generators_for_test_like_cpp().item;
        let result = PlayerMovementHostLikeCpp::apply_native_player_damage_like_cpp(
            &mut session,
            1_000,
            PlayerDeathOperationContext::new(attacker, true),
            &Default::default(),
            item_guid_generator.as_ref(),
        )
        .await
        .expect("Creature lethal damage completes its native Player body");
        assert!(result.killed);
        assert_eq!(result.health_after, 0);
        assert_eq!(
            session.core.with_owned_player_like_cpp(|player| (
                player.unit().death_state(),
                player.unit().data().health,
                player.native_death_operation_snapshot(),
            )),
            Some((DeathState::JustDied, 0, None)),
            "the admitted native death operation completes and retires"
        );

        let instance_opcodes = sent_opcodes(&send);
        let realm_opcodes = sent_opcodes(&realm);
        assert_eq!(
            realm_opcodes
                .iter()
                .filter(|opcode| **opcode == ServerOpcodes::DurabilityDamageDeath as u16)
                .count(),
            expected_realm_durability_packets,
            "normal Creature kills send exactly one realm wear packet; battleground kills send none"
        );
        assert!(
            !instance_opcodes.contains(&(ServerOpcodes::DurabilityDamageDeath as u16)),
            "Unit::Kill durability publication never uses the instance connection"
        );
        assert_eq!(
            session
                .core
                .with_owned_player_like_cpp(|player| player.guid()),
            Some(player_guid)
        );
    }
}

#[tokio::test]
async fn native_damage_preserves_native_width_and_does_not_require_death_catalogs_for_nonlethal_hit()
 {
    let (mut session, send, realm, guid) = live_player(94_002);
    session.core.item_guid_generator_like_cpp = None;
    let item_guid_generator = session.id_generators_for_test_like_cpp().item;
    let before = u64::from(u32::MAX) + 100;
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_max_health(before);
            player.unit_mut().set_health(before);
        })
        .unwrap();
    let _ = sent_opcodes(&send);
    let _ = sent_opcodes(&realm);
    let result = PlayerMovementHostLikeCpp::apply_native_player_damage_like_cpp(
        &mut session,
        7,
        PlayerDeathOperationContext::new(guid, false),
        &Default::default(),
        item_guid_generator.as_ref(),
    )
    .await
    .unwrap();
    assert_eq!(result.health_before, before);
    assert_eq!(result.health_after, before - 7);
    assert!(!result.killed);
    assert_eq!(
        session
            .core
            .with_owned_player_like_cpp(|player| player.unit().data().health),
        Some(before - 7)
    );
    let realm_packets = realm.try_iter().collect::<Vec<_>>();
    assert_eq!(realm_packets.len(), 1, "one owner HealthUpdate uses realm");
    let mut health_update = wow_packet::WorldPacket::from_bytes(&realm_packets[0]);
    assert_eq!(
        health_update.server_opcode(),
        Some(ServerOpcodes::HealthUpdate)
    );
    health_update.skip_opcode();
    assert_eq!(health_update.read_packed_guid().unwrap(), guid);
    assert_eq!(health_update.read_int64().unwrap(), (before - 7) as i64);
    assert!(send.is_empty(), "no duplicate HealthUpdate uses instance");
}

#[tokio::test]
async fn native_damage_failure_retains_operation_and_defers_save_without_publishing_damage_success()
{
    let (mut session, send, realm, guid) = live_player(94_003);
    let item_guid_generator = session.id_generators_for_test_like_cpp().item;
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            let mut spells = player.spell_runtime_like_cpp().clone();
            spells.set_row_authority_like_cpp(false, false);
            player.replace_spell_runtime_like_cpp(spells);
        })
        .unwrap();
    let _ = sent_opcodes(&send);
    let _ = sent_opcodes(&realm);
    let result = PlayerMovementHostLikeCpp::apply_native_player_damage_like_cpp(
        &mut session,
        1_000,
        PlayerDeathOperationContext::new(guid, false),
        &Default::default(),
        item_guid_generator.as_ref(),
    )
    .await;
    assert!(result.is_err());
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| (
            player.unit().data().health,
            player.unit().death_state(),
            player.has_unresolved_native_death_operation(),
        )),
        Some((1_000, DeathState::Alive, true))
    );
    assert_eq!(
        wow_world_lifecycle::save_current_player_to_db_with_generator_like_cpp(
            &mut session,
            item_guid_generator.as_ref()
        )
        .await,
        wow_world_lifecycle::PlayerSaveOutcomeLikeCpp::Deferred
    );
    assert!(!sent_opcodes(&send).contains(&(ServerOpcodes::EnvironmentalDamageLog as u16)));
    assert!(!sent_opcodes(&realm).contains(&(ServerOpcodes::EnvironmentalDamageLog as u16)));
    assert!(current_corpse_guid_like_cpp(&session, guid).is_none());
}
