// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Fixture module that composition must refuse: its manifest depends on a
//! server crate besides `wow-module-api` (#1445 S0). Never compiled.

use wow_module_api::{ModuleConfig, ModuleRegistrationError, ModuleRegistry};

/// The registrar the generated compositor would call.
pub fn register(
    _registry: &mut ModuleRegistry,
    _config: ModuleConfig,
) -> Result<(), ModuleRegistrationError> {
    Ok(())
}
