// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Shared fixture setup and packet decoding for spirit-healer scenarios.

use super::*;
use crate::session::WorldSession;
use std::sync::Arc;
use wow_constants::CreatureTypeFlags;
use wow_constants::unit::{NPCFlags1, UnitFlags2};
use wow_data::character_progression::{ChrClassesStore, ChrRacesStore};
use wow_data::spell::aura_types::{
    SPELL_AURA_MOD_DAMAGE_PERCENT_DONE, SPELL_AURA_MOD_PERCENT_STAT, SPELL_AURA_NO_PVP_CREDIT,
};
use wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA;

pub(super) fn packet_opcode(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

pub(super) fn move_teleport_sequence_index(bytes: &[u8]) -> u32 {
    assert_eq!(packet_opcode(bytes), ServerOpcodes::MoveTeleport as u16);
    u32::from_le_bytes(
        bytes[18..22]
            .try_into()
            .expect("MoveTeleport sequence index follows opcode and full mover GUID"),
    )
}

pub(super) fn drain_packets(receiver: &flume::Receiver<Vec<u8>>) -> Vec<Vec<u8>> {
    std::iter::from_fn(|| receiver.try_recv().ok()).collect()
}

pub(super) fn keep_recovery_input_open(session: &mut WorldSession) -> flume::Sender<WorldPacket> {
    let (packet_tx, packet_rx) = flume::bounded(8);
    session.set_packet_rx(packet_rx);
    packet_tx
}

pub(super) fn healer_packet(healer: ObjectGuid) -> WorldPacket {
    let mut packet = WorldPacket::new_empty();
    packet.write_packed_guid(&healer);
    packet.reset_read();
    packet
}

pub(super) fn insert_healer(
    session: &WorldSession,
    guid_counter: i64,
    position: Position,
    npc_flags: u32,
) -> ObjectGuid {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 91, guid_counter);
    let mut creature = wow_entities::Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().object_mut().set_entry(91);
    creature.unit_mut().world_mut().set_map(571, 0).unwrap();
    creature.unit_mut().world_mut().relocate(position);
    creature.unit_mut().world_mut().set_combat_reach(1.0);
    creature.unit_mut().set_level(80);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.unit_mut().world_mut().object_mut().add_to_world();
    creature.set_ai_identity_runtime(1, 35, npc_flags, 0);
    creature.set_type_flags_runtime_like_cpp(CreatureTypeFlags::VISIBLE_TO_GHOSTS.bits());
    creature.set_unit_flags2_runtime_like_cpp(UnitFlags2::INTERACT_WHILE_HOSTILE.bits());

    let manager = crate::session::hub_ref(session)
        .core
        .canonical_map_manager
        .clone()
        .expect("corpse fixture canonical map manager");
    manager
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .expect("corpse fixture world map")
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    guid
}

pub(super) async fn resurrect_with_recruiter_settings(
    guid_counter: i64,
    is_a_recruiter: bool,
    recruiter_id: u32,
) -> u32 {
    let (mut session, _send_rx, _realm_rx, _port) = logged_in_released_ghost(guid_counter).await;
    session.set_is_a_recruiter_like_cpp(is_a_recruiter);
    session.set_recruiter_id_like_cpp(recruiter_id);
    let position = session.player_position_like_cpp().expect("ghost position");
    let healer = insert_healer(
        &session,
        guid_counter + 1,
        position,
        NPCFlags1::SPIRIT_HEALER.bits(),
    );

    session
        .handle_spirit_healer_activate(healer_packet(healer))
        .await;

    session
        .core
        .with_owned_player_like_cpp(|player| player.unit().world().object().dynamic_flags())
        .expect("canonical Player dynamic flags after resurrection")
}

pub(super) fn install_sickness_data(
    session: &mut WorldSession,
    spell_id: i32,
    visual_id: Option<u32>,
) {
    let spell_info = wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: SPELL_EFFECT_APPLY_AURA,
        effect_base_points: -76,
        effect_bonus_coefficient: 0.0,
        aura_type: Some(SPELL_AURA_MOD_PERCENT_STAT),
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: vec![
            wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: SPELL_EFFECT_APPLY_AURA,
                effect_aura: SPELL_AURA_MOD_PERCENT_STAT,
                effect_base_points: -76,
                effect_die_sides: 1,
                effect_misc_value_1: -1,
                ..Default::default()
            },
            wow_data::SpellEffectInfo {
                effect_index: 1,
                effect: SPELL_EFFECT_APPLY_AURA,
                effect_aura: SPELL_AURA_MOD_DAMAGE_PERCENT_DONE,
                effect_base_points: -76,
                effect_die_sides: 1,
                effect_misc_value_1: 127,
                ..Default::default()
            },
            wow_data::SpellEffectInfo {
                effect_index: 2,
                effect: SPELL_EFFECT_APPLY_AURA,
                effect_aura: SPELL_AURA_NO_PVP_CREDIT,
                effect_base_points: -1,
                effect_die_sides: 1,
                effect_misc_value_1: 127,
                ..Default::default()
            },
        ],
    };
    let existing_spells = session.catalogs.spell_store().cloned();
    let mut spells = wow_data::SpellStore::new();
    if let Some(existing_spells) = existing_spells.as_deref() {
        for selected_spell in [
            wow_world_entities::SPELL_GHOST_LIKE_CPP,
            wow_world_entities::SPELL_WISP_SPIRIT_AURA_LIKE_CPP,
        ] {
            if let Some(info) = existing_spells.get(selected_spell) {
                spells.insert(selected_spell, info.clone());
            }
            if let Some(attributes) =
                existing_spells.misc_attributes_for_difficulty_like_cpp(selected_spell, 0, None)
            {
                spells.insert_spell_misc_attributes_like_cpp(selected_spell, attributes);
            }
        }
    }
    spells.insert(spell_id, spell_info);
    let mut attributes = [0; 15];
    attributes[0] = 0x2C80_0100;
    attributes[1] = 0x88;
    spells.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    session.set_spell_store(Arc::new(spells));

    let mut misc_rows = crate::session::hub_ref(session)
        .catalogs
        .spell_catalogs
        .spell_misc_store()
        .map(|store| store.entries_like_cpp().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    misc_rows.retain(|row| !(row.spell_id == spell_id as u32 && row.difficulty_id == 0));
    misc_rows.push(wow_data::SpellMiscEntry {
        id: spell_id as u32,
        spell_id: spell_id as u32,
        difficulty_id: 0,
        duration_index: 427,
        attributes: attributes.map(|value| value as i32),
        ..Default::default()
    });
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries(misc_rows)));
    session.set_spell_duration_store(Arc::new(wow_data::SpellDurationStore::from_entries([
        wow_data::SpellDurationEntry {
            id: 427,
            duration: -600_000,
            duration_per_level: 60_000,
            max_duration: 600_000,
        },
    ])));
    let mut aura_options_rows = crate::session::hub_ref(session)
        .catalogs
        .spell_catalogs
        .spell_aura_options_store
        .as_deref()
        .map(|store| store.entries_like_cpp().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    aura_options_rows.retain(|row| !(row.spell_id == spell_id as u32 && row.difficulty_id == 0));
    session.set_spell_aura_options_store(Arc::new(wow_data::SpellAuraOptionsStore::from_entries(
        aura_options_rows,
    )));
    session.set_difficulty_store(Arc::new(wow_data::DifficultyStore::from_ids([0])));
    let visual_rows = visual_id
        .map(|spell_visual_id| wow_data::SpellXSpellVisualEntry {
            id: 245_039,
            difficulty_id: 0,
            spell_visual_id,
            probability: 1.0,
            flags: 0,
            priority: 0,
            spell_icon_file_id: 0,
            active_icon_file_id: 0,
            viewer_unit_condition_id: 0,
            viewer_player_condition_id: 0,
            caster_unit_condition_id: 0,
            caster_player_condition_id: 0,
            spell_id: spell_id as u32,
        })
        .into_iter()
        .collect::<Vec<_>>();
    session.set_creature_aggro_config_like_cpp(crate::session::CreatureAggroConfigLikeCpp {
        spell_x_spell_visual_store: Some(Arc::new(wow_data::SpellXSpellVisualStore::from_entries(
            visual_rows,
        ))),
        ..Default::default()
    });

    let (race, class, level) = {
        let player = crate::session::hub_ref(session);
        (
            player.player_race_like_cpp(),
            player.player_class_like_cpp(),
            player.player_level_like_cpp(),
        )
    };
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (race, class, level),
        wow_data::PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect: 10,
            spirit: 10,
            base_mana: 100,
        },
    )])));
    let mut class_entry = wow_data::character_progression::ChrClassesEntry::default();
    class_entry.id = u32::from(class);
    session.set_chr_classes_store(Arc::new(ChrClassesStore::from_entries([class_entry])));
}

pub(super) fn prepare_loaded_sickness_for_recast(
    session: &mut WorldSession,
    slot: u8,
    caster_guid: ObjectGuid,
) {
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            let auras = &mut player.unit_mut().subsystems_mut().auras;
            let application = auras
                .runtime_application_mut_like_cpp(slot)
                .expect("fresh sickness AuraApplication");
            application.duration_total = 900_000;
            application.duration_remaining = 10_000;
            application.stack_count = 3;
            let mut metadata = auras
                .aura_persistence_metadata(slot)
                .cloned()
                .expect("fresh sickness persistence metadata");
            metadata.charges = 5;
            metadata.recalculate_mask = 0;
            metadata.base_amounts = vec![(0, -10), (1, -11), (2, -12)];
            assert!(auras.set_aura_persistence_metadata(slot, metadata));
            let snapshot = auras
                .threat_snapshot_like_cpp(slot)
                .cloned()
                .expect("fresh sickness effect snapshot");
            let mut effects = snapshot.effects().to_vec();
            for (bit, _, amount, _) in &mut effects {
                if *bit == 0b001 {
                    *amount = -29;
                    let applied = wow_entities::AppliedAuraRef::new(
                        SICKNESS_SPELL as u32,
                        caster_guid,
                        slot,
                        *bit,
                    );
                    auras.applied_aura_amounts.insert(applied, *amount);
                }
            }
            auras.insert_threat_snapshot_like_cpp(
                slot,
                wow_entities::AuraThreatSnapshotLikeCpp::new(snapshot.interrupt_flags(), effects),
            );
        })
        .expect("canonical sickness Aura owner");
}

pub(super) fn refreshed_sickness_state(
    session: &WorldSession,
    slot: u8,
) -> (
    Vec<i32>,
    wow_entities::AuraPersistenceMetadata,
    wow_entities::AuraCastProvenanceLikeCpp,
) {
    session
        .core
        .with_owned_player_like_cpp(|player| {
            let auras = &player.unit().subsystems().auras;
            (
                auras
                    .threat_snapshot_like_cpp(slot)
                    .expect("refreshed sickness effect snapshot")
                    .effects()
                    .iter()
                    .map(|effect| effect.2)
                    .collect(),
                auras
                    .aura_persistence_metadata(slot)
                    .cloned()
                    .expect("refreshed sickness persistence metadata"),
                auras.aura_cast_provenance_like_cpp(slot),
            )
        })
        .expect("canonical refreshed sickness Aura")
}

pub(super) fn set_race_sickness_spell(session: &mut WorldSession, spell_id: i32) {
    let mut race = corpse::human_race_entry_with_faction(1);
    race.res_sickness_spell_id = spell_id;
    session.set_chr_races_store(Arc::new(ChrRacesStore::from_entries([race])));
}

pub(super) fn decode_sickness_auras(bytes: &[u8]) -> Vec<SicknessAuraUpdate> {
    let mut packet = WorldPacket::from_bytes(bytes);
    assert_eq!(packet.server_opcode(), Some(ServerOpcodes::AuraUpdate));
    packet.skip_opcode();
    let update_all = packet.read_bit().unwrap();
    let count = packet.read_bits(9).unwrap();
    let mut updates = Vec::new();
    for _ in 0..count {
        let _slot = packet.read_uint8().unwrap();
        if !packet.read_bit().unwrap() {
            packet.reset_bits();
            continue;
        }
        let cast_id = packet.read_packed_guid().unwrap();
        let spell_id = packet.read_int32().unwrap();
        let visual_id = packet.read_int32().unwrap();
        let aura_flags = packet.read_uint16().unwrap();
        let active_flags = packet.read_uint32().unwrap();
        let cast_level = packet.read_uint16().unwrap();
        let applications = packet.read_uint8().unwrap();
        let _content_tuning_id = packet.read_int32().unwrap();
        let has_caster = packet.read_bit().unwrap();
        let has_duration = packet.read_bit().unwrap();
        let has_remaining = packet.read_bit().unwrap();
        let _has_time_mod = packet.read_bit().unwrap();
        let point_count = packet.read_bits(6).unwrap();
        let _estimated_point_count = packet.read_bits(6).unwrap();
        let _has_content_tuning = packet.read_bit().unwrap();
        packet.reset_bits();
        if has_caster {
            let _caster = packet.read_packed_guid().unwrap();
        }
        let duration_ms = has_duration.then(|| packet.read_uint32().unwrap());
        let remaining_ms = has_remaining.then(|| packet.read_uint32().unwrap());
        for _ in 0..point_count {
            let _point = packet.read_float().unwrap();
        }
        updates.push(SicknessAuraUpdate {
            update_all,
            cast_id,
            spell_id,
            visual_id,
            aura_flags,
            active_flags,
            cast_level,
            applications,
            duration_ms,
            remaining_ms,
        });
    }
    updates
}

pub(super) fn decode_spell_go(bytes: &[u8]) -> (ObjectGuid, ObjectGuid, i32, u32, u32) {
    let mut packet = WorldPacket::from_bytes(bytes);
    assert_eq!(packet.server_opcode(), Some(ServerOpcodes::SpellGo));
    packet.skip_opcode();
    let _caster = packet.read_packed_guid().unwrap();
    let _caster_unit = packet.read_packed_guid().unwrap();
    let cast_id = packet.read_packed_guid().unwrap();
    let original_cast_id = packet.read_packed_guid().unwrap();
    let spell_id = packet.read_int32().unwrap();
    let visual = wow_packet::packets::spell::SpellCastVisual::read(&mut packet).unwrap();
    let flags = packet.read_uint32().unwrap();
    (
        cast_id,
        original_cast_id,
        spell_id,
        visual.spell_visual_id,
        flags,
    )
}

pub(super) fn visible_sickness_aura(
    session: &WorldSession,
    spell_id: i32,
) -> Vec<wow_entities::AuraApplicationLikeCpp> {
    crate::session::hub_ref(session)
        .resolved_player_visible_auras_like_cpp()
        .expect("canonical Player aura applications")
        .into_values()
        .filter(|aura| aura.spell_id == spell_id)
        .collect()
}

pub(super) fn durability_update_packet(session: &WorldSession, guid: ObjectGuid) -> Vec<u8> {
    let mut mask = wow_entities::UpdateMask::new(wow_entities::ITEM_DATA_BITS);
    mask.set(wow_entities::ITEM_DATA_DURABILITY_BIT);
    let update = wow_entities::ItemValuesUpdate {
        changed_object_type_mask: 1 << wow_entities::TYPEID_ITEM,
        object_data: None,
        item_data: Some(wow_entities::ItemDataUpdate {
            mask,
            values: session
                .resolved_inventory_item_object_like_cpp(guid)
                .expect("durability item")
                .data()
                .clone(),
        }),
    };
    wow_world_core::entity_update_bridge::item_values_update_to_update_object(guid, 571, &update)
        .expect("durability-only item update")
        .to_bytes()
}
