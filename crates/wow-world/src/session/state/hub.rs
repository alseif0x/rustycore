// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Hub views (#1241 F3): split borrows of the hub members, built from disjoint WorldSession fields.

use super::*;
pub(crate) use wow_world_core::session::{HubMut, HubRef};

/// Builds the shared view from disjoint WorldSession fields (free fn: not an `impl WorldSession` item).
pub(crate) fn hub_ref(s: &WorldSession) -> HubRef<'_> {
    HubRef {
        core: &s.core,
        catalogs: &s.catalogs,
        config: &s.config,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixtures: &s.fixtures,
    }
}

pub(crate) fn hub_mut(s: &mut WorldSession) -> HubMut<'_> {
    HubMut {
        core: &mut s.core,
        catalogs: &s.catalogs,
        config: &s.config,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixtures: &mut s.fixtures,
    }
}

/// `&mut` group state plus the shared hub, borrowed from disjoint WorldSession fields.
pub(crate) fn split_interaction(s: &mut WorldSession) -> (&mut InteractionState, HubRef<'_>) {
    (
        &mut s.interaction,
        HubRef {
            core: &s.core,
            catalogs: &s.catalogs,
            config: &s.config,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures: &s.fixtures,
        },
    )
}

/// Shared group state plus the shared hub for `&self` methods.
pub(crate) fn split_interaction_ref(s: &WorldSession) -> (&InteractionState, HubRef<'_>) {
    (&s.interaction, hub_ref(s))
}

/// `&mut` group state plus the mutable hub (core and fixtures), borrowed from disjoint fields.
pub(crate) fn split_social_mut(s: &mut WorldSession) -> (&mut SessionSocialLimits, HubMut<'_>) {
    (
        &mut s.social,
        HubMut {
            core: &mut s.core,
            catalogs: &s.catalogs,
            config: &s.config,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures: &mut s.fixtures,
        },
    )
}

/// `&mut` social and lifecycle state plus the mutable hub, borrowed from disjoint fields.
pub(crate) fn split_social_lifecycle_mut(
    s: &mut WorldSession,
) -> (
    &mut SessionSocialLimits,
    &mut SessionLifecycleState,
    HubMut<'_>,
) {
    (
        &mut s.social,
        &mut s.lifecycle,
        HubMut {
            core: &mut s.core,
            catalogs: &s.catalogs,
            config: &s.config,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures: &mut s.fixtures,
        },
    )
}

/// Shared group state plus the shared hub for `&self` methods.
pub(crate) fn split_social_ref(s: &WorldSession) -> (&SessionSocialLimits, HubRef<'_>) {
    (&s.social, hub_ref(s))
}

/// `&mut` group state plus the mutable hub (core and fixtures), borrowed from disjoint fields.
pub(crate) fn split_visibility_mut(s: &mut WorldSession) -> (&mut VisibilityState, HubMut<'_>) {
    (
        &mut s.visibility,
        HubMut {
            core: &mut s.core,
            catalogs: &s.catalogs,
            config: &s.config,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures: &mut s.fixtures,
        },
    )
}

/// Shared group state plus the shared hub for `&self` methods.
pub(crate) fn split_visibility_ref(s: &WorldSession) -> (&VisibilityState, HubRef<'_>) {
    (&s.visibility, hub_ref(s))
}

/// `&mut` group state plus the mutable hub (core and fixtures), borrowed from disjoint fields.
pub(crate) fn split_spell_state_mut(s: &mut WorldSession) -> (&mut SessionSpellState, HubMut<'_>) {
    (
        &mut s.spell_state,
        HubMut {
            core: &mut s.core,
            catalogs: &s.catalogs,
            config: &s.config,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures: &mut s.fixtures,
        },
    )
}

/// Shared group state plus the shared hub for `&self` methods.
pub(crate) fn split_spell_state_ref(s: &WorldSession) -> (&SessionSpellState, HubRef<'_>) {
    (&s.spell_state, hub_ref(s))
}

/// `&mut` group state plus the mutable hub (core and fixtures), borrowed from disjoint fields.
pub(crate) fn split_instances_mut(s: &mut WorldSession) -> (&mut InstanceState, HubMut<'_>) {
    (
        &mut s.instances,
        HubMut {
            core: &mut s.core,
            catalogs: &s.catalogs,
            config: &s.config,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures: &mut s.fixtures,
        },
    )
}

/// Shared group state plus the shared hub for `&self` methods.
pub(crate) fn split_instances_ref(s: &WorldSession) -> (&InstanceState, HubRef<'_>) {
    (&s.instances, hub_ref(s))
}

/// `&mut` group state plus the mutable hub (core and fixtures), borrowed from disjoint fields.
pub(crate) fn split_lifecycle_mut(
    s: &mut WorldSession,
) -> (&mut SessionLifecycleState, HubMut<'_>) {
    (
        &mut s.lifecycle,
        HubMut {
            core: &mut s.core,
            catalogs: &s.catalogs,
            config: &s.config,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures: &mut s.fixtures,
        },
    )
}

/// Shared group state plus the shared hub for `&self` methods.
pub(crate) fn split_lifecycle_ref(s: &WorldSession) -> (&SessionLifecycleState, HubRef<'_>) {
    (&s.lifecycle, hub_ref(s))
}

/// Player-handler state plus the mutable hub, borrowed from disjoint fields.
pub(crate) fn split_player_handler_states_mut(
    s: &mut WorldSession,
) -> (
    &mut SessionQuestState,
    &mut InventoryState,
    &SessionLifecycleState,
    &mut VisibilityState,
    &InstanceState,
    HubMut<'_>,
) {
    (
        &mut s.quest_state,
        &mut s.inventory,
        &s.lifecycle,
        &mut s.visibility,
        &s.instances,
        HubMut {
            core: &mut s.core,
            catalogs: &s.catalogs,
            config: &s.config,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures: &mut s.fixtures,
        },
    )
}

/// Social, lifecycle and loot state plus the mutable hub, borrowed from disjoint fields.
pub(crate) fn split_social_lifecycle_loot_mut(
    s: &mut WorldSession,
) -> (
    &mut SessionSocialLimits,
    &mut SessionLifecycleState,
    &mut LootState,
    HubMut<'_>,
) {
    (
        &mut s.social,
        &mut s.lifecycle,
        &mut s.loot,
        HubMut {
            core: &mut s.core,
            catalogs: &s.catalogs,
            config: &s.config,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures: &mut s.fixtures,
        },
    )
}

/// `&mut` group state plus the mutable hub (core and fixtures), borrowed from disjoint fields.
pub(crate) fn split_world_entities_mut(
    s: &mut WorldSession,
) -> (&mut WorldEntitiesState, HubMut<'_>) {
    (
        &mut s.world_entities,
        HubMut {
            core: &mut s.core,
            catalogs: &s.catalogs,
            config: &s.config,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures: &mut s.fixtures,
        },
    )
}

/// Shared group state plus the shared hub for `&self` methods.
pub(crate) fn split_world_entities_ref(s: &WorldSession) -> (&WorldEntitiesState, HubRef<'_>) {
    (&s.world_entities, hub_ref(s))
}

/// `&mut` group state plus the mutable hub (core and fixtures), borrowed from disjoint fields.
pub(crate) fn split_inventory_mut(s: &mut WorldSession) -> (&mut InventoryState, HubMut<'_>) {
    (
        &mut s.inventory,
        HubMut {
            core: &mut s.core,
            catalogs: &s.catalogs,
            config: &s.config,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures: &mut s.fixtures,
        },
    )
}

/// Shared group state plus the shared hub for `&self` methods.
pub(crate) fn split_inventory_ref(s: &WorldSession) -> (&InventoryState, HubRef<'_>) {
    (&s.inventory, hub_ref(s))
}

/// `&mut` group state plus the mutable hub (core and fixtures), borrowed from disjoint fields.
pub(crate) fn split_loot_mut(s: &mut WorldSession) -> (&mut LootState, HubMut<'_>) {
    (
        &mut s.loot,
        HubMut {
            core: &mut s.core,
            catalogs: &s.catalogs,
            config: &s.config,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures: &mut s.fixtures,
        },
    )
}

/// Shared group state plus the shared hub for `&self` methods.
pub(crate) fn split_loot_ref(s: &WorldSession) -> (&LootState, HubRef<'_>) {
    (&s.loot, hub_ref(s))
}

/// `&mut` group state plus the mutable hub (core and fixtures), borrowed from disjoint fields.
pub(crate) fn split_quest_state_mut(s: &mut WorldSession) -> (&mut SessionQuestState, HubMut<'_>) {
    (
        &mut s.quest_state,
        HubMut {
            core: &mut s.core,
            catalogs: &s.catalogs,
            config: &s.config,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures: &mut s.fixtures,
        },
    )
}

/// Shared group state plus the shared hub for `&self` methods.
pub(crate) fn split_quest_state_ref(s: &WorldSession) -> (&SessionQuestState, HubRef<'_>) {
    (&s.quest_state, hub_ref(s))
}

/// Capped group context (#1241 F3): the `inventory` state, sibling states and hub members its
/// moved fns read, borrowed from disjoint WorldSession fields.
pub(crate) struct InventoryCx<'a> {
    pub(crate) lifecycle: &'a mut SessionLifecycleState,
}

pub(crate) fn cx_inventory(s: &mut WorldSession) -> InventoryCx<'_> {
    InventoryCx {
        lifecycle: &mut s.lifecycle,
    }
}

/// Shared counterpart of `InventoryCx` for `&self` methods.
pub(crate) struct InventoryCxRef<'a> {
    pub(crate) lifecycle: &'a SessionLifecycleState,
    pub(crate) world_entities: &'a WorldEntitiesState,
    pub(crate) hub: HubRef<'a>,
}

pub(crate) fn cx_inventory_ref(s: &WorldSession) -> InventoryCxRef<'_> {
    InventoryCxRef {
        lifecycle: &s.lifecycle,
        world_entities: &s.world_entities,
        hub: hub_ref(s),
    }
}

/// Capped group context (#1241 F3): the `lifecycle` state, sibling states and hub members its
/// moved fns read, borrowed from disjoint WorldSession fields.
pub(crate) struct LifecycleCx<'a> {
    pub(crate) lifecycle: &'a mut SessionLifecycleState,
    pub(crate) instances: &'a mut InstanceState,
    pub(crate) inventory: &'a mut InventoryState,
    pub(crate) hub: HubMut<'a>,
}

pub(crate) fn cx_lifecycle(s: &mut WorldSession) -> LifecycleCx<'_> {
    LifecycleCx {
        lifecycle: &mut s.lifecycle,
        instances: &mut s.instances,
        inventory: &mut s.inventory,
        hub: HubMut {
            core: &mut s.core,
            catalogs: &s.catalogs,
            config: &s.config,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures: &mut s.fixtures,
        },
    }
}

/// Shared counterpart of `LifecycleCx` for `&self` methods.
pub(crate) struct LifecycleCxRef<'a> {
    pub(crate) inventory: &'a InventoryState,
    pub(crate) hub: HubRef<'a>,
}

pub(crate) fn cx_lifecycle_ref(s: &WorldSession) -> LifecycleCxRef<'_> {
    LifecycleCxRef {
        inventory: &s.inventory,
        hub: hub_ref(s),
    }
}

/// Capped group context (#1241 F3): the `loot` state, sibling states and hub members its
/// moved fns read, borrowed from disjoint WorldSession fields.
pub(crate) struct LootCx<'a> {
    pub(crate) loot: &'a mut LootState,
    pub(crate) lifecycle: &'a mut SessionLifecycleState,
    pub(crate) world_entities: &'a mut WorldEntitiesState,
    pub(crate) hub: HubMut<'a>,
}

pub(crate) fn cx_loot(s: &mut WorldSession) -> LootCx<'_> {
    LootCx {
        loot: &mut s.loot,
        lifecycle: &mut s.lifecycle,
        world_entities: &mut s.world_entities,
        hub: HubMut {
            core: &mut s.core,
            catalogs: &s.catalogs,
            config: &s.config,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures: &mut s.fixtures,
        },
    }
}

/// Shared counterpart of `LootCx` for `&self` methods.
pub(crate) struct LootCxRef<'a> {
    pub(crate) loot: &'a LootState,
    pub(crate) inventory: &'a InventoryState,
    pub(crate) lifecycle: &'a SessionLifecycleState,
    pub(crate) world_entities: &'a WorldEntitiesState,
    pub(crate) hub: HubRef<'a>,
}

pub(crate) fn cx_loot_ref(s: &WorldSession) -> LootCxRef<'_> {
    LootCxRef {
        loot: &s.loot,
        inventory: &s.inventory,
        lifecycle: &s.lifecycle,
        world_entities: &s.world_entities,
        hub: hub_ref(s),
    }
}

/// Capped group context (#1241 F3): the `pets` state, sibling states and hub members its
/// moved fns read, borrowed from disjoint WorldSession fields.
pub(crate) struct PetsCx<'a> {
    pub(crate) lifecycle: &'a mut SessionLifecycleState,
    #[cfg(test)]
    pub(crate) social: &'a mut SessionSocialLimits,
    pub(crate) hub: HubMut<'a>,
}

pub(crate) fn cx_pets(s: &mut WorldSession) -> PetsCx<'_> {
    PetsCx {
        lifecycle: &mut s.lifecycle,
        #[cfg(test)]
        social: &mut s.social,
        hub: HubMut {
            core: &mut s.core,
            catalogs: &s.catalogs,
            config: &s.config,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures: &mut s.fixtures,
        },
    }
}

impl PetsCx<'_> {
    pub(crate) fn shared(&self) -> PetsCxRef<'_> {
        PetsCxRef {
            lifecycle: &*self.lifecycle,
            hub: self.hub.shared(),
        }
    }
}

/// Shared counterpart of `PetsCx` for `&self` methods.
pub(crate) struct PetsCxRef<'a> {
    pub(crate) lifecycle: &'a SessionLifecycleState,
    pub(crate) hub: HubRef<'a>,
}

pub(crate) fn cx_pets_ref(s: &WorldSession) -> PetsCxRef<'_> {
    PetsCxRef {
        lifecycle: &s.lifecycle,
        hub: hub_ref(s),
    }
}

/// Capped group context (#1241 F3): the `quest_state` state, sibling states and hub members its
/// moved fns read, borrowed from disjoint WorldSession fields.
pub(crate) struct QuestStateCx<'a> {
    #[cfg(test)]
    pub(crate) inventory: &'a mut InventoryState,
    pub(crate) lifecycle: &'a mut SessionLifecycleState,
    pub(crate) world_entities: &'a mut WorldEntitiesState,
}

pub(crate) fn cx_quest_state(s: &mut WorldSession) -> QuestStateCx<'_> {
    QuestStateCx {
        #[cfg(test)]
        inventory: &mut s.inventory,
        lifecycle: &mut s.lifecycle,
        world_entities: &mut s.world_entities,
    }
}

/// Shared counterpart of `QuestStateCx` for `&self` methods.
pub(crate) struct QuestStateCxRef<'a> {
    pub(crate) inventory: &'a InventoryState,
    pub(crate) lifecycle: &'a SessionLifecycleState,
    pub(crate) world_entities: &'a WorldEntitiesState,
    pub(crate) hub: HubRef<'a>,
}

pub(crate) fn cx_quest_state_ref(s: &WorldSession) -> QuestStateCxRef<'_> {
    QuestStateCxRef {
        inventory: &s.inventory,
        lifecycle: &s.lifecycle,
        world_entities: &s.world_entities,
        hub: hub_ref(s),
    }
}
