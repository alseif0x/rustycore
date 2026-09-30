//! Original APP builders shared by the moved melee contracts.
use super::*;
use wow_world::session::directory::{PlayerDirectoryIdentityLikeCpp, PlayerDirectoryPlacementLikeCpp, PlayerSessionRegistrationLikeCpp};

pub(super) fn broadcast_info_with_command(
    guid: ObjectGuid,
    send_tx: flume::Sender<Vec<u8>>,
    command_tx: flume::Sender<SessionCommand>,
) -> PlayerSessionRegistrationLikeCpp {
    PlayerSessionRegistrationLikeCpp {
        identity: PlayerDirectoryIdentityLikeCpp::new(
            format!("Player{}", guid.counter()),
            guid.counter() as u32,
            0,
            1,
            1,
            0,
            2,
        ),
        placement: PlayerDirectoryPlacementLikeCpp {
            map_id: 0,
            instance_id: 0,
            position: Position::ZERO,
            is_in_world: true,
            level: 1,
            is_alive: true,
        },
        active_loot_rolls: Vec::new(),
        realm_send_tx: send_tx.clone(),
        send_tx,
        command_tx,
        session_phase_tx: wow_world::session::directory::detached_session_phase_rail_like_cpp(),
        durable_creature_runtime_commands_like_cpp: Default::default(),
        client_visible_guids_like_cpp: Default::default(),
        client_visible_transports_like_cpp: Default::default(),
        advanced_combat_logging_enabled_like_cpp: Default::default(),
        visibility_refresh_pending_like_cpp: Default::default(),
    }
}

pub(super) fn damage_aura_spell_like_cpp(
    spell_id: i32,
    aura_type: i32,
    amount: i32,
    misc_value: i32,
) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_base_points: amount,
        effect_bonus_coefficient: 0.0,
        aura_type: Some(aura_type),
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: vec![wow_data::SpellEffectInfo {
            effect_index: 0,
            effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_aura: aura_type,
            effect_misc_value_1: misc_value,
            effect_base_points: amount,
            ..Default::default()
        }],
    }
}

pub(super) fn attach_share_test_player_like_cpp(
    session: &mut WorldSession,
    canonical: &SharedCanonicalMapManager,
    map_store: Arc<wow_data::MapStore>,
    guid: ObjectGuid,
    name: &str,
) {
    session.set_canonical_map_manager(Arc::clone(canonical));
    session.set_map_store(map_store);
    session.fixture_melee_attach_player_controller(SessionPlayerController::new(
        guid,
        name.to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.fixture_melee_ensure_world_map();
    session
        .fixture_melee_mutate_player(|player| {
            player.unit_mut().set_max_health(100);
            player.unit_mut().set_health(100);
            let mut stats = *player.effective_combat_stats_like_cpp();
            stats.dodge_pct = 0.0;
            stats.parry_pct = 0.0;
            stats.block_pct = 0.0;
            player.replace_effective_combat_stats_like_cpp(stats);
        })
        .unwrap();
}

pub(super) fn set_player_aura_caster_like_cpp(session: &mut WorldSession, spell_id: i32, caster: ObjectGuid) {
    session
        .fixture_melee_mutate_auras(|auras| {
            let slot = auras
                .runtime_applications_like_cpp()
                .iter()
                .find_map(|(slot, aura)| (aura.spell_id == spell_id).then_some(*slot))
                .expect("aura slot");
            auras
                .runtime_application_mut_like_cpp(slot)
                .expect("aura application")
                .caster_guid = caster;
        })
        .unwrap();
}

