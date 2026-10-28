// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spirit-healer interaction, resurrection sickness and durability scenarios.

use super::corpse;
use super::*;
use crate::session::WorldSession;
use wow_constants::ItemContext;
use wow_constants::ItemUpdateState;
use wow_constants::unit::{NPCFlags1, UnitDynFlags};
use wow_entities::{EQUIPMENT_SLOT_CHEST, INVENTORY_SLOT_BAG_START, INVENTORY_SLOT_ITEM_START};

const SICKNESS_SPELL: i32 = 15_007;
const CUSTOM_SICKNESS_SPELL: i32 = 846_201;

#[path = "spirit_healer_lifecycle_stat.rs"]
mod lifecycle_stat;
#[path = "spirit_healer/recast_policy.rs"]
mod recast_policy;
#[path = "spirit_healer/selected_cast_threat.rs"]
mod selected_cast_threat;
#[path = "spirit_healer/selected_cast_visual.rs"]
mod selected_cast_visual;
#[path = "spirit_healer/selected_charges.rs"]
mod selected_charges;
#[path = "spirit_healer/selected_creation.rs"]
mod selected_creation;
#[path = "spirit_healer/selected_execution_effects.rs"]
mod selected_execution_effects;
#[path = "spirit_healer/selected_refresh.rs"]
mod selected_refresh;
#[path = "spirit_healer/selected_stats.rs"]
mod selected_stats;
use selected_execution_effects::{
    cast_sickness, difficulty, effect_row, install_player_on_difficulty_map,
    selected_session_with_visual_inputs, selected_session_without_selected_spell_info_key,
};
#[path = "spirit_healer/native_outcome.rs"]
mod native_outcome;
pub(super) use native_outcome::remove_ghost_spell_metadata;
#[path = "spirit_healer/obtain.rs"]
mod obtain;
#[path = "spirit_healer/recovery_fences.rs"]
mod recovery_fences;
#[path = "spirit_healer/selected_lifetime.rs"]
mod selected_lifetime;
#[path = "spirit_healer/selected_visual.rs"]
mod selected_visual;
#[path = "spirit_healer/setup.rs"]
mod setup;
use setup::*;

pub(super) fn install_recovery_sickness_fixture(session: &mut WorldSession) {
    set_race_sickness_spell(session, SICKNESS_SPELL);
    install_sickness_data(session, SICKNESS_SPELL, Some(69));
}

pub(super) fn install_recovery_sickness_fixture_with_normal_cast_costs(session: &mut WorldSession) {
    set_race_sickness_spell(session, SICKNESS_SPELL);
    install_sickness_data_with_cast_costs(
        session,
        SICKNESS_SPELL,
        Some(69),
        Some(wow_data::SpellCastCostStoreLikeCpp::default()),
    );
    install_normal_sickness_cast_payload(session, wow_data::SpellCastCostStoreLikeCpp::default());
    session.set_spell_category_store(std::sync::Arc::new(
        wow_data::SpellCategoryStore::from_entries([]),
    ));
    session.mark_represented_character_spell_cooldowns_loaded_like_cpp();
    session.mark_represented_character_spell_charges_loaded_like_cpp();
}

pub(super) fn recovery_sickness_is_present(session: &WorldSession) -> bool {
    !visible_sickness_aura(session, SICKNESS_SPELL).is_empty()
}

#[derive(Debug)]
struct SicknessAuraUpdate {
    update_all: bool,
    cast_id: ObjectGuid,
    spell_id: i32,
    visual_id: i32,
    aura_flags: u16,
    active_flags: u32,
    cast_level: u16,
    applications: u8,
    duration_ms: Option<u32>,
    remaining_ms: Option<u32>,
}

pub(super) async fn logged_in_released_ghost(
    guid_counter: i64,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    flume::Receiver<Vec<u8>>,
    Arc<corpse::RecordingCorpsePortLikeCpp>,
) {
    let (mut session, send_rx, realm_rx, port) =
        corpse::released_ghost_without_reclaim_delay_like_cpp(guid_counter).await;
    session.core.state = crate::session::SessionState::LoggedIn;

    let player_guid = session.player_guid().expect("released ghost Player GUID");
    assert!(matches!(
        wow_world_entities::handle_move_teleport_ack_like_cpp(&mut session, player_guid, 0, 0,),
        wow_world_core::session::movement_protocol::MoveTeleportAckActionLikeCpp::Accepted
    ));
    assert_eq!(
        session.pending_teleport_like_cpp(),
        None,
        "acknowledge repop's initial near teleport before healer assertions"
    );
    let teleport_state = crate::session::hub_ref(&session)
        .player_teleport_state_snapshot_like_cpp()
        .expect("canonical teleport state after repop ACK");
    assert!(!teleport_state.near_pending);
    assert!(!teleport_state.far_pending);
    let _ = drain_packets(&send_rx);
    let _ = drain_packets(&realm_rx);
    (session, send_rx, realm_rx, port)
}

#[tokio::test]
async fn spirit_healer_gate_rejects_wrong_flag_and_distant_npc() {
    let (mut session, send_rx, _realm_rx, _port) = logged_in_released_ghost(43_101).await;
    let position = session.player_position_like_cpp().unwrap();
    let wrong_flag = insert_healer(&session, 43_102, position, NPCFlags1::QUEST_GIVER.bits());
    let far_healer = insert_healer(
        &session,
        43_103,
        Position::new(
            position.x + 1_000.0,
            position.y,
            position.z,
            position.orientation,
        ),
        NPCFlags1::SPIRIT_HEALER.bits(),
    );

    session
        .handle_spirit_healer_activate(healer_packet(wrong_flag))
        .await;
    session
        .handle_spirit_healer_activate(healer_packet(far_healer))
        .await;

    assert!(!session.player_is_alive_like_cpp());
    assert!(crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn spirit_healer_sets_recruiter_flag_for_either_session_predicate() {
    let recruited = resurrect_with_recruiter_settings(43_301, true, 0).await;
    let recruiter = resurrect_with_recruiter_settings(43_401, false, 42).await;
    let ordinary = resurrect_with_recruiter_settings(43_501, false, 0).await;
    let flag = UnitDynFlags::ReferAFriend as u32;

    assert_ne!(recruited & flag, 0, "IsARecruiter sets REFER_A_FRIEND");
    assert_ne!(
        recruiter & flag,
        0,
        "nonzero recruiter ID sets REFER_A_FRIEND"
    );
    assert_eq!(ordinary & flag, 0, "neither predicate leaves the bit clear");
}

#[tokio::test]
async fn spirit_healer_clears_canonical_self_res_and_out_of_bounds_state() {
    let (mut session, _send_rx, _realm_rx, _port) = logged_in_released_ghost(43_601).await;
    let player_guid = session.player_guid().expect("ghost Player GUID");
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player.set_player_flag(0x0000_4000);
            player.add_self_res_spell(21_169);
        })
        .expect("canonical Player");
    let position = session.player_position_like_cpp().expect("ghost position");
    let healer = insert_healer(&session, 43_602, position, NPCFlags1::SPIRIT_HEALER.bits());

    session
        .handle_spirit_healer_activate(healer_packet(healer))
        .await;

    let (is_alive, player_flags, self_res_spells, self_res_mask, active_changes) = session
        .core
        .with_owned_player_like_cpp(|player| {
            (
                player.unit().is_alive(),
                player.data().player_flags,
                player.self_res_spells().to_vec(),
                player.active_data().self_res_spells_update_mask.clone(),
                player.active_player_data_changes_mask().clone(),
            )
        })
        .expect("canonical Player after resurrection");
    assert!(is_alive);
    assert_eq!(player_flags & 0x0000_4000, 0, "ResurrectPlayer clears OOB");
    assert!(
        self_res_spells.is_empty(),
        "alive-from-dead clears the Player-owned self-res spells"
    );
    assert_eq!(self_res_mask, Some(Vec::new()));
    assert!(active_changes.is_set(wow_entities::ACTIVE_PLAYER_DATA_SELF_RES_SPELLS_BIT));
    assert!(active_changes.is_set(wow_entities::ACTIVE_PLAYER_DATA_PARENT_BIT));
    assert_eq!(session.player_guid(), Some(player_guid));
}

#[tokio::test]
async fn spirit_healer_resurrects_applies_sickness_and_only_damages_carried_items() {
    let (mut session, send_rx, _realm_rx, port) = logged_in_released_ghost(43_201).await;
    let save_port = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_player_lifecycle_port_like_cpp(save_port.clone());
    let _packet_peer = keep_recovery_input_open(&mut session);
    let player_guid = session.player_guid().unwrap();
    let position = session.player_position_like_cpp().unwrap();
    let healer = insert_healer(&session, 43_202, position, NPCFlags1::SPIRIT_HEALER.bits());
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 11, 0);
    session.set_death_policy_like_cpp(wow_world_core::session::DeathPolicyLikeCpp {
        sickness_level: 11,
        corpse_reclaim_delay_pve: false,
        ..Default::default()
    });
    set_race_sickness_spell(&mut session, SICKNESS_SPELL);
    install_sickness_data(&mut session, SICKNESS_SPELL, Some(69));
    let original_player_level = session
        .core
        .with_owned_player_like_cpp(|player| player.unit().data().level)
        .expect("canonical Player level");
    let original_captured_level = u8::try_from(original_player_level).unwrap();
    let refreshed_captured_level = original_captured_level
        .checked_add(1)
        .expect("test Player level can advance once");
    assert_eq!(original_captured_level, 11);
    assert_eq!(refreshed_captured_level, 12);
    let spell_level_fallback = if original_captured_level == 239 {
        238
    } else {
        239
    };
    session.set_spell_levels_store(Arc::new(wow_data::SpellLevelsStore::from_entries([
        wow_data::SpellLevelsEntry {
            id: 1,
            difficulty_id: 0,
            base_level: 0,
            max_level: 0,
            spell_level: i16::from(spell_level_fallback),
            max_passive_aura_level: 0,
            spell_id: SICKNESS_SPELL as u32,
        },
    ])));
    session.set_player_inventory_slot_count_like_cpp(wow_entities::INVENTORY_DEFAULT_SIZE);

    let player_stats_for_test_level = || wow_data::PlayerLevelStats {
        strength: 10,
        agility: 10,
        stamina: 10,
        intellect: 10,
        spirit: 10,
        base_mana: 100,
    };
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([
        ((1, 1, 11), player_stats_for_test_level()),
        (
            (1, 1, original_captured_level),
            player_stats_for_test_level(),
        ),
        (
            (1, 1, refreshed_captured_level),
            player_stats_for_test_level(),
        ),
    ])));
    session.set_chr_classes_store(Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([{
            let mut entry = wow_data::character_progression::ChrClassesEntry::default();
            entry.id = 1;
            entry
        }]),
    ));
    let gear_guid = ObjectGuid::create_item(1, 43_203);
    let backpack_guid = ObjectGuid::create_item(1, 43_204);
    let carried_bag_guid = ObjectGuid::create_item(1, 43_205);
    let carried_child_guid = ObjectGuid::create_item(1, 43_206);
    let bank_guid = ObjectGuid::create_item(1, 43_207);
    let bank_bag_guid = ObjectGuid::create_item(1, 43_208);
    let bank_child_guid = ObjectGuid::create_item(1, 43_209);
    install_known_no_obtain_item_catalogs(
        &mut session,
        [
            (43_210, wow_constants::InventoryType::Chest, 0),
            (43_211, wow_constants::InventoryType::NonEquip, 0),
            (43_212, wow_constants::InventoryType::NonEquip, 0),
            (43_213, wow_constants::InventoryType::NonEquip, 0),
            (43_214, wow_constants::InventoryType::NonEquip, 0),
            (43_206, wow_constants::InventoryType::NonEquip, 0),
            (43_209, wow_constants::InventoryType::NonEquip, 0),
        ],
        [(
            43_210,
            wow_data::ItemStatEntry {
                stats: std::array::from_fn(|index| {
                    if index == 0 {
                        (wow_constants::ItemModType::Strength as i8, 10)
                    } else {
                        (wow_constants::ItemModType::None as i8, 0)
                    }
                }),
                resistances: [0; 7],
                armor: 0,
            },
        )],
    );
    add_top_level_item(
        &mut session,
        player_guid,
        gear_guid,
        43_210,
        EQUIPMENT_SLOT_CHEST,
        100,
        25,
        ItemUpdateState::Unchanged,
    );
    session.apply_inventory_item_store_side_effects_like_cpp(
        wow_entities::INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_CHEST,
        gear_guid,
    );
    add_top_level_item(
        &mut session,
        player_guid,
        backpack_guid,
        43_211,
        INVENTORY_SLOT_ITEM_START,
        100,
        100,
        ItemUpdateState::Unchanged,
    );
    add_top_level_item(
        &mut session,
        player_guid,
        carried_bag_guid,
        43_212,
        INVENTORY_SLOT_BAG_START,
        0,
        0,
        ItemUpdateState::Unchanged,
    );
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .register_bag_storage(INVENTORY_SLOT_BAG_START, carried_bag_guid, 1)
                .expect("canonical carried bag topology");
        })
        .expect("canonical Player carried inventory");
    add_bag_child(
        &mut session,
        player_guid,
        carried_child_guid,
        carried_bag_guid,
        INVENTORY_SLOT_BAG_START,
        0,
        100,
        100,
        ItemUpdateState::New,
    );
    add_top_level_item(
        &mut session,
        player_guid,
        bank_guid,
        43_213,
        wow_entities::BANK_SLOT_ITEM_START,
        100,
        100,
        ItemUpdateState::Unchanged,
    );
    add_top_level_item(
        &mut session,
        player_guid,
        bank_bag_guid,
        43_214,
        wow_entities::BANK_SLOT_BAG_START,
        0,
        0,
        ItemUpdateState::Unchanged,
    );
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .register_bag_storage(wow_entities::BANK_SLOT_BAG_START, bank_bag_guid, 1)
                .expect("canonical bank bag topology");
        })
        .expect("canonical Player bank inventory");
    add_bag_child(
        &mut session,
        player_guid,
        bank_child_guid,
        bank_bag_guid,
        wow_entities::BANK_SLOT_BAG_START,
        0,
        100,
        100,
        ItemUpdateState::Unchanged,
    );
    let _ = session.send_stat_update();
    let pre_resurrection_max_health = session
        .core
        .with_owned_player_like_cpp(|player| player.unit().data().max_health)
        .expect("canonical Player maximum health");
    assert_eq!(
        session
            .canonical_player_effective_combat_stats_like_cpp()
            .unwrap()
            .stats[wow_constants::Stats::Strength as usize],
        20
    );
    let _ = drain_packets(&send_rx);

    session
        .handle_spirit_healer_activate(healer_packet(healer))
        .await;

    assert_eq!(
        save_port.character_save_count(),
        1,
        "the healer full save follows confirmed corpse deletion"
    );
    let save_requests = save_port.character_save_requests();
    assert_eq!(save_requests.len(), 1);
    let saved_items = save_requests[0]
        .inventory_items
        .as_ref()
        .expect("the completed inventory authority contributes its save group");
    let saved_item = |guid: ObjectGuid| {
        saved_items
            .iter()
            .find(|item| item.item_guid == guid.counter() as u64)
            .expect("dirty inventory item is captured by the full save")
    };
    let saved_gear = saved_item(gear_guid);
    assert_eq!(saved_gear.item_entry, 43_210);
    assert_eq!(saved_gear.owner_guid, player_guid.counter() as u64);
    assert_eq!(saved_gear.slot, EQUIPMENT_SLOT_CHEST);
    assert_eq!(
        saved_gear.state,
        wow_persistence::PlayerItemSaveStateLikeCpp::Changed
    );
    assert_eq!(saved_gear.durability, 0);
    let saved_backpack = saved_item(backpack_guid);
    assert_eq!(saved_backpack.item_entry, 43_211);
    assert_eq!(saved_backpack.owner_guid, player_guid.counter() as u64);
    assert_eq!(saved_backpack.slot, INVENTORY_SLOT_ITEM_START);
    assert_eq!(
        saved_backpack.state,
        wow_persistence::PlayerItemSaveStateLikeCpp::Changed
    );
    assert_eq!(saved_backpack.durability, 75);
    let saved_carried_child = saved_item(carried_child_guid);
    assert_eq!(saved_carried_child.item_entry, 43_206);
    assert_eq!(saved_carried_child.owner_guid, player_guid.counter() as u64);
    assert_eq!(
        saved_carried_child.bag_guid,
        carried_bag_guid.counter() as u64
    );
    assert_eq!(saved_carried_child.slot, 0);
    assert_eq!(
        saved_carried_child.state,
        wow_persistence::PlayerItemSaveStateLikeCpp::New
    );
    assert_eq!(saved_carried_child.durability, 75);
    assert!(saved_items.iter().all(|item| {
        item.item_guid != bank_guid.counter() as u64
            && item.item_guid != bank_child_guid.counter() as u64
    }));

    let post_resurrection_vitals = session
        .core
        .with_owned_player_like_cpp(|player| {
            (
                player.unit().death_state(),
                player.unit().data().health,
                player.unit().data().max_health,
            )
        })
        .expect("canonical Player after spirit-healer resurrection");
    assert!(
        session.player_is_alive_like_cpp(),
        "spirit-healer resurrection should leave positive health: {post_resurrection_vitals:?}"
    );
    assert!(!crate::session::hub_ref(&session).player_has_ghost_flag_like_cpp());
    let (current_health, final_max_health) = session
        .core
        .with_owned_player_like_cpp(|player| {
            (player.unit().data().health, player.unit().data().max_health)
        })
        .expect("canonical Player vitals");
    assert_eq!(
        current_health,
        (pre_resurrection_max_health / 2).min(final_max_health),
        "ResurrectPlayer halves pre-sickness max health and SetMaxHealth clamps it"
    );
    assert!(corpse::current_corpse_guid_like_cpp(&session, player_guid).is_none());
    assert!(!port.delete_requests().is_empty());
    assert_eq!(item_durability(&session, gear_guid), 0);
    assert_eq!(item_durability(&session, backpack_guid), 75);
    assert_eq!(item_durability(&session, carried_child_guid), 75);
    assert_eq!(item_durability(&session, bank_guid), 100);
    assert_eq!(item_durability(&session, bank_child_guid), 100);
    assert_eq!(
        item_update_state(&session, gear_guid),
        ItemUpdateState::Unchanged
    );
    assert_eq!(
        item_update_state(&session, backpack_guid),
        ItemUpdateState::Unchanged
    );
    assert_eq!(
        item_update_state(&session, carried_child_guid),
        ItemUpdateState::Unchanged
    );
    assert_eq!(
        item_update_state(&session, bank_guid),
        ItemUpdateState::Unchanged
    );
    assert_eq!(
        item_update_state(&session, bank_child_guid),
        ItemUpdateState::Unchanged
    );
    assert_eq!(
        session
            .canonical_player_effective_combat_stats_like_cpp()
            .unwrap()
            .stats[wow_constants::Stats::Strength as usize],
        2,
        "broken equipment is removed after the sickness BASE_PCT refresh"
    );

    let first_packets = drain_packets(&send_rx);
    let realm_packets = drain_packets(&_realm_rx);
    assert_eq!(
        realm_packets
            .iter()
            .map(|bytes| packet_opcode(bytes))
            .collect::<Vec<_>>(),
        vec![ServerOpcodes::DeathReleaseLoc as u16]
    );
    assert!(
        first_packets
            .iter()
            .all(|bytes| { packet_opcode(bytes) != ServerOpcodes::DurabilityDamageDeath as u16 })
    );

    let expected_durability_updates = [
        durability_update_packet(&session, gear_guid),
        durability_update_packet(&session, backpack_guid),
        durability_update_packet(&session, carried_child_guid),
    ];
    let durability_packet_positions = expected_durability_updates
        .iter()
        .map(|expected| {
            first_packets
                .iter()
                .position(|packet| packet == expected)
                .expect("durability-only item values update")
        })
        .collect::<Vec<_>>();
    assert!(
        durability_packet_positions
            .windows(2)
            .all(|pair| pair[0] < pair[1])
    );
    assert!(!first_packets.contains(&durability_update_packet(&session, bank_guid)));
    assert!(!first_packets.contains(&durability_update_packet(&session, bank_child_guid)));
    let spell_go = first_packets
        .iter()
        .find(|bytes| packet_opcode(bytes) == ServerOpcodes::SpellGo as u16)
        .expect("visible sickness cast publishes SpellGo");
    let (cast_id, original_cast_id, spell_id, visual_id, flags) = decode_spell_go(spell_go);
    assert_eq!(spell_id, SICKNESS_SPELL);
    assert_eq!(visual_id, 245_039);
    assert_eq!(flags, 0x0004_0101);
    assert!(!cast_id.is_empty());
    assert!(original_cast_id.is_empty());
    let aura_updates = first_packets
        .iter()
        .filter(|bytes| packet_opcode(bytes) == ServerOpcodes::AuraUpdate as u16)
        .flat_map(|bytes| decode_sickness_auras(bytes))
        .filter(|aura| aura.spell_id == SICKNESS_SPELL)
        .collect::<Vec<_>>();
    assert!(
        aura_updates.is_empty(),
        "the sickness application remains pending until the selected Unit update"
    );
    let pending_slot = visible_sickness_aura(&session, SICKNESS_SPELL)
        .first()
        .expect("fresh sickness application")
        .slot;
    assert!(
        session
            .core
            .with_owned_player_like_cpp(|player| player
                .unit()
                .subsystems()
                .auras
                .visible_auras_to_update
                .contains(&pending_slot))
            .expect("canonical sickness owner")
    );
    session.update(0).await;
    let recovery_packets = drain_packets(&send_rx);
    let aura_updates = recovery_packets
        .iter()
        .filter(|bytes| packet_opcode(bytes) == ServerOpcodes::AuraUpdate as u16)
        .flat_map(|bytes| decode_sickness_auras(bytes))
        .filter(|aura| aura.spell_id == SICKNESS_SPELL)
        .collect::<Vec<_>>();
    assert_eq!(aura_updates.len(), 1, "Recovery publishes one AuraUpdate");
    let sickness = &aura_updates[0];
    assert_eq!(sickness.cast_id, cast_id);
    assert_eq!(sickness.visual_id, 245_039);
    assert_eq!(sickness.duration_ms, Some(600_000));
    assert_eq!(sickness.remaining_ms, Some(60_000));
    assert_eq!(sickness.aura_flags, 0x15);
    assert_eq!(sickness.active_flags, 0x7);
    let sickness_auras = visible_sickness_aura(&session, SICKNESS_SPELL);
    assert_eq!(sickness_auras.len(), 1);
    assert_eq!(sickness_auras[0].aura_flags, 0x11);
    assert_eq!(sickness_auras[0].effect_mask, 0x7);
    assert_eq!(sickness_auras[0].duration_total, 600_000);
    assert_eq!(sickness_auras[0].duration_remaining, 60_000);
    assert_eq!(
        session
            .canonical_player_effective_combat_stats_like_cpp()
            .unwrap()
            .mod_damage_done_percent,
        [0.25; 7]
    );

    let original_slot = sickness_auras[0].slot;
    let original_provenance = session
        .core
        .with_owned_player_like_cpp(|player| {
            let auras = &player.unit().subsystems().auras;
            auras.aura_cast_provenance_like_cpp(original_slot)
        })
        .expect("canonical sickness Aura provenance");
    assert_eq!(original_provenance.cast_id, sickness.cast_id);
    assert_eq!(
        original_provenance.caster_level,
        Some(original_captured_level)
    );
    assert_ne!(
        original_provenance.caster_level,
        Some(spell_level_fallback),
        "the self caster uses the canonical Player level, not SpellLevels fallback"
    );
    prepare_loaded_sickness_for_recast(&mut session, original_slot, sickness_auras[0].caster_guid);
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_level(refreshed_captured_level)
        })
        .expect("canonical Player owner");

    // C++ permits invoking the healer again while alive. The existing
    // same-caster/mask aura is refreshed in place and cannot multiply BASE_PCT.
    session
        .handle_spirit_healer_activate(healer_packet(healer))
        .await;
    assert_eq!(
        save_port.character_save_count(),
        1,
        "the later no-corpse activation does not trigger another save"
    );
    let refreshed = visible_sickness_aura(&session, SICKNESS_SPELL);
    assert_eq!(refreshed.len(), 1);
    assert_eq!(refreshed[0].slot, original_slot);
    assert_eq!(refreshed[0].duration_total, 900_000);
    assert_eq!(refreshed[0].duration_remaining, 120_000);
    assert_eq!(refreshed[0].stack_count, 1);
    let refreshed_stats = session
        .canonical_player_effective_combat_stats_like_cpp()
        .unwrap();
    assert_eq!(
        refreshed_stats.stats[wow_constants::Stats::Strength as usize],
        2,
        "refresh keeps BASE_PCT at 0.25 instead of multiplying it again"
    );
    assert_eq!(refreshed_stats.mod_damage_done_percent, [0.25; 7]);
    let refreshed_packets = drain_packets(&send_rx);
    let refresh_spell_go = refreshed_packets
        .iter()
        .find(|bytes| packet_opcode(bytes) == ServerOpcodes::SpellGo as u16)
        .expect("sickness recast publishes its own SpellGo");
    let (refresh_cast_id, _, refresh_spell_id, _, _) = decode_spell_go(refresh_spell_go);
    assert_eq!(refresh_spell_id, SICKNESS_SPELL);
    assert_ne!(refresh_cast_id, original_provenance.cast_id);
    assert!(
        refreshed_packets
            .iter()
            .filter(|bytes| packet_opcode(bytes) == ServerOpcodes::AuraUpdate as u16)
            .flat_map(|bytes| decode_sickness_auras(bytes))
            .all(|aura| aura.spell_id != SICKNESS_SPELL),
        "the refreshed sickness remains pending until Recovery"
    );
    session.update(0).await;
    let refreshed_recovery_packets = drain_packets(&send_rx);
    let refreshed_updates = refreshed_recovery_packets
        .iter()
        .filter(|bytes| packet_opcode(bytes) == ServerOpcodes::AuraUpdate as u16)
        .flat_map(|bytes| decode_sickness_auras(bytes))
        .filter(|aura| aura.spell_id == SICKNESS_SPELL)
        .collect::<Vec<_>>();
    assert_eq!(refreshed_updates.len(), 1);
    assert_eq!(refreshed_updates[0].cast_id, sickness.cast_id);
    assert_eq!(refreshed_updates[0].visual_id, sickness.visual_id);
    assert_eq!(refreshed_updates[0].duration_ms, Some(900_000));
    assert_eq!(refreshed_updates[0].remaining_ms, Some(120_000));
    // Every healer resurrection has its own `Player::ResurrectPlayer` full
    // values update. The Aura-specific focused test above checks that sickness
    // does not add a synthetic stat packet of its own.
    let (refreshed_amounts, refreshed_metadata, refreshed_provenance) =
        refreshed_sickness_state(&session, original_slot);
    assert_eq!(refreshed_amounts, vec![-75, -75, 0]);
    assert_eq!(refreshed_metadata.charges, 5);
    assert_eq!(refreshed_metadata.recalculate_mask, 0);
    assert_eq!(
        refreshed_metadata.base_amounts,
        vec![(0, -76), (1, -76), (2, -1)]
    );
    assert_eq!(refreshed_provenance, original_provenance);
    assert_eq!(
        refreshed_provenance.caster_level,
        Some(original_captured_level)
    );
    assert_eq!(
        session
            .core
            .with_owned_player_like_cpp(|player| player.unit().data().level),
        Some(i32::from(refreshed_captured_level)),
        "refresh keeps the original Aura level after the Player level changes"
    );

    // Once the stack is one, the same non-unique Spell refreshes timers and
    // charges. The preceding 3-to-1 recast changed effects/stack but retained
    // its old timer and charges, matching `Aura::ModStackAmount`'s pre-change
    // refresh predicate.
    session
        .handle_spirit_healer_activate(healer_packet(healer))
        .await;
    let single_stack_refresh = visible_sickness_aura(&session, SICKNESS_SPELL);
    assert_eq!(single_stack_refresh.len(), 1);
    assert_eq!(single_stack_refresh[0].slot, original_slot);
    assert_eq!(single_stack_refresh[0].stack_count, 1);
    assert_eq!(single_stack_refresh[0].duration_total, 600_000);
    assert_eq!(single_stack_refresh[0].duration_remaining, 120_000);
    let (_, single_stack_metadata, _) = refreshed_sickness_state(&session, original_slot);
    assert_eq!(single_stack_metadata.charges, 0);
    let single_stack_packets = drain_packets(&send_rx);
    assert!(
        single_stack_packets
            .iter()
            .filter(|bytes| packet_opcode(bytes) == ServerOpcodes::AuraUpdate as u16)
            .flat_map(|bytes| decode_sickness_auras(bytes))
            .all(|aura| aura.spell_id != SICKNESS_SPELL),
        "the final refresh also waits for Recovery"
    );
    session.update(0).await;
    let single_stack_recovery_packets = drain_packets(&send_rx);
    assert_eq!(
        single_stack_recovery_packets
            .iter()
            .filter(|bytes| packet_opcode(bytes) == ServerOpcodes::AuraUpdate as u16)
            .flat_map(|bytes| decode_sickness_auras(bytes))
            .filter(|aura| aura.spell_id == SICKNESS_SPELL)
            .count(),
        1
    );

    let stale = std::time::Instant::now() - std::time::Duration::from_secs(121);
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            let auras = &mut player.unit_mut().subsystems_mut().auras;
            let slot = refreshed[0].slot;
            auras
                .runtime_application_mut_like_cpp(slot)
                .unwrap()
                .applied_at = stale;
        })
        .expect("canonical Player aura owner");
    let expired = wow_world_spell::tick_selected_aura_lifetimes(&mut session, 120_000)
        .expect("the selected Unit update expires the sickness by its remaining duration");
    assert_eq!(expired.expired, 1);
    assert!(visible_sickness_aura(&session, SICKNESS_SPELL).is_empty());
    let expired_stats = session
        .canonical_player_effective_combat_stats_like_cpp()
        .unwrap();
    assert_eq!(
        expired_stats.stats[wow_constants::Stats::Strength as usize],
        10
    );
    assert_eq!(expired_stats.mod_damage_done_percent, [1.0; 7]);
}

#[tokio::test]
async fn sickness_uses_configured_threshold_and_race_spell_even_without_visible_go() {
    for (case, level, configured_threshold, expected_remaining) in [
        (0, 10, None, None),
        (1, 11, None, Some(60_000)),
        (2, 19, None, Some(540_000)),
        (3, 20, None, Some(600_000)),
        (4, 14, Some(12), Some(180_000)),
    ] {
        let base = 43_300 + case * 10;
        let (mut session, send_rx, _realm_rx, _port) = logged_in_released_ghost(base).await;
        let _packet_peer = keep_recovery_input_open(&mut session);
        let healer = insert_healer(
            &session,
            base + 1,
            session.player_position_like_cpp().unwrap(),
            NPCFlags1::SPIRIT_HEALER.bits(),
        );
        session.set_loaded_player_identity_like_cpp(571, 1, 1, level, 0);
        if let Some(sickness_level) = configured_threshold {
            session.set_death_policy_like_cpp(wow_world_core::session::DeathPolicyLikeCpp {
                sickness_level,
                corpse_reclaim_delay_pve: false,
                ..Default::default()
            });
        }
        set_race_sickness_spell(&mut session, CUSTOM_SICKNESS_SPELL);
        install_sickness_data(&mut session, CUSTOM_SICKNESS_SPELL, None);
        let _ = drain_packets(&send_rx);

        session
            .handle_spirit_healer_activate(healer_packet(healer))
            .await;

        let aura = visible_sickness_aura(&session, CUSTOM_SICKNESS_SPELL);
        let player_vitals = session
            .core
            .with_owned_player_like_cpp(|player| {
                (
                    player.unit().death_state(),
                    player.unit().data().health,
                    player.unit().data().max_health,
                )
            })
            .expect("canonical Player after configured spirit-healer case");
        let packets = drain_packets(&send_rx);
        assert!(
            packets
                .iter()
                .all(|bytes| packet_opcode(bytes) != ServerOpcodes::SpellGo as u16),
            "a missing visual mapping and no client-log attributes suppress only SpellGo"
        );
        let immediate_updates = packets
            .iter()
            .filter(|bytes| packet_opcode(bytes) == ServerOpcodes::AuraUpdate as u16)
            .flat_map(|bytes| decode_sickness_auras(bytes))
            .filter(|aura| aura.spell_id == CUSTOM_SICKNESS_SPELL)
            .collect::<Vec<_>>();
        assert!(
            immediate_updates.is_empty(),
            "selected sickness waits for the actual Unit update"
        );
        if let Some(expected_remaining) = expected_remaining {
            assert_eq!(
                aura.len(),
                1,
                "configured race spell case {case}, vitals {player_vitals:?}"
            );
            assert_eq!(aura[0].effect_mask, 0x7);
            assert_eq!(aura[0].duration_total, 600_000);
            assert_eq!(aura[0].duration_remaining, expected_remaining);
            session.update(0).await;
            let recovery_packets = drain_packets(&send_rx);
            assert!(
                recovery_packets
                    .iter()
                    .all(|bytes| packet_opcode(bytes) != ServerOpcodes::SpellGo as u16),
                "Recovery publishes the pending aura without adding a SpellGo"
            );
            let updates = recovery_packets
                .iter()
                .filter(|bytes| packet_opcode(bytes) == ServerOpcodes::AuraUpdate as u16)
                .flat_map(|bytes| decode_sickness_auras(bytes))
                .filter(|aura| aura.spell_id == CUSTOM_SICKNESS_SPELL)
                .collect::<Vec<_>>();
            assert_eq!(updates.len(), 1, "aura applies without SpellGo");
            assert_eq!(updates[0].visual_id, 0);
            assert_eq!(updates[0].duration_ms, Some(600_000));
            assert_eq!(
                updates[0].remaining_ms,
                Some(u32::try_from(expected_remaining).expect("positive aura duration"))
            );
        } else {
            assert!(
                aura.is_empty(),
                "level below configured threshold gets no aura"
            );
            assert!(
                immediate_updates.is_empty(),
                "below-threshold cast sends no AuraUpdate"
            );
        }
    }
}

#[tokio::test]
async fn healer_uses_cached_corpse_graveyard_without_corpse_object() {
    let (mut session, send_rx, _realm_rx, _port) = logged_in_released_ghost(43_501).await;
    let owner = session.player_guid().unwrap();
    // Without terrain, z <= -500 bypasses default graveyard 4 and exercises
    // the linked closest-graveyard lookup at safe location 5.
    let corpse_position = Position::new(1.0, 2.0, -600.0, 0.0);
    let corpse_guid = corpse::current_corpse_guid_like_cpp(&session, owner).unwrap();
    let manager = crate::session::hub_ref(&session)
        .core
        .canonical_map_manager
        .clone()
        .unwrap();
    manager
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .remove_map_object(corpse_guid)
        .expect("map corpse removed while location remains cached");
    assert!(corpse::current_corpse_guid_like_cpp(&session, owner).is_none());
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player.resurrection_state_mut_like_cpp().corpse_location = Some((571, corpse_position));
        })
        .expect("canonical Player resurrection state");
    assert_eq!(
        wow_world_entities::player_corpse_location_like_cpp(&session),
        Some((571, corpse_position))
    );

    let maps = crate::session::hub_ref(&session)
        .catalogs
        .maps
        .store
        .clone()
        .expect("map store");
    let (safe_locs, _) = wow_data::WorldSafeLocStore::from_rows_like_cpp(
        [
            wow_data::WorldSafeLocRow {
                id: wow_data::ALLIANCE_GRAVEYARD_SAFE_LOC_ID_LIKE_CPP,
                map_id: 571,
                x: 50.0,
                y: 60.0,
                z: 70.0,
                facing_degrees: 0.0,
            },
            wow_data::WorldSafeLocRow {
                id: 5,
                map_id: 571,
                x: corpse_position.x,
                y: corpse_position.y,
                z: corpse_position.z,
                facing_degrees: 0.0,
            },
        ],
        &maps,
    );
    session.set_world_safe_loc_store_like_cpp(Arc::new(safe_locs));
    let (graveyards, _) = graveyard_store_with_links(
        0,
        [wow_data::ALLIANCE_GRAVEYARD_SAFE_LOC_ID_LIKE_CPP, 5],
        [],
    );
    session.set_graveyard_store(graveyards.clone());

    let corpse_graveyard = wow_world_entities::closest_graveyard_like_cpp(
        &mut session,
        graveyards.as_ref(),
        571,
        corpse_position,
    );
    assert_eq!(corpse_graveyard, Some(5));
    let ghost_position = session.player_position_like_cpp().unwrap();
    let ghost_map = u32::from(
        crate::session::hub_ref(&session)
            .core
            .player_map_id_like_cpp(),
    );
    let ghost_graveyard = wow_world_entities::closest_graveyard_like_cpp(
        &mut session,
        graveyards.as_ref(),
        ghost_map,
        ghost_position,
    );
    assert_eq!(
        ghost_graveyard,
        Some(wow_data::ALLIANCE_GRAVEYARD_SAFE_LOC_ID_LIKE_CPP)
    );

    let corpse_map = maps.get(571).copied().expect("corpse map is available");
    assert!(corpse_position.is_valid_map_coord_like_cpp());
    assert!(!corpse_map.is_battleground_or_arena());
    assert!(session.core.expansion >= corpse_map.expansion_like_cpp());
    assert!(
        !wow_world_entities::PlayerTeleportHostLikeCpp::is_map_disabled_for_player_like_cpp(
            &session, 571
        )
    );
    assert_eq!(session.core.state, crate::session::SessionState::LoggedIn);
    let teleport_state = crate::session::hub_ref(&session)
        .player_teleport_state_snapshot_like_cpp()
        .expect("canonical teleport state");
    assert_ne!(
        teleport_state.recovery,
        wow_entities::PlayerTransferRecovery::Terminal
    );
    assert!(!teleport_state.near_pending);
    assert!(!teleport_state.far_pending);
    assert!(!teleport_state.can_delay);
    assert_eq!(
        crate::session::hub_ref(&session)
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.map_id),
        Some(571)
    );
    let healer = insert_healer(
        &session,
        43_502,
        session.player_position_like_cpp().unwrap(),
        NPCFlags1::SPIRIT_HEALER.bits(),
    );
    let _ = drain_packets(&send_rx);

    session
        .handle_spirit_healer_activate(healer_packet(healer))
        .await;

    let teleport_state = crate::session::hub_ref(&session)
        .player_teleport_state_snapshot_like_cpp()
        .expect("canonical teleport state");
    assert!(teleport_state.near_pending);
    assert_eq!(
        teleport_state.near_destination,
        Some((571, corpse_position)),
        "same-map teleport relocates to the cached corpse graveyard without a corpse object"
    );
    let movement_packets = drain_packets(&send_rx);
    let teleport_packet = movement_packets
        .iter()
        .find(|bytes| packet_opcode(bytes) == ServerOpcodes::MoveTeleport as u16)
        .expect("same-map near teleport packet");
    let ack_index = i32::try_from(move_teleport_sequence_index(teleport_packet))
        .expect("MoveTeleport sequence index fits ACK field");
    assert_eq!(
        wow_world_entities::handle_move_teleport_ack_like_cpp(&mut session, owner, ack_index, 0,),
        wow_world_core::session::movement_protocol::MoveTeleportAckActionLikeCpp::Accepted
    );
    assert_eq!(session.player_position_like_cpp(), Some(corpse_position));
    let acknowledged_teleport = crate::session::hub_ref(&session)
        .player_teleport_state_snapshot_like_cpp()
        .expect("canonical teleport state after healer graveyard ACK");
    assert!(!acknowledged_teleport.near_pending);
    assert!(corpse::current_corpse_guid_like_cpp(&session, owner).is_none());
}

#[tokio::test]
async fn healer_skips_teleport_when_graveyard_matches_or_corpse_location_is_missing() {
    for (case, corpse_location) in [(0, true), (1, false)] {
        let base = 43_600 + case * 10;
        let (mut session, send_rx, _realm_rx, _port) = logged_in_released_ghost(base).await;
        let owner = session.player_guid().unwrap();
        if corpse_location {
            let position = session.player_position_like_cpp().unwrap();
            let corpse_guid = corpse::current_corpse_guid_like_cpp(&session, owner).unwrap();
            let manager = crate::session::hub_ref(&session)
                .core
                .canonical_map_manager
                .clone()
                .unwrap();
            manager
                .lock()
                .unwrap()
                .find_map_mut(571, 0)
                .unwrap()
                .map_mut()
                .remove_map_object(corpse_guid)
                .expect("remove corpse map object");
            session
                .core
                .with_owned_player_mut_like_cpp(|player| {
                    player.resurrection_state_mut_like_cpp().corpse_location =
                        Some((571, position));
                })
                .expect("canonical Player resurrection state");
        } else {
            session
                .core
                .with_owned_player_mut_like_cpp(|player| {
                    player.resurrection_state_mut_like_cpp().corpse_location = None;
                })
                .expect("canonical Player resurrection state");
        }
        let (graveyards, _) =
            graveyard_store_with_links(0, [wow_data::ALLIANCE_GRAVEYARD_SAFE_LOC_ID_LIKE_CPP], []);
        session.set_graveyard_store(graveyards.clone());
        let ghost_position = session.player_position_like_cpp().unwrap();
        let ghost_map = u32::from(
            crate::session::hub_ref(&session)
                .core
                .player_map_id_like_cpp(),
        );
        assert_eq!(
            wow_world_entities::closest_graveyard_like_cpp(
                &mut session,
                graveyards.as_ref(),
                ghost_map,
                ghost_position,
            ),
            Some(wow_data::ALLIANCE_GRAVEYARD_SAFE_LOC_ID_LIKE_CPP)
        );
        if corpse_location {
            let (corpse_map, corpse_position) =
                wow_world_entities::player_corpse_location_like_cpp(&session).unwrap();
            assert_eq!(
                wow_world_entities::closest_graveyard_like_cpp(
                    &mut session,
                    graveyards.as_ref(),
                    corpse_map,
                    corpse_position,
                ),
                Some(wow_data::ALLIANCE_GRAVEYARD_SAFE_LOC_ID_LIKE_CPP)
            );
        }
        let healer = insert_healer(
            &session,
            base + 1,
            session.player_position_like_cpp().unwrap(),
            NPCFlags1::SPIRIT_HEALER.bits(),
        );
        let _ = drain_packets(&send_rx);

        session
            .handle_spirit_healer_activate(healer_packet(healer))
            .await;

        assert_eq!(session.pending_teleport_like_cpp(), None);
        let teleport_state = crate::session::hub_ref(&session)
            .player_teleport_state_snapshot_like_cpp()
            .expect("canonical teleport state after same-graveyard/no-corpse activation");
        assert!(!teleport_state.near_pending);
        assert!(
            drain_packets(&send_rx)
                .iter()
                .all(|bytes| packet_opcode(bytes) != ServerOpcodes::MoveTeleport as u16)
        );
    }
}

fn add_top_level_item(
    session: &mut WorldSession,
    owner: ObjectGuid,
    guid: ObjectGuid,
    entry: u32,
    slot: u8,
    max_durability: u32,
    durability: u32,
    state: ItemUpdateState,
) {
    session.insert_inventory_item_like_cpp(
        slot,
        crate::session::InventoryItem {
            guid,
            entry_id: entry,
            db_guid: guid.counter() as u64,
            inventory_type: None,
        },
    );
    let mut item =
        session.make_inventory_item_object(guid, entry, owner, 1, 0, ItemContext::None, slot);
    item.set_max_durability(max_durability);
    item.set_durability(durability);
    item.force_state(state);
    session.insert_inventory_item_object(item);
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .store_top_level_item(slot, guid)
                .expect("canonical Player inventory row");
        })
        .expect("canonical Player item topology");
}

fn add_bag_child(
    session: &mut WorldSession,
    owner: ObjectGuid,
    guid: ObjectGuid,
    bag_guid: ObjectGuid,
    container_slot: u8,
    child_slot: u8,
    max_durability: u32,
    durability: u32,
    state: ItemUpdateState,
) {
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .store_bag_item(container_slot, child_slot, guid)
                .expect("canonical Player bag child row");
        })
        .expect("canonical Player bag topology");
    let mut item = session.make_inventory_item_object(
        guid,
        guid.counter() as u32,
        owner,
        1,
        0,
        ItemContext::None,
        child_slot,
    );
    item.set_container_guid_and_slot(bag_guid, container_slot);
    item.set_max_durability(max_durability);
    item.set_durability(durability);
    item.force_state(state);
    session.insert_inventory_item_object(item);
}

fn item_durability(session: &WorldSession, guid: ObjectGuid) -> u32 {
    session
        .resolved_inventory_item_object_like_cpp(guid)
        .expect("inventory item object")
        .data()
        .durability
}

fn item_update_state(session: &WorldSession, guid: ObjectGuid) -> ItemUpdateState {
    session
        .resolved_inventory_item_object_like_cpp(guid)
        .expect("inventory item object")
        .update_state()
}
