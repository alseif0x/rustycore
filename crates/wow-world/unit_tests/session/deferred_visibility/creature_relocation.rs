use super::*;

#[tokio::test]
async fn moving_creature_updates_stationary_observer_on_leaving_and_returning() {
    let mut fixture = Fixture::new();
    fixture.establish_visible_creature().await;
    let range = fixture
        .canonical
        .lock()
        .unwrap()
        .find_map(KEY.map_id, KEY.instance_id)
        .unwrap()
        .map()
        .visibility_range();
    // Both endpoints are in cells visited by the player. The diagonal puts
    // the creature outside the circular sight radius at the first endpoint.
    let near = Position::new(12.0, 10.0, 0.0, 0.0);
    let far = Position::new(
        fixture.position.x + range * 0.85,
        fixture.position.y + range * 0.85,
        0.0,
        0.0,
    );
    let distant = Position::new(1500.0, 1500.0, 0.0, 0.0);
    fixture
        .canonical
        .lock()
        .unwrap()
        .find_map_mut(KEY.map_id, KEY.instance_id)
        .unwrap()
        .map_mut()
        .load_grid(distant.x, distant.y);
    for destination in [far, near, distant, near] {
        fixture
            .session
            .mutate_world_creature(fixture.creature, |creature| {
                creature.set_ai_position(destination)
            })
            .expect("fixture creature belongs to this map");
        let intents = fixture.tick();
        assert_eq!(intents.len(), 1);
        assert!(
            fixture
                .registry
                .request_deferred_player_visibility_refresh_like_cpp(intents[0])
        );
        fixture.pump().await;
        if destination == near {
            fixture.assert_created_once();
        } else {
            assert!(
                !fixture
                    .session
                    .core
                    .client_visible_guids_like_cpp
                    .contains(&fixture.creature)
            );
            let packets: Vec<_> = fixture.output.try_iter().collect();
            let mut removed = Vec::new();
            for bytes in packets {
                let mut packet = WorldPacket::from_bytes(&bytes);
                if packet.read_uint16().unwrap() != ServerOpcodes::UpdateObject as u16 {
                    continue;
                }
                let _blocks = packet.read_uint32().unwrap();
                assert_eq!(packet.read_uint16().unwrap(), KEY.map_id as u16);
                if packet.read_bit().unwrap() {
                    assert_eq!(packet.read_uint16().unwrap(), 0, "out-of-range section");
                    let count = packet.read_uint32().unwrap();
                    for _ in 0..count {
                        removed.push(packet.read_packed_guid().unwrap());
                    }
                }
            }
            assert_eq!(removed, vec![fixture.creature]);
        }
        assert!(
            fixture.tick().is_empty(),
            "no repeat notify for an idle creature"
        );
    }
}
