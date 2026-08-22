//! __MODULE_DISPLAY__ — a RustyCore trusted linked module.
//!
//! This is in-process code compiled into the server with full privileges.
//! Changing it requires a rebuild and restart; there is no sandbox and no
//! hot reload.

use wow_module_api::{
    ModuleConfig, ModuleDescriptor, ModuleRegistrationError, ModuleRegistry, ModuleVersion,
    PlayerLoginModule, PlayerLoginSnapshot, ScopedEffects,
};

/// The immutable snapshot the login callback reads. Resolved once at
/// registration, so no callback ever touches a file.
pub(crate) struct Module {
    enabled: bool,
    welcome_text: String,
}

impl PlayerLoginModule for Module {
    fn on_player_login(&self, snapshot: &PlayerLoginSnapshot, effects: &mut ScopedEffects<'_>) {
        if self.enabled && snapshot.first_login {
            effects.send_system_message_self(self.welcome_text.clone());
        }
    }
}

/// Registrar called by the generated compositor. Keep the name in sync with
/// `registrar` in `module.toml`.
///
/// Configuration is validated here, before the module is registered, so an
/// invalid value prevents activation instead of surfacing at a player's login.
pub fn register(
    registry: &mut ModuleRegistry,
    mut config: ModuleConfig,
) -> Result<(), ModuleRegistrationError> {
    let enabled = config.bool("enabled")?.unwrap_or(true);
    let welcome_text = config
        .text("welcome_text")?
        .ok_or_else(|| config.missing("welcome_text"))?;
    if welcome_text.trim().is_empty() {
        return Err(config.invalid("welcome_text", "must not be blank").into());
    }
    // Any key the module did not read is an operator typo, not a silent no-op.
    config.finish()?;

    let descriptor = ModuleDescriptor::new(
        "__MODULE_ID__",
        ModuleVersion { major: 0, minor: 1, patch: 0 },
        "__MODULE_DISPLAY__",
    )?;
    registry.register_player_login_module(descriptor, Box::new(Module { enabled, welcome_text }))
}
