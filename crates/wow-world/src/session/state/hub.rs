// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Hub views (#1241 F3): split borrows of the hub members, built from disjoint WorldSession fields.

use super::*;

/// Shared hub view: core, catalogs, config and the cfg(test) fixtures. Copy; it holds only
/// shared references and never a lock guard, so it is Send wherever the session is Sync.
#[derive(Clone, Copy)]
pub(crate) struct HubRef<'a> {
    pub(crate) core: &'a SessionCore,
    pub(crate) catalogs: &'a SessionCatalogs,
    pub(in crate::session) config: &'a SessionWorldConfig,
    #[cfg(test)]
    pub(crate) fixtures: &'a SessionFixtures,
}

/// Mutable hub view for moved fns that take `&mut self` (core and fixtures writable).
pub(crate) struct HubMut<'a> {
    pub(crate) core: &'a mut SessionCore,
    pub(crate) catalogs: &'a SessionCatalogs,
    pub(in crate::session) config: &'a SessionWorldConfig,
    #[cfg(test)]
    pub(crate) fixtures: &'a mut SessionFixtures,
}

impl HubMut<'_> {
    pub(crate) fn shared(&self) -> HubRef<'_> {
        HubRef {
            core: &*self.core,
            catalogs: self.catalogs,
            config: self.config,
            #[cfg(test)]
            fixtures: &*self.fixtures,
        }
    }
}

/// Builds the shared view from disjoint WorldSession fields (free fn: not an `impl WorldSession` item).
pub(crate) fn hub_ref(s: &WorldSession) -> HubRef<'_> {
    HubRef {
        core: &s.core,
        catalogs: &s.catalogs,
        config: &s.config,
        #[cfg(test)]
        fixtures: &s.fixtures,
    }
}

pub(crate) fn hub_mut(s: &mut WorldSession) -> HubMut<'_> {
    HubMut {
        core: &mut s.core,
        catalogs: &s.catalogs,
        config: &s.config,
        #[cfg(test)]
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
            #[cfg(test)]
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
            #[cfg(test)]
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
            #[cfg(test)]
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
            #[cfg(test)]
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
            #[cfg(test)]
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
            #[cfg(test)]
            fixtures: &mut s.fixtures,
        },
    )
}

/// Shared group state plus the shared hub for `&self` methods.
pub(crate) fn split_lifecycle_ref(s: &WorldSession) -> (&SessionLifecycleState, HubRef<'_>) {
    (&s.lifecycle, hub_ref(s))
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
            #[cfg(test)]
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
            #[cfg(test)]
            fixtures: &mut s.fixtures,
        },
    )
}

/// Shared group state plus the shared hub for `&self` methods.
pub(crate) fn split_inventory_ref(s: &WorldSession) -> (&InventoryState, HubRef<'_>) {
    (&s.inventory, hub_ref(s))
}
