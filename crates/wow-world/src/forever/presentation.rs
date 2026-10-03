//! Account snapshot -> ordered target initialization, no SQL or fake players.
use super::{CharacterCatalog, Identity, InitializationPolicy, Outgoing, SessionError};
use wow_data::HotfixBlobCache;
use wow_packet::forever as packet;
use wow_persistence::forever::{AccountSnapshot, GLOBAL_CACHE_MASK};

pub(super) fn initialize(
    identity: &Identity,
    catalog: &CharacterCatalog,
    policy: &InitializationPolicy,
    snapshot: &AccountSnapshot,
    hotfixes: &HotfixBlobCache,
    time: i64,
) -> Result<Vec<Outgoing>, SessionError> {
    let codec = |_| SessionError::Codec;
    let auth = packet::AuthResponsePayload {
        result: 0,
        wait_info: None,
        success_info: Some(packet::AuthSuccessInfo {
            active_expansion_level: 0,
            account_expansion_level: identity.account_expansion,
            time_rested: 0,
            virtual_realm_address: identity.realm_address,
            time_seconds_until_pc_kick: 0,
            currency_id: 0,
            time,
            game_time: packet::GameTime {
                billing_type: 0,
                minutes_remaining: 0,
                real_billing_type: 0,
                is_in_igr: false,
                is_paid_for_by_igr: false,
                is_cais_enabled: false,
            },
            virtual_realms: vec![packet::VirtualRealmInfo {
                realm_address: identity.realm_address,
                is_local: true,
                is_internal_realm: false,
                realm_name_actual: policy.realm_name.clone(),
                realm_name_normalized: policy.normalized_realm_name.clone(),
            }],
            available_classes: catalog
                .races()
                .iter()
                .map(|race| packet::RaceClassAvailability {
                    race_id: race.id,
                    classes: race
                        .classes
                        .iter()
                        .map(|class| packet::ClassAvailability {
                            class_id: class.id,
                            active_expansion_level: class.active_expansion,
                            account_expansion_level: class.account_expansion,
                            min_active_expansion_level: class.minimum_active_expansion,
                        })
                        .collect(),
                })
                .collect(),
            // AuthHandler.cpp:53-55, CharacterTemplateDataStore and the same
            // immutable records used by GetStartLevel. No fabricated labels.
            templates: if identity.permissions.use_character_templates() {
                policy
                    .character_templates
                    .iter()
                    .map(|template| packet::CharacterTemplate {
                        template_set_id: template.id(),
                        classes: template
                            .classes()
                            .iter()
                            .map(|class| packet::CharacterTemplateClass {
                                class_id: class.class(),
                                faction_group: class.faction_group(),
                            })
                            .collect(),
                        name: template.name().to_owned(),
                        description: template.description().to_owned(),
                    })
                    .collect()
            } else {
                vec![]
            },
            is_expansion_trial: false,
            force_character_template: false,
            num_players_horde: None,
            num_players_alliance: None,
            expansion_trial_expiration: None,
            current_build: None,
        }),
    }
    .encode_payload()
    .map_err(codec)?;
    let timezone = packet::SetTimeZoneInformation {
        server_time_tz: policy.timezone.clone(),
        game_time_tz: policy.timezone.clone(),
        server_regional_time_tz: policy.timezone.clone(),
    }
    .encode_payload()
    .map_err(codec)?;
    // Explicit isolated policy: no shop/commerce/support integrations. Defaults
    // below are SystemPackets.h, not zero-filled guesses at required fields.
    let glue = packet::ClassicGlueScreen70009 {
        commerce_price_poll_time_seconds: 0,
        redeem_for_balance_amount: 0,
        max_characters_on_realm: policy.max_characters,
        active_boost_type: 0,
        trial_boost_type: 0,
        minimum_expansion_level: 0,
        maximum_expansion_level: 0,
        content_set_id: policy.content_set,
        // Pinned Classic Write explicitly ignores AuthHandler's [8] list.
        available_game_mode_ids: vec![],
        active_timerunning_season_id: 0,
        remaining_timerunning_season_seconds: 0,
        timerunning_conversion_min_character_age: 86400,
        timerunning_conversion_max_season_id: -1,
        max_player_guid_lookups_per_request: 50,
        name_lookup_telemetry_interval: 600,
        not_found_cache_time_seconds: 10,
        most_recent_time_event_id: 0,
        event_realm_queues: 0,
    }
    .encode_payload()
    .map_err(codec)?;
    let hotfixes = packet::AvailableHotfixes {
        virtual_realm_address: identity.realm_address as i32,
        hotfixes: hotfixes
            .available_hotfix_ids("esES")
            .into_iter()
            .map(|id| packet::HotfixId {
                push_id: id.push_id,
                unique_id: id.unique_id,
            })
            .collect(),
    }
    .encode_payload()
    .map_err(codec)?;
    let times = packet::AccountDataTimes {
        player_guid: wow_core::ObjectGuid::EMPTY,
        server_time: time,
        account_times: std::array::from_fn(|index| {
            if GLOBAL_CACHE_MASK & (1 << index) != 0 {
                snapshot.account_data[index].time
            } else {
                0
            }
        }),
    }
    .encode_payload();
    // 02245dcd WorldSession.cpp::InitializeSessionCallback source order.
    Ok(vec![
        Outgoing::new(0x460001, auth),
        Outgoing::new(0x460123, timezone),
        Outgoing::new(0x460064, glue),
        Outgoing::new(
            0x4A000E,
            packet::ClientCacheVersion {
                cache_version: policy.cache_version,
            }
            .encode_payload(),
        ),
        Outgoing::new(0x4A0001, hotfixes),
        Outgoing::new(0x4601B5, times),
        Outgoing::new(
            0x460268,
            packet::TutorialFlags {
                values: snapshot.tutorials,
            }
            .encode_payload(),
        ),
        Outgoing::new(
            0x4602B1,
            packet::ConnectionStatus {
                state: 1,
                // BattlenetPackets.h default; InitializeSessionCallback only sets State.
                suppress_notification: true,
            }
            .encode_payload()
            .map_err(codec)?,
        ),
    ])
}
