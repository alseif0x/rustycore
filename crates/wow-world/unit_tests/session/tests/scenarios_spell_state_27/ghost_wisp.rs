use super::*;
use std::sync::Arc;
use std::time::{Duration, Instant};

const GHOST: i32 = 8_326;
const WISP: i32 = 20_584;
const WISP_TEMPLATE: u32 = 12_861;
const NEGATIVE_TRANSFORM: i32 = 99_011;
const FORM_SPELL: i32 = 99_012;
const GHOST_VISUAL_ROW: u32 = 241_720;
const PLAYER_DISPLAY_BIT: usize = wow_entities::UNIT_DATA_DISPLAY_ID_BIT;
const PLAYER_FLAGS_BIT: usize = wow_entities::PLAYER_DATA_FLAGS_BIT;

struct GhostFixture {
    session: crate::session::WorldSession,
    canonical: SharedCanonicalMapManager,
    player: ObjectGuid,
    send_rx: flume::Receiver<Vec<u8>>,
}

fn aura_spell(
    spell_id: i32,
    effects: impl IntoIterator<Item = (i32, i32, i32)>,
) -> wow_data::SpellInfo {
    let effects = effects
        .into_iter()
        .enumerate()
        .map(|(index, (aura_type, base_points, misc_value))| wow_data::SpellEffectInfo {
            effect_index: index as u32,
            effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_aura: aura_type,
            effect_base_points: base_points,
            effect_die_sides: 1,
            effect_misc_value_1: misc_value,
            ..Default::default()
        })
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

fn selected_aura_attributes(spell_id: i32) -> [u32; 15] {
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

fn install_spell_catalog(
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

    let mut store = wow_data::SpellStore::new();
    let mut misc_rows = Vec::new();
    for spell in [ghost, wisp] {
        let spell_id = spell.spell_id;
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
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries(misc_rows)));
    session.set_spell_duration_store(Arc::new(wow_data::SpellDurationStore::from_entries([
        wow_data::SpellDurationEntry {
            id: 21,
            duration: -1,
            duration_per_level: 0,
            max_duration: -1,
        },
    ])));
}

fn install_wisp_templates(
    session: &mut crate::session::WorldSession,
    templates: &[(u32, u32)],
) {
    let template_rows = templates.iter().map(|(entry, display)| {
        wow_data::CreatureTemplateLifecycleRecordLikeCpp {
            entry: *entry,
            name: format!("Creature {entry}"),
            scale: 1.0,
            models: vec![wow_data::CreatureTemplateLifecycleModelLikeCpp {
                creature_display_id: *display,
                display_scale: 1.0,
                probability: 1.0,
            }],
            ..Default::default()
        }
    });
    let model_rows = templates.iter().map(|(_, display)| {
        wow_data::CreatureModelInfoLikeCpp {
            display_id: *display,
            bounding_radius: 0.5,
            combat_reach: 1.5,
            display_id_other_gender: 0,
            is_trigger: false,
        }
    });
    session.set_creature_template_lifecycle_store_like_cpp(Arc::new(
        wow_data::CreatureTemplateLifecycleStoreLikeCpp::from_templates(template_rows),
    ));
    session.set_creature_model_info_store(Arc::new(
        wow_data::CreatureModelInfoStoreLikeCpp::from_entries(model_rows),
    ));
}

fn fixture() -> GhostFixture {
    let (mut session, _, send_rx) = make_session();
    let player = ObjectGuid::create_player(1, 96_110);
    session.set_player_guid(Some(player));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_shapeshift_display_links_absent(true);
    session.set_creature_aggro_config_like_cpp(crate::session::CreatureAggroConfigLikeCpp {
        spell_x_spell_visual_store: Some(Arc::new(
            wow_data::SpellXSpellVisualStore::from_entries([wow_data::SpellXSpellVisualEntry {
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
            }]),
        )),
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
        canonical,
        player,
        send_rx,
    }
}

async fn cast_selected(fixture: &mut GhostFixture, spell_id: i32, original: Option<ObjectGuid>) {
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

fn outgoing(fixture: &GhostFixture) -> Vec<Vec<u8>> {
    fixture.send_rx.try_iter().collect()
}

fn opcode(bytes: &[u8]) -> u16 {
    wow_packet::WorldPacket::from_bytes(bytes)
        .read_uint16()
        .expect("packet opcode")
}

fn aura_update_slots(packets: &[Vec<u8>]) -> Vec<u8> {
    packets
        .iter()
        .filter_map(|bytes| {
            let mut packet = wow_packet::WorldPacket::from_bytes(bytes);
            if packet.read_uint16().ok()? != ServerOpcodes::AuraUpdate as u16 {
                return None;
            }
            let _update_all = packet.read_bit().ok()?;
            let count = packet.read_bits(9).ok()?;
            assert_eq!(count, 1, "each selected AuraApplication publishes one slot");
            Some(packet.read_uint8().expect("AuraInfo slot"))
        })
        .collect()
}

fn spell_go_visual_rows(packets: &[Vec<u8>]) -> Vec<(i32, u32)> {
    packets
        .iter()
        .filter_map(|bytes| {
            let mut packet = wow_packet::WorldPacket::from_bytes(bytes);
            if packet.read_uint16().ok()? != ServerOpcodes::SpellGo as u16 {
                return None;
            }
            for _ in 0..4 {
                packet.read_packed_guid().ok()?;
            }
            let spell_id = packet.read_int32().ok()?;
            let visual = wow_packet::packets::spell::SpellCastVisual::read(&mut packet).ok()?;
            Some((spell_id, visual.spell_visual_id))
        })
        .collect()
}

fn aura_update_visual_rows(packets: &[Vec<u8>]) -> Vec<(i32, i32)> {
    packets
        .iter()
        .filter_map(|bytes| {
            let mut packet = wow_packet::WorldPacket::from_bytes(bytes);
            if packet.read_uint16().ok()? != ServerOpcodes::AuraUpdate as u16 {
                return None;
            }
            let _update_all = packet.read_bit().ok()?;
            let count = packet.read_bits(9).ok()?;
            assert_eq!(count, 1, "tick publishes each selected aura as a slot delta");
            let _slot = packet.read_uint8().ok()?;
            if !packet.read_bit().ok()? {
                return None;
            }
            packet.read_packed_guid().ok()?;
            let spell_id = packet.read_int32().ok()?;
            let visual_id = packet.read_int32().ok()?;
            Some((spell_id, visual_id))
        })
        .collect()
}

fn player_speed(fixture: &GhostFixture, move_type: UnitMoveTypeLikeCpp) -> f32 {
    fixture.session.player_movement_speed_like_cpp(move_type)
}

fn player_display(fixture: &GhostFixture) -> u32 {
    let manager = fixture.canonical.lock().expect("canonical manager");
    let display = manager
        .find_map(571, 0)
        .expect("Player map")
        .map()
        .get_typed_player(fixture.player)
        .expect("canonical Player")
        .unit()
        .data()
        .display_id;
    u32::try_from(display).unwrap_or_default()
}

fn aura_amount(fixture: &GhostFixture, slot: u8, aura_type: i32) -> Option<i32> {
    let manager = fixture.canonical.lock().expect("canonical manager");
    let auras = &manager
        .find_map(571, 0)?
        .map()
        .get_typed_player(fixture.player)?
        .unit()
        .subsystems()
        .auras;
    auras
        .applied_aura_types
        .get(&aura_type)?
        .iter()
        .find(|aura| aura.slot == slot)
        .and_then(|aura| auras.applied_aura_amounts.get(aura).copied())
}

fn is_ghost_flag_set(fixture: &GhostFixture) -> bool {
    let manager = fixture.canonical.lock().expect("canonical manager");
    manager
        .find_map(571, 0)
        .expect("Player map")
        .map()
        .get_typed_player(fixture.player)
        .expect("canonical Player")
        .has_player_flag(wow_world_core::session::PLAYER_FLAGS_GHOST_LIKE_CPP)
}

fn insert_map_player(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    position: Position,
) {
    let mut player = Box::new(wow_entities::Player::new(Some(1), false));
    player.unit_mut().world_mut().object_mut().create(guid);
    player.unit_mut().world_mut().set_map(571, 0).expect("Player map");
    player.unit_mut().world_mut().relocate(position);
    player.unit_mut().world_mut().object_mut().add_to_world();
    canonical
        .lock()
        .expect("canonical manager")
        .find_map_mut(571, 0)
        .expect("Player map")
        .map_mut()
        .add_map_object_record_to_map_like_cpp(
            wow_entities::MapObjectRecord::new_player(*player).expect("Player map record"),
        )
        .expect("Player insertion");
}

#[tokio::test]
async fn server_ghost_and_wisp_casts_apply_full_permanent_auras_and_publish_in_slot_order() {
    let mut fixture = fixture();
    let base_run = player_speed(&fixture, UnitMoveTypeLikeCpp::Run);
    let base_swim = player_speed(&fixture, UnitMoveTypeLikeCpp::Swim);

    cast_selected(&mut fixture, WISP, None).await;
    let wisp_packets = outgoing(&fixture);
    assert!(!wisp_packets
        .iter()
        .map(|packet| opcode(packet))
        .any(|opcode| opcode == ServerOpcodes::SpellGo as u16));
    assert!(aura_update_slots(&wisp_packets).is_empty());
    assert_eq!(player_display(&fixture), 1_825);

    cast_selected(&mut fixture, GHOST, None).await;
    let cast_packets = outgoing(&fixture);
    assert!(cast_packets
        .iter()
        .map(|packet| opcode(packet))
        .any(|opcode| opcode == ServerOpcodes::SpellGo as u16));
    assert_eq!(
        spell_go_visual_rows(&cast_packets),
        vec![(GHOST, GHOST_VISUAL_ROW)]
    );
    assert!(aura_update_slots(&cast_packets).is_empty());
    assert!(is_ghost_flag_set(&fixture));
    assert!((player_speed(&fixture, UnitMoveTypeLikeCpp::Run) - base_run * 1.75).abs() < 0.0001);
    assert!((player_speed(&fixture, UnitMoveTypeLikeCpp::Swim) - base_swim * 1.75).abs() < 0.0001);
    assert_eq!(aura_amount(&fixture, 0, wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED), Some(75));
    assert_eq!(aura_amount(&fixture, 1, wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED), Some(50));

    {
        let manager = fixture.canonical.lock().expect("canonical manager");
        let auras = &manager
            .find_map(571, 0)
            .expect("Player map")
            .map()
            .get_typed_player(fixture.player)
            .expect("canonical Player")
            .unit()
            .subsystems()
            .auras;
        let applications = auras.runtime_applications_like_cpp();
        let wisp = applications.get(&0).expect("Wisp application slot");
        let ghost = applications.get(&1).expect("Ghost application slot");
        for aura in [wisp, ghost] {
            assert_eq!(aura.effect_mask, 0b111);
            assert_eq!(aura.duration_total, 0, "SpellDuration 21 is permanent");
            assert_eq!(aura.duration_remaining, 0);
            assert_eq!(
                aura.aura_flags,
                if aura.spell_id == GHOST { 0x0011 } else { 0x0101 }
            );
            assert!(!auras.aura_cast_provenance_like_cpp(aura.slot).cast_id.is_empty());
        }
        assert_eq!(wisp.spell_id, WISP);
        assert_eq!(ghost.spell_id, GHOST);
        assert_eq!(auras.aura_cast_provenance_like_cpp(wisp.slot).spell_visual_id, 0);
        assert_eq!(
            auras.aura_cast_provenance_like_cpp(ghost.slot).spell_visual_id,
            GHOST_VISUAL_ROW as i32
        );
        assert_eq!(auras.visible_auras_to_update, [0, 1].into_iter().collect());
    }

    // C++ Map::SendObjectUpdates publishes the update-field owners after the
    // aura effects have mutated them; there is no direct VALUES flush here.
    let summary = {
        let mut manager = fixture.canonical.lock().expect("canonical manager");
        let _ = manager.update(1);
        manager
            .find_map(571, 0)
            .expect("Player map")
            .last_send_object_updates_summary_like_cpp()
    };
    let values = summary
        .player_values_updates
        .iter()
        .find(|update| update.guid == fixture.player)
        .expect("Map captured the canonical Player VALUES delta");
    assert!(values
        .values_update
        .player_data
        .as_ref()
        .is_some_and(|data| data.mask.is_set(PLAYER_FLAGS_BIT)));
    assert!(values
        .values_update
        .unit_data
        .as_ref()
        .is_some_and(|data| data.mask.is_set(PLAYER_DISPLAY_BIT)));
    assert_eq!(fixture.session.send_map_unit_values_updates_for_test_like_cpp(), 1);
    assert!(outgoing(&fixture)
        .iter()
        .map(|packet| opcode(packet))
        .any(|opcode| opcode == ServerOpcodes::UpdateObject as u16));

    wow_world_spell::tick_auras(&mut fixture.session);
    let update_packets = outgoing(&fixture);
    assert_eq!(aura_update_slots(&update_packets), vec![0, 1]);
    assert_eq!(
        aura_update_visual_rows(&update_packets),
        vec![(WISP, 0), (GHOST, GHOST_VISUAL_ROW as i32)]
    );
    {
        let mut manager = fixture.canonical.lock().expect("canonical manager");
        let player = manager
            .find_map_mut(571, 0)
            .expect("Player map")
            .map_mut()
            .get_typed_player_mut(fixture.player)
            .expect("canonical Player");
        let aged_at = Instant::now() - Duration::from_secs(7 * 60);
        for slot in [0, 1] {
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .runtime_application_mut_like_cpp(slot)
                .expect("active resurrection AuraApplication")
                .applied_at = aged_at;
        }
    }
    wow_world_spell::tick_auras(&mut fixture.session);
    assert!(aura_update_slots(&outgoing(&fixture)).is_empty());
    let manager = fixture.canonical.lock().expect("canonical manager");
    let auras = &manager
        .find_map(571, 0)
        .expect("Player map")
        .map()
        .get_typed_player(fixture.player)
        .expect("canonical Player")
        .unit()
        .subsystems()
        .auras;
    assert_eq!(auras.runtime_applications_like_cpp().len(), 2);
    assert!(auras
        .runtime_applications_like_cpp()
        .values()
        .all(|aura| aura.duration_total == 0 && aura.duration_remaining == 0));
    drop(manager);

    fixture.session.remove_aura(0).expect("remove Wisp");
    assert!((player_speed(&fixture, UnitMoveTypeLikeCpp::Run) - base_run * 1.5).abs() < 0.0001);
    assert!((player_speed(&fixture, UnitMoveTypeLikeCpp::Swim) - base_swim * 1.5).abs() < 0.0001);
    assert_eq!(player_display(&fixture), 1_000);
    fixture.session.remove_aura(1).expect("remove Ghost");
    assert!((player_speed(&fixture, UnitMoveTypeLikeCpp::Run) - base_run).abs() < 0.0001);
    assert!((player_speed(&fixture, UnitMoveTypeLikeCpp::Swim) - base_swim).abs() < 0.0001);
    assert!(!is_ghost_flag_set(&fixture));
}

#[tokio::test]
async fn ghost_flag_and_visibility_remain_until_the_last_caster_application_is_removed() {
    let mut fixture = fixture();
    let observer = ObjectGuid::create_player(1, 96_111);
    insert_map_player(&fixture.canonical, observer, Position::new(1.0, 1.0, 0.0, 0.0));
    let can_observer_see_player = |fixture: &GhostFixture| {
        let manager = fixture.canonical.lock().expect("canonical manager");
        let map = manager.find_map(571, 0).expect("Player map").map();
        map.get_typed_player(observer)
            .expect("observer")
            .unit()
            .can_see_or_detect_unit_like_cpp(
                &map.get_typed_player(fixture.player)
                    .expect("target")
                    .unit(),
                false,
                true,
                false,
            )
    };
    assert!(can_observer_see_player(&fixture));

    let absent_caster = ObjectGuid::create_player(1, 96_112);
    let aura_rows = [fixture.player, absent_caster].map(|caster_guid| {
        wow_world_spell::CharacterAuraRowLikeCpp {
            caster_guid,
            item_guid: ObjectGuid::EMPTY,
            spell_id: GHOST as u32,
            effect_mask: 0b111,
            recalculate_mask: 0,
            difficulty: 0,
            stack_count: 1,
            max_duration_ms: -1,
            remain_time_ms: -1,
            remain_charges: 0,
            cast_item_id: 0,
            cast_item_level: -1,
        }
    });
    let effect_rows = [fixture.player, absent_caster]
        .into_iter()
        .flat_map(|caster_guid| {
            [(-21, 0), (50, 0), (50, 0)]
                .into_iter()
                .enumerate()
                .map(move |(effect_index, (amount, base_amount))| {
                    wow_world_spell::CharacterAuraEffectRowLikeCpp {
                        caster_guid,
                        item_guid: ObjectGuid::EMPTY,
                        spell_id: GHOST as u32,
                        effect_mask: 0b111,
                        effect_index: effect_index as u8,
                        amount,
                        base_amount,
                    }
                })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        fixture
            .session
            .load_represented_character_auras_like_cpp(aura_rows, effect_rows, 0),
        2
    );
    assert!(is_ghost_flag_set(&fixture));
    assert!(!can_observer_see_player(&fixture));
    {
        let manager = fixture.canonical.lock().expect("canonical manager");
        let applications = manager
            .find_map(571, 0)
            .expect("Player map")
            .map()
            .get_typed_player(fixture.player)
            .expect("canonical Player")
            .unit()
            .subsystems()
            .auras
            .runtime_applications_like_cpp();
        assert_eq!(applications.len(), 2);
        assert_eq!(applications.get(&0).expect("self cast").aura_flags, 0x0011);
        assert_eq!(applications.get(&1).expect("missing external caster").aura_flags, 0x0010);
        assert_eq!(applications.get(&0).expect("self cast").caster_guid, fixture.player);
        assert_eq!(
            applications
                .get(&1)
                .expect("missing external caster")
                .caster_guid,
            absent_caster
        );
    }

    fixture.session.remove_aura(0).expect("remove self Ghost source");
    assert!(is_ghost_flag_set(&fixture));
    assert!(!can_observer_see_player(&fixture));
    fixture
        .session
        .remove_aura(1)
        .expect("remove missing-caster Ghost source");
    assert!(!is_ghost_flag_set(&fixture));
    assert!(can_observer_see_player(&fixture));
}

#[tokio::test]
async fn selected_resurrection_ghost_cast_rejects_external_original_caster() {
    let mut fixture = fixture();
    let observer = ObjectGuid::create_player(1, 96_113);
    insert_map_player(&fixture.canonical, observer, Position::new(1.0, 1.0, 0.0, 0.0));
    let generators = fixture.session.id_generators_for_test_like_cpp();
    let spawns = fixture.session.creature_spawn_catalogs_for_test_like_cpp();

    let result = wow_world_spell::execute_server_triggered_spell_like_cpp(
        &mut fixture.session,
        generators.item.as_ref(),
        &spawns,
        GHOST,
        fixture.player,
        SpellCastMetadata {
            triggered_ignores_global_cooldown_like_cpp: true,
            original_caster_guid: Some(observer),
            ..Default::default()
        },
    )
    .await;

    assert!(result.is_err(), "the #43 producer only supports the Player self-cast");
    assert!(!is_ghost_flag_set(&fixture));
    let manager = fixture.canonical.lock().expect("canonical manager");
    assert!(manager
        .find_map(571, 0)
        .expect("Player map")
        .map()
        .get_typed_player(fixture.player)
        .expect("canonical Player")
        .unit()
        .subsystems()
        .auras
        .runtime_applications_like_cpp()
        .is_empty());
    assert!(aura_update_slots(&outgoing(&fixture)).is_empty());
}

#[tokio::test]
async fn removing_wisp_restores_negative_transform_then_shapeshift_then_native_display() {
    let mut negative_fixture = fixture();
    let mut negative = aura_spell(
        NEGATIVE_TRANSFORM,
        [(wow_data::spell::aura_types::SPELL_AURA_TRANSFORM, 0, 12_862)],
    );
    negative.effects[0].implicit_target_1 = 2;
    let form = aura_spell(
        FORM_SPELL,
        [(wow_data::spell::aura_types::SPELL_AURA_MOD_SHAPESHIFT, 1, 1)],
    );
    install_spell_catalog(&mut negative_fixture.session, [negative, form]);
    negative_fixture.session.set_spell_shapeshift_form_store(Arc::new(
        wow_data::SpellShapeshiftFormStore::from_entries([wow_data::SpellShapeshiftFormEntry {
            id: 1,
            name: "Cat Form".into(),
            creature_type: 0,
            flags: 0,
            attack_icon_file_id: 0,
            bonus_action_bar: 0,
            combat_round_time: 1_000,
            damage_variance: 0.0,
            mount_type_id: 0,
            creature_display_id: [1_827, 0, 0, 0],
            preset_spell_id: [0; wow_data::MAX_SHAPESHIFT_SPELLS],
        }]),
    ));
    install_wisp_templates(
        &mut negative_fixture.session,
        &[(WISP_TEMPLATE, 1_825), (12_862, 1_826), (1_827, 1_827)],
    );

    cast_selected(&mut negative_fixture, WISP, None).await;
    negative_fixture
        .session
        .apply_aura(NEGATIVE_TRANSFORM, negative_fixture.player, 0, 0x0010)
        .expect("apply a negative transform application");
    negative_fixture.session.remove_aura(0).expect("remove Wisp");
    assert_eq!(
        player_display(&negative_fixture),
        1_826,
        "negative transform wins restore priority"
    );

    let mut form_fixture = fixture();
    let form = aura_spell(
        FORM_SPELL,
        [(wow_data::spell::aura_types::SPELL_AURA_MOD_SHAPESHIFT, 1, 1)],
    );
    install_spell_catalog(&mut form_fixture.session, [form]);
    install_wisp_templates(&mut form_fixture.session, &[(WISP_TEMPLATE, 1_825)]);
    form_fixture.session.set_spell_shapeshift_form_store(Arc::new(
        wow_data::SpellShapeshiftFormStore::from_entries([wow_data::SpellShapeshiftFormEntry {
            id: 1,
            name: "Cat Form".into(),
            creature_type: 0,
            flags: 0,
            attack_icon_file_id: 0,
            bonus_action_bar: 0,
            combat_round_time: 1_000,
            damage_variance: 0.0,
            mount_type_id: 0,
            creature_display_id: [1_827, 0, 0, 0],
            preset_spell_id: [0; wow_data::MAX_SHAPESHIFT_SPELLS],
        }]),
    ));
    form_fixture
        .session
        .apply_aura(FORM_SPELL, form_fixture.player, 0, 0x0100)
        .expect("apply shapeshift form");
    cast_selected(&mut form_fixture, WISP, None).await;
    form_fixture.session.remove_aura(1).expect("remove Wisp");
    assert_eq!(player_display(&form_fixture), 1_827, "form model follows transform removal");
    form_fixture.session.remove_aura(0).expect("remove form");
    cast_selected(&mut form_fixture, WISP, None).await;
    form_fixture.session.remove_aura(0).expect("remove final Wisp");
    assert_eq!(player_display(&form_fixture), 1_000, "native model is last fallback");

    let mut glyph_fixture = fixture();
    let glyph_form = aura_spell(
        24_858,
        [(wow_data::spell::aura_types::SPELL_AURA_MOD_SHAPESHIFT, 1, 1)],
    );
    let glyph_spell = aura_spell(
        114_301,
        [(wow_data::spell::aura_types::SPELL_AURA_DUMMY, 1, 0)],
    );
    install_spell_catalog(&mut glyph_fixture.session, [glyph_form, glyph_spell]);
    glyph_fixture.session.set_spell_shapeshift_form_store(Arc::new(
        wow_data::SpellShapeshiftFormStore::from_entries([wow_data::SpellShapeshiftFormEntry {
            id: 1,
            name: "Moonkin Form".into(),
            creature_type: 0,
            flags: 0,
            attack_icon_file_id: 0,
            bonus_action_bar: 0,
            combat_round_time: 1_000,
            damage_variance: 0.0,
            mount_type_id: 0,
            creature_display_id: [1_827, 0, 0, 0],
            preset_spell_id: [0; wow_data::MAX_SHAPESHIFT_SPELLS],
        }]),
    ));
    install_wisp_templates(&mut glyph_fixture.session, &[(WISP_TEMPLATE, 1_825)]);
    glyph_fixture
        .session
        .apply_aura(24_858, glyph_fixture.player, 0, 0x0100)
        .expect("apply Glyph of Stars shapeshift");
    glyph_fixture
        .session
        .apply_aura(114_301, glyph_fixture.player, 0, 0x0100)
        .expect("apply Glyph of Stars");
    cast_selected(&mut glyph_fixture, WISP, None).await;
    glyph_fixture.session.remove_aura(2).expect("remove Wisp");
    assert_eq!(
        player_display(&glyph_fixture),
        1_000,
        "GetModelForForm zero falls through to native display"
    );

    let mut mounted_fixture = fixture();
    let form = aura_spell(
        FORM_SPELL,
        [(wow_data::spell::aura_types::SPELL_AURA_MOD_SHAPESHIFT, 1, 1)],
    );
    install_spell_catalog(&mut mounted_fixture.session, [form]);
    mounted_fixture
        .session
        .set_spell_shapeshift_form_store(Arc::new(
            wow_data::SpellShapeshiftFormStore::from_entries([
                wow_data::SpellShapeshiftFormEntry {
                    id: 1,
                    name: "Cat Form".into(),
                    creature_type: 0,
                    flags: 0,
                    attack_icon_file_id: 0,
                    bonus_action_bar: 0,
                    combat_round_time: 1_000,
                    damage_variance: 0.0,
                    mount_type_id: 0,
                    creature_display_id: [1_827, 0, 0, 0],
                    preset_spell_id: [0; wow_data::MAX_SHAPESHIFT_SPELLS],
                },
            ]),
        ));
    install_wisp_templates(&mut mounted_fixture.session, &[(WISP_TEMPLATE, 1_825)]);
    mounted_fixture
        .session
        .apply_aura(FORM_SPELL, mounted_fixture.player, 0, 0x0100)
        .expect("apply shapeshift form before mounting");
    cast_selected(&mut mounted_fixture, WISP, None).await;
    {
        let mut manager = mounted_fixture
            .canonical
            .lock()
            .expect("canonical manager");
        let player = manager
            .find_map_mut(571, 0)
            .expect("Player map")
            .map_mut()
            .get_typed_player_mut(mounted_fixture.player)
            .expect("canonical Player");
        let mut unit_flags = player.unit().unit_flags_like_cpp();
        unit_flags.insert(wow_constants::UnitFlags::MOUNT);
        player.unit_mut().set_unit_flags_like_cpp(unit_flags);
    }
    mounted_fixture.session.remove_aura(1).expect("remove mounted Wisp");
    assert_eq!(
        player_display(&mounted_fixture),
        1_000,
        "mounted non-stance form is disallowed by Unit::IsDisallowedMountForm"
    );
}

#[test]
fn login_aura_loader_applies_ghost_effects_from_supplied_rows() {
    let mut fixture = fixture();
    // These rows exercise the loader's representation of C++ Player::_LoadAuras;
    // they do not establish that PlayerSave currently captures these spells.
    let aura_rows = [
        wow_world_spell::CharacterAuraRowLikeCpp {
            caster_guid: fixture.player,
            item_guid: ObjectGuid::EMPTY,
            spell_id: WISP as u32,
            effect_mask: 0b111,
            recalculate_mask: 0,
            difficulty: 0,
            stack_count: 1,
            max_duration_ms: -1,
            remain_time_ms: -1,
            remain_charges: 0,
            cast_item_id: 0,
            cast_item_level: -1,
        },
        wow_world_spell::CharacterAuraRowLikeCpp {
            caster_guid: fixture.player,
            item_guid: ObjectGuid::EMPTY,
            spell_id: GHOST as u32,
            effect_mask: 0b111,
            recalculate_mask: 0,
            difficulty: 0,
            stack_count: 1,
            max_duration_ms: -1,
            remain_time_ms: -1,
            remain_charges: 0,
            cast_item_id: 0,
            cast_item_level: -1,
        },
    ];
    let effect_rows = [
        (WISP, 0, 75, 74),
        (WISP, 1, 75, 74),
        (WISP, 2, 0, -1),
        (GHOST, 0, -20, -21),
        (GHOST, 1, 50, 49),
        (GHOST, 2, 50, 49),
    ]
    .map(|(spell_id, effect_index, amount, base_amount)| {
        wow_world_spell::CharacterAuraEffectRowLikeCpp {
            caster_guid: fixture.player,
            item_guid: ObjectGuid::EMPTY,
            spell_id: spell_id as u32,
            effect_mask: 0b111,
            effect_index,
            amount,
            base_amount,
        }
    });

    assert_eq!(
        fixture
            .session
            .load_represented_character_auras_like_cpp(aura_rows, effect_rows, 0),
        2
    );
    assert!(is_ghost_flag_set(&fixture));
    assert_eq!(player_display(&fixture), 1_825);
    let manager = fixture.canonical.lock().expect("canonical manager");
    let auras = &manager
        .find_map(571, 0)
        .expect("Player map")
        .map()
        .get_typed_player(fixture.player)
        .expect("canonical Player")
        .unit()
        .subsystems()
        .auras;
    assert_eq!(auras.runtime_applications_like_cpp().len(), 2);
    assert!(auras.runtime_applications_like_cpp().values().all(|aura| {
        aura.effect_mask == 0b111
            && aura.duration_total == 0
            && aura.aura_flags
                == if aura.spell_id == GHOST {
                    0x0011
                } else {
                    0x0101
                }
    }));
    assert_eq!(
        auras.total_aura_modifier_like_cpp(
            wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED
        ),
        125
    );
    drop(manager);

    let (state, hub) = crate::session::split_spell_state_ref(&fixture.session);
    state.send_initial_player_auras_like_cpp(hub);
    let full_packets = outgoing(&fixture);
    let initial_opcodes = full_packets.iter().map(|packet| opcode(packet)).collect::<Vec<_>>();
    assert!(initial_opcodes.contains(&(ServerOpcodes::MoveSetRunSpeed as u16)));
    assert!(initial_opcodes.contains(&(ServerOpcodes::MoveSetSwimSpeed as u16)));
    assert_eq!(
        initial_opcodes
            .iter()
            .filter(|opcode| **opcode == ServerOpcodes::AuraUpdate as u16)
            .count(),
        1,
        "login sends one full AuraUpdate alongside movement speed updates"
    );
    let full_aura_packet = full_packets
        .iter()
        .find(|packet| opcode(packet) == ServerOpcodes::AuraUpdate as u16)
        .expect("full AuraUpdate packet");
    let mut full_packet = wow_packet::WorldPacket::from_bytes(full_aura_packet);
    assert_eq!(
        full_packet.read_uint16().expect("AuraUpdate opcode"),
        ServerOpcodes::AuraUpdate as u16
    );
    assert!(full_packet.read_bit().expect("full aura update bit"));
    assert_eq!(full_packet.read_bits(9).expect("full aura count"), 2);

    wow_world_spell::tick_auras(&mut fixture.session);
    assert_eq!(aura_update_slots(&outgoing(&fixture)), vec![0, 1]);
    let manager = fixture.canonical.lock().expect("canonical manager");
    assert_eq!(
        manager
            .find_map(571, 0)
            .expect("Player map")
            .map()
            .get_typed_player(fixture.player)
            .expect("canonical Player")
            .unit()
            .subsystems()
            .auras
            .runtime_applications_like_cpp()
            .len(),
        2,
        "the full packet plus pending tick deltas do not duplicate AuraApplications"
    );
}
