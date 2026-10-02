// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player binding data shared with the WorldSession shell.

#[cfg(any(test, feature = "test-fixtures"))]
pub struct PlayerTransportLoginStateLikeCpp {
    pub info: wow_packet::packets::movement::TransportInfo,
}

/// Login-only identity input consumed while the canonical Player is being
/// constructed. Once a generation-checked Player exists this value is retired;
/// it is not a second runtime identity authority.
#[derive(Debug, Clone, Default)]
pub struct PlayerIdentityBootstrapLikeCpp {
    pub name: Option<String>,
    pub race: u8,
    pub class: u8,
    pub level: u8,
    pub gender: u8,
}
