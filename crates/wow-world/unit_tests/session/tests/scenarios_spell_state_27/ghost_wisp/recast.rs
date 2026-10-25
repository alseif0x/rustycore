use super::*;

fn spell_go_cast_rows(packets: &[Vec<u8>]) -> Vec<(i32, ObjectGuid)> {
    packets
        .iter()
        .filter_map(|bytes| {
            let mut packet = wow_packet::WorldPacket::from_bytes(bytes);
            if packet.read_uint16().ok()? != ServerOpcodes::SpellGo as u16 {
                return None;
            }
            packet.read_packed_guid().ok()?;
            packet.read_packed_guid().ok()?;
            let cast_id = packet.read_packed_guid().ok()?;
            packet.read_packed_guid().ok()?;
            Some((packet.read_int32().ok()?, cast_id))
        })
        .collect()
}

fn aura_update_cast_rows(packets: &[Vec<u8>]) -> Vec<(u8, i32, ObjectGuid, i32)> {
    packets
        .iter()
        .flat_map(|bytes| {
            let mut packet = wow_packet::WorldPacket::from_bytes(bytes);
            if packet.read_uint16().ok() != Some(ServerOpcodes::AuraUpdate as u16) {
                return Vec::new();
            }
            let _update_all = packet.read_bit().expect("AuraUpdate update-all bit");
            let count = packet.read_bits(9).expect("AuraUpdate slot count");
            (0..count)
                .filter_map(|_| {
                    let slot = packet.read_uint8().ok()?;
                    if !packet.read_bit().ok()? {
                        return None;
                    }
                    let cast_id = packet.read_packed_guid().ok()?;
                    let spell_id = packet.read_int32().ok()?;
                    let visual_id = packet.read_int32().ok()?;
                    Some((slot, spell_id, cast_id, visual_id))
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

#[tokio::test]
async fn ghost_recasts_refresh_the_loaded_self_application_in_place() {
    let mut fixture = fixture();
    let external_caster = ObjectGuid::create_player(1, 96_114);
    let aura_rows = [external_caster, fixture.player].map(|caster_guid| {
        let self_cast = caster_guid == fixture.player;
        wow_world_spell::CharacterAuraRowLikeCpp {
            caster_guid,
            item_guid: ObjectGuid::EMPTY,
            spell_id: GHOST as u32,
            effect_mask: 0b111,
            recalculate_mask: if self_cast { 0b100 } else { 0 },
            difficulty: 0,
            stack_count: if self_cast { 3 } else { 1 },
            max_duration_ms: -1,
            remain_time_ms: -1,
            remain_charges: if self_cast { 2 } else { 0 },
            cast_item_id: 0,
            cast_item_level: -1,
        }
    });
    let effect_rows = [
        (
            fixture.player,
            ObjectGuid::EMPTY,
            [(-19, -18), (25, 24), (30, 29)],
        ),
        (
            external_caster,
            ObjectGuid::EMPTY,
            [(-21, -21), (50, 49), (50, 49)],
        ),
    ]
    .into_iter()
    .flat_map(|(caster_guid, item_guid, amounts)| {
        amounts
            .into_iter()
            .enumerate()
            .map(move |(effect_index, (amount, base_amount))| {
                wow_world_spell::CharacterAuraEffectRowLikeCpp {
                    caster_guid,
                    item_guid,
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
    outgoing(&fixture);
    wow_world_spell::tick_auras(&mut fixture.session);
    outgoing(&fixture);

    let (self_slot, initial_application, original_provenance, applied_order) = {
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
        let application = auras
            .runtime_applications_like_cpp()
            .values()
            .find(|aura| aura.spell_id == GHOST && aura.caster_guid == fixture.player)
            .expect("loaded self Ghost application");
        (
            application.slot,
            application.clone(),
            auras.aura_cast_provenance_like_cpp(application.slot),
            auras.applied_auras.clone(),
        )
    };

    cast_selected(&mut fixture, GHOST, None).await;
    let first_refresh_packets = outgoing(&fixture);
    let first_cast_id = spell_go_cast_rows(&first_refresh_packets)
        .into_iter()
        .find_map(|(spell_id, cast_id)| (spell_id == GHOST).then_some(cast_id))
        .expect("server refresh emits a new SpellGo cast ID");
    assert_ne!(first_cast_id, original_provenance.cast_id);

    let refreshed_application;
    let refreshed_effects;
    let refreshed_metadata;
    let refreshed_type_order;
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
        let application = auras
            .runtime_application_like_cpp(self_slot)
            .expect("same self Ghost slot remains active");
        assert_eq!(application.slot, initial_application.slot);
        assert_eq!(application.caster_guid, fixture.player);
        assert_eq!(application.stack_count, 1);
        assert_eq!(application.duration_total, 0);
        assert_eq!(application.duration_remaining, 0);
        assert_eq!(application.applied_at, initial_application.applied_at);
        assert_eq!(
            application.represented_effect_amounts,
            vec![
                wow_entities::RepresentedAuraEffectAmountLikeCpp {
                    effect_index: 0,
                    amount: -20,
                },
                wow_entities::RepresentedAuraEffectAmountLikeCpp {
                    effect_index: 1,
                    amount: 50,
                },
                wow_entities::RepresentedAuraEffectAmountLikeCpp {
                    effect_index: 2,
                    amount: 50,
                },
            ]
        );
        assert_eq!(
            auras.aura_cast_provenance_like_cpp(self_slot),
            original_provenance
        );
        assert_eq!(auras.runtime_applications_like_cpp().len(), 1);
        assert!(
            auras
                .runtime_applications_like_cpp()
                .values()
                .all(|aura| aura.caster_guid == fixture.player)
        );
        assert_eq!(auras.applied_auras, applied_order);
        let self_effects = auras
            .threat_snapshot_like_cpp(self_slot)
            .expect("refreshed canonical effect snapshot")
            .effects();
        assert_eq!(
            self_effects,
            &[
                (0b001, wow_data::spell::aura_types::SPELL_AURA_GHOST, -20, 0,),
                (
                    0b010,
                    wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
                    50,
                    0,
                ),
                (
                    0b100,
                    wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
                    50,
                    0,
                ),
            ]
        );
        for (bit, aura_type, expected_amount) in [
            (0b001, wow_data::spell::aura_types::SPELL_AURA_GHOST, -20),
            (
                0b010,
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
                50,
            ),
            (
                0b100,
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
                50,
            ),
        ] {
            let applied =
                wow_entities::AppliedAuraRef::new(GHOST as u32, fixture.player, self_slot, bit);
            assert_eq!(
                auras.applied_aura_amounts.get(&applied),
                Some(&expected_amount)
            );
            assert_eq!(
                auras
                    .applied_aura_types
                    .get(&aura_type)
                    .expect("effect type index")
                    .last(),
                Some(&applied),
                "reapplied effect is newest in its canonical type index"
            );
        }
        let metadata = auras
            .aura_persistence_metadata(self_slot)
            .expect("loaded persistence metadata");
        assert_eq!(metadata.item_guid, ObjectGuid::EMPTY);
        assert_eq!(metadata.cast_item_id, 0);
        assert_eq!(metadata.cast_item_level, -1);
        assert_eq!(metadata.charges, 0);
        assert_eq!(metadata.recalculate_mask, 0b100);
        assert_eq!(metadata.base_amounts, vec![(0, -21), (1, 49), (2, 49)]);
        refreshed_application = application.clone();
        refreshed_effects = self_effects.to_vec();
        refreshed_metadata = metadata.clone();
        refreshed_type_order = [
            wow_data::spell::aura_types::SPELL_AURA_GHOST,
            wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
            wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
        ]
        .map(|aura_type| {
            auras
                .applied_aura_types
                .get(&aura_type)
                .expect("aura type index")
                .clone()
        });
    }
    assert_ne!(refreshed_application, initial_application);
    assert_eq!(
        aura_amount(
            &fixture,
            self_slot,
            wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED
        ),
        Some(50)
    );

    wow_world_spell::tick_auras(&mut fixture.session);
    let first_update = outgoing(&fixture);
    assert_eq!(aura_update_slots(&first_update), vec![self_slot]);
    assert_eq!(
        aura_update_cast_rows(&first_update),
        vec![(
            self_slot,
            GHOST,
            original_provenance.cast_id,
            GHOST_VISUAL_ROW as i32
        )]
    );

    {
        let mut manager = fixture.canonical.lock().expect("canonical manager");
        let player = manager
            .find_map_mut(571, 0)
            .expect("Player map")
            .map_mut()
            .get_typed_player_mut(fixture.player)
            .expect("canonical Player");
        let auras = &mut player.unit_mut().subsystems_mut().auras;
        for slot in 1..u8::MAX {
            auras.insert_runtime_application_like_cpp(wow_entities::AuraApplicationLikeCpp {
                spell_id: 90_000 + i32::from(slot),
                difficulty_id: 0,
                caster_guid: fixture.player,
                slot,
                duration_total: 0,
                duration_remaining: 0,
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
                applied_at: Instant::now(),
            });
        }
        assert_eq!(
            auras.runtime_applications_like_cpp().len(),
            usize::from(u8::MAX)
        );
        assert!(auras.visible_auras_to_update.is_empty());
    }

    let run_speed = player_speed(&fixture, UnitMoveTypeLikeCpp::Run);
    let swim_speed = player_speed(&fixture, UnitMoveTypeLikeCpp::Swim);
    let removed_count = {
        let manager = fixture.canonical.lock().expect("canonical manager");
        manager
            .find_map(571, 0)
            .expect("Player map")
            .map()
            .get_typed_player(fixture.player)
            .expect("canonical Player")
            .unit()
            .subsystems()
            .auras
            .removed_auras_count
    };
    let mut recast_ids = Vec::new();
    for _ in 0..2 {
        cast_selected(&mut fixture, GHOST, None).await;
        let packets = outgoing(&fixture);
        let cast_id = spell_go_cast_rows(&packets)
            .into_iter()
            .find_map(|(spell_id, cast_id)| (spell_id == GHOST).then_some(cast_id))
            .expect("each recast has a new SpellGo cast ID");
        assert_ne!(cast_id, original_provenance.cast_id);
        assert!(!recast_ids.contains(&cast_id));
        recast_ids.push(cast_id);
        assert!(aura_update_slots(&packets).is_empty());
        assert!(!packets.iter().map(|packet| opcode(packet)).any(|opcode| {
            opcode == ServerOpcodes::MoveSetRunSpeed as u16
                || opcode == ServerOpcodes::MoveSetSwimSpeed as u16
        }));
        assert!((player_speed(&fixture, UnitMoveTypeLikeCpp::Run) - run_speed).abs() < 0.0001);
        assert!((player_speed(&fixture, UnitMoveTypeLikeCpp::Swim) - swim_speed).abs() < 0.0001);

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
        assert_eq!(
            auras.runtime_application_like_cpp(self_slot),
            Some(&refreshed_application)
        );
        assert_eq!(
            auras.aura_cast_provenance_like_cpp(self_slot),
            original_provenance
        );
        assert_eq!(
            auras
                .threat_snapshot_like_cpp(self_slot)
                .expect("canonical refreshed effect snapshot")
                .effects(),
            refreshed_effects
        );
        assert_eq!(auras.applied_auras, applied_order);
        assert_eq!(
            auras
                .aura_persistence_metadata(self_slot)
                .expect("preserved loaded metadata"),
            &refreshed_metadata
        );
        for (index, aura_type) in [
            wow_data::spell::aura_types::SPELL_AURA_GHOST,
            wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
            wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(
                auras.applied_aura_types.get(&aura_type),
                Some(&refreshed_type_order[index])
            );
        }
        assert_eq!(
            auras.runtime_applications_like_cpp().len(),
            usize::from(u8::MAX)
        );
        assert_eq!(
            auras.visible_auras_to_update,
            [self_slot].into_iter().collect()
        );
        assert!(
            auras
                .runtime_applications_like_cpp()
                .values()
                .all(|aura| { aura.spell_id != GHOST || aura.caster_guid == fixture.player })
        );
        assert_eq!(auras.removed_auras_count, removed_count);
    }

    wow_world_spell::tick_auras(&mut fixture.session);
    let update_packets = outgoing(&fixture);
    assert_eq!(aura_update_slots(&update_packets), vec![self_slot]);
    assert_eq!(
        aura_update_cast_rows(&update_packets),
        vec![(
            self_slot,
            GHOST,
            original_provenance.cast_id,
            GHOST_VISUAL_ROW as i32
        )]
    );
    assert!(
        !update_packets
            .iter()
            .map(|packet| opcode(packet))
            .any(|opcode| {
                opcode == ServerOpcodes::MoveSetRunSpeed as u16
                    || opcode == ServerOpcodes::MoveSetSwimSpeed as u16
            })
    );
}

#[tokio::test]
async fn wisp_recast_preserves_negative_transform_priority_and_display() {
    let mut fixture = fixture();
    let mut negative = aura_spell(
        NEGATIVE_TRANSFORM,
        [(wow_data::spell::aura_types::SPELL_AURA_TRANSFORM, 0, 12_862)],
    );
    negative.effects[0].implicit_target_1 = 2;
    install_spell_catalog(&mut fixture.session, [negative]);
    install_wisp_templates(
        &mut fixture.session,
        &[(WISP_TEMPLATE, 1_825), (12_862, 1_826)],
    );

    cast_selected(&mut fixture, WISP, None).await;
    outgoing(&fixture);
    fixture
        .session
        .apply_aura(NEGATIVE_TRANSFORM, fixture.player, 0, 0x0010)
        .expect("apply negative transform");
    outgoing(&fixture);
    let (initial_wisp_slot, negative_slot, initial_provenance) = {
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
        let wisp = auras
            .runtime_applications_like_cpp()
            .values()
            .find(|aura| aura.spell_id == WISP)
            .expect("Wisp AuraApplication");
        let negative = auras
            .runtime_applications_like_cpp()
            .values()
            .find(|aura| aura.spell_id == NEGATIVE_TRANSFORM)
            .expect("negative transform AuraApplication");
        (
            wisp.slot,
            negative.slot,
            auras.aura_cast_provenance_like_cpp(wisp.slot),
        )
    };
    fixture
        .session
        .remove_aura(initial_wisp_slot)
        .expect("remove first Wisp source");
    outgoing(&fixture);
    assert_eq!(
        player_display(&fixture),
        1_826,
        "production Wisp removal restores the still-active negative transform"
    );

    cast_selected(&mut fixture, WISP, None).await;
    outgoing(&fixture);
    assert_eq!(player_display(&fixture), 1_826);
    let (wisp_slot, fresh_application, provenance) = {
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
        let application = auras
            .runtime_applications_like_cpp()
            .values()
            .find(|aura| aura.spell_id == WISP)
            .expect("new Wisp AuraApplication after production removal");
        assert_eq!(application.slot, initial_wisp_slot);
        assert_eq!(application.duration_total, 0);
        assert_eq!(application.duration_remaining, 0);
        assert_eq!(auras.runtime_applications_like_cpp().len(), 2);
        (
            application.slot,
            application.clone(),
            auras.aura_cast_provenance_like_cpp(application.slot),
        )
    };
    assert_ne!(provenance, initial_provenance);

    cast_selected(&mut fixture, WISP, None).await;
    let packets = outgoing(&fixture);
    assert_eq!(player_display(&fixture), 1_826);
    assert!(
        !packets.iter().map(|packet| opcode(packet)).any(|opcode| {
            opcode == ServerOpcodes::MoveSetRunSpeed as u16
                || opcode == ServerOpcodes::MoveSetSwimSpeed as u16
        }),
        "an unchanged Wisp refresh emits no speed update"
    );
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
        assert_eq!(auras.runtime_applications_like_cpp().len(), 2);
        let refreshed = auras
            .runtime_application_like_cpp(wisp_slot)
            .expect("refreshed canonical Wisp application");
        assert_eq!(refreshed.slot, fresh_application.slot);
        assert_eq!(refreshed.spell_id, fresh_application.spell_id);
        assert_eq!(refreshed.caster_guid, fresh_application.caster_guid);
        assert_eq!(refreshed.applied_at, fresh_application.applied_at);
        assert_eq!(refreshed.effect_mask, fresh_application.effect_mask);
        assert_eq!(refreshed.duration_total, fresh_application.duration_total);
        assert_eq!(
            refreshed.duration_remaining,
            fresh_application.duration_remaining
        );
        assert_eq!(refreshed.stack_count, fresh_application.stack_count);
        assert_eq!(
            refreshed.represented_effect_amounts,
            vec![
                wow_entities::RepresentedAuraEffectAmountLikeCpp {
                    effect_index: 0,
                    amount: 75,
                },
                wow_entities::RepresentedAuraEffectAmountLikeCpp {
                    effect_index: 1,
                    amount: 75,
                },
                wow_entities::RepresentedAuraEffectAmountLikeCpp {
                    effect_index: 2,
                    amount: 0,
                },
            ],
            "refresh projects the selected speed and transform amounts"
        );
        let transform_order = auras
            .applied_aura_types
            .get(&wow_data::spell::aura_types::SPELL_AURA_TRANSFORM)
            .expect("transform effect index")
            .iter()
            .map(|effect| effect.slot)
            .collect::<Vec<_>>();
        assert_eq!(transform_order, vec![negative_slot, wisp_slot]);
        assert_eq!(
            auras.aura_cast_provenance_like_cpp(wisp_slot),
            provenance,
            "recast preserves the original transform aura provenance"
        );
    }

    fixture
        .session
        .remove_aura(wisp_slot)
        .expect("remove Wisp source");
    assert_eq!(
        player_display(&fixture),
        1_826,
        "negative transform remains the display restoration winner"
    );
}
