use super::*;
use crate::session::tests::lifecycle_persistence::RecordingPortLikeCpp;
use wow_persistence::PersistenceOutcomeLikeCpp;

fn install_selected_sickness_snapshot(session: &mut WorldSession) -> i32 {
    const SPELL_ID: i32 = 74_321;
    let player_guid = session.player_guid().expect("canonical Player GUID");
    session.set_loaded_player_identity_like_cpp(571, 2, 5, 80, 0);
    session
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_race(2);
            player.set_player_aura_authority_complete_like_cpp(true);
        })
        .expect("canonical Player owner");
    session.set_chr_races_store(Arc::new(
        wow_data::character_progression::ChrRacesStore::from_entries([
            wow_data::character_progression::ChrRacesEntry {
                id: 2,
                res_sickness_spell_id: SPELL_ID,
                client_prefix: String::new(),
                client_file_string: String::new(),
                name: "Recovery sickness test race".into(),
                flags: 0,
                male_display_id: 0,
                female_display_id: 0,
                high_res_male_display_id: 0,
                high_res_female_display_id: 0,
                splash_sound_id: 0,
                create_screen_file_data_id: 0,
                select_screen_file_data_id: 0,
                low_res_screen_file_data_id: 0,
                altered_form_start_visual_kit_id: [0; 3],
                altered_form_finish_visual_kit_id: [0; 3],
                heritage_armor_achievement_id: 0,
                starting_level: 1,
                ui_display_order: 0,
                playable_race_bit: 0,
                female_skeleton_file_data_id: 0,
                male_skeleton_file_data_id: 0,
                helmet_anim_scaling_race_id: 0,
                transmogrify_disabled_slot_mask: 0,
                faction_id: 0,
                cinematic_sequence_id: 0,
                base_language: 0,
                creature_type: 0,
                alliance: 0,
                race_related: 0,
                unaltered_visual_race_id: 0,
                default_class_id: 0,
                neutral_race_id: 0,
            },
        ]),
    ));

    let effect_specs = [
        (0_u32, wow_data::spell::aura_types::SPELL_AURA_MOD_PERCENT_STAT, 6),
        (
            1,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE,
            -75,
        ),
        (2, wow_data::spell::aura_types::SPELL_AURA_NO_PVP_CREDIT, 0),
    ];
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        SPELL_ID,
        wow_data::SpellInfo {
            spell_id: SPELL_ID,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: effect_specs
                .iter()
                .map(|&(effect_index, effect_aura, effect_base_points)| {
                    wow_data::SpellEffectInfo {
                        effect_index,
                        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                        effect_aura,
                        effect_base_points,
                        ..Default::default()
                    }
                })
                .collect(),
        },
    );
    spell_store.insert_spell_misc_attributes_like_cpp(SPELL_ID, [0; 15]);
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_aura_options_store(Arc::new(
        wow_data::SpellAuraOptionsStore::from_entries([]),
    ));
    session.set_difficulty_store(Arc::new(wow_data::DifficultyStore::from_ids([0])));
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (2, 5, 80),
        wow_data::PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect: 20,
            spirit: 30,
            base_mana: 1_000,
        },
    )])));

    let effects = effect_specs
        .iter()
        .map(|&(index, aura_type, amount)| (1_u32 << index, aura_type, amount, 0))
        .collect::<Vec<_>>();
    let snapshot = wow_entities::AuraThreatSnapshotLikeCpp::new([0; 2], effects);
    let slot = 3;
    let provenance = wow_entities::AuraCastProvenanceLikeCpp {
        cast_id: ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Cast,
            3,
            1,
            571,
            0,
            SPELL_ID as u32,
            74_321,
        ),
        spell_visual_id: 0,
        caster_level: Some(80),
    };
    let application = wow_entities::AuraApplicationLikeCpp {
        spell_id: SPELL_ID,
        difficulty_id: 0,
        caster_guid: player_guid,
        slot,
        duration_total: 600_000,
        duration_remaining: 120_000,
        stack_count: 1,
        aura_flags: 1,
        effect_mask: 0b111,
        aura_interrupt_flags: 0,
        aura_interrupt_flags2: 0,
        represented_effect: None,
        represented_amount: 0,
        represented_effect_amounts: Vec::new(),
        represented_misc_value: None,
        represented_multiplier: 1.0,
        applied_at: std::time::Instant::now() - std::time::Duration::from_secs(3_600),
    };
    session
        .with_owned_player_mut_like_cpp(|player| {
            player.apply_player_threat_aura_like_cpp(
                SPELL_ID as u32,
                player_guid,
                slot,
                snapshot.clone(),
            );
            player.install_player_threat_aura_like_cpp(slot, snapshot, application);
            let auras = &mut player.unit_mut().subsystems_mut().auras;
            assert!(auras.set_aura_persistence_metadata(
                slot,
                wow_entities::AuraPersistenceMetadata {
                    item_guid: ObjectGuid::EMPTY,
                    cast_item_id: 0,
                    cast_item_level: -1,
                    recalculate_mask: 0,
                    charges: 0,
                    base_amounts: vec![(0, 6), (1, -75), (2, 0)],
                },
            ));
            auras.set_aura_cast_provenance_like_cpp(slot, provenance);
            auras.visible_auras_to_update.insert(slot);
        })
        .expect("canonical selected sickness application");
    SPELL_ID
}

#[tokio::test]
async fn selected_recovery_periodic_save_captures_exact_duration_before_death_timer() {
    let (mut session, admission, _packet_tx, _send_rx) = logged_in_recovery_session();
    install_recovery_save_snapshot_inputs(&mut session);
    let sickness_spell = install_selected_sickness_snapshot(&mut session);
    session.set_player_save_interval_ms_like_cpp(1);
    let port = RecordingPortLikeCpp::new(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    let lifecycle_port: Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp> = port.clone();
    session.set_player_lifecycle_port_like_cpp(lifecycle_port);

    let reply = run_recovery_request(
        &mut session,
        admission,
        SessionPhasePermitLikeCpp::new_like_cpp(),
    )
    .await;

    assert_eq!(reply.status, Some(PlayerRecoveryStatus::Completed));
    let requests = port.character_save_requests();
    assert_eq!(requests.len(), 1, "Recovery performs the due periodic save");
    let sickness = requests[0]
        .resurrection_sickness
        .as_ref()
        .expect("complete selected sickness snapshot");
    assert_eq!(sickness.spell_id, sickness_spell as u32);
    assert_eq!(sickness.families.len(), 1);
    assert_eq!(sickness.families[0].auras.len(), 1);
    assert_eq!(sickness.families[0].auras[0].max_duration, 600_000);
    assert_eq!(
        sickness.families[0].auras[0].remaining_duration,
        120_000 - admission.diff_ms as i32,
        "Save captures Unit's exact countdown without applied_at elapsed time"
    );
    assert_eq!(
        death_timer(&session),
        300_000 - admission.diff_ms,
        "Player death timer advances after the periodic snapshot"
    );
}
