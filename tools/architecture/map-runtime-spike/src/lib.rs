//! Executable architecture spike for #578/#133.
//!
//! This crate is deliberately outside RustyCore's production workspace. It
//! tests the ownership boundary and backend properties before `hecs` is added
//! to `wow-map`; none of these types are a second production world model.

use std::{collections::HashMap, time::Duration};

use hecs::{Entity, World};

pub type Guid = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectHandle {
    guid: Guid,
    generation: u64,
}

impl ObjectHandle {
    pub const fn guid(self) -> Guid {
        self.guid
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vitals {
    pub health: u32,
    pub max_health: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerState {
    pub name: String,
    pub money: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerBundle {
    pub transform: Transform,
    pub vitals: Vitals,
    pub state: PlayerState,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CreatureRuntime {
    velocity_x: f32,
    regeneration_per_tick: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Identity {
    guid: Guid,
}

#[derive(Debug, Clone, Copy)]
struct PlayerTag;

#[derive(Debug, Clone, Copy)]
struct CreatureTag;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Residence {
    Detached,
    Active(Entity),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OwnerRow {
    generation: u64,
    residence: Residence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidenceView {
    Detached,
    Active,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    EmptyGuid,
    GenerationExhausted,
    StaleHandle,
    NotDetached,
    NotActive,
    MissingEntity,
    WrongObjectKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapPhase {
    DynamicTree,
    SessionUpdate,
    Respawns,
    ObjectUpdate,
    TransportUpdate,
    SendObjectUpdates,
    Scripts,
    Weather,
    PersonalPhases,
    MoveLists,
    RelocationNotifies,
    MapHookAndMetrics,
    MapManagerBarrier,
    FarSpellCallbacks,
    RemoveList,
    GridState,
}

pub const CPP_FRAME_PHASES: [MapPhase; 16] = [
    MapPhase::DynamicTree,
    MapPhase::SessionUpdate,
    MapPhase::Respawns,
    MapPhase::ObjectUpdate,
    MapPhase::TransportUpdate,
    MapPhase::SendObjectUpdates,
    MapPhase::Scripts,
    MapPhase::Weather,
    MapPhase::PersonalPhases,
    MapPhase::MoveLists,
    MapPhase::RelocationNotifies,
    MapPhase::MapHookAndMetrics,
    MapPhase::MapManagerBarrier,
    MapPhase::FarSpellCallbacks,
    MapPhase::RemoveList,
    MapPhase::GridState,
];

#[derive(Debug, Clone, PartialEq)]
pub enum MapOutcome {
    CreatureAdvanced {
        guid: Guid,
        x: f32,
        health: u32,
    },
    DamageApplied {
        attacker: Guid,
        victim: Guid,
        amount: u32,
        health_after: u32,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct FrameOutcome {
    pub phases: Vec<MapPhase>,
    pub effects: Vec<MapOutcome>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapCommand {
    ApplyDamage {
        attacker: ObjectHandle,
        victim: ObjectHandle,
        amount: u32,
    },
}

/// Single-writer core. `World` and `Entity` never cross this boundary.
pub struct HecsMapRuntime {
    world: World,
    by_guid: HashMap<Guid, Entity>,
    owners: HashMap<Guid, OwnerRow>,
    detached_players: HashMap<Guid, PlayerBundle>,
    next_generation: u64,
}

impl Default for HecsMapRuntime {
    fn default() -> Self {
        Self {
            world: World::new(),
            by_guid: HashMap::new(),
            owners: HashMap::new(),
            detached_players: HashMap::new(),
            next_generation: 1,
        }
    }
}

impl HecsMapRuntime {
    pub fn install_detached_player(
        &mut self,
        guid: Guid,
        player: PlayerBundle,
    ) -> Result<ObjectHandle, RuntimeError> {
        if guid == 0 {
            return Err(RuntimeError::EmptyGuid);
        }
        self.retire_guid(guid)?;
        let generation = self.allocate_generation()?;
        self.detached_players.insert(guid, player);
        self.owners.insert(
            guid,
            OwnerRow {
                generation,
                residence: Residence::Detached,
            },
        );
        Ok(ObjectHandle { guid, generation })
    }

    pub fn spawn_creature(
        &mut self,
        guid: Guid,
        transform: Transform,
        vitals: Vitals,
        velocity_x: f32,
        regeneration_per_tick: u32,
    ) -> Result<ObjectHandle, RuntimeError> {
        if guid == 0 {
            return Err(RuntimeError::EmptyGuid);
        }
        self.retire_guid(guid)?;
        let generation = self.allocate_generation()?;
        let entity = self.world.spawn((
            Identity { guid },
            CreatureTag,
            transform,
            vitals,
            CreatureRuntime {
                velocity_x,
                regeneration_per_tick,
            },
        ));
        self.by_guid.insert(guid, entity);
        self.owners.insert(
            guid,
            OwnerRow {
                generation,
                residence: Residence::Active(entity),
            },
        );
        Ok(ObjectHandle { guid, generation })
    }

    pub fn residence(&self, handle: ObjectHandle) -> Option<ResidenceView> {
        self.current_owner(handle)
            .map(|owner| match owner.residence {
                Residence::Detached => ResidenceView::Detached,
                Residence::Active(_) => ResidenceView::Active,
            })
    }

    pub fn attach_player(&mut self, handle: ObjectHandle) -> Result<(), RuntimeError> {
        let owner = self
            .current_owner(handle)
            .ok_or(RuntimeError::StaleHandle)?;
        if owner.residence != Residence::Detached {
            return Err(RuntimeError::NotDetached);
        }
        let player = self
            .detached_players
            .remove(&handle.guid)
            .ok_or(RuntimeError::MissingEntity)?;
        let entity = self.world.spawn((
            Identity { guid: handle.guid },
            PlayerTag,
            player.transform,
            player.vitals,
            player.state,
        ));
        self.by_guid.insert(handle.guid, entity);
        self.owners
            .get_mut(&handle.guid)
            .expect("a current handle retains its owner row")
            .residence = Residence::Active(entity);
        Ok(())
    }

    pub fn detach_player(&mut self, handle: ObjectHandle) -> Result<(), RuntimeError> {
        let entity = match self
            .current_owner(handle)
            .ok_or(RuntimeError::StaleHandle)?
            .residence
        {
            Residence::Detached => return Err(RuntimeError::NotActive),
            Residence::Active(entity) => entity,
        };

        if self.world.get::<&PlayerTag>(entity).is_err() {
            return Err(RuntimeError::WrongObjectKind);
        }
        let player = PlayerBundle {
            transform: *self
                .world
                .get::<&Transform>(entity)
                .map_err(|_| RuntimeError::MissingEntity)?,
            vitals: *self
                .world
                .get::<&Vitals>(entity)
                .map_err(|_| RuntimeError::MissingEntity)?,
            state: (*self
                .world
                .get::<&PlayerState>(entity)
                .map_err(|_| RuntimeError::MissingEntity)?)
            .clone(),
        };
        self.world
            .despawn(entity)
            .map_err(|_| RuntimeError::MissingEntity)?;
        self.by_guid.remove(&handle.guid);
        self.detached_players.insert(handle.guid, player);
        self.owners
            .get_mut(&handle.guid)
            .expect("a current handle retains its owner row")
            .residence = Residence::Detached;
        Ok(())
    }

    pub fn player_snapshot(&self, handle: ObjectHandle) -> Option<PlayerBundle> {
        match self.current_owner(handle)?.residence {
            Residence::Detached => self.detached_players.get(&handle.guid).cloned(),
            Residence::Active(entity) => {
                self.world.get::<&PlayerTag>(entity).ok()?;
                Some(PlayerBundle {
                    transform: *self.world.get::<&Transform>(entity).ok()?,
                    vitals: *self.world.get::<&Vitals>(entity).ok()?,
                    state: (*self.world.get::<&PlayerState>(entity).ok()?).clone(),
                })
            }
        }
    }

    pub fn apply_command(&mut self, command: MapCommand) -> Result<MapOutcome, RuntimeError> {
        match command {
            MapCommand::ApplyDamage {
                attacker,
                victim,
                amount,
            } => {
                self.active_entity(attacker)?;
                let victim_entity = self.active_entity(victim)?;
                let health_after = {
                    let mut vitals = self
                        .world
                        .get::<&mut Vitals>(victim_entity)
                        .map_err(|_| RuntimeError::MissingEntity)?;
                    vitals.health = vitals.health.saturating_sub(amount);
                    vitals.health
                };
                Ok(MapOutcome::DamageApplied {
                    attacker: attacker.guid,
                    victim: victim.guid,
                    amount,
                    health_after,
                })
            }
        }
    }

    pub fn frame(&mut self) -> FrameOutcome {
        let mut effects = Vec::new();
        for phase in CPP_FRAME_PHASES {
            if phase == MapPhase::ObjectUpdate {
                effects.extend(self.update_creatures());
            }
        }
        FrameOutcome {
            phases: CPP_FRAME_PHASES.to_vec(),
            effects,
        }
    }

    pub fn semantic_checksum(&self) -> u128 {
        let mut rows = self
            .world
            .query::<(&Identity, &Transform, &Vitals, &CreatureTag)>()
            .iter()
            .map(|(identity, transform, vitals, _)| {
                (
                    identity.guid,
                    transform.x.to_bits(),
                    vitals.health,
                    vitals.max_health,
                )
            })
            .collect::<Vec<_>>();
        rows.sort_unstable_by_key(|row| row.0);
        checksum_rows(rows)
    }

    fn update_creatures(&mut self) -> Vec<MapOutcome> {
        let mut effects = self
            .world
            .query_mut::<(
                &Identity,
                &mut Transform,
                &mut Vitals,
                &CreatureRuntime,
                &CreatureTag,
            )>()
            .into_iter()
            .map(|(identity, transform, vitals, runtime, _)| {
                transform.x += runtime.velocity_x;
                vitals.health = vitals
                    .health
                    .saturating_add(runtime.regeneration_per_tick)
                    .min(vitals.max_health);
                MapOutcome::CreatureAdvanced {
                    guid: identity.guid,
                    x: transform.x,
                    health: vitals.health,
                }
            })
            .collect::<Vec<_>>();
        effects.sort_unstable_by_key(|effect| match effect {
            MapOutcome::CreatureAdvanced { guid, .. } => *guid,
            MapOutcome::DamageApplied { .. } => unreachable!(),
        });
        effects
    }

    fn current_owner(&self, handle: ObjectHandle) -> Option<OwnerRow> {
        self.owners
            .get(&handle.guid)
            .copied()
            .filter(|owner| owner.generation == handle.generation)
    }

    fn active_entity(&self, handle: ObjectHandle) -> Result<Entity, RuntimeError> {
        match self
            .current_owner(handle)
            .ok_or(RuntimeError::StaleHandle)?
            .residence
        {
            Residence::Detached => Err(RuntimeError::NotActive),
            Residence::Active(entity) if self.by_guid.get(&handle.guid) == Some(&entity) => {
                Ok(entity)
            }
            Residence::Active(_) => Err(RuntimeError::MissingEntity),
        }
    }

    fn allocate_generation(&mut self) -> Result<u64, RuntimeError> {
        let generation = self.next_generation;
        self.next_generation = generation
            .checked_add(1)
            .ok_or(RuntimeError::GenerationExhausted)?;
        Ok(generation)
    }

    fn retire_guid(&mut self, guid: Guid) -> Result<(), RuntimeError> {
        let Some(owner) = self.owners.get(&guid).copied() else {
            return Ok(());
        };
        match owner.residence {
            Residence::Detached => {
                self.detached_players
                    .remove(&guid)
                    .ok_or(RuntimeError::MissingEntity)?;
            }
            Residence::Active(entity) => {
                self.world
                    .despawn(entity)
                    .map_err(|_| RuntimeError::MissingEntity)?;
                self.by_guid.remove(&guid);
            }
        }
        self.owners.remove(&guid);
        Ok(())
    }
}

#[derive(Default)]
struct HashMapBaseline {
    creatures: HashMap<Guid, (Transform, Vitals, CreatureRuntime)>,
}

impl HashMapBaseline {
    fn insert(
        &mut self,
        guid: Guid,
        transform: Transform,
        vitals: Vitals,
        runtime: CreatureRuntime,
    ) {
        self.creatures.insert(guid, (transform, vitals, runtime));
    }

    fn update_creatures(&mut self) -> Vec<MapOutcome> {
        let mut effects = self
            .creatures
            .iter_mut()
            .map(|(guid, (transform, vitals, runtime))| {
                transform.x += runtime.velocity_x;
                vitals.health = vitals
                    .health
                    .saturating_add(runtime.regeneration_per_tick)
                    .min(vitals.max_health);
                MapOutcome::CreatureAdvanced {
                    guid: *guid,
                    x: transform.x,
                    health: vitals.health,
                }
            })
            .collect::<Vec<_>>();
        effects.sort_unstable_by_key(|effect| match effect {
            MapOutcome::CreatureAdvanced { guid, .. } => *guid,
            MapOutcome::DamageApplied { .. } => unreachable!(),
        });
        effects
    }

    fn semantic_checksum(&self) -> u128 {
        let mut rows = self
            .creatures
            .iter()
            .map(|(guid, (transform, vitals, _))| {
                (
                    *guid,
                    transform.x.to_bits(),
                    vitals.health,
                    vitals.max_health,
                )
            })
            .collect::<Vec<_>>();
        rows.sort_unstable_by_key(|row| row.0);
        checksum_rows(rows)
    }
}

fn checksum_rows(rows: Vec<(Guid, u32, u32, u32)>) -> u128 {
    rows.into_iter().fold(0_u128, |checksum, row| {
        checksum
            .wrapping_mul(0x100000001b3)
            .wrapping_add(u128::from(row.0))
            .wrapping_add(u128::from(row.1) << 32)
            .wrapping_add(u128::from(row.2) << 64)
            .wrapping_add(u128::from(row.3) << 96)
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkResult {
    pub architecture: &'static str,
    pub objects: usize,
    pub frames: usize,
    pub elapsed: Duration,
    pub checksum: u128,
    pub resident_kib: Option<u64>,
    pub peak_resident_kib: Option<u64>,
}

pub fn benchmark_hash_map(objects: usize, frames: usize) -> BenchmarkResult {
    let mut hash_map = HashMapBaseline::default();
    for index in 0..objects {
        let guid = index as Guid + 1;
        let transform = Transform {
            x: index as f32,
            y: (index % 100) as f32,
            z: 0.0,
        };
        let vitals = Vitals {
            health: 500,
            max_health: 1_000,
        };
        let runtime = CreatureRuntime {
            velocity_x: 0.25,
            regeneration_per_tick: 1,
        };
        hash_map.insert(guid, transform, vitals, runtime);
    }

    let started = std::time::Instant::now();
    for _ in 0..frames {
        std::hint::black_box(hash_map.update_creatures());
    }
    let elapsed = started.elapsed();
    let (resident_kib, peak_resident_kib) = linux_memory_status_kib();
    BenchmarkResult {
        architecture: "HashMap<ObjectGuid, record>",
        objects,
        frames,
        elapsed,
        checksum: hash_map.semantic_checksum(),
        resident_kib,
        peak_resident_kib,
    }
}

pub fn benchmark_hecs(objects: usize, frames: usize) -> BenchmarkResult {
    let mut hecs = HecsMapRuntime::default();
    for index in 0..objects {
        let guid = index as Guid + 1;
        let transform = Transform {
            x: index as f32,
            y: (index % 100) as f32,
            z: 0.0,
        };
        let vitals = Vitals {
            health: 500,
            max_health: 1_000,
        };
        hecs.spawn_creature(guid, transform, vitals, 0.25, 1)
            .expect("benchmark GUIDs are valid and unique");
    }

    let started = std::time::Instant::now();
    for _ in 0..frames {
        std::hint::black_box(hecs.update_creatures());
    }
    let elapsed = started.elapsed();
    let (resident_kib, peak_resident_kib) = linux_memory_status_kib();
    BenchmarkResult {
        architecture: "hecs private EntityWorld",
        objects,
        frames,
        elapsed,
        checksum: hecs.semantic_checksum(),
        resident_kib,
        peak_resident_kib,
    }
}

pub fn compare_backends(objects: usize, frames: usize) -> [BenchmarkResult; 2] {
    [
        benchmark_hash_map(objects, frames),
        benchmark_hecs(objects, frames),
    ]
}

fn linux_memory_status_kib() -> (Option<u64>, Option<u64>) {
    let Ok(status) = std::fs::read_to_string("/proc/self/status") else {
        return (None, None);
    };
    let parse = |name: &str| {
        status.lines().find_map(|line| {
            let value = line.strip_prefix(name)?.trim();
            value.strip_suffix(" kB")?.trim().parse().ok()
        })
    };
    (parse("VmRSS:"), parse("VmHWM:"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player(name: &str, money: u64) -> PlayerBundle {
        PlayerBundle {
            transform: Transform {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            vitals: Vitals {
                health: 90,
                max_health: 100,
            },
            state: PlayerState {
                name: name.to_owned(),
                money,
            },
        }
    }

    #[test]
    fn one_player_value_moves_active_detached_active_without_defaults() {
        let mut runtime = HecsMapRuntime::default();
        let handle = runtime
            .install_detached_player(1, player("Thrall", 42))
            .unwrap();

        assert_eq!(runtime.residence(handle), Some(ResidenceView::Detached));
        runtime.attach_player(handle).unwrap();
        assert_eq!(runtime.residence(handle), Some(ResidenceView::Active));
        runtime.detach_player(handle).unwrap();
        assert_eq!(runtime.residence(handle), Some(ResidenceView::Detached));
        assert_eq!(runtime.player_snapshot(handle), Some(player("Thrall", 42)));
        runtime.attach_player(handle).unwrap();
        assert_eq!(runtime.player_snapshot(handle), Some(player("Thrall", 42)));
    }

    #[test]
    fn replacement_invalidates_stale_generation() {
        let mut runtime = HecsMapRuntime::default();
        let stale = runtime
            .install_detached_player(2, player("old", 1))
            .unwrap();
        let current = runtime
            .install_detached_player(2, player("new", 2))
            .unwrap();

        assert_ne!(stale.generation(), current.generation());
        assert_eq!(runtime.player_snapshot(stale), None);
        assert_eq!(runtime.residence(stale), None);
        assert_eq!(runtime.player_snapshot(current), Some(player("new", 2)));
    }

    #[test]
    fn detached_target_fails_closed_instead_of_fabricating_state() {
        let mut runtime = HecsMapRuntime::default();
        let player = runtime
            .install_detached_player(3, player("detached", 3))
            .unwrap();
        let creature = runtime
            .spawn_creature(
                4,
                Transform {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Vitals {
                    health: 50,
                    max_health: 50,
                },
                0.0,
                0,
            )
            .unwrap();

        assert_eq!(
            runtime.apply_command(MapCommand::ApplyDamage {
                attacker: creature,
                victim: player,
                amount: 10,
            }),
            Err(RuntimeError::NotActive)
        );
        assert_eq!(runtime.player_snapshot(player).unwrap().vitals.health, 90);
    }

    #[test]
    fn creature_batch_updates_transform_and_vitals_in_guid_order() {
        let mut runtime = HecsMapRuntime::default();
        for guid in [12, 10, 11] {
            runtime
                .spawn_creature(
                    guid,
                    Transform {
                        x: guid as f32,
                        y: 0.0,
                        z: 0.0,
                    },
                    Vitals {
                        health: 40,
                        max_health: 50,
                    },
                    0.5,
                    2,
                )
                .unwrap();
        }

        let frame = runtime.frame();
        let observed = frame
            .effects
            .into_iter()
            .map(|effect| match effect {
                MapOutcome::CreatureAdvanced { guid, x, health } => (guid, x, health),
                MapOutcome::DamageApplied { .. } => unreachable!(),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            observed,
            vec![(10, 10.5, 42), (11, 11.5, 42), (12, 12.5, 42)]
        );
    }

    #[test]
    fn combat_outcome_is_owned_and_survives_later_world_mutation() {
        let mut runtime = HecsMapRuntime::default();
        let player = runtime
            .install_detached_player(20, player("target", 0))
            .unwrap();
        runtime.attach_player(player).unwrap();
        let creature = runtime
            .spawn_creature(
                21,
                Transform {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Vitals {
                    health: 50,
                    max_health: 50,
                },
                0.0,
                0,
            )
            .unwrap();

        let outcome = runtime
            .apply_command(MapCommand::ApplyDamage {
                attacker: creature,
                victim: player,
                amount: 7,
            })
            .unwrap();
        runtime.detach_player(player).unwrap();

        assert_eq!(
            outcome,
            MapOutcome::DamageApplied {
                attacker: 21,
                victim: 20,
                amount: 7,
                health_after: 83,
            }
        );
        assert_eq!(runtime.player_snapshot(player).unwrap().vitals.health, 83);
    }

    #[test]
    fn phase_trace_preserves_cpp_map_update_and_delayed_update_barrier() {
        let frame = HecsMapRuntime::default().frame();
        assert_eq!(frame.phases, CPP_FRAME_PHASES);
        assert_eq!(
            frame
                .phases
                .iter()
                .position(|phase| *phase == MapPhase::MapManagerBarrier),
            Some(12)
        );
        assert_eq!(
            frame
                .phases
                .iter()
                .position(|phase| *phase == MapPhase::FarSpellCallbacks),
            Some(13)
        );
    }

    #[test]
    fn guid_index_survives_archetype_relocation_and_is_removed_with_entity() {
        #[derive(Debug, Clone, Copy)]
        struct TemporaryComponent;

        let mut runtime = HecsMapRuntime::default();
        let creature = runtime
            .spawn_creature(
                30,
                Transform {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                },
                Vitals {
                    health: 40,
                    max_health: 50,
                },
                0.0,
                0,
            )
            .unwrap();
        let entity = runtime.active_entity(creature).unwrap();

        runtime
            .world
            .insert_one(entity, TemporaryComponent)
            .unwrap();
        assert_eq!(runtime.active_entity(creature), Ok(entity));
        assert!(runtime.world.get::<&TemporaryComponent>(entity).is_ok());

        runtime.retire_guid(creature.guid()).unwrap();
        assert_eq!(
            runtime.active_entity(creature),
            Err(RuntimeError::StaleHandle)
        );
        assert!(!runtime.by_guid.contains_key(&creature.guid()));
        assert!(!runtime.world.contains(entity));
    }

    #[test]
    fn candidate_and_baseline_have_the_same_semantic_checksum() {
        let [baseline, candidate] = compare_backends(2_000, 20);
        assert_eq!(baseline.checksum, candidate.checksum);
    }
}
