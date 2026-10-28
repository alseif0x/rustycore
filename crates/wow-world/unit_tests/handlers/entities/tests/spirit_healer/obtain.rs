use super::*;
use std::sync::Arc;
use wow_constants::{InventoryType, ItemBondingType, ItemClass, ItemQuality, ItemSpelltriggerType};
use wow_data::{
    ItemEffectEntry, ItemEffectStore, ItemRandomPropertyTemplateEntry, ItemRecord,
    ItemSparseTemplateEntry, ItemStatsStore, ItemStore,
};

const OBTAIN_ITEM: u32 = 846_721;
const UNAVAILABLE_ITEM: u32 = 846_722;
const MISSING_OBTAIN_SPELL: i32 = 846_799;

fn install_obtain_item_catalogs(session: &mut WorldSession, spell_id: i32) {
    let sparse = |_entry| ItemSparseTemplateEntry {
        flags: [0; 4],
        bag_family: 0,
        start_quest_id: 0,
        stackable: 1,
        max_count: 0,
        lock_id: 0,
        required_reputation_rank: 0,
        sell_price: 0,
        buy_price: 0,
        vendor_stack_count: 1,
        price_variance: 0.0,
        price_random_value: 0.0,
        max_durability: 0,
        other_faction_item_id: 0,
        content_tuning_id: 0,
        player_level_to_item_level_curve_id: 0,
        limit_category: 0,
        instance_bound: 0,
        zone_bound: [0, 0],
        required_reputation_faction: 0,
        allowable_class: -1,
        required_expansion: 0,
        bonding: ItemBondingType::None as u8,
        container_slots: 0,
        inventory_type: InventoryType::NonEquip as i8,
    };
    session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
        id: OBTAIN_ITEM,
        class_id: ItemClass::Consumable as u8,
        subclass_id: 0,
        material: 0,
        inventory_type: InventoryType::NonEquip as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }])));
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [(OBTAIN_ITEM, sparse(OBTAIN_ITEM))],
            [(
                OBTAIN_ITEM,
                ItemRandomPropertyTemplateEntry {
                    item_level: 10,
                    quality: ItemQuality::Normal as i8,
                    inventory_type: InventoryType::NonEquip as i8,
                },
            )],
        ),
    ));
    session.set_item_effect_store(Arc::new(ItemEffectStore::from_entries([ItemEffectEntry {
        id: 846_723,
        legacy_slot_index: 0,
        trigger_type: ItemSpelltriggerType::OnPickup as i8,
        charges: 0,
        cooldown_msec: 5_000,
        category_cooldown_msec: -1,
        spell_category_id: 0,
        spell_id,
        chr_specialization_id: 0,
        parent_item_id: OBTAIN_ITEM,
    }])));
}

fn mark_obtain_test_inventory_complete(session: &mut WorldSession) {
    crate::player_inventory_test_fixture::complete_known_empty_player_inventory_like_cpp(session);
}

#[tokio::test]
async fn known_no_pickup_and_legacy_items_do_not_require_item_level_facts() {
    let (mut session, send_rx, _realm_rx, _port) = logged_in_released_ghost(43_760).await;
    let owner = session.player_guid().expect("canonical Player GUID");
    let normal_item = ObjectGuid::create_item(1, 43_761);
    let legacy_item = ObjectGuid::create_item(1, 43_762);
    install_known_no_obtain_item_catalogs(
        &mut session,
        [
            (846_730, InventoryType::NonEquip, 0),
            (
                846_731,
                InventoryType::NonEquip,
                wow_constants::ItemFlags::LEGACY.bits() as u32,
            ),
        ],
        [],
    );
    add_obtain_top_level_item(
        &mut session,
        owner,
        normal_item,
        846_730,
        INVENTORY_SLOT_ITEM_START,
    );
    add_obtain_top_level_item(
        &mut session,
        owner,
        legacy_item,
        846_731,
        INVENTORY_SLOT_ITEM_START + 1,
    );
    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawns = session.creature_spawn_catalogs_for_test_like_cpp();

    assert_eq!(
        wow_world_spell::apply_inventory_item_obtain_spells_with_generator_like_cpp(
            &mut session,
            generators.item.as_ref(),
            &creature_spawns,
            normal_item,
        )
        .await,
        Ok(Vec::new()),
        "known empty ItemEffect rows do not request cast-item level facts"
    );

    session.catalogs.items.effect_store = None;
    assert_eq!(
        wow_world_spell::apply_inventory_item_obtain_spells_with_generator_like_cpp(
            &mut session,
            generators.item.as_ref(),
            &creature_spawns,
            legacy_item,
        )
        .await,
        Ok(Vec::new()),
        "the source LEGACY flag returns before ItemEffect or cast-item level lookup"
    );
    assert!(drain_packets(&send_rx).is_empty());
}

fn add_obtain_top_level_item(
    session: &mut WorldSession,
    owner_guid: ObjectGuid,
    item_guid: ObjectGuid,
    entry: u32,
    slot: u8,
) {
    add_top_level_item(
        session,
        owner_guid,
        item_guid,
        entry,
        slot,
        0,
        0,
        ItemUpdateState::Unchanged,
    );
}

fn add_bag_obtain_item(
    session: &mut WorldSession,
    owner_guid: ObjectGuid,
    bag_guid: ObjectGuid,
    item_guid: ObjectGuid,
) {
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .store_top_level_item(INVENTORY_SLOT_BAG_START, bag_guid)
                .expect("equipped inventory bag slot");
            player
                .register_bag_storage(INVENTORY_SLOT_BAG_START, bag_guid, 1)
                .expect("bag storage");
            player
                .store_bag_item(INVENTORY_SLOT_BAG_START, 0, item_guid)
                .expect("bag child slot");

            let mut bag = wow_entities::Item::default();
            bag.object_mut().create(bag_guid);
            bag.set_owner_guid(owner_guid);
            player
                .inventory_runtime_mut_like_cpp()
                .store_item_object_like_cpp(bag);

            let mut child = wow_entities::Item::default();
            child.object_mut().create(item_guid);
            child.object_mut().set_entry(OBTAIN_ITEM);
            child.set_owner_guid(owner_guid);
            child.set_container_guid_and_slot(bag_guid, 0);
            player
                .inventory_runtime_mut_like_cpp()
                .store_item_object_like_cpp(child);
        })
        .expect("canonical Player bag inventory");
}

fn spell_opcode_count(packets: &[Vec<u8>], opcode: ServerOpcodes) -> usize {
    packets
        .iter()
        .filter(|packet| u16::from_le_bytes([packet[0], packet[1]]) == opcode as u16)
        .count()
}

#[tokio::test]
async fn healer_resurrection_runs_obtain_between_visibility_and_optional_sickness() {
    let (mut session, send_rx, _realm_rx, _port) = logged_in_released_ghost(43_771).await;
    install_recovery_sickness_fixture_with_normal_cast_costs(&mut session);
    install_obtain_item_catalogs(&mut session, SICKNESS_SPELL);
    let owner = session.player_guid().expect("canonical Player GUID");
    mark_obtain_test_inventory_complete(&mut session);

    let first = ObjectGuid::create_item(1, 43_772);
    let second = ObjectGuid::create_item(1, 43_773);
    let invalid_gear = ObjectGuid::create_item(1, 43_774);
    let invalid_bank = ObjectGuid::create_item(1, 43_775);
    add_obtain_top_level_item(
        &mut session,
        owner,
        first,
        OBTAIN_ITEM,
        INVENTORY_SLOT_ITEM_START,
    );
    add_obtain_top_level_item(
        &mut session,
        owner,
        second,
        OBTAIN_ITEM,
        INVENTORY_SLOT_ITEM_START + 1,
    );
    add_obtain_top_level_item(
        &mut session,
        owner,
        invalid_gear,
        UNAVAILABLE_ITEM,
        EQUIPMENT_SLOT_CHEST,
    );
    add_obtain_top_level_item(
        &mut session,
        owner,
        invalid_bank,
        UNAVAILABLE_ITEM,
        wow_entities::BANK_SLOT_ITEM_START,
    );
    let healer = insert_healer(
        &session,
        43_776,
        session.player_position_like_cpp().expect("ghost position"),
        NPCFlags1::SPIRIT_HEALER.bits(),
    );

    session
        .handle_spirit_healer_activate(healer_packet(healer))
        .await;

    assert!(session.player_is_alive_like_cpp());
    assert!(!visible_sickness_aura(&session, SICKNESS_SPELL).is_empty());
    let packets = drain_packets(&send_rx);
    assert_eq!(
        spell_opcode_count(&packets, ServerOpcodes::SpellGo),
        2,
        "one pickup cast completes, the second live HasAura query skips its duplicate, and the optional healer sickness cast follows"
    );
    assert!(
        packets
            .iter()
            .filter(|packet| u16::from_le_bytes([packet[0], packet[1]])
                == ServerOpcodes::SpellGo as u16)
            .all(|packet| decode_spell_go(packet).2 == SICKNESS_SPELL)
    );
}

#[tokio::test]
async fn known_missing_obtain_spell_is_ignored_and_healer_continues_to_sickness() {
    let (mut session, send_rx, _realm_rx, _port) = logged_in_released_ghost(43_781).await;
    install_recovery_sickness_fixture_with_normal_cast_costs(&mut session);
    install_obtain_item_catalogs(&mut session, MISSING_OBTAIN_SPELL);
    let owner = session.player_guid().expect("canonical Player GUID");
    mark_obtain_test_inventory_complete(&mut session);
    add_obtain_top_level_item(
        &mut session,
        owner,
        ObjectGuid::create_item(1, 43_782),
        OBTAIN_ITEM,
        INVENTORY_SLOT_ITEM_START,
    );
    let healer = insert_healer(
        &session,
        43_783,
        session.player_position_like_cpp().expect("ghost position"),
        NPCFlags1::SPIRIT_HEALER.bits(),
    );

    session
        .handle_spirit_healer_activate(healer_packet(healer))
        .await;

    assert!(session.player_is_alive_like_cpp());
    assert!(!visible_sickness_aura(&session, SICKNESS_SPELL).is_empty());
    let packets = drain_packets(&send_rx);
    assert_eq!(
        spell_opcode_count(&packets, ServerOpcodes::SpellStart),
        0,
        "a known missing SpellInfo is an ordinary ignored CastSpell rejection"
    );
    assert_eq!(
        spell_opcode_count(&packets, ServerOpcodes::SpellGo),
        1,
        "the healer's optional sickness cast follows the rejected obtain spell"
    );
}

#[tokio::test]
async fn healer_holds_obtain_phase_when_item_effect_catalog_is_unavailable() {
    let (mut session, send_rx, _realm_rx, corpse_port) = logged_in_released_ghost(43_786).await;
    install_recovery_sickness_fixture_with_normal_cast_costs(&mut session);
    install_obtain_item_catalogs(&mut session, SICKNESS_SPELL);
    let owner = session.player_guid().expect("canonical Player GUID");
    mark_obtain_test_inventory_complete(&mut session);
    add_obtain_top_level_item(
        &mut session,
        owner,
        ObjectGuid::create_item(1, 43_787),
        OBTAIN_ITEM,
        INVENTORY_SLOT_ITEM_START,
    );
    session.catalogs.items.effect_store = None;
    let healer = insert_healer(
        &session,
        43_788,
        session.player_position_like_cpp().expect("ghost position"),
        NPCFlags1::SPIRIT_HEALER.bits(),
    );

    session
        .handle_spirit_healer_activate(healer_packet(healer))
        .await;

    assert!(session.player_is_alive_like_cpp());
    assert!(visible_sickness_aura(&session, SICKNESS_SPELL).is_empty());
    assert_eq!(
        spell_opcode_count(&drain_packets(&send_rx), ServerOpcodes::SpellGo),
        0
    );
    assert_eq!(
        session
            .core
            .with_owned_player_like_cpp(|player| {
                player
                    .resurrection_state_like_cpp()
                    .delayed_after_teleport
                    .map(|continuation| continuation.phase)
            })
            .flatten(),
        Some(wow_entities::PlayerResurrectionPhase::ObtainSpellsInFlight)
    );
    assert!(corpse_port.delete_requests().is_empty());
}

#[tokio::test]
async fn obtain_scan_casts_a_live_bag_child_with_its_physical_item() {
    let (mut session, send_rx, _realm_rx, _port) = logged_in_released_ghost(43_781).await;
    install_recovery_sickness_fixture_with_normal_cast_costs(&mut session);
    install_obtain_item_catalogs(&mut session, SICKNESS_SPELL);
    let owner = session.player_guid().expect("canonical Player GUID");
    mark_obtain_test_inventory_complete(&mut session);
    let bag_guid = ObjectGuid::create_item(1, 43_782);
    let child_guid = ObjectGuid::create_item(1, 43_783);
    add_bag_obtain_item(&mut session, owner, bag_guid, child_guid);
    let invalid_gear = ObjectGuid::create_item(1, 43_784);
    let invalid_bank = ObjectGuid::create_item(1, 43_785);
    add_top_level_item(
        &mut session,
        owner,
        invalid_gear,
        UNAVAILABLE_ITEM,
        EQUIPMENT_SLOT_CHEST,
        0,
        0,
        ItemUpdateState::Unchanged,
    );
    add_top_level_item(
        &mut session,
        owner,
        invalid_bank,
        UNAVAILABLE_ITEM,
        wow_entities::BANK_SLOT_ITEM_START,
        0,
        0,
        ItemUpdateState::Unchanged,
    );
    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawns = session.creature_spawn_catalogs_for_test_like_cpp();

    wow_world_entities::resurrect_player_like_cpp(
        &mut session,
        wow_world_entities::ResurrectionOperationCatalogs {
            item_guid_generator: generators.item.as_ref(),
            creature_spawns: &creature_spawns,
            graveyards: None,
        },
        0.5,
        false,
    )
    .await
    .expect("bag obtain cast succeeds without scanning gear or bank");

    let packets = drain_packets(&send_rx);
    let start = packets
        .iter()
        .find(|packet| {
            u16::from_le_bytes([packet[0], packet[1]]) == ServerOpcodes::SpellStart as u16
        })
        .expect("bag child's normal-cast SpellStart");
    let mut start_payload = WorldPacket::from_bytes(&start[2..]);
    assert_eq!(start_payload.read_packed_guid().unwrap(), child_guid);
    assert_eq!(spell_opcode_count(&packets, ServerOpcodes::SpellGo), 1);
}

#[tokio::test]
async fn reclaim_holds_after_an_obtain_item_authority_error_without_replaying_prior_cast() {
    let (mut session, send_rx, _realm_rx, corpse_port) =
        corpse::released_ghost_without_reclaim_delay_like_cpp(43_791).await;
    let owner = session.player_guid().expect("canonical Player GUID");
    let corpse_guid =
        corpse::current_corpse_guid_like_cpp(&session, owner).expect("released ghost corpse");
    session.set_player_position_like_cpp(Position::new(10.0, 2.0, 3.0, 0.0));
    install_recovery_sickness_fixture_with_normal_cast_costs(&mut session);
    install_obtain_item_catalogs(&mut session, SICKNESS_SPELL);
    mark_obtain_test_inventory_complete(&mut session);
    let first = ObjectGuid::create_item(1, 43_792);
    let unavailable = ObjectGuid::create_item(1, 43_793);
    add_obtain_top_level_item(
        &mut session,
        owner,
        first,
        OBTAIN_ITEM,
        INVENTORY_SLOT_ITEM_START,
    );
    add_obtain_top_level_item(
        &mut session,
        owner,
        unavailable,
        UNAVAILABLE_ITEM,
        INVENTORY_SLOT_ITEM_START + 1,
    );
    let save_port = crate::session::RecordingPlayerLifecyclePort::new(
        wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_player_lifecycle_port_like_cpp(save_port.clone());
    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawns = session.creature_spawn_catalogs_for_test_like_cpp();
    let graveyards = wow_data::GraveyardStore::default();
    let catalogs = || wow_world_entities::CorpseLifecycleCatalogsLikeCpp {
        item_guid_generator: generators.item.as_ref(),
        creature_spawns: &creature_spawns,
        graveyards: &graveyards,
    };

    assert!(
        !wow_world_entities::reclaim_corpse_like_cpp(&mut session, catalogs()).await,
        "the second physical item has no effective item-level authority"
    );
    assert!(session.player_is_alive_like_cpp());
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| player
            .resurrection_state_like_cpp()
            .delayed_after_teleport
            .map(|continuation| continuation.phase)),
        Some(Some(
            wow_entities::PlayerResurrectionPhase::ObtainSpellsInFlight
        ))
    );
    assert!(corpse_port.delete_requests().is_empty());
    assert_eq!(save_port.character_save_count(), 0);
    let first_attempt_packets = drain_packets(&send_rx);
    assert_eq!(
        spell_opcode_count(&first_attempt_packets, ServerOpcodes::SpellGo),
        1,
        "the first instant obtain spell completes before the later authority error"
    );
    let item_cooldown = session
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .spells
                .history
                .cooldown(SICKNESS_SPELL as u32)
        })
        .flatten()
        .expect("normal item cast starts its item-specific cooldown");
    assert_eq!(
        item_cooldown.item_id, OBTAIN_ITEM,
        "the normal cooldown path resolved the physical cast item's entry and GUID"
    );
    let item_aura = session
        .core
        .with_owned_player_like_cpp(|player| {
            let auras = &player.unit().subsystems().auras;
            let application = auras
                .runtime_applications_like_cpp()
                .values()
                .find(|aura| aura.spell_id == SICKNESS_SPELL)?;
            auras.aura_persistence_metadata(application.slot).cloned()
        })
        .flatten()
        .expect("normal item cast retains physical cast-item Aura metadata");
    assert_eq!(item_aura.item_guid, first);
    assert_eq!(item_aura.cast_item_id, OBTAIN_ITEM);
    assert!(item_aura.cast_item_level > 0);

    <WorldSession as wow_world_entities::CorpseHostLikeCpp>::remove_auras_due_to_spell_like_cpp(
        &mut session,
        SICKNESS_SPELL,
    )
    .expect("remove the first obtain aura before the held retry");
    assert!(
        !wow_world_entities::reclaim_corpse_like_cpp(&mut session, catalogs()).await,
        "an in-flight obtain continuation remains held"
    );
    assert!(
        drain_packets(&send_rx)
            .iter()
            .all(|packet| u16::from_le_bytes([packet[0], packet[1]])
                != ServerOpcodes::SpellGo as u16),
        "the consumed obtain cast is not replayed after the aura is removed"
    );
    assert_eq!(
        corpse::current_corpse_guid_like_cpp(&session, owner),
        Some(corpse_guid)
    );
    assert_eq!(save_port.character_save_count(), 0);
    assert_eq!(corpse_port.delete_requests(), Vec::<u64>::new());
}
