//! Canonical `Map::SendObjectUpdates` snapshot and mask-consumption boundary.

use super::*;
use wow_entities::{
    AccessorObjectKind, AreaTriggerValuesUpdate, ConversationValuesUpdate, CorpseValuesUpdate,
    DynamicObjectValuesUpdate, GameObjectValuesUpdate, PlayerValuesUpdate, SceneObjectValuesUpdate,
    UnitValuesUpdate,
};

#[derive(Debug, Clone, PartialEq)]
pub struct RepresentedDynamicObjectValuesUpdateLikeCpp {
    pub guid: ObjectGuid,
    pub values_update: DynamicObjectValuesUpdate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepresentedPlayerValuesUpdateLikeCpp {
    pub guid: ObjectGuid,
    pub values_update: PlayerValuesUpdate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepresentedUnitValuesUpdateLikeCpp {
    pub guid: ObjectGuid,
    pub kind: AccessorObjectKind,
    pub values_update: UnitValuesUpdate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepresentedGameObjectValuesUpdateLikeCpp {
    pub guid: ObjectGuid,
    pub values_update: GameObjectValuesUpdate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepresentedCorpseValuesUpdateLikeCpp {
    pub guid: ObjectGuid,
    pub values_update: CorpseValuesUpdate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepresentedAreaTriggerValuesUpdateLikeCpp {
    pub guid: ObjectGuid,
    pub values_update: AreaTriggerValuesUpdate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepresentedSceneObjectValuesUpdateLikeCpp {
    pub guid: ObjectGuid,
    pub values_update: SceneObjectValuesUpdate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepresentedConversationValuesUpdateLikeCpp {
    pub guid: ObjectGuid,
    pub values_update: ConversationValuesUpdate,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct SendObjectUpdatesSummaryLikeCpp {
    /// Objects in canonical `Map::entity_world` with represented
    /// `Object::m_objectUpdated` set at snapshot time.
    pub queued_before: usize,
    /// In-world updated objects consumed through the represented BuildUpdate seam.
    pub processed: usize,
    /// Objects whose update masks were cleared via `ClearUpdateMask(false)`.
    pub cleared_update_masks: usize,
    /// Defense for impossible/stale Rust state where the represented update queue
    /// contains a not-in-world object. C++ asserts in `Map::SendObjectUpdates`.
    pub skipped_not_in_world: usize,
    /// Snapshot GUIDs that disappeared before mutable consumption.
    pub missing_or_stale: usize,
    /// Objects whose packet publication is deferred until the map owner has
    /// released its guard. The world-server delivery adapter accounts for the
    /// actual recipient selection and queue result.
    pub publication_deferred: usize,
    /// Stable represented DynamicObject VALUES snapshots captured before clear.
    pub dynamic_object_values_updates: Vec<RepresentedDynamicObjectValuesUpdateLikeCpp>,
    /// Complete Player/Unit/ActivePlayer VALUES snapshots captured before clear.
    pub player_values_updates: Vec<RepresentedPlayerValuesUpdateLikeCpp>,
    /// Complete Unit VALUES snapshots captured before clear.
    pub unit_values_updates: Vec<RepresentedUnitValuesUpdateLikeCpp>,
    /// Typed GameObject VALUES snapshots captured before clear.
    pub game_object_values_updates: Vec<RepresentedGameObjectValuesUpdateLikeCpp>,
    /// Typed Corpse VALUES snapshots captured before clear.
    pub corpse_values_updates: Vec<RepresentedCorpseValuesUpdateLikeCpp>,
    /// Typed AreaTrigger VALUES snapshots captured before clear.
    pub area_trigger_values_updates: Vec<RepresentedAreaTriggerValuesUpdateLikeCpp>,
    /// Typed SceneObject VALUES snapshots captured before clear.
    pub scene_object_values_updates: Vec<RepresentedSceneObjectValuesUpdateLikeCpp>,
    /// Typed Conversation VALUES snapshots captured before clear.
    pub conversation_values_updates: Vec<RepresentedConversationValuesUpdateLikeCpp>,
}

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    /// Bounded represented consumption seam for C++ `Map::SendObjectUpdates()`.
    ///
    /// C++ anchors:
    /// - `Map.cpp:777` calls `SendObjectUpdates()` after ObjectUpdater/Transport
    ///   visitation during `Map::Update`.
    /// - `Map.cpp:1929-1948` drains `_updateObjects`, calls `BuildUpdate` and
    ///   builds per-player packets from `UpdateDataMapType`.
    /// - `Object.cpp:797-806` clears changed values and `m_objectUpdated`.
    /// - `Object.cpp:3722-3728` visits visible players then clears the mask.
    ///
    /// The map producer captures typed VALUES before clearing the masks. Packet
    /// construction and recipient fanout are completed by the world-server
    /// adapter after this map operation releases its guard.
    pub fn send_object_updates_like_cpp(&mut self) -> SendObjectUpdatesSummaryLikeCpp {
        let updated_guids = self
            .entity_world
            .iter()
            .filter_map(|(guid, record)| {
                record
                    .object()
                    .object()
                    .is_object_updated()
                    .then_some(*guid)
            })
            .collect::<Vec<_>>();

        let mut summary = SendObjectUpdatesSummaryLikeCpp {
            queued_before: updated_guids.len(),
            ..Default::default()
        };

        for guid in updated_guids {
            let Some(record) = self.entity_world.get_mut(&guid) else {
                summary.missing_or_stale += 1;
                continue;
            };
            if !record.object().object().is_in_world() {
                summary.skipped_not_in_world += 1;
                continue;
            }

            match record.kind() {
                AccessorObjectKind::Player => {
                    let player = record.player_mut().expect("typed Player record");
                    let values_update = player.values_update(true);
                    if values_update.has_data() {
                        summary
                            .player_values_updates
                            .push(RepresentedPlayerValuesUpdateLikeCpp {
                                guid,
                                values_update,
                            });
                    }
                    player.clear_data_changes();
                }
                AccessorObjectKind::Creature => {
                    let creature = record.creature_mut().expect("typed Creature record");
                    let values_update = creature.unit().values_update();
                    if values_update.has_data() {
                        summary
                            .unit_values_updates
                            .push(RepresentedUnitValuesUpdateLikeCpp {
                                guid,
                                kind: AccessorObjectKind::Creature,
                                values_update,
                            });
                    }
                    creature.clear_data_changes();
                }
                AccessorObjectKind::Pet => {
                    let pet = record.pet_mut().expect("typed Pet record");
                    let values_update = pet.creature().unit().values_update();
                    if values_update.has_data() {
                        summary
                            .unit_values_updates
                            .push(RepresentedUnitValuesUpdateLikeCpp {
                                guid,
                                kind: AccessorObjectKind::Pet,
                                values_update,
                            });
                    }
                    pet.creature_mut().clear_data_changes();
                }
                AccessorObjectKind::GameObject | AccessorObjectKind::Transport => {
                    let game_object = record.game_object_mut().expect("typed GameObject record");
                    let values_update = game_object.values_update();
                    if values_update.has_data() {
                        summary.game_object_values_updates.push(
                            RepresentedGameObjectValuesUpdateLikeCpp {
                                guid,
                                values_update,
                            },
                        );
                    }
                    game_object.clear_game_object_data_changes();
                }
                AccessorObjectKind::Corpse => {
                    let corpse = record.corpse_mut().expect("typed Corpse record");
                    let values_update = corpse.values_update();
                    if values_update.has_data() {
                        summary
                            .corpse_values_updates
                            .push(RepresentedCorpseValuesUpdateLikeCpp {
                                guid,
                                values_update,
                            });
                    }
                    corpse.clear_corpse_data_changes();
                }
                AccessorObjectKind::AreaTrigger => {
                    let area_trigger = record.area_trigger_mut().expect("typed AreaTrigger record");
                    let values_update = area_trigger.values_update();
                    if values_update.has_data() {
                        summary.area_trigger_values_updates.push(
                            RepresentedAreaTriggerValuesUpdateLikeCpp {
                                guid,
                                values_update,
                            },
                        );
                    }
                    area_trigger.clear_area_trigger_data_changes();
                }
                AccessorObjectKind::SceneObject => {
                    let scene_object = record.scene_object_mut().expect("typed SceneObject record");
                    let values_update = scene_object.values_update();
                    if values_update.has_data() {
                        summary.scene_object_values_updates.push(
                            RepresentedSceneObjectValuesUpdateLikeCpp {
                                guid,
                                values_update,
                            },
                        );
                    }
                    scene_object.clear_scene_object_data_changes();
                }
                AccessorObjectKind::Conversation => {
                    let conversation = record
                        .conversation_mut()
                        .expect("typed Conversation record");
                    let values_update = conversation.values_update();
                    if values_update.has_data() {
                        summary.conversation_values_updates.push(
                            RepresentedConversationValuesUpdateLikeCpp {
                                guid,
                                values_update,
                            },
                        );
                    }
                    conversation.clear_conversation_data_changes();
                }
                AccessorObjectKind::DynamicObject => {
                    if let Some(dynamic_object) = record.dynamic_object_mut() {
                        let values_update = dynamic_object.values_update();
                        if values_update.has_data() {
                            summary.dynamic_object_values_updates.push(
                                RepresentedDynamicObjectValuesUpdateLikeCpp {
                                    guid,
                                    values_update,
                                },
                            );
                        }
                        dynamic_object.clear_dynamic_object_data_changes();
                    }
                }
            }
            record.object_mut().object_mut().clear_update_mask(false);
            summary.processed += 1;
            summary.cleared_update_masks += 1;
            summary.publication_deferred += 1;
        }
        summary
    }
}
