//! Feature-only setup/access for the original creature-melee APP contracts.
use super::*;
use std::sync::{Arc, Mutex};
use wow_packet::WorldPacket;
use wow_entities::Player;
use wow_core::{ObjectGuid, Position};

mod builders;
pub use builders::*;
mod controller;
pub use controller::CreatureMeleePlayerController;
mod canonical_creatures;
pub use canonical_creatures::*;

impl WorldSession {
    pub fn fixture_melee_set_map_position(&mut self, map_id: u16, position: Position) {
        self.set_player_map_position_like_cpp(map_id, position);
    }
    pub async fn fixture_melee_process_commands(&mut self) {
        self.process_represented_session_commands_like_cpp().await;
    }
    pub fn fixture_melee_set_state(&mut self, state: SessionState) { self.state = state; }
    pub fn fixture_melee_make_visible(&mut self, guid: ObjectGuid) {
        self.client_visible_guids_like_cpp.insert(guid);
    }
    pub fn fixture_melee_set_health(&mut self, health: u32, max_health: u32) {
        self.set_player_health_like_cpp(health, max_health);
    }
    pub fn fixture_melee_health(&self) -> u32 { self.resolved_player_vitals_like_cpp().unwrap().0 }
    pub fn fixture_melee_is_alive(&self) -> bool { self.player_is_alive_like_cpp() }
    pub fn fixture_melee_mutate_player<R>(&self, f: impl FnOnce(&mut Player)->R) -> Option<R> {
        self.mutate_canonical_player_like_cpp(f)
    }
    pub fn fixture_melee_adopt_registered_player(&mut self) -> bool {
        self.adopt_registered_canonical_player_fixture_like_cpp()
    }
    pub fn fixture_melee_mutate_creature<R>(&mut self, guid: ObjectGuid,
        f: impl FnOnce(&mut crate::map_manager::WorldCreature)->R) -> Option<R> {
        self.mutate_world_creature(guid, f)
    }
}

pub fn apply_creature_melee_victim_sync_to_legacy_like_cpp(
    victim: &mut crate::map_manager::WorldCreature,
    sync: &wow_map::map::CreatureMeleeVictimSyncStateLikeCpp,
    game_time_secs: i64,
) -> bool {
    super::apply_creature_melee_victim_sync_to_legacy_like_cpp(victim, sync, game_time_secs)
}

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
    session.set_active_player_local_flags_like_cpp(
        PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP,
    );

    (session, pkt_tx, send_rx)
}

pub fn shared_map_manager() -> crate::map_manager::SharedMapManager {
    Arc::new(std::sync::RwLock::new(crate::map_manager::MapManager::new()))
}

pub fn shared_canonical_map_manager() -> SharedCanonicalMapManager {
    Arc::new(Mutex::new(wow_map::MapManager::default()))
}

pub fn test_creature_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 0, 1, counter)
}

pub fn creature_melee_sync_state_for_test_like_cpp(
    victim: &crate::map_manager::WorldCreature,
    applied_damage: u32,
) -> CreatureMeleeVictimSyncStateLikeCpp {
    let mut canonical = victim.clone();
    let identity_authority = canonical.creature.loot_authority_like_cpp().clone();
    let identity_health_authority = canonical
        .creature
        .unit()
        .health_state_revision_authority_like_cpp();
    let health_before = canonical.creature.unit().data().health;
    let revision_before = canonical.creature.unit().health_state_revision_like_cpp();
    let lifecycle_before = canonical.creature.loot_lifecycle_revision_like_cpp();
    let death_before = canonical.creature.unit().death_state();
    let ai_before = canonical.creature.ai_ownership().state;
    let killed = canonical.take_damage_before_death_state_at_game_time_like_cpp(
        applied_damage,
        wow_entities::game_time_secs_like_cpp(),
    );
    if killed {
        canonical.complete_death_state_after_kill_hooks_like_cpp();
    }
    CreatureMeleeVictimSyncStateLikeCpp {
        applied_damage,
        threat: None,
        victim_health_before: health_before,
        victim_health_after: canonical.creature.unit().data().health,
        victim_health_state_revision_before: revision_before,
        victim_health_state_revision_after: canonical
            .creature
            .unit()
            .health_state_revision_like_cpp(),
        identity: CreatureMeleeVictimSyncIdentityLikeCpp {
            authority: identity_authority,
            health_state_revision_authority: identity_health_authority,
            spawn_id: canonical.creature.spawn_id(),
            loot_lifecycle_revision_before: lifecycle_before,
            loot_lifecycle_revision_after: canonical.creature.loot_lifecycle_revision_like_cpp(),
            death_state_before: death_before,
            death_state_after: canonical.creature.unit().death_state(),
            ai_state_before: ai_before,
            ai_state_after: canonical.creature.ai_ownership().state,
        },
    }
}

pub fn install_committed_canonical_player_health_for_melee_test_like_cpp(
    session: &mut WorldSession,
    victim_guid: ObjectGuid,
    health: u64,
    death_state: wow_constants::DeathState,
) -> u64 {
    let canonical = shared_canonical_map_manager();
    add_canonical_test_player_on_map(&canonical, victim_guid, Position::ZERO, 571, 0);
    let revision = {
        let mut guard = canonical.lock().unwrap();
        let player = guard
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(victim_guid)
            .unwrap();
        player.unit_mut().set_max_health(100);
        player.unit_mut().set_health(100);
        player.unit_mut().set_health(health);
        player.unit_mut().set_death_state(death_state);
        player.unit().health_state_revision_like_cpp()
    };
    session.set_canonical_map_manager(canonical);
    revision
}
