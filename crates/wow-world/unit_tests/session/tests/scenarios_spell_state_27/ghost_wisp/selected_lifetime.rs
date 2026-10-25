use super::*;

fn install_race(session: &mut crate::session::WorldSession, sickness_spell_id: i32) {
    session.set_chr_races_store(std::sync::Arc::new(
        wow_data::character_progression::ChrRacesStore::from_entries([
            wow_data::character_progression::ChrRacesEntry {
                id: 1,
                client_prefix: String::new(),
                client_file_string: String::new(),
                name: "test race".to_owned(),
                flags: 0,
                male_display_id: 0,
                female_display_id: 0,
                high_res_male_display_id: 0,
                high_res_female_display_id: 0,
                res_sickness_spell_id: sickness_spell_id,
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
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 10, 0);
}

fn set_application_duration(
    fixture: &GhostFixture,
    spell_id: i32,
    duration_total: i32,
    duration_remaining: i32,
) -> (u8, std::time::Instant) {
    fixture
        .session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            let auras = &mut player.unit_mut().subsystems_mut().auras;
            let slot = auras
                .runtime_applications_like_cpp()
                .iter()
                .find_map(|(&slot, aura)| (aura.spell_id == spell_id).then_some(slot))
                .expect("selected Aura slot");
            let aura = auras
                .runtime_application_mut_like_cpp(slot)
                .expect("selected Aura application");
            aura.duration_total = duration_total;
            aura.duration_remaining = duration_remaining;
            (slot, aura.applied_at)
        })
        .expect("current canonical Player")
}

fn selected_slot(fixture: &GhostFixture, spell_id: i32) -> u8 {
    fixture
        .session
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .runtime_applications_like_cpp()
                .iter()
                .find_map(|(&slot, aura)| (aura.spell_id == spell_id).then_some(slot))
        })
        .flatten()
        .expect("selected canonical Aura slot")
}

#[tokio::test]
async fn selected_lifetime_decrements_positive_time_and_preserves_negative_time() {
    let mut fixture = fixture();
    fixture.session.core.state = crate::session::SessionState::LoggedIn;
    install_race(&mut fixture.session, 0);
    cast_selected(&mut fixture, WISP, None).await;
    cast_selected(&mut fixture, GHOST, None).await;
    outgoing(&fixture);
    let (ghost_slot, _) = set_application_duration(&fixture, GHOST, -1, 25);
    let (wisp_slot, wisp_applied_at) = set_application_duration(&fixture, WISP, -1, -1);
    let stale_applied_at = std::time::Instant::now() - std::time::Duration::from_secs(3_600);
    fixture
        .session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .insert_runtime_application_like_cpp(wow_entities::AuraApplicationLikeCpp {
                    spell_id: 99_900,
                    difficulty_id: 0,
                    caster_guid: fixture.player,
                    slot: 200,
                    duration_total: 10,
                    duration_remaining: 3,
                    stack_count: 1,
                    aura_flags: 0,
                    effect_mask: 0,
                    aura_interrupt_flags: 0,
                    aura_interrupt_flags2: 0,
                    represented_effect: None,
                    represented_amount: 0,
                    represented_effect_amounts: Vec::new(),
                    represented_misc_value: None,
                    represented_multiplier: 1.0,
                    applied_at: std::time::Instant::now(),
                });
        })
        .expect("current canonical Player");
    fixture
        .session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .runtime_application_mut_like_cpp(ghost_slot)
                .expect("selected Ghost application")
                .applied_at = stale_applied_at;
        })
        .expect("current canonical Player");

    let first = wow_world_spell::tick_selected_aura_lifetimes(&mut fixture.session, 20)
        .expect("selected Aura countdown");
    assert_eq!(first.decremented, 1);
    assert_eq!(first.expired, 0);
    fixture
        .session
        .core
        .with_owned_player_like_cpp(|player| {
            let auras = &player.unit().subsystems().auras;
            assert_eq!(
                auras
                    .runtime_application_like_cpp(ghost_slot)
                    .unwrap()
                    .duration_remaining,
                5
            );
            assert_eq!(
                auras
                    .runtime_application_like_cpp(wisp_slot)
                    .unwrap()
                    .duration_remaining,
                -1
            );
            assert_eq!(
                auras
                    .runtime_application_like_cpp(200)
                    .unwrap()
                    .duration_remaining,
                3
            );
            assert_eq!(
                auras
                    .runtime_application_like_cpp(ghost_slot)
                    .unwrap()
                    .applied_at,
                stale_applied_at
            );
            assert_eq!(
                auras
                    .runtime_application_like_cpp(wisp_slot)
                    .unwrap()
                    .applied_at,
                wisp_applied_at
            );
        })
        .expect("current canonical Player");

    let wrapped = wow_world_spell::tick_selected_aura_lifetimes(&mut fixture.session, u32::MAX)
        .expect("source uint32 subtraction wraps before conversion to int32");
    assert_eq!(wrapped.decremented, 1);
    assert_eq!(wrapped.expired, 0);
    assert_eq!(
        fixture
            .session
            .core
            .with_owned_player_like_cpp(|player| player
                .unit()
                .subsystems()
                .auras
                .runtime_application_like_cpp(ghost_slot)
                .unwrap()
                .duration_remaining)
            .expect("current canonical Player"),
        6
    );

    let overrun = wow_world_spell::tick_selected_aura_lifetimes(&mut fixture.session, 25)
        .expect("ordinary overrun clamps at zero");
    assert_eq!(overrun.decremented, 1);
    assert_eq!(overrun.expired, 1);
    assert!(
        fixture
            .session
            .core
            .with_owned_player_like_cpp(|player| player
                .unit()
                .subsystems()
                .auras
                .runtime_application_like_cpp(ghost_slot)
                .is_none())
            .expect("current canonical Player")
    );
}

#[tokio::test]
async fn selected_lifetime_expires_zero_in_spell_and_application_order() {
    let mut fixture = fixture();
    fixture.session.core.state = crate::session::SessionState::LoggedIn;
    install_race(&mut fixture.session, 0);
    cast_selected(&mut fixture, WISP, None).await;
    cast_selected(&mut fixture, GHOST, None).await;
    outgoing(&fixture);
    let ghost_slot = selected_slot(&fixture, GHOST);
    let wisp_slot = selected_slot(&fixture, WISP);
    set_application_duration(&fixture, GHOST, -1, 0);
    set_application_duration(&fixture, WISP, 0, 0);

    let result = wow_world_spell::tick_selected_aura_lifetimes(&mut fixture.session, 0)
        .expect("zero remaining time expires even when diff is zero");
    assert_eq!(result.decremented, 0);
    assert_eq!(result.expired, 2);
    let packets = outgoing(&fixture);
    assert_eq!(aura_update_slots(&packets), vec![ghost_slot, wisp_slot]);
    assert!(
        fixture
            .session
            .core
            .with_owned_player_like_cpp(|player| {
                player
                    .unit()
                    .subsystems()
                    .auras
                    .runtime_applications_like_cpp()
                    .values()
                    .all(|aura| !matches!(aura.spell_id, GHOST | WISP))
            })
            .expect("current canonical Player")
    );
}

#[tokio::test]
async fn selected_lifetime_missing_race_or_stale_owner_leaves_duration_unchanged() {
    let mut fixture = fixture();
    fixture.session.core.state = crate::session::SessionState::LoggedIn;
    fixture
        .session
        .set_loaded_player_identity_like_cpp(571, 1, 1, 10, 0);
    cast_selected(&mut fixture, GHOST, None).await;
    outgoing(&fixture);
    let (slot, _) = set_application_duration(&fixture, GHOST, -1, 50);
    assert!(wow_world_spell::tick_selected_aura_lifetimes(&mut fixture.session, 20).is_err());
    assert_eq!(
        fixture
            .session
            .core
            .with_owned_player_like_cpp(|player| player
                .unit()
                .subsystems()
                .auras
                .runtime_application_like_cpp(slot)
                .unwrap()
                .duration_remaining)
            .expect("current canonical Player"),
        50
    );

    install_race(&mut fixture.session, 0);
    let handle = fixture
        .session
        .core
        .player_handle_like_cpp
        .expect("canonical Player handle");
    let retired = fixture
        .canonical
        .lock()
        .expect("canonical map manager")
        .retire_player_like_cpp(handle)
        .expect("retire Player and make Session handle stale");
    assert!(wow_world_spell::tick_selected_aura_lifetimes(&mut fixture.session, 20).is_err());
    assert_eq!(
        retired
            .unit()
            .subsystems()
            .auras
            .runtime_application_like_cpp(slot)
            .unwrap()
            .duration_remaining,
        50
    );
}
