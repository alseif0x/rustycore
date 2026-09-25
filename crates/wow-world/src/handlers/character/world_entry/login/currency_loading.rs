// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Hydrate canonical Player currencies during character login.

use super::*;

impl WorldSession {
    pub(super) async fn load_character_currencies_for_login_like_cpp(
        &mut self,
        player_lifecycle_port: &Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
        guid: ObjectGuid,
    ) -> bool {
        // C++ `Player::_LoadCurrency` skips rows not found in sCurrencyTypesStore.
        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::Currencies {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::Currencies(rows),
            ) => {
                let Some(mut currencies) = self.player_currencies_like_cpp() else {
                    warn!("canonical Player currency owner unavailable after currency load");
                    return false;
                };
                for row in rows {
                    let currency_id = u32::from(row.currency_id);
                    let known_currency = self
                        .currency_types_store()
                        .is_some_and(|store| store.has_record(currency_id));
                    if known_currency {
                        currencies.entry(currency_id).or_insert_with(|| {
                            crate::session::PlayerCurrency {
                                state: crate::session::PlayerCurrencyState::Unchanged,
                                quantity: row.quantity,
                                weekly_quantity: row.weekly_quantity,
                                tracked_quantity: row.tracked_quantity,
                                increased_cap_quantity: row.increased_cap_quantity,
                                earned_quantity: row.earned_quantity,
                                flags: row.flags,
                            }
                        });
                    }
                }
                let currency_count = currencies.len();
                if !self.set_player_currencies_like_cpp(currencies) {
                    warn!("canonical Player currency owner unavailable after currency load");
                    return false;
                }
                info!("Loaded {} currencies for {:?}", currency_count, guid);
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to load currencies for {:?}: {}", guid, reason);
            }
            _ => unreachable!("currency request returned a different row family"),
        }
        true
    }
}
