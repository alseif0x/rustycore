// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1444 H2: `world.update` on the booted canonical map loop.
//!
//! AzerothCore `World::Update` ends with `OnWorldUpdate(diff)`
//! (`World.cpp:1342`), after the maps. RustyCore dispatches it once per
//! completed canonical tick, after the tick released the map manager, the
//! spawn metadata and the respawn mutation-order gate (#787). A module runs
//! there, so it must never find one of them held: this probe tries each guard
//! from inside the callback, on the loop's own task.

use wow_module_api::{
    ModuleDescriptor, ModuleHooks, ModuleRegistry, ModuleVersion, WorldEffects, WorldModule,
    WorldUpdateSnapshot,
};

use super::*;

/// What one `world.update` callback saw: the tick, then whether the map
/// manager, the spawn metadata and the respawn-order gate were free.
type GuardSample = (u64, bool, bool, bool);

struct GuardProbe {
    canonical: wow_world::SharedCanonicalMapManager,
    guards: D3bLoopGuardsLikeCpp,
    seen: Mutex<Vec<GuardSample>>,
}

impl WorldModule for GuardProbe {
    fn on_update(&self, snapshot: &WorldUpdateSnapshot, _effects: &mut WorldEffects<'_>) {
        let sample = (
            snapshot.tick,
            self.canonical.try_lock().is_ok(),
            self.guards.spawn_metadata.try_lock().is_ok(),
            self.guards.respawn_db_mutation_order.try_lock().is_ok(),
        );
        self.seen.lock().unwrap().push(sample);
    }
}

#[tokio::test]
async fn world_update_runs_once_per_tick_with_every_tick_guard_released() {
    let world = d3b_world_like_cpp(96_501);
    let guards = D3bLoopGuardsLikeCpp::default();
    let probe = Arc::new(GuardProbe {
        canonical: Arc::clone(&world.canonical),
        guards: guards.clone(),
        seen: Mutex::new(Vec::new()),
    });
    let mut registry = ModuleRegistry::new();
    registry
        .register_module(
            ModuleDescriptor::new(
                "probe.world",
                ModuleVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                "Guard probe",
            )
            .unwrap(),
            ModuleHooks::new().world(probe.clone()),
        )
        .unwrap();
    let hooks = crate::module_hooks::WorldModuleHooksLikeCpp::new(Arc::new(registry));
    let (handle, _creature_probe) = d3b_spawn_canonical_loop_with_like_cpp(&world, &guards, hooks);

    let started = Instant::now();
    while probe.seen.lock().unwrap().len() < 5 && started.elapsed() < Duration::from_secs(5) {
        tokio::time::sleep(Duration::from_millis(D3B_TICK_MS_LIKE_CPP.into())).await;
    }
    handle.abort();
    let _ = handle.await;

    let seen = probe.seen.lock().unwrap().clone();
    assert!(
        seen.len() >= 5,
        "the loop dispatched world.update: {seen:?}"
    );
    for &(tick, manager_free, metadata_free, respawn_order_free) in &seen {
        assert!(
            manager_free && metadata_free && respawn_order_free,
            "tick {tick}: a tick guard was held during world.update: {seen:?}"
        );
    }
    assert!(
        seen.windows(2).all(|pair| pair[0].0 < pair[1].0),
        "one dispatch per tick, in tick order: {seen:?}"
    );
}
