// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character handler family: barber-chair appearance requests (`HandleAlterAppearance`, `HandleConfirmBarbersChoice`).

use tracing::warn;
use wow_packet::packets::character::{
    AlterAppearance, BARBER_SHOP_RESULT_NOT_ON_CHAIR_LIKE_CPP, BARBER_SHOP_RESULT_SUCCESS_LIKE_CPP,
    BarberShopResult, ConfirmBarbersChoice,
};
use wow_packet::{ClientPacket, WorldPacket};
use wow_world_core::session::{
    RepresentedAlterAppearanceLikeCpp, RepresentedConfirmBarbersChoiceLikeCpp,
};
use wow_world_entities::RepresentedGameObjectUseEffect;

use crate::character_handlers::CharacterHandlerCxLikeCpp;

impl CharacterHandlerCxLikeCpp<'_> {
    /// C++ `Player::IsSittingOnBarberChair` over the represented chair-use
    /// effects: the latest chair use by this Player at its current stand state.
    fn represented_is_on_barber_chair_like_cpp(&self) -> bool {
        let Some(player_guid) = self.hub.shared().core.player_guid() else {
            return false;
        };
        // `UnitStandStateType` is a fieldless `#[repr(u8)]` enum, so the
        // discriminant cast equals the former `ToPrimitive::to_u32` result.
        let Some(current_stand_state) = self
            .hub
            .shared()
            .resolved_player_stand_state_like_cpp()
            .map(|state| state as u32)
        else {
            return false;
        };

        self.world_entities
            .represented_gameobject_use_effects_since_like_cpp(0)
            .iter()
            .rev()
            .any(|effect| {
                matches!(
                    effect,
                    RepresentedGameObjectUseEffect::BarberChairUsed {
                        player_guid: effect_player_guid,
                        stand_state,
                        ..
                    } if *effect_player_guid == player_guid && *stand_state == current_stand_state
                )
            })
    }

    /// Handle CMSG_ALTER_APPEARANCE.
    ///
    /// C++ `HandleAlterAppearance` validates customization DB2 requirements,
    /// requires the player to be sitting on a nearby barber chair, checks
    /// `GetBarberShopCost`, sends `SMSG_BARBER_SHOP_RESULT`, then mutates
    /// player gender/customizations and criteria.
    ///
    /// Rust currently represents barber-chair use and stand-state, but does
    /// not yet own the full ChrCustomization/BarberShop cost/runtime mutation.
    /// This seam preserves packet/dispatch, the C++ not-on-chair result, and
    /// records accepted requests without fabricating the full appearance change.
    pub async fn handle_alter_appearance(&mut self, mut pkt: WorldPacket) {
        let request = match AlterAppearance::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!("Bad AlterAppearance: {error}");
                return;
            }
        };

        if !self.represented_is_on_barber_chair_like_cpp() {
            self.publication_like_cpp().send_packet(&BarberShopResult {
                result: BARBER_SHOP_RESULT_NOT_ON_CHAIR_LIKE_CPP,
            });
            return;
        }

        let cost = 0;
        self.publication_like_cpp().send_packet(&BarberShopResult {
            result: BARBER_SHOP_RESULT_SUCCESS_LIKE_CPP,
        });
        self.inventory.record_represented_alter_appearance_like_cpp(
            &mut self.hub,
            RepresentedAlterAppearanceLikeCpp {
                new_sex: request.new_sex,
                customizations: request.customizations,
                customized_race: request.customized_race,
                customized_chr_model_id: request.customized_chr_model_id,
                cost,
            },
        );
    }

    /// Handle CMSG_CONFIRM_BARBERS_CHOICE.
    ///
    /// C++ `HandleConfirmBarbersChoice` converts the barber rows into
    /// `ChrCustomizationChoice`, checks `GetBarberShopCost`, sends only the
    /// no-money failure, and otherwise mutates money/customizations/criteria
    /// without a success packet. Rust records the accepted request until the
    /// Player customization/cost/criteria runtime is canonical.
    pub async fn handle_confirm_barbers_choice(&mut self, mut pkt: WorldPacket) {
        let request = match ConfirmBarbersChoice::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!("Bad ConfirmBarbersChoice: {error}");
                return;
            }
        };

        let cost = 0;
        self.hub.record_represented_confirm_barbers_choice_like_cpp(
            RepresentedConfirmBarbersChoiceLikeCpp {
                customizations: request.customizations,
                cost,
            },
        );
    }
}
