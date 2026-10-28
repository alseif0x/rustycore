// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spirit-healer resurrection lifecycle.

use wow_entities::{PlayerResurrectionOperation, PlayerResurrectionPhase};

use crate::{CorpseHostLikeCpp, CorpseLifecycleCatalogsLikeCpp};

/// C++ `WorldSession::SendSpiritResurrect` (`NPCHandler.cpp:347-370`).
pub async fn send_spirit_resurrect<H: CorpseHostLikeCpp>(
    host: &mut H,
    catalogs: CorpseLifecycleCatalogsLikeCpp<'_>,
) {
    if !crate::recovery_fences::begin(
        host,
        PlayerResurrectionOperation::SpiritHealer {
            corpse_graveyard: None,
        },
        PlayerResurrectionPhase::ResurrectPlayerReady,
    ) {
        return;
    }
    match crate::recovery_fences::continue_operation(host, catalogs.resurrection()).await {
        crate::recovery_fences::Progress::Complete | crate::recovery_fences::Progress::Waiting => {}
        crate::recovery_fences::Progress::Continue | crate::recovery_fences::Progress::Held => {
            host.core_mut()
                .kick("spirit healer recovery did not complete; retain Player owner");
        }
    }
}
