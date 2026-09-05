//! Controlled storage comparison; NOT production AI, physics, protocol or DB durability proof.
//! Both stores execute this exact synchronous owner/operation driver. C++ anchors are local
//! `/home/server/woltk-trinity-legacy/src/server/`:
//! - scripts/Northrend/Nexus/Nexus/boss_anomalus.cpp:131–181 (partial timer/summon flow),
//! - game/Entities/Creature/TemporarySummon.cpp:249–264 (synchronous callbacks),
//! - game/Maps/MapManager.cpp:287–318 (map update barrier before delayed work),
//! - game/Entities/Unit/Unit.cpp:5645–5745 (subset of attack admission/reciprocal links).
//!
//! Payload and optional pulse/policy arithmetic are synthetic workload, not ported gameplay.
mod bench;
mod checks;
mod store;

pub use bench::{BenchResult, Config, benchmark};
pub use checks::{CheckReport, check};
use std::collections::HashMap;
use store::{Bundle, Encounter, Guid, Policy, Row, Store};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Handle {
    guid: Guid,
    generation: u64,
}
#[derive(Debug, Clone, Copy)]
struct Owner {
    generation: u64,
    map: Option<usize>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Error {
    Stale,
    Detached,
    WrongMap,
    Missing,
    Conflict,
    Rejected,
}
type LabResult<T> = Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Event {
    kind: &'static str,
    guid: Guid,
    value: u64,
}

struct Driver<S: Store> {
    maps: [S; 2],
    owners: HashMap<Guid, Owner>,
    detached: HashMap<Guid, Bundle>,
    next_generation: u64,
    next_guid: Guid,
    trace: Vec<Event>,
    deferred: Vec<Handle>,
    pending_attacks: Vec<(Handle, Handle)>,
    completed_maps: u8,
    callback_depth: u8,
}

impl<S: Store> Default for Driver<S> {
    fn default() -> Self {
        Self {
            maps: [S::default(), S::default()],
            owners: HashMap::new(),
            detached: HashMap::new(),
            next_generation: 1,
            next_guid: 1,
            trace: Vec::new(),
            deferred: Vec::new(),
            pending_attacks: Vec::new(),
            completed_maps: 0,
            callback_depth: 0,
        }
    }
}

impl<S: Store> Driver<S> {
    fn record(&mut self, kind: &'static str, guid: Guid, value: u64) {
        self.trace.push(Event { kind, guid, value });
    }
    fn install(&mut self, map: usize, bundle: Bundle) -> Handle {
        let guid = bundle.core.guid;
        assert!(!self.owners.contains_key(&guid));
        let h = Handle {
            guid,
            generation: self.next_generation,
        };
        self.next_generation = self.next_generation.checked_add(1).unwrap();
        self.next_guid = self.next_guid.max(guid.checked_add(1).unwrap());
        self.maps[map].insert(bundle);
        self.owners.insert(
            guid,
            Owner {
                generation: h.generation,
                map: Some(map),
            },
        );
        h
    }
    fn owner(&self, h: Handle) -> LabResult<Owner> {
        self.owners
            .get(&h.guid)
            .copied()
            .filter(|o| o.generation == h.generation)
            .ok_or(Error::Stale)
    }
    fn active(&self, h: Handle) -> LabResult<usize> {
        self.owner(h)?.map.ok_or(Error::Detached)
    }
    fn handle(&self, guid: Guid) -> LabResult<Handle> {
        Ok(Handle {
            guid,
            generation: self.owners.get(&guid).ok_or(Error::Missing)?.generation,
        })
    }
    fn read<R>(&self, h: Handle, f: impl FnOnce(&store::Core) -> R) -> LabResult<R> {
        match self.owner(h)?.map {
            Some(map) => self.maps[map].read(h.guid, f).ok_or(Error::Missing),
            None => self
                .detached
                .get(&h.guid)
                .map(|b| f(&b.core))
                .ok_or(Error::Missing),
        }
    }
    fn write<R>(&mut self, h: Handle, f: impl FnOnce(&mut store::Core) -> R) -> LabResult<R> {
        match self.owner(h)?.map {
            Some(map) => self.maps[map].write(h.guid, f).ok_or(Error::Missing),
            None => self
                .detached
                .get_mut(&h.guid)
                .map(|b| f(&mut b.core))
                .ok_or(Error::Missing),
        }
    }
    fn encounter(&self, h: Handle) -> LabResult<Encounter> {
        self.maps[self.active(h)?]
            .encounter(h.guid)
            .ok_or(Error::Missing)
    }
    fn mutate_encounter<R>(
        &mut self,
        h: Handle,
        f: impl FnOnce(&mut Encounter) -> R,
    ) -> LabResult<R> {
        let map = self.active(h)?;
        self.maps[map]
            .encounter_write(h.guid, f)
            .ok_or(Error::Missing)
    }
    fn optional(&mut self, h: Handle, enabled: bool) -> LabResult<()> {
        let map = self.active(h)?;
        self.maps[map].set_optional(h.guid, enabled);
        if enabled && self.maps[map].policy(h.guid).is_none() {
            self.policy(
                h,
                Policy {
                    module: 2,
                    bonus: 3,
                },
            )?;
        }
        if !enabled {
            self.maps[map].remove_policy(h.guid);
        }
        Ok(())
    }
    fn policy(&mut self, h: Handle, p: Policy) -> LabResult<()> {
        let map = self.active(h)?;
        if self.maps[map].policy(h.guid).is_some() {
            return Err(Error::Conflict);
        }
        self.maps[map].insert_policy(h.guid, p);
        Ok(())
    }
    // Validate both participants before any mutation. Reciprocal links are changed under
    // the same exclusive driver borrow; no callback/publication can see half the operation.
    fn attack(&mut self, a: Handle, b: Handle) -> LabResult<()> {
        let map = self.active(a)?;
        if a == b || self.active(b)? != map {
            return Err(Error::WrongMap);
        }
        let (alive, old) = self.read(a, |c| (c.health != 0, c.victim))?;
        if !alive || !self.read(b, |c| c.health != 0)? {
            return Err(Error::Rejected);
        }
        if let Some(old) = old {
            self.maps[map].write(old, |c| {
                c.attackers.remove(&a.guid);
            });
        }
        self.write(a, |c| {
            c.victim = Some(b.guid);
            c.revision += 1;
        })?;
        self.write(b, |c| {
            c.attackers.insert(a.guid);
            c.revision += 1;
        })?;
        self.record("attack", a.guid, b.guid);
        Ok(())
    }
    fn stop(&mut self, h: Handle) -> LabResult<()> {
        let (victim, attackers) = self.read(h, |c| {
            (c.victim, c.attackers.iter().copied().collect::<Vec<_>>())
        })?;
        if let Some(victim) = victim.and_then(|g| self.handle(g).ok()) {
            self.write(victim, |c| {
                c.attackers.remove(&h.guid);
            })?;
        }
        for guid in attackers {
            if let Ok(a) = self.handle(guid) {
                self.write(a, |c| c.victim = None)?;
            }
        }
        self.write(h, |c| {
            c.victim = None;
            c.attackers.clear();
        })?;
        Ok(())
    }
    fn detach(&mut self, h: Handle) -> LabResult<()> {
        let map = self.active(h)?;
        self.stop(h)?;
        let bundle = self.maps[map].remove(h.guid).ok_or(Error::Missing)?;
        assert!(self.detached.insert(h.guid, bundle).is_none());
        self.owners.get_mut(&h.guid).unwrap().map = None;
        Ok(())
    }
    fn attach(&mut self, h: Handle, map: usize, reject: bool) -> LabResult<()> {
        if self.owner(h)?.map.is_some() {
            return Err(Error::WrongMap);
        }
        if map >= 2 || reject || self.maps[map].read(h.guid, |_| ()).is_some() {
            return Err(Error::Rejected);
        }
        let bundle = self.detached.remove(&h.guid).ok_or(Error::Missing)?;
        self.maps[map].insert(bundle);
        self.owners.get_mut(&h.guid).unwrap().map = Some(map);
        Ok(())
    }
    fn retire(&mut self, h: Handle) -> LabResult<()> {
        if self.owner(h)?.map.is_some() {
            self.detach(h)?;
        }
        self.detached.remove(&h.guid).ok_or(Error::Missing)?;
        self.owners.remove(&h.guid);
        Ok(())
    }
    fn reset(&mut self, h: Handle) -> LabResult<()> {
        self.mutate_encounter(h, |s| *s = Encounter::default())?;
        self.record("reset", h.guid, 0);
        Ok(())
    }
    fn summoned_callback(&mut self, parent: Handle, child: Handle) -> LabResult<()> {
        if self.callback_depth >= 4 {
            return Err(Error::Rejected);
        }
        self.callback_depth += 1;
        let result = (|| {
            self.mutate_encounter(parent, |s| s.callbacks += 1)?;
            // LAB extension action: a real structural insertion inside the callback, after
            // releasing the parent's borrow. Not an Anomalus gameplay claim.
            self.policy(
                child,
                Policy {
                    module: 7,
                    bonus: 1,
                },
            )?;
            let child_map = self.active(child)?;
            let bonus = self.maps[child_map]
                .policy(child.guid)
                .ok_or(Error::Missing)?
                .bonus;
            self.record("callback_policy", child.guid, bonus.into());
            let observed = self.encounter(parent)?.callbacks;
            self.record("summoned_callback", parent.guid, observed.into());
            self.write(child, |c| c.revision += 1)?;
            Ok(())
        })();
        // Recoverable capability errors restore bookkeeping, but do not roll back already
        // completed actions. This is not panic isolation or a recursive callback stress test.
        self.callback_depth -= 1;
        result
    }
    /// Partial Anomalus control flow only; cast is a LAB shield transition, not Spell::Cast.
    /// C++ boss_anomalus.cpp:131–181: early returns pause timer, phase/shield survive failure.
    fn encounter_step(
        &mut self,
        h: Handle,
        diff: u32,
        fail_summon: bool,
    ) -> LabResult<Option<Handle>> {
        let map = self.active(h)?;
        let (health, target) = self.read(h, |c| (c.health, c.victim))?;
        let Some(target) = target else {
            return Ok(None);
        };
        if health == 0 {
            return Ok(None);
        }
        let state = self.encounter(h)?;
        if state.shield && state.summon.is_some() {
            return Ok(None);
        }
        let mut summoned = None;
        if state.phase == 0 && health < 500 {
            self.mutate_encounter(h, |s| {
                s.phase = 1;
                s.shield = true;
            })?;
            self.record("shield", h.guid, 1);
            if fail_summon {
                self.record("summon_failed", h.guid, 0);
            } else {
                let child = self.install(map, Bundle::new(self.next_guid, false));
                self.summoned_callback(h, child)?;
                self.record("summon_return", h.guid, child.guid);
                if let Ok(target) = self.handle(target) {
                    self.attack(child, target)?;
                }
                let observed = self.encounter(h)?.callbacks;
                self.record("read_after_callback", h.guid, observed.into());
                self.mutate_encounter(h, |s| s.summon = Some(child.guid))?;
                summoned = Some(child);
            }
        }
        self.mutate_encounter(h, |s| {
            s.timer = if s.timer <= diff {
                5000
            } else {
                s.timer - diff
            }
        })?;
        Ok(summoned)
    }
    /// Invocations produce the trace; there is no static expected-phase vector in this path.
    /// This models the barrier's ordering, NOT C++'s complete per-cell scheduling semantics.
    fn frame(&mut self, focus: &[(Handle, Handle)], fail_summon: bool) -> LabResult<()> {
        self.completed_maps = 0;
        let pending = std::mem::take(&mut self.pending_attacks);
        for map in 0..2 {
            self.record("admission", map as u64, 0);
            for &(a, b) in &pending {
                if self.active(a)? == map {
                    self.attack(a, b)?;
                }
            }
            self.maps[map].advance();
            for &(boss, target) in focus {
                if self.active(boss)? != map {
                    continue;
                }
                self.reset(boss)?;
                self.write(boss, |c| c.health = 400)?;
                self.attack(boss, target)?;
                if let Some(child) = self.encounter_step(boss, 10, fail_summon)? {
                    self.deferred.push(child);
                }
            }
            self.completed_maps |= 1 << map;
            self.record("objects_done", map as u64, self.maps[map].len() as u64);
        }
        assert_eq!(self.completed_maps, 3);
        self.record("barrier", 0, self.completed_maps.into());
        for h in std::mem::take(&mut self.deferred) {
            let alive = self.read(h, |c| c.health != 0)?;
            self.record("far_callback_before_remove", h.guid, alive.into());
            self.retire(h)?;
            self.record("removed", h.guid, 0);
        }
        Ok(())
    }
    fn rows(&self) -> Vec<Row> {
        self.maps.iter().flat_map(Store::rows).collect()
    }
}

#[cfg(test)]
mod tests;
