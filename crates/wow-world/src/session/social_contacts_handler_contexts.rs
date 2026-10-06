// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the social-contact handler context (#1263 F5).
//!
//! The social crate owns the handlers and their context; the session only
//! splits its lifecycle state (for the social persistence port) from the hub,
//! so no session reference crosses into the handler.

use wow_world_social::{SocialContactsHandlerCxLikeCpp, SocialContactsHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl SocialContactsHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn social_contacts_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> SocialContactsHandlerCxLikeCpp<'a> {
        let (lifecycle, hub) = crate::session::split_lifecycle_mut(self);
        let persistence = lifecycle.social_persistence_port_like_cpp();
        SocialContactsHandlerCxLikeCpp::new(hub, persistence)
    }
}

impl WorldSession {
    /// C++ `PlayerSocial::SendSocialList`, reached by login and by the packet.
    pub(crate) async fn send_contact_list_like_cpp(&mut self, flags: u32) {
        let (lifecycle, hub) = crate::session::split_lifecycle_mut(self);
        let persistence = lifecycle.social_persistence_port_like_cpp();
        SocialContactsHandlerCxLikeCpp::new(hub, persistence)
            .send_contact_list_like_cpp(flags)
            .await;
    }
}
