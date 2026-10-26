use super::*;
use std::sync::Arc;

pub(super) struct GhostFixture {
    pub(super) session: crate::session::WorldSession,
    pub(super) _input: flume::Sender<wow_packet::WorldPacket>,
    pub(super) canonical: SharedCanonicalMapManager,
    pub(super) player: ObjectGuid,
    pub(super) send_rx: flume::Receiver<Vec<u8>>,
}

pub(super) fn aura_spell(
    spell_id: i32,
    effects: impl IntoIterator<Item = (i32, i32, i32)>,
) -> wow_data::SpellInfo {
    let effects = effects
        .into_iter()
        .enumerate()
        .map(
            |(index, (aura_type, base_points, misc_value))| wow_data::SpellEffectInfo {
                effect_index: index as u32,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: aura_type,
                effect_base_points: base_points,
                effect_die_sides: 1,
                effect_misc_value_1: misc_value,
                ..Default::default()
            },
        )
        .collect::<Vec<_>>();
    let first = effects.first();
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_base_points: first.map_or(0, |effect| effect.effect_base_points),
        effect_bonus_coefficient: 0.0,
        aura_type: first.map(|effect| effect.effect_aura),
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects,
    }
}

pub(super) fn selected_aura_attributes(spell_id: i32) -> [u32; 15] {
    let mut attributes = [0; 15];
    attributes[0] = 0x2D80_0100;
    attributes[1] = if spell_id == WISP {
        0x1000_0400
    } else {
        0x0000_0400
    };
    attributes[3] = 0x0010_0000;
    attributes
}

pub(super) fn install_spell_catalog(
    session: &mut crate::session::WorldSession,
    extras: impl IntoIterator<Item = wow_data::SpellInfo>,
) {
    let ghost = aura_spell(
        GHOST,
        [
            (wow_data::spell::aura_types::SPELL_AURA_GHOST, -21, 0),
            (
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
                49,
                0,
            ),
            (
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
                49,
                0,
            ),
        ],
    );
    let wisp = aura_spell(
        WISP,
        [
            (
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
                74,
                0,
            ),
            (
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
                74,
                0,
            ),
            (
                wow_data::spell::aura_types::SPELL_AURA_TRANSFORM,
                -1,
                WISP_TEMPLATE as i32,
            ),
        ],
    );

    let mut store = crate::selected_spell_fixtures::canonical_spell_store([
        (GHOST as u32, 0),
        (WISP as u32, 0),
    ]);
    let mut misc_rows = Vec::new();
    let mut spell_level_rows = Vec::new();
    for spell in [ghost, wisp] {
        let spell_id = spell.spell_id;
        spell_level_rows.push(wow_data::SpellLevelsEntry {
            id: spell_id as u32,
            difficulty_id: 0,
            base_level: 0,
            max_level: 0,
            spell_level: 239,
            max_passive_aura_level: 0,
            spell_id: spell_id as u32,
        });
        let attributes = selected_aura_attributes(spell_id);
        store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
        misc_rows.push(wow_data::SpellMiscEntry {
            id: spell_id as u32,
            attributes: attributes.map(|value| value as i32),
            difficulty_id: 0,
            casting_time_index: 0,
            duration_index: 21,
            range_index: 0,
            school_mask: 0,
            speed: 0.0,
            launch_delay: 0.0,
            min_duration: 0.0,
            spell_icon_file_data_id: 0,
            active_icon_file_data_id: 0,
            content_tuning_id: 0,
            show_future_spell_player_condition_id: 0,
            spell_id: spell_id as u32,
        });
        store.insert(spell_id, spell);
    }
    for spell in extras {
        store.insert(spell.spell_id, spell);
    }
    session.set_spell_store(Arc::new(store));
    session.set_spell_levels_store(Arc::new(wow_data::SpellLevelsStore::from_entries(
        spell_level_rows,
    )));
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries(misc_rows)));
    session.set_spell_duration_store(Arc::new(wow_data::SpellDurationStore::from_entries([
        wow_data::SpellDurationEntry {
            id: 21,
            duration: -1,
            duration_per_level: 0,
            max_duration: -1,
        },
    ])));
    session
        .set_spell_aura_options_store(Arc::new(wow_data::SpellAuraOptionsStore::from_entries([])));
    session.set_spell_proc_store(Arc::new(wow_data::SpellProcStoreLikeCpp::default()));
    session.set_spell_class_options_store(Arc::new(
        wow_data::SpellClassOptionsStore::from_entries([]),
    ));
    session.set_spell_label_store(Arc::new(wow_data::SpellLabelStore::from_entries([])));
    crate::session::install_condition_presence_catalog(
        session,
        Arc::new(wow_data::ConditionPresenceCatalog::from_effective_ids(
            [],
            [],
        )),
    );
    crate::session::install_serverside_spell_catalog(
        session,
        Arc::new(wow_data::ServersideSpellStoreLikeCpp::default()),
    );
}

pub(super) fn install_wisp_templates(
    session: &mut crate::session::WorldSession,
    templates: &[(u32, u32)],
) {
    let template_rows =
        templates
            .iter()
            .map(|(entry, display)| wow_data::CreatureTemplateLifecycleRecordLikeCpp {
                entry: *entry,
                name: format!("Creature {entry}"),
                scale: 1.0,
                models: vec![wow_data::CreatureTemplateLifecycleModelLikeCpp {
                    creature_display_id: *display,
                    display_scale: 1.0,
                    probability: 1.0,
                }],
                ..Default::default()
            });
    let model_rows = templates
        .iter()
        .map(|(_, display)| wow_data::CreatureModelInfoLikeCpp {
            display_id: *display,
            bounding_radius: 0.5,
            combat_reach: 1.5,
            display_id_other_gender: 0,
            is_trigger: false,
        });
    session.set_creature_template_lifecycle_store_like_cpp(Arc::new(
        wow_data::CreatureTemplateLifecycleStoreLikeCpp::from_templates(template_rows),
    ));
    session.set_creature_model_info_store(Arc::new(
        wow_data::CreatureModelInfoStoreLikeCpp::from_entries(model_rows),
    ));
}

pub(super) fn fixture() -> GhostFixture {
    let (mut session, input, send_rx) = make_session();
    let player = ObjectGuid::create_player(1, 96_110);
    session.set_player_guid(Some(player));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_shapeshift_display_links_absent(true);
    session.set_creature_aggro_config_like_cpp(crate::session::CreatureAggroConfigLikeCpp {
        spell_x_spell_visual_store: Some(Arc::new(wow_data::SpellXSpellVisualStore::from_entries(
            [wow_data::SpellXSpellVisualEntry {
                id: GHOST_VISUAL_ROW,
                difficulty_id: 0,
                spell_visual_id: 886,
                probability: 1.0,
                flags: 0,
                priority: 0,
                spell_icon_file_id: 0,
                active_icon_file_id: 0,
                viewer_unit_condition_id: 0,
                viewer_player_condition_id: 0,
                caster_unit_condition_id: 0,
                caster_player_condition_id: 0,
                spell_id: GHOST as u32,
            }],
        ))),
        ..Default::default()
    });
    install_spell_catalog(&mut session, []);
    install_wisp_templates(&mut session, &[(WISP_TEMPLATE, 1_825)]);
    let canonical = session
        .core
        .canonical_map_manager
        .as_ref()
        .cloned()
        .expect("canonical Player manager");
    {
        let mut manager = canonical.lock().expect("canonical manager");
        manager
            .find_map_mut(571, 0)
            .expect("Player map")
            .map_mut()
            .get_typed_player_mut(player)
            .expect("canonical Player")
            .unit_mut()
            .set_display_id(1_000, true);
        let _ = manager.update(1);
    }
    session.core.client_visible_guids_like_cpp.insert(player);
    let _ = send_rx.try_iter().collect::<Vec<_>>();
    GhostFixture {
        session,
        _input: input,
        canonical,
        player,
        send_rx,
    }
}

pub(super) async fn cast_selected(
    fixture: &mut GhostFixture,
    spell_id: i32,
    original: Option<ObjectGuid>,
) {
    let generators = fixture.session.id_generators_for_test_like_cpp();
    let spawns = fixture.session.creature_spawn_catalogs_for_test_like_cpp();
    wow_world_spell::execute_server_triggered_spell_like_cpp(
        &mut fixture.session,
        generators.item.as_ref(),
        &spawns,
        spell_id,
        fixture.player,
        SpellCastMetadata {
            triggered_ignores_global_cooldown_like_cpp: true,
            original_caster_guid: original,
            ..Default::default()
        },
    )
    .await
    .expect("production server-triggered ghost aura cast");
}
