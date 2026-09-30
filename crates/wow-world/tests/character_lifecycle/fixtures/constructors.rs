// Original channel/configuration inputs; these builders install no implicit owner.

use super::*;

pub fn make_session() -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded(8);
    let (send_tx, send_rx) = flume::bounded(16);
    (
        WorldSession::new_character_lifecycle_fixture(
            1,
            "TestAccount".into(),
            0,
            2,
            9,
            54261,
            vec![0; 40],
            "enUS".into(),
            pkt_rx,
            send_tx,
        ),
        send_rx,
    )
}

pub mod session {
    use super::*;
    pub fn make_session() -> (
        WorldSession,
        flume::Sender<WorldPacket>,
        flume::Receiver<Vec<u8>>,
    ) {
        let (pkt_tx, pkt_rx) = flume::bounded(100);
        let (send_tx, send_rx) = flume::unbounded();

        let mut session = WorldSession::new_character_lifecycle_fixture(
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
        session.character_set_active_player_local_flags_for_test(
            PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP,
        );

        (session, pkt_tx, send_rx)
    }
}

pub fn run_canonical_player_owner_test(test: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .name("canonical-player-owner".into())
        .stack_size(8 * 1024 * 1024)
        .spawn(test)
        .unwrap()
        .join()
        .unwrap();
}

pub fn shared_canonical_map_manager() -> SharedCanonicalMapManager {
    Arc::new(Mutex::new(wow_map::MapManager::default()))
}

pub fn canonical_player_transfer_test_map_store_like_cpp() -> Arc<wow_data::MapStore> {
    Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ]))
}

pub fn drain_server_packet_bytes(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<Vec<u8>> {
    let mut packets = Vec::new();
    while let Ok(bytes) = send_rx.try_recv() {
        packets.push(bytes);
    }
    packets
}

pub fn world_maps<const N: usize>(ids: [u32; N]) -> Arc<MapStore> {
    Arc::new(MapStore::from_entries(ids.map(|id| MapEntry {
        id,
        instance_type: wow_data::map::MAP_COMMON,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    })))
}
