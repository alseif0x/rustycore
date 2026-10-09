// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The frozen admission of one admitted transition and its fence.

use super::*;

/// One map incarnation admitted for this transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AdmittedCreatureExecutionMapLikeCpp {
    pub map_id: u32,
    pub instance_id: u32,
    pub incarnation: u64,
}

/// One selected object of the admitted tick, with its incarnation and the
/// identity of the live canonical record the admission froze.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AdmittedCreatureExecutionObjectLikeCpp {
    pub map_id: u32,
    pub instance_id: u32,
    pub incarnation: u64,
    pub creature_guid: ObjectGuid,
    /// Identity of the canonical record instance itself: the address of the
    /// live `Creature` the map currently owns for this GUID. A record replaced
    /// inside the same map incarnation is a different allocation.
    pub record_identity: u64,
}

/// The frozen admission of one canonical creature execution transition.
///
/// The diff is the plan's saved effective diff, the maps are the plan's
/// admitted incarnations and the objects are the tick's selected canonical
/// `Creature` records. Nothing here is a live handle: the whole point of the
/// fence is that every entry is re-checked before it is used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedCreatureExecutionLikeCpp {
    pub coordinator_id: u64,
    pub tick_epoch: u64,
    /// The admitted tick's saved diff; the single clock advancement of the
    /// transition.
    pub diff_ms: u32,
    pub game_time_secs: i64,
    pub maps: Vec<AdmittedCreatureExecutionMapLikeCpp>,
    pub objects: Vec<AdmittedCreatureExecutionObjectLikeCpp>,
}

/// Why an admission was refused, or that it was admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureExecutionAdmissionLikeCpp {
    Admitted,
    /// The manager is not holding the tick whose diff this admission froze.
    RefusedSessionBarrier {
        expected_epoch: u64,
        current: wow_map::MapTickCoordinationStateLikeCpp,
    },
    /// The map key now resolves to a different incarnation: the map was
    /// recreated after the admission was captured.
    RefusedMapIncarnation {
        map_id: u32,
        instance_id: u32,
        expected: u64,
        current: Option<u64>,
    },
    /// The selected object is gone from the admitted incarnation.
    RefusedObjectMissing {
        map_id: u32,
        instance_id: u32,
        creature_guid: ObjectGuid,
    },
    /// The GUID resolves to a different record allocation than the admitted one.
    RefusedObjectReplaced {
        map_id: u32,
        instance_id: u32,
        creature_guid: ObjectGuid,
        expected: u64,
        current: u64,
    },
    /// Another owner already holds this transition.
    RefusedOwnerLease {
        held_by: CreatureExecutionOwnerLikeCpp,
    },
}

impl CreatureExecutionAdmissionLikeCpp {
    #[must_use]
    pub const fn is_admitted_like_cpp(self) -> bool {
        matches!(self, Self::Admitted)
    }

    #[must_use]
    pub const fn refusal_label_like_cpp(self) -> Option<&'static str> {
        match self {
            Self::Admitted => None,
            Self::RefusedSessionBarrier { .. } => Some("session-barrier"),
            Self::RefusedMapIncarnation { .. } => Some("map-incarnation"),
            Self::RefusedObjectMissing { .. } => Some("object-missing"),
            Self::RefusedObjectReplaced { .. } => Some("object-replaced"),
            Self::RefusedOwnerLease { .. } => Some("owner-lease"),
        }
    }
}

/// The typed outcome of one isolated admitted creature execution.
#[derive(Debug, Clone)]
pub struct IsolatedCreatureExecutionOutcomeLikeCpp {
    pub admission: CreatureExecutionAdmissionLikeCpp,
    pub owner: CreatureExecutionOwnerLikeCpp,
    /// The one clock advancement of the transition: the saved diff applied to
    /// every admitted creature that executed.
    pub clock_advanced_ms: u64,
    /// Due `CombatAI::_events` slots the engine consumed, one per creature.
    pub due_casts: usize,
    /// Damage committed by the one admitted swing of the creature.
    pub swing_damage: u64,
    /// Canonical effects applied inside the execution window.
    pub effects_consumed: usize,
    /// The deferred publication of the transition. It is returned to the
    /// caller and never delivered while the map guard is held.
    pub deferred_publication: Vec<crate::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand>,
    /// The deferred batch was handed to a publication consumer.
    pub publication_delivered: bool,
}

impl IsolatedCreatureExecutionOutcomeLikeCpp {
    pub(in crate::session) fn refused_like_cpp(
        admission: CreatureExecutionAdmissionLikeCpp,
        owner: CreatureExecutionOwnerLikeCpp,
    ) -> Self {
        Self {
            admission,
            owner,
            clock_advanced_ms: 0,
            due_casts: 0,
            swing_damage: 0,
            effects_consumed: 0,
            deferred_publication: Vec::new(),
            publication_delivered: false,
        }
    }

    /// Number of publication events this transition produced.
    #[must_use]
    pub fn publication_events_like_cpp(&self) -> usize {
        self.deferred_publication.len()
    }

    /// Whether the engine actually ran for this transition.
    #[must_use]
    pub const fn executed_like_cpp(&self) -> bool {
        self.admission.is_admitted_like_cpp()
    }
}

/// Capture the admission of one transition under the canonical guard.
///
/// `objects` is the tick's selected object set (the map's `ObjectUpdater`
/// selection for the same diff), already filtered by the caller to the family
/// it owns. `None` means the canonical owner is unreadable: an unreadable owner
/// is not proof of absence, so no admission is captured.
#[must_use]
pub fn capture_admitted_creature_execution_like_cpp(
    canonical_map_manager: &SharedCanonicalMapManager,
    coordinator_id: u64,
    tick_epoch: u64,
    diff_ms: u32,
    maps: impl IntoIterator<Item = wow_map::MapKey>,
    objects: impl IntoIterator<Item = (wow_map::MapKey, ObjectGuid)>,
) -> Option<AdmittedCreatureExecutionLikeCpp> {
    let manager = canonical_map_manager.lock().ok()?;
    let mut admitted_maps = Vec::new();
    for key in maps {
        let Some(incarnation) = manager.map_incarnation_like_cpp(key) else {
            continue;
        };
        admitted_maps.push(AdmittedCreatureExecutionMapLikeCpp {
            map_id: key.map_id,
            instance_id: key.instance_id,
            incarnation,
        });
    }
    admitted_maps.sort_unstable();
    admitted_maps.dedup();

    let mut admitted_objects = Vec::new();
    for (key, creature_guid) in objects {
        let Some(incarnation) = manager.map_incarnation_like_cpp(key) else {
            continue;
        };
        let Some(record_identity) = manager
            .find_map(key.map_id, key.instance_id)
            .and_then(|map| {
                map.map().with_creature_like_cpp(
                    creature_guid,
                    canonical_creature_record_identity_like_cpp,
                )
            })
        else {
            continue;
        };
        admitted_objects.push(AdmittedCreatureExecutionObjectLikeCpp {
            map_id: key.map_id,
            instance_id: key.instance_id,
            incarnation,
            creature_guid,
            record_identity,
        });
    }
    admitted_objects.sort_unstable();
    admitted_objects.dedup();

    Some(AdmittedCreatureExecutionLikeCpp {
        coordinator_id,
        tick_epoch,
        diff_ms,
        game_time_secs: wow_entities::game_time_secs_like_cpp(),
        maps: admitted_maps,
        objects: admitted_objects,
    })
}

/// Identity of one live canonical `Creature` record.
///
/// The record owns its `Creature`; a replacement allocates a new one. The
/// address is only ever compared while the same map guard that captured it is
/// held, so it cannot be observed across a free.
fn canonical_creature_record_identity_like_cpp(creature: &wow_entities::Creature) -> u64 {
    std::ptr::from_ref(creature) as usize as u64
}

impl AdmittedCreatureExecutionLikeCpp {
    /// The fence. Evaluated against the canonical manager **before** any
    /// mutation, and again under the same guard the execution uses.
    #[must_use]
    pub fn fence_like_cpp(
        &self,
        manager: &wow_map::MapManager,
    ) -> CreatureExecutionAdmissionLikeCpp {
        let current = manager.tick_coordination_like_cpp();
        if current != wow_map::MapTickCoordinationStateLikeCpp::AwaitingSessions(self.tick_epoch) {
            return CreatureExecutionAdmissionLikeCpp::RefusedSessionBarrier {
                expected_epoch: self.tick_epoch,
                current,
            };
        }
        for map in &self.maps {
            let key = wow_map::MapKey::new(map.map_id, map.instance_id);
            let current = manager.map_incarnation_like_cpp(key);
            if current != Some(map.incarnation) {
                return CreatureExecutionAdmissionLikeCpp::RefusedMapIncarnation {
                    map_id: map.map_id,
                    instance_id: map.instance_id,
                    expected: map.incarnation,
                    current,
                };
            }
        }
        for object in &self.objects {
            let Some(map) = manager.find_map(object.map_id, object.instance_id) else {
                return CreatureExecutionAdmissionLikeCpp::RefusedObjectMissing {
                    map_id: object.map_id,
                    instance_id: object.instance_id,
                    creature_guid: object.creature_guid,
                };
            };
            let Some(current_identity) = map.map().with_creature_like_cpp(
                object.creature_guid,
                canonical_creature_record_identity_like_cpp,
            ) else {
                return CreatureExecutionAdmissionLikeCpp::RefusedObjectMissing {
                    map_id: object.map_id,
                    instance_id: object.instance_id,
                    creature_guid: object.creature_guid,
                };
            };
            if current_identity != object.record_identity {
                return CreatureExecutionAdmissionLikeCpp::RefusedObjectReplaced {
                    map_id: object.map_id,
                    instance_id: object.instance_id,
                    creature_guid: object.creature_guid,
                    expected: object.record_identity,
                    current: current_identity,
                };
            }
        }
        CreatureExecutionAdmissionLikeCpp::Admitted
    }

    /// The transition of one admitted object.
    #[must_use]
    pub fn transition_like_cpp(
        &self,
        object: &AdmittedCreatureExecutionObjectLikeCpp,
    ) -> CreatureExecutionTransitionLikeCpp {
        CreatureExecutionTransitionLikeCpp {
            coordinator_id: self.coordinator_id,
            tick_epoch: self.tick_epoch,
            map_id: object.map_id,
            instance_id: object.instance_id,
            incarnation: object.incarnation,
            creature_guid: object.creature_guid,
        }
    }
}
