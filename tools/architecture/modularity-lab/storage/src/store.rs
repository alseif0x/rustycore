//! Private interchangeable stores. This is a LAB model, not a production entity port.
use std::collections::{BTreeSet, HashMap};

pub type Guid = u64;

/// Deliberately NOT Clone: detach must transfer this allocation, not reproduce equal values.
#[derive(Debug)]
pub struct Core {
    pub guid: Guid,
    pub payload: Box<[u8; 128]>,
    pub x: i64,
    pub health: u64,
    pub victim: Option<Guid>,
    pub attackers: BTreeSet<Guid>,
    pub revision: u64,
}

impl Core {
    pub fn new(guid: Guid) -> Self {
        Self {
            guid,
            payload: Box::new([guid as u8; 128]),
            x: 0,
            health: 750,
            victim: None,
            attackers: BTreeSet::new(),
            revision: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Encounter {
    pub phase: u8,
    pub timer: u32,
    pub shield: bool,
    pub summon: Option<Guid>,
    pub callbacks: u32,
    pub pulses: u64,
}

impl Default for Encounter {
    fn default() -> Self {
        Self {
            phase: 0,
            timer: 5000,
            shield: false,
            summon: None,
            callbacks: 0,
            pulses: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    pub module: u16,
    pub bonus: u32,
}

#[derive(Debug)]
pub struct Bundle {
    pub core: Core,
    pub encounter: Option<Encounter>,
    pub policy: Option<Policy>,
}

impl Bundle {
    pub fn new(guid: Guid, optional: bool) -> Self {
        Self {
            core: Core::new(guid),
            encounter: optional.then(Encounter::default),
            policy: optional.then_some(Policy {
                module: 2,
                bonus: 3,
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub guid: Guid,
    pub x: i64,
    pub health: u64,
    pub revision: u64,
    pub victim: Option<Guid>,
    pub attackers: Vec<Guid>,
    pub encounter: Option<Encounter>,
    pub policy: Option<Policy>,
    pub payload: [u8; 128],
}

fn row(core: &Core, encounter: Option<Encounter>, policy: Option<Policy>) -> Row {
    Row {
        guid: core.guid,
        x: core.x,
        health: core.health,
        revision: core.revision,
        victim: core.victim,
        attackers: core.attackers.iter().copied().collect(),
        encounter,
        policy,
        payload: *core.payload,
    }
}

/// Shared arithmetic; neither backend is allowed its own gameplay algorithm.
fn advance(core: &mut Core, encounter: Option<&mut Encounter>, policy: Option<&Policy>) {
    let bonus = policy.map_or(0, |p| p.bonus as i64);
    core.x = core.x.wrapping_add(1 + bonus);
    if core.health != 0 {
        core.health = (core.health + 1).min(1000);
    }
    core.revision += 1;
    // LAB-only optional pulse behavior, separate from the C++-anchored AI timer.
    // Both state and output participate; this is not a marker-only ECS query.
    if let Some(state) = encounter {
        state.pulses += 1;
        if state.pulses.is_multiple_of(8) {
            core.x = core.x.wrapping_add(2);
        }
    }
}

pub trait Store: Default {
    const NAME: &'static str;
    fn insert(&mut self, bundle: Bundle);
    fn remove(&mut self, guid: Guid) -> Option<Bundle>;
    fn read<R>(&self, guid: Guid, f: impl FnOnce(&Core) -> R) -> Option<R>;
    fn write<R>(&mut self, guid: Guid, f: impl FnOnce(&mut Core) -> R) -> Option<R>;
    fn encounter(&self, guid: Guid) -> Option<Encounter>;
    fn encounter_write<R>(&mut self, guid: Guid, f: impl FnOnce(&mut Encounter) -> R) -> Option<R>;
    fn set_optional(&mut self, guid: Guid, enabled: bool);
    fn policy(&self, guid: Guid) -> Option<Policy>;
    fn insert_policy(&mut self, guid: Guid, policy: Policy);
    fn remove_policy(&mut self, guid: Guid);
    fn advance(&mut self);
    fn rows(&self) -> Vec<Row>;
    fn len(&self) -> usize;
}

#[derive(Default)]
pub struct Aggregate {
    cores: HashMap<Guid, Core>,
    encounters: HashMap<Guid, Encounter>,
    policies: HashMap<Guid, Policy>,
}

impl Store for Aggregate {
    const NAME: &'static str = "aggregate";
    fn insert(&mut self, b: Bundle) {
        let guid = b.core.guid;
        assert!(!self.cores.contains_key(&guid));
        self.cores.insert(guid, b.core);
        if let Some(e) = b.encounter {
            self.encounters.insert(guid, e);
        }
        if let Some(p) = b.policy {
            self.policies.insert(guid, p);
        }
    }
    fn remove(&mut self, guid: Guid) -> Option<Bundle> {
        Some(Bundle {
            core: self.cores.remove(&guid)?,
            encounter: self.encounters.remove(&guid),
            policy: self.policies.remove(&guid),
        })
    }
    fn read<R>(&self, guid: Guid, f: impl FnOnce(&Core) -> R) -> Option<R> {
        self.cores.get(&guid).map(f)
    }
    fn write<R>(&mut self, guid: Guid, f: impl FnOnce(&mut Core) -> R) -> Option<R> {
        self.cores.get_mut(&guid).map(f)
    }
    fn encounter(&self, guid: Guid) -> Option<Encounter> {
        self.encounters.get(&guid).copied()
    }
    fn encounter_write<R>(&mut self, guid: Guid, f: impl FnOnce(&mut Encounter) -> R) -> Option<R> {
        self.encounters.get_mut(&guid).map(f)
    }
    fn set_optional(&mut self, guid: Guid, enabled: bool) {
        assert!(self.cores.contains_key(&guid));
        if enabled {
            self.encounters.insert(guid, Encounter::default());
        } else {
            self.encounters.remove(&guid);
        }
    }
    fn policy(&self, guid: Guid) -> Option<Policy> {
        self.policies.get(&guid).copied()
    }
    fn insert_policy(&mut self, guid: Guid, p: Policy) {
        assert!(self.policies.insert(guid, p).is_none());
    }
    fn remove_policy(&mut self, guid: Guid) {
        self.policies.remove(&guid);
    }
    fn advance(&mut self) {
        for (guid, core) in &mut self.cores {
            advance(core, self.encounters.get_mut(guid), self.policies.get(guid));
        }
    }
    fn rows(&self) -> Vec<Row> {
        self.cores
            .values()
            .map(|c| row(c, self.encounter(c.guid), self.policy(c.guid)))
            .collect()
    }
    fn len(&self) -> usize {
        self.cores.len()
    }
}

#[derive(Default)]
pub struct Ecs {
    world: hecs::World,
    by_guid: HashMap<Guid, hecs::Entity>,
}

impl Store for Ecs {
    const NAME: &'static str = "hecs";
    fn insert(&mut self, b: Bundle) {
        let guid = b.core.guid;
        assert!(!self.by_guid.contains_key(&guid));
        let e = match (b.encounter, b.policy) {
            (Some(s), Some(p)) => self.world.spawn((b.core, s, p)),
            (Some(s), None) => self.world.spawn((b.core, s)),
            (None, Some(p)) => self.world.spawn((b.core, p)),
            (None, None) => self.world.spawn((b.core,)),
        };
        self.by_guid.insert(guid, e);
    }
    fn remove(&mut self, guid: Guid) -> Option<Bundle> {
        let e = self.by_guid.remove(&guid)?;
        let has_encounter = self.world.get::<&Encounter>(e).is_ok();
        let has_policy = self.world.get::<&Policy>(e).is_ok();
        let (core, encounter, policy) = match (has_encounter, has_policy) {
            (true, true) => {
                let (c, s, p) = self.world.remove::<(Core, Encounter, Policy)>(e).unwrap();
                (c, Some(s), Some(p))
            }
            (true, false) => {
                let (c, s) = self.world.remove::<(Core, Encounter)>(e).unwrap();
                (c, Some(s), None)
            }
            (false, true) => {
                let (c, p) = self.world.remove::<(Core, Policy)>(e).unwrap();
                (c, None, Some(p))
            }
            (false, false) => (self.world.remove_one::<Core>(e).unwrap(), None, None),
        };
        self.world.despawn(e).unwrap();
        Some(Bundle {
            core,
            encounter,
            policy,
        })
    }
    fn read<R>(&self, guid: Guid, f: impl FnOnce(&Core) -> R) -> Option<R> {
        Some(f(&*self
            .world
            .get::<&Core>(*self.by_guid.get(&guid)?)
            .ok()?))
    }
    fn write<R>(&mut self, guid: Guid, f: impl FnOnce(&mut Core) -> R) -> Option<R> {
        Some(f(self
            .world
            .query_one_mut::<&mut Core>(*self.by_guid.get(&guid)?)
            .ok()?))
    }
    fn encounter(&self, guid: Guid) -> Option<Encounter> {
        Some(
            *self
                .world
                .get::<&Encounter>(*self.by_guid.get(&guid)?)
                .ok()?,
        )
    }
    fn encounter_write<R>(&mut self, guid: Guid, f: impl FnOnce(&mut Encounter) -> R) -> Option<R> {
        Some(f(self
            .world
            .query_one_mut::<&mut Encounter>(*self.by_guid.get(&guid)?)
            .ok()?))
    }
    fn set_optional(&mut self, guid: Guid, enabled: bool) {
        let e = self.by_guid[&guid];
        if enabled {
            self.world.insert_one(e, Encounter::default()).unwrap();
        } else {
            let _ = self.world.remove_one::<Encounter>(e);
        }
    }
    fn policy(&self, guid: Guid) -> Option<Policy> {
        Some(*self.world.get::<&Policy>(*self.by_guid.get(&guid)?).ok()?)
    }
    fn insert_policy(&mut self, guid: Guid, p: Policy) {
        assert!(self.policy(guid).is_none());
        self.world.insert_one(self.by_guid[&guid], p).unwrap();
    }
    fn remove_policy(&mut self, guid: Guid) {
        let _ = self.world.remove_one::<Policy>(self.by_guid[&guid]);
    }
    fn advance(&mut self) {
        for (c, e, p) in self
            .world
            .query_mut::<(&mut Core, Option<&mut Encounter>, Option<&Policy>)>()
        {
            advance(c, e, p);
        }
    }
    fn rows(&self) -> Vec<Row> {
        self.world
            .query::<(&Core, Option<&Encounter>, Option<&Policy>)>()
            .iter()
            .map(|(c, e, p)| row(c, e.copied(), p.copied()))
            .collect()
    }
    fn len(&self) -> usize {
        self.by_guid.len()
    }
}
