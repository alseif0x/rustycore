use super::*;

pub(super) fn make_appearance_session() -> (
    WorldSession, flume::Sender<WorldPacket>, flume::Receiver<Vec<u8>>,
) {
    let (pkt_tx, pkt_rx) = flume::bounded(100);
    let (send_tx, send_rx) = flume::unbounded();
    let mut session = WorldSession::new(
        1, "TestAccount".into(), 0, 2, 9, 54261,
        vec![0u8; 40], "esES".into(), pkt_rx, send_tx,
    );
    wow_world::test_fixtures::appearance_install_local_flags_for_test(&mut session, 0x0000_8000);
    (session, pkt_tx, send_rx)
}

pub(super) fn shared_canonical_map_manager() -> wow_world::session::SharedCanonicalMapManager {
    Arc::new(Mutex::new(wow_map::MapManager::default()))
}

pub(super) fn add_canonical_test_player_on_map(
    canonical: &wow_world::session::SharedCanonicalMapManager,
    guid: ObjectGuid,
    position: Position,
    map_id: u32,
    instance_id: u32,
) {
    let mut player = Player::new(Some(1), false);
    player.unit_mut().world_mut().object_mut().create(guid);
    player.unit_mut().world_mut().set_name("InstanceOwner");
    player.unit_mut().world_mut().set_map(map_id, instance_id).unwrap();
    player.unit_mut().world_mut().relocate(position);
    player.unit_mut().world_mut().object_mut().add_to_world();
    canonical.lock().unwrap()
        .create_map_entry(map_id, instance_id, 0, wow_map::ManagedMapKind::World)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
        .unwrap();
}

pub(super) fn grant_learned_weapon_proficiency_like_cpp(
    session: &mut WorldSession, subclass_mask: u32,
) {
    assert!(
        mutate_canonical_player_for_test(session, |player| {
            player.add_weapon_proficiency_like_cpp(subclass_mask);
        }).is_some(),
        "the canonical Player must own the learned weapon proficiency"
    );
}

pub(super) fn test_quest_template(id: u32) -> QuestTemplate {
    let mut quest = quest_template(id);
    // These are the only differing defaults in the original Session quest fixture.
    quest.quest_type = 0;
    quest.log_title = String::new();
    quest
}

mod item_rows;
pub(super) use item_rows::*;
