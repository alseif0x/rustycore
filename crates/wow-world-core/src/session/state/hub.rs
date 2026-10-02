// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Hub views (#1241 F3): split borrows of the hub members, built from disjoint WorldSession fields.

#[cfg(any(test, feature = "test-fixtures"))]
use super::SessionFixtures;
use super::{SessionCatalogs, SessionCore, SessionWorldConfig};

/// Shared hub view: core, catalogs, config and the cfg(test) fixtures. Copy; it holds only
/// shared references and never a lock guard, so it is Send wherever the session is Sync.
#[derive(Clone, Copy)]
pub struct HubRef<'a> {
    pub core: &'a SessionCore,
    pub catalogs: &'a SessionCatalogs,
    pub config: &'a SessionWorldConfig,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fixtures: &'a SessionFixtures,
}

/// Mutable hub view for moved fns that take `&mut self` (core and fixtures writable).
pub struct HubMut<'a> {
    pub core: &'a mut SessionCore,
    pub catalogs: &'a SessionCatalogs,
    pub config: &'a SessionWorldConfig,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fixtures: &'a mut SessionFixtures,
}

impl HubMut<'_> {
    pub fn shared(&self) -> HubRef<'_> {
        HubRef {
            core: &*self.core,
            catalogs: self.catalogs,
            config: self.config,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures: &*self.fixtures,
        }
    }
}
