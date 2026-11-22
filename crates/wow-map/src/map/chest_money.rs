use super::*;
use std::sync::{Arc, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChestMoneyRestockTransition {
    /// `GameObject::Update` expired a timer that began while the chest was
    /// partially looted. The pool is cleared and becomes ready immediately.
    PartialTimerExpired,
    /// `GO_JUST_DEACTIVATED` fully looted a non-consumable chest. Its new
    /// timer begins at `deadline`, after durable source arming is confirmed.
    FullyLootedNonConsumable,
    /// A consumable chest is entering its normal persistent respawn path.
    GenericRespawn,
}

#[derive(Debug, Clone)]
pub struct ChestMoneyRestock {
    pub realm_id: u16,
    pub spawn_id: u64,
    pub generation: u64,
    pub deadline: i64,
    /// The map may apply the physical restock transition only after the
    /// database writer confirms the source head update.
    pub acknowledged: Arc<OnceLock<bool>>,
    /// The writer must atomically persist the ordinary GameObject respawn
    /// row with this source deadline when C++ would call SaveRespawnTime.
    pub persist_respawn: bool,
    pub(crate) gameobject_guid: ObjectGuid,
    pub(crate) lifecycle_revision: u64,
    /// Retain and compare the exact shared pool lifetime across map removal or
    /// same-GUID replacement; a GUID and local revision may both be reused.
    pub(crate) loot_authority: wow_loot::OwnedLootAuthority,
    pub(crate) transition: ChestMoneyRestockTransition,
    submitted: bool,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ChestMoneyRestockCandidate {
    pub deadline: i64,
    pub persist_respawn: bool,
    pub transition: ChestMoneyRestockTransition,
}

pub(super) struct ChestMoneyUpdateGate {
    pub completed: Option<ChestMoneyRestock>,
    pub deferred: bool,
    pub partial_expired: bool,
}

impl<Terrain: TerrainGridLoader, Lifecycle: GridLifecycle> Map<Terrain, Lifecycle> {
    pub(super) fn chest_money_restock_candidate_like_cpp(
        &self,
        guid: ObjectGuid,
        game_time_secs: i64,
    ) -> Option<ChestMoneyRestockCandidate> {
        let game_object = self.get_typed_game_object(guid)?;
        if game_object.data().type_id as u32 != GAMEOBJECT_TYPE_CHEST {
            return None;
        }
        let source = game_object.represented_chest_loot_source_like_cpp()?;

        // C++ GameObject::Update clears a still-live non-consumable Loot pool
        // as soon as its activation-time restock deadline expires. This is
        // evaluated before Map's JustDeactivated cleanup, so a due partial
        // timer wins when both edges are visible in one map tick.
        if !source.chest_consumable
            && game_object.loot_state() == LootState::Activated
            && game_object.shared_loot_like_cpp().is_some()
            && game_object.restock_time() > 0
            && game_object.restock_time() <= game_time_secs
        {
            return Some(ChestMoneyRestockCandidate {
                deadline: game_object.restock_time(),
                persist_respawn: false,
                transition: ChestMoneyRestockTransition::PartialTimerExpired,
            });
        }

        if game_object.loot_state() != LootState::JustDeactivated {
            return None;
        }
        if (game_object.owner_guid() != ObjectGuid::EMPTY || game_object.spell_id() != 0)
            && game_object.respawn_time() == 0
        {
            return None;
        }

        if !source.chest_consumable {
            return Some(ChestMoneyRestockCandidate {
                deadline: game_time_secs.saturating_add(i64::from(source.chest_restock_time_secs)),
                persist_respawn: false,
                transition: ChestMoneyRestockTransition::FullyLootedNonConsumable,
            });
        }

        // The ordinary C++ respawn path only schedules static/default-spawn
        // objects with a positive delay. SaveRespawnTime persists only when
        // GameObject data, a database spawn id, and a future deadline exist;
        // compatibility mode still performs its DB-only persistence call.
        if !game_object.spawned_by_default() || game_object.respawn_delay_time() == 0 {
            return None;
        }
        let deadline = game_time_secs.saturating_add(i64::from(game_object.respawn_delay_time()));
        Some(ChestMoneyRestockCandidate {
            deadline,
            persist_respawn: game_object.has_represented_gameobject_data_like_cpp()
                && game_object.spawn_id() != 0
                && deadline > game_time_secs,
            transition: ChestMoneyRestockTransition::GenericRespawn,
        })
    }

    pub(super) fn begin_chest_money_update_like_cpp(
        &mut self,
        guid: ObjectGuid,
        game_time_secs: i64,
        may_start_transition: bool,
    ) -> ChestMoneyUpdateGate {
        let completed = self.take_acknowledged_chest_money_restock_like_cpp(guid);
        let mut immediate_partial_expiry = false;
        if completed.is_none()
            && !self.chest_money_reset_pending_like_cpp(guid)
            && may_start_transition
        {
            if let Some(candidate) =
                self.chest_money_restock_candidate_like_cpp(guid, game_time_secs)
            {
                if !self.record_chest_money_restock(
                    guid,
                    candidate.deadline,
                    candidate.persist_respawn,
                    candidate.transition,
                ) && candidate.transition == ChestMoneyRestockTransition::PartialTimerExpired
                {
                    immediate_partial_expiry = true;
                }
            }
        }

        ChestMoneyUpdateGate {
            partial_expired: immediate_partial_expiry
                || completed.as_ref().is_some_and(|restock| {
                    restock.transition == ChestMoneyRestockTransition::PartialTimerExpired
                }),
            deferred: self.chest_money_reset_pending_like_cpp(guid),
            completed,
        }
    }

    pub(super) fn apply_partial_chest_money_expiry_like_cpp(&mut self, guid: ObjectGuid) -> bool {
        let Some(game_object) = self.get_typed_game_object_mut(guid) else {
            return false;
        };
        game_object.set_restock_time_like_cpp(0);
        game_object.set_loot_state(LootState::Ready, None);
        game_object.clear_loot_like_cpp();
        true
    }

    pub(super) fn clear_loot_for_chest_money_transition_like_cpp(
        &mut self,
        guid: ObjectGuid,
        partial_expired: bool,
        may_clear_just_deactivated: bool,
    ) -> bool {
        if partial_expired {
            return self.apply_partial_chest_money_expiry_like_cpp(guid);
        }
        if !may_clear_just_deactivated {
            return false;
        }
        let Some(game_object) = self
            .entity_world
            .get_mut(&guid)
            .and_then(MapObjectRecord::game_object_mut)
            .filter(|game_object| game_object.loot_state() == LootState::JustDeactivated)
        else {
            return false;
        };
        game_object.clear_loot_like_cpp();
        true
    }

    fn chest_money_restock_matches_current_like_cpp(&self, restock: &ChestMoneyRestock) -> bool {
        self.get_typed_game_object(restock.gameobject_guid)
            .is_some_and(|game_object| {
                game_object.spawn_id() == restock.spawn_id
                    && game_object.loot_lifecycle_revision_like_cpp() == restock.lifecycle_revision
                    && game_object
                        .loot_authority_like_cpp()
                        .shares_storage_like_cpp(&restock.loot_authority)
            })
    }

    pub fn take_chest_money_restocks(&mut self) -> Vec<ChestMoneyRestock> {
        let mut submitted = Vec::new();
        let mut retained = Vec::with_capacity(self.pending_chest_money_restocks.len());
        for mut restock in std::mem::take(&mut self.pending_chest_money_restocks) {
            if restock.submitted && !self.chest_money_restock_matches_current_like_cpp(&restock) {
                continue;
            }
            if !restock.submitted {
                restock.submitted = true;
                submitted.push(restock.clone());
            }
            retained.push(restock);
        }
        self.pending_chest_money_restocks = retained;
        submitted
    }

    /// Keep a map resident only while the exact GameObject/pool transition is live.
    pub fn has_pending_chest_money_resets_like_cpp(&self) -> bool {
        self.pending_chest_money_restocks
            .iter()
            .any(|restock| self.chest_money_restock_matches_current_like_cpp(restock))
    }

    /// GameObject updates for durable resets must run even when the map has no
    /// active players. The coordinator visits these acknowledged transitions
    /// independently of the ordinary active-map tick set.
    pub fn acknowledged_chest_money_guids_like_cpp(&self) -> impl Iterator<Item = ObjectGuid> + '_ {
        self.pending_chest_money_restocks
            .iter()
            .filter(|restock| {
                restock.acknowledged.get() == Some(&true)
                    && self.chest_money_restock_matches_current_like_cpp(restock)
            })
            .map(|restock| restock.gameobject_guid)
    }

    /// Prevent an opener from reinstalling loot while this source's restock
    /// transition is waiting for durable acknowledgement.
    pub fn chest_money_reset_pending_like_cpp(&self, guid: ObjectGuid) -> bool {
        self.pending_chest_money_restocks.iter().any(|restock| {
            restock.gameobject_guid == guid
                && self.chest_money_restock_matches_current_like_cpp(restock)
        })
    }

    pub(super) fn record_chest_money_restock(
        &mut self,
        guid: ObjectGuid,
        deadline: i64,
        persist_respawn: bool,
        transition: ChestMoneyRestockTransition,
    ) -> bool {
        if self.chest_money_reset_pending_like_cpp(guid) {
            return true;
        }
        let Some(chest) = self.get_typed_game_object_mut(guid) else {
            return false;
        };
        let spawn_id = chest.spawn_id();
        if spawn_id == 0 {
            return false;
        }
        let lifecycle_revision = chest.loot_lifecycle_revision_like_cpp();
        let loot_authority = chest.loot_authority_like_cpp().clone();
        let Some((realm_id, generation)) = chest.take_chest_money_generation() else {
            return false;
        };

        // Close admission at the real reset edge. `retire_like_cpp` keeps
        // persistence-protected pre-edge claims commit-capable and rejects new
        // claims while the durable arm is in flight.
        chest.loot_authority_like_cpp().retire_like_cpp();
        self.pending_chest_money_restocks.push(ChestMoneyRestock {
            realm_id,
            spawn_id,
            generation,
            deadline,
            acknowledged: Arc::new(OnceLock::new()),
            persist_respawn,
            gameobject_guid: guid,
            lifecycle_revision,
            loot_authority,
            transition,
            submitted: false,
        });
        true
    }

    pub(super) fn take_acknowledged_chest_money_restock_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<ChestMoneyRestock> {
        let index = self
            .pending_chest_money_restocks
            .iter()
            .position(|restock| {
                restock.gameobject_guid == guid
                    && restock.acknowledged.get() == Some(&true)
                    && self.chest_money_restock_matches_current_like_cpp(restock)
            })?;
        let restock = &self.pending_chest_money_restocks[index];
        let (realm_id, generation, deadline, transition) = (
            restock.realm_id,
            restock.generation,
            restock.deadline,
            restock.transition,
        );
        let state_matches =
            self.get_typed_game_object(guid)
                .is_some_and(|chest| match transition {
                    ChestMoneyRestockTransition::PartialTimerExpired => {
                        chest.loot_state() == LootState::Activated
                            && chest.shared_loot_like_cpp().is_some()
                            && chest.restock_time() == deadline
                    }
                    ChestMoneyRestockTransition::FullyLootedNonConsumable
                    | ChestMoneyRestockTransition::GenericRespawn => {
                        chest.loot_state() == LootState::JustDeactivated
                    }
                });
        if !state_matches {
            // Keep the exact source resident and closed when another map-side
            // transition changed its C++ state before this ack could apply.
            return None;
        }

        let finished = self
            .get_typed_game_object_mut(guid)
            .is_some_and(|chest| chest.finish_chest_money_restock(realm_id, generation));
        if !finished {
            // Keep the source closed and the acknowledged event resident. A
            // later map visit may retry after the marker is reconciled.
            return None;
        }

        Some(self.pending_chest_money_restocks.remove(index))
    }
}

pub(super) fn chest_money_transition_deadline_like_cpp(
    restock: Option<&ChestMoneyRestock>,
    transition: ChestMoneyRestockTransition,
    fallback: i64,
) -> i64 {
    restock
        .filter(|restock| restock.transition == transition)
        .map_or(fallback, |restock| restock.deadline)
}

pub(super) fn chest_money_transition_persist_respawn_like_cpp(
    restock: Option<&ChestMoneyRestock>,
    scheduled_deadline: i64,
    current_time: i64,
) -> bool {
    restock.is_some_and(|restock| {
        restock.transition == ChestMoneyRestockTransition::GenericRespawn && restock.persist_respawn
    }) || scheduled_deadline > current_time
}
