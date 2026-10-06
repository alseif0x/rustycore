// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the character query handlers moved to
//! `wow-world-application` (#1263 F5).

use wow_packet::packets::query::{QueryCreature, QueryGameObject, QueryRealmName};
use wow_world_application::CharacterQueryHandlerCxLikeCpp;

use crate::session::WorldSession;

impl WorldSession {
    fn character_query_test_cx_like_cpp(&mut self) -> CharacterQueryHandlerCxLikeCpp<'_> {
        let catalogs = self
            .world_query_catalogs_like_cpp()
            .cloned()
            .unwrap_or_default();
        CharacterQueryHandlerCxLikeCpp::new(
            crate::session::hub_mut(self),
            std::sync::Arc::new(catalogs),
        )
    }

    pub async fn handle_query_creature(&mut self, query: QueryCreature) {
        self.character_query_test_cx_like_cpp()
            .handle_query_creature(query)
            .await;
    }

    pub async fn handle_query_game_object(&mut self, query: QueryGameObject) {
        self.character_query_test_cx_like_cpp()
            .handle_query_game_object(query)
            .await;
    }

    pub async fn handle_query_realm_name(&mut self, query: QueryRealmName) {
        self.character_query_test_cx_like_cpp()
            .handle_query_realm_name(query)
            .await;
    }

    pub fn realm_query_response_like_cpp(
        &self,
        virtual_realm_address: u32,
    ) -> wow_packet::packets::query::RealmQueryResponse {
        wow_world_application::realm_query_response_like_cpp(&self.core, virtual_realm_address)
    }
}
