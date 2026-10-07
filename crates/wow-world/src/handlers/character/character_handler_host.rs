// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the character handler context (#1263 F5).
//!
//! The application crate owns the bodies and their context; the session only
//! lends its hub, inventory and world-entity state. The character-delete,
//! rename, enumeration and create bodies also need the character-administration
//! and enumeration ports, the rename callback rail, the login-DB
//! `realmcharacters` refresh, the support-feature policy and the player GUID
//! generator, which live on the session's lifecycle state and handler catalogs
//! and stay behind this host.

use std::sync::Arc;

use wow_core::{ObjectGuid, ObjectGuidGenerator};
use wow_handler::HandlerFuture;
use wow_persistence::{
    CharacterAdministrationPersistencePortLikeCpp, CharacterEnumerationPersistencePortLikeCpp,
};
use wow_world_application::{CharacterHandlerCxLikeCpp, CharacterHandlerHostLikeCpp};
use wow_world_core::session::SupportFeaturePolicyLikeCpp;

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl CharacterHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn character_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> CharacterHandlerCxLikeCpp<'a> {
        let (inventory, world_entities, hub) = crate::session::split_character_handler_mut(self);
        CharacterHandlerCxLikeCpp::new(hub, inventory, world_entities)
    }

    fn character_administration_persistence_port_like_cpp(
        &mut self,
    ) -> Option<Arc<dyn CharacterAdministrationPersistencePortLikeCpp>> {
        self.lifecycle
            .character_administration_persistence_port_like_cpp()
    }

    fn update_realm_characters_like_cpp(&mut self) -> HandlerFuture<'_, ()> {
        Box::pin(async move {
            self.update_realm_characters().await;
        })
    }

    fn submit_character_rename_like_cpp(
        &mut self,
        port: Arc<dyn CharacterAdministrationPersistencePortLikeCpp>,
        guid: ObjectGuid,
        name: String,
    ) -> bool {
        self.lifecycle
            .submit_character_rename_like_cpp(port, guid, name)
    }

    fn character_enumeration_persistence_port_like_cpp(
        &mut self,
    ) -> Option<Arc<dyn CharacterEnumerationPersistencePortLikeCpp>> {
        self.lifecycle
            .character_enumeration_persistence_port_like_cpp()
    }

    fn support_feature_policy_like_cpp(
        catalogs: &SessionHandlerCatalogsLikeCpp,
    ) -> &SupportFeaturePolicyLikeCpp {
        catalogs.support_feature_policy.as_ref()
    }

    fn character_guid_generator_like_cpp(
        catalogs: &SessionHandlerCatalogsLikeCpp,
    ) -> &ObjectGuidGenerator {
        catalogs.id_generators.player.as_ref()
    }
}
