// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `player.creature_kill` (#1444 H3) through a real kill: TrinityCore
//! `Unit::Kill` ends with `OnCreatureKill(killer, creature)`
//! (`Unit.cpp:10755`), once per `setDeathState(JUST_DIED)`.

use super::*;

struct KillProbe {
    map: SharedCanonicalMapManager,
    seen: std::sync::Mutex<Vec<(ObjectGuid, u32, u8, u32, bool)>>,
}

impl wow_module_api::PlayerModule for KillProbe {
    fn on_creature_kill(
        &self,
        s: &wow_module_api::CreatureKillSnapshot,
        e: &mut wow_module_api::ScopedEffects<'_>,
    ) {
        let map_free = self.map.try_lock().is_ok();
        self.seen.lock().unwrap().push((
            s.creature,
            s.creature_entry,
            s.creature_level,
            s.map_id,
            map_free,
        ));
        e.send_system_message_self("kill");
    }
}

fn install_kill_probe(
    session: &mut WorldSession,
    map: SharedCanonicalMapManager,
) -> Arc<KillProbe> {
    let probe = Arc::new(KillProbe {
        map,
        seen: std::sync::Mutex::new(Vec::new()),
    });
    let mut registry = wow_module_api::ModuleRegistry::new();
    registry
        .register_module(
            wow_module_api::ModuleDescriptor::new(
                "alpha.kills",
                wow_module_api::ModuleVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                "Kills",
            )
            .unwrap(),
            wow_module_api::ModuleHooks::new().player(Arc::clone(&probe) as _),
        )
        .unwrap();
    session.install_module_hooks_like_cpp(Arc::new(registry));
    probe
}

#[tokio::test]
async fn a_player_kill_raises_creature_kill_once_with_no_map_lock() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(18_101);
    let player = ObjectGuid::create_player(1, 4801);
    session.core.player_guid = Some(player);
    let manager = session
        .core
        .canonical_map_manager
        .clone()
        .unwrap_or_else(|| manager.clone());
    register_test_creature(&mut session, manager.clone(), guid, 40);
    let probe = install_kill_probe(&mut session, manager.clone());
    let (entry, level, map_id) = session
        .mutate_world_creature(guid, |creature| {
            (creature.entry(), creature.level(), creature.map_id())
        })
        .expect("live creature");

    session.apply_damage(None, guid, 100).await.unwrap();
    // A second blow on the corpse is no second `JUST_DIED`.
    let _ = session.apply_damage(None, guid, 100).await;

    assert_eq!(
        *probe.seen.lock().unwrap(),
        vec![(guid, entry, level, map_id, true)],
        "one kill, reported after the map lock of the death transition was released"
    );
}

#[tokio::test]
async fn damage_that_does_not_kill_raises_no_creature_kill() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(18_102);
    session.core.player_guid = Some(ObjectGuid::create_player(1, 4802));
    let manager = session
        .core
        .canonical_map_manager
        .clone()
        .unwrap_or_else(|| manager.clone());
    register_test_creature(&mut session, manager.clone(), guid, 400);
    let probe = install_kill_probe(&mut session, manager);

    session.apply_damage(None, guid, 10).await.unwrap();

    assert!(probe.seen.lock().unwrap().is_empty());
}
