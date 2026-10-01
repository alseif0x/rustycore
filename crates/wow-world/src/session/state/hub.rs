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
