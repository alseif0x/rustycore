//! Feature-only access to existing Character Gossip composition.

use crate::session::WorldSession;

pub const GOSSIP_TRAINER_OPTION_ID_FOR_TEST: i32 =
    super::super::GOSSIP_OPTION_ID_AUTO_TRAINER_LIKE_CPP;
pub const GOSSIP_TRAINER_OPTION_NPC_FOR_TEST: u8 =
    super::super::GOSSIP_OPTION_NPC_TRAINER_LIKE_CPP;

pub(crate) fn make_gossip_bank_session_for_test(
    capacity: usize,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    std::sync::Arc<std::sync::Mutex<wow_map::MapManager>>,
) {
    let (mut session, send_rx, canonical) =
        super::super::bank_test_support::make_bank_slot_session(capacity);
    session.enable_gossip_fixture_for_test();
    (session, send_rx, canonical)
}

pub(crate) fn insert_gossip_binder_creature_for_test(
    manager: &std::sync::Arc<std::sync::Mutex<wow_map::MapManager>>,
    guid: wow_core::ObjectGuid,
    npc_flags: u32,
) {
    super::super::bank_test_support::insert_binder_creature(manager, guid, npc_flags);
}
