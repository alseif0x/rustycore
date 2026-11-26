use super::*;
use std::time::Instant;
use wow_packet::ServerPacket;
use wow_packet::packets::spell::{SpellGoPkt, SpellStartPkt, SpellTargetData};
use wow_world::session::mailbox::{SendCreatureSpellCastIfVisibleLikeCppCommand, SessionCommand};
use wow_world_core::session::directory::PlayerRegistry;

/// Commit distinct START/GO frames to the current Player's durable queue.
fn queue_cast(registry: &PlayerRegistry, player: ObjectGuid, counter: i64) -> [Vec<u8>; 2] {
    let recipient = registry.runtime_recipient(player).unwrap();
    assert_eq!((recipient.map_id, recipient.instance_id), (0, 0));
    let caster = ObjectGuid::create_creature_like_cpp(1, 0, 299, 91);
    recipient.committed_visibility.insert(caster);
    let cast_id =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Cast, 3, 1, 0, 0, 133, counter);
    let target = SpellTargetData {
        flags: 2,
        unit: player,
        ..Default::default()
    };
    let start = SpellStartPkt {
        caster,
        cast_id,
        original_cast_id: ObjectGuid::EMPTY,
        spell_id: 133,
        visual: Default::default(),
        cast_flags: 0,
        cast_flags_ex: 0,
        cast_time_ms: 0,
        target: target.clone(),
        cast_data: Default::default(),
    }
    .to_bytes();
    let go = SpellGoPkt {
        caster,
        cast_id,
        original_cast_id: ObjectGuid::EMPTY,
        spell_id: 133,
        visual: Default::default(),
        cast_flags: 0,
        cast_flags_ex: 0,
        cast_time_ms: 1,
        target,
        cast_data: Default::default(),
        hit_targets: vec![player],
        miss_targets: vec![],
    }
    .to_bytes();
    assert!(registry.publish_current_creature_spell_cast_if_visible(
        recipient.registration,
        SendCreatureSpellCastIfVisibleLikeCppCommand {
            queued_at: Instant::now(),
            source_guid: caster,
            map_id: 0,
            instance_id: 0,
            start_packet_bytes: start.clone(),
            go_packet_bytes: go.clone(),
            committed_visibility_like_cpp: recipient.committed_visibility,
        },
    ));
    [start, go]
}

#[tokio::test]
async fn same_character_relogin_delivers_only_the_new_creature_cast() {
    let mut fixture = module_hook::fixture();
    let registry = Arc::new(PlayerRegistry::new());
    fixture.session.set_player_registry(registry.clone());
    let player = ObjectGuid::create_player(1, 42);
    module_hook::run_login(&mut fixture.session, &fixture.maps).await;
    let old = queue_cast(&registry, player, 1);

    *fixture.port.save_probe.lock().unwrap() = Some(Arc::new(save::SaveProbe {
        requests: Mutex::new(vec![]),
        released: AtomicBool::new(true),
        outcome: PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    }));
    let generator = wow_core::ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);
    let release = fixture
        .session
        .finalize_session_with_generator_like_cpp(Mode::CharacterSelection, &generator)
        .await;
    assert_eq!(
        release.outcome(Step::Release),
        Outcome::Applied,
        "{release:?}"
    );
    assert_eq!(fixture.session.state(), SessionState::Authed);
    fixture.session.set_player_loading(Some(player));
    module_hook::run_login(&mut fixture.session, &fixture.maps).await;
    assert_eq!(fixture.session.state(), SessionState::LoggedIn);
    let current = queue_cast(&registry, player, 2);

    while fixture.send_rx.try_recv().is_ok() {}
    let mut queued = Vec::new();
    fixture
        .session
        .run_session_commands_and_kills_for_test_like_cpp(|command| {
            if let SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(cast) = command {
                queued.push([
                    cast.start_packet_bytes.clone(),
                    cast.go_packet_bytes.clone(),
                ]);
            }
        })
        .await;
    assert_eq!(
        queued,
        [old, current.clone()],
        "both committed casts reached dispatch"
    );
    let delivered: Vec<_> = fixture
        .send_rx
        .try_iter()
        .filter(|packet| {
            packet.starts_with(&(wow_constants::ServerOpcodes::SpellStart as u16).to_le_bytes())
                || packet.starts_with(&(wow_constants::ServerOpcodes::SpellGo as u16).to_le_bytes())
        })
        .collect();
    assert_eq!(delivered, current, "only the new Player receives its cast");
}
