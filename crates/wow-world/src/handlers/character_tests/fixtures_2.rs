//! Shared character-handler test fixtures, part 2.
//!
//! Separated from the character_tests root under #662; every fixture is unchanged.

use super::*;

impl PlayerLifecyclePortLikeCpp for HomebindPortFixtureLikeCpp {
    fn mark_offline_like_cpp<'a>(
        &'a self,
        _mark: PlayerOfflineMarkLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 0 } })
    }

    fn load_initial_world_states_like_cpp<'a>(
        &'a self,
    ) -> PersistenceFutureLikeCpp<'a, PlayerInitialWorldStatesLoadOutcomeLikeCpp> {
        Box::pin(async {
            PlayerInitialWorldStatesLoadOutcomeLikeCpp {
                templates: PlayerInitialWorldStateRowsLikeCpp::Failed {
                    reason: "homebind-only fixture".to_owned(),
                },
                saved_values: PlayerInitialWorldStateRowsLikeCpp::Failed {
                    reason: "homebind-only fixture".to_owned(),
                },
            }
        })
    }

    fn load_login_transports_like_cpp<'a>(
        &'a self,
        _request: PlayerLoginTransportLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginTransportLoadOutcomeLikeCpp> {
        Box::pin(async {
            PlayerLoginTransportLoadOutcomeLikeCpp::Failed {
                reason: "homebind-only fixture".to_owned(),
            }
        })
    }

    fn persist_homebind_like_cpp<'a>(
        &'a self,
        request: PlayerHomebindPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        self.requests.lock().unwrap().push(request);
        let outcome = self
            .outcomes
            .lock()
            .unwrap()
            .pop_front()
            .expect("one typed homebind outcome per request");
        Box::pin(async move { outcome })
    }

    fn clear_buyback_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerBuybackClearRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 0 } })
    }

    fn persist_money_transaction_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerMoneyTransactionRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp> {
        Box::pin(async { wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::Committed })
    }

    fn persist_bank_slot_purchase_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerBankSlotPurchaseRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp> {
        Box::pin(async { wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::Committed })
    }

    fn load_uncage_item_state_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerUncageItemStateRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp>
    {
        Box::pin(async {
            wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp::Failed {
                reason: "homebind-only fixture".to_owned(),
            }
        })
    }

    fn persist_durability_repair_like_cpp<'a>(
        &'a self,
        _repair: wow_persistence::PlayerDurabilityRepairSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 1 } })
    }

    fn persist_money_write_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerMoneyWriteRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 1 } })
    }

    fn persist_currency_save_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerCurrencySaveRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 0 } })
    }

    fn persist_talent_reset_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerTalentResetPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 0 } })
    }

    fn persist_xp_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerXpPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 0 } })
    }

    fn refresh_realm_character_count_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerRealmCharacterCountRefreshRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 0 } })
    }

    fn load_account_collection_like_cpp<'a>(
        &'a self,
        _request: AccountCollectionLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, AccountCollectionLoadOutcomeLikeCpp> {
        Box::pin(async {
            AccountCollectionLoadOutcomeLikeCpp::Failed {
                reason: "homebind-only fixture".to_owned(),
            }
        })
    }

    fn load_character_base_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerCharacterBaseLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, wow_persistence::PlayerCharacterBaseLoadOutcomeLikeCpp> {
        Box::pin(async {
            wow_persistence::PlayerCharacterBaseLoadOutcomeLikeCpp::Failed {
                reason: "homebind-only fixture".to_owned(),
            }
        })
    }

    fn load_login_admission_like_cpp<'a>(
        &'a self,
        _request: wow_persistence::PlayerLoginAdmissionLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, wow_persistence::PlayerLoginAdmissionLoadOutcomeLikeCpp> {
        Box::pin(async {
            wow_persistence::PlayerLoginAdmissionLoadOutcomeLikeCpp::Failed {
                reason: "homebind-only fixture".to_owned(),
            }
        })
    }

    fn load_login_auxiliary_like_cpp<'a>(
        &'a self,
        _request: PlayerLoginAuxiliaryLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginAuxiliaryLoadOutcomeLikeCpp> {
        Box::pin(async {
            PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed {
                reason: "homebind-only fixture".to_owned(),
            }
        })
    }

    fn persist_login_item_repairs_like_cpp<'a>(
        &'a self,
        _request: PlayerLoginItemRepairRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 0 } })
    }

    fn reset_login_pet_talents_like_cpp<'a>(
        &'a self,
        _player_guid: u64,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginPetTalentResetOutcomeLikeCpp> {
        Box::pin(async {
            PlayerLoginPetTalentResetOutcomeLikeCpp {
                spell_delete: PersistenceOutcomeLikeCpp::Applied { rows: 0 },
                specialization_reset: PersistenceOutcomeLikeCpp::Applied { rows: 0 },
            }
        })
    }

    fn mark_player_online_like_cpp<'a>(
        &'a self,
        _request: PlayerOnlineMarkRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 0 } })
    }

    fn save_account_collection_like_cpp<'a>(
        &'a self,
        _save: AccountCollectionSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 0 } })
    }

    fn save_character_like_cpp<'a>(
        &'a self,
        _request: PlayerCharacterSaveRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerCharacterSaveResultLikeCpp> {
        Box::pin(async {
            PlayerCharacterSaveResultLikeCpp {
                outcome: PersistenceOutcomeLikeCpp::Applied { rows: 0 },
                committed: wow_persistence::PlayerCharacterCommittedGroupsLikeCpp::default(),
            }
        })
    }
}

pub(super) fn test_item_enchantments_db_string(entries: &[(usize, i32, u32, i16)]) -> String {
    let mut fields = vec!["0".to_string(); wow_entities::MAX_ENCHANTMENT_SLOT * 3];
    for &(slot, id, duration, charges) in entries {
        let base = slot * 3;
        fields[base] = id.to_string();
        fields[base + 1] = duration.to_string();
        fields[base + 2] = charges.to_string();
    }
    fields.join(" ")
}

pub(super) fn map_corpse_session_with_port_like_cpp(
    outcome: PersistedMapCorpseLoadOutcomeLikeCpp,
) -> (
    WorldSession,
    Arc<std::sync::Mutex<wow_map::MapManager>>,
    Arc<MapCorpseLoadPortFixtureLikeCpp>,
) {
    let port = MapCorpseLoadPortFixtureLikeCpp::new([outcome]);
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(571, 9);
    let manager = Arc::new(std::sync::Mutex::new(manager));
    let (mut session, _) = make_session_with_send_capacity(16);
    session.set_canonical_map_manager(Arc::clone(&manager));
    session.set_map_corpse_persistence_port_like_cpp(port.clone());
    (session, manager, port)
}

pub(super) fn invalid_map_corpse_load_row_like_cpp() -> MapCorpseLoadRowLikeCpp {
    MapCorpseLoadRowLikeCpp {
        pos_x: 10.0,
        pos_y: 20.0,
        pos_z: 30.0,
        orientation: 1.5,
        map_id: 571,
        display_id: 12_345,
        item_cache: String::new(),
        race: 4,
        class: 1,
        sex: 0,
        flags: 0x20,
        dynamic_flags: 0x01,
        ghost_time: 1_000,
        corpse_type: 0,
        instance_id: 9,
        owner_guid: 77,
    }
}

pub(crate) fn make_session_with_send_capacity(
    capacity: usize,
) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(capacity);
    let mut session = WorldSession::new(
        1,
        "TestAccount".into(),
        0,
        2,
        9,
        54261,
        vec![0u8; 40],
        "esES".into(),
        pkt_rx,
        send_tx,
    );
    session.set_item_guid_generator_like_cpp(Arc::new(ObjectGuidGenerator::new(HighGuid::Item, 1)));
    session.set_equipment_set_guid_generator_like_cpp(Arc::new(
        EquipmentSetGuidGeneratorLikeCpp::new(1),
    ));
    (session, send_rx)
}

pub(super) fn inventory_failure_result(packet: &[u8]) -> i32 {
    assert_eq!(
        u16::from_le_bytes([packet[0], packet[1]]),
        ServerOpcodes::InventoryChangeFailure as u16
    );
    i32::from_le_bytes(packet[2..6].try_into().expect("inventory result bytes"))
}

pub(super) fn run_login_grid_cleanup_test(test: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .name("login-grid-cleanup".into())
        .stack_size(8 * 1024 * 1024)
        .spawn(test)
        .unwrap()
        .join()
        .unwrap();
}

pub(super) fn make_session_with_realm_send_capacity(
    capacity: usize,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    flume::Receiver<Vec<u8>>,
) {
    let (mut session, instance_rx) = make_session_with_send_capacity(capacity);
    let (realm_tx, realm_rx) = flume::bounded::<Vec<u8>>(capacity);
    session.install_realm_send_channel_for_test(realm_tx);
    (session, instance_rx, realm_rx)
}

pub(super) fn make_quest_status_session() -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_faction_template_like_cpp(1);
    session.set_player_position_like_cpp(Position::new(10.0, 0.0, 0.0, 0.0));
    (session, send_rx)
}

pub(super) fn strength_item_stats_store(entry_id: u32, amount: i16) -> ItemStatsStore {
    ItemStatsStore::from_parts(
        [(
            entry_id,
            ItemStatEntry {
                stats: std::array::from_fn(|i| {
                    if i == 0 {
                        (ItemModType::Strength as i8, amount)
                    } else {
                        (ItemModType::None as i8, 0)
                    }
                }),
                resistances: [0; 7],
                armor: 0,
            },
        )],
        [],
    )
}

pub(super) fn set_priest_level80_stats(session: &mut WorldSession, base_mana: u32, intellect: u16) {
    session.set_spell_store(Arc::new(wow_data::SpellStore::new()));
    session.set_player_stats(Arc::new(PlayerStatsStore::from_entries([(
        (1, 5, 80),
        PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect,
            spirit: 30,
            base_mana,
        },
    )])));
    session.set_chr_classes_store(Arc::new(ChrClassesStore::from_entries([chr_class_entry(
        5, 0,
    )])));
}

pub(super) fn total_stat_percentage_spell_store_like_cpp(
    spell_id: i32,
    is_ability: bool,
) -> wow_data::SpellStore {
    let mut store = wow_data::SpellStore::new();
    store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE,
                effect_base_points: 99,
                effect_die_sides: 1,
                effect_misc_value_2: 1 << 2,
                ..Default::default()
            }],
        },
    );
    let mut attributes = [0; 15];
    if is_ability {
        attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_IS_ABILITY;
    }
    store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    store
}

pub(super) fn passive_combat_capability_spell_store_like_cpp(
    parry_spell_id: i32,
    block_spell_id: i32,
) -> wow_data::SpellStore {
    let mut store = wow_data::SpellStore::new();
    for (spell_id, effect) in [
        (
            parry_spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_PARRY,
        ),
        (
            block_spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_BLOCK,
        ),
    ] {
        store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: effect,
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                aura_type: None,
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect,
                    ..Default::default()
                }],
            },
        );
        let mut attributes = [0; 15];
        attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_PASSIVE;
        store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    }
    store
}

pub(super) fn attach_stat_update_player_with_mana(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    current_mana: i32,
    max_mana: i32,
) {
    attach_stat_update_player_with_mana_and_health(
        session,
        player_guid,
        current_mana,
        max_mana,
        100,
        100,
    );
}

pub(super) fn attach_stat_update_player_with_mana_and_health(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    current_mana: i32,
    max_mana: i32,
    current_health: u32,
    max_health: u32,
) {
    let mut player = wow_entities::Player::new(Some(1), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    player.unit_mut().world_mut().set_map(571, 0).unwrap();
    player
        .unit_mut()
        .world_mut()
        .relocate(Position::new(10.0, 20.0, 30.0, 0.0));
    player.unit_mut().set_max_health(u64::from(max_health));
    player.unit_mut().set_health(u64::from(current_health));
    player.unit_mut().set_power_index(PowerType::Mana, Some(0));
    player.unit_mut().set_max_power(PowerType::Mana, max_mana);
    player.unit_mut().set_power(PowerType::Mana, current_mana);

    let mut manager = wow_map::MapManager::default();
    manager
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    attach_map_manager(session, manager);
    assert!(
        session.adopt_registered_canonical_player_fixture_like_cpp(),
        "stat fixture must register the same map-owned Player identity as production"
    );
}

pub(super) fn drain_server_opcodes(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<ServerOpcodes> {
    let mut opcodes = Vec::new();
    while let Ok(bytes) = send_rx.try_recv() {
        let packet = WorldPacket::from_bytes(&bytes);
        if let Some(opcode) = packet.server_opcode() {
            opcodes.push(opcode);
        }
    }
    opcodes
}

pub(super) fn alter_appearance_packet(
    new_sex: u8,
    customized_race: i32,
    customized_chr_model_id: i32,
    customizations: &[(i32, i32)],
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(customizations.len() as u32);
    pkt.write_uint8(new_sex);
    pkt.write_int32(customized_race);
    pkt.write_int32(customized_chr_model_id);
    for (option_id, choice_id) in customizations {
        pkt.write_int32(*option_id);
        pkt.write_int32(*choice_id);
    }
    pkt
}

pub(super) fn confirm_barbers_choice_packet(customizations: &[(u32, u32)]) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(customizations.len() as u32);
    for (option_id, choice_id) in customizations {
        pkt.write_uint32(*option_id);
        pkt.write_uint32(*choice_id);
    }
    pkt
}

pub(super) fn read_barber_shop_result(encoded: Vec<u8>) -> i32 {
    let mut packet = WorldPacket::new_client(encoded.as_slice().into());
    assert_eq!(
        packet.server_opcode(),
        Some(wow_constants::ServerOpcodes::BarberShopResult)
    );
    packet.skip_opcode();
    let result = packet.read_int32().unwrap();
    assert_eq!(packet.remaining(), 0);
    result
}

pub(super) fn declined_names_packet(player: ObjectGuid, names: [&str; 5]) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_guid(&player);
    for name in names {
        pkt.write_bits(name.len() as u32, 7);
    }
    for name in names {
        pkt.write_string(name);
    }
    pkt
}

pub(super) fn read_declined_names_result(encoded: Vec<u8>) -> (i32, ObjectGuid) {
    let mut packet = WorldPacket::new_client(encoded.as_slice().into());
    assert_eq!(
        packet.server_opcode(),
        Some(wow_constants::ServerOpcodes::SetPlayerDeclinedNamesResult)
    );
    packet.skip_opcode();
    let result = packet.read_int32().unwrap();
    let player = packet.read_guid().unwrap();
    assert_eq!(packet.remaining(), 0);
    (result, player)
}

pub(super) fn assign_equipment_set_spec_packet(set_id: u32, spec_index: u32) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(set_id);
    pkt.write_uint32(spec_index);
    pkt
}

pub(super) fn delete_equipment_set_packet(id: u64) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint64(id);
    pkt
}

pub(super) fn use_equipment_set_packet(
    guid: u64,
    items: [ObjectGuid; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bits(0, 2);
    for (slot, item) in items.iter().enumerate() {
        pkt.write_guid(item);
        pkt.write_uint8(255);
        pkt.write_uint8(slot as u8);
    }
    pkt.write_uint64(guid);
    pkt
}

pub(super) fn save_equipment_set_packet(
    set_type: i32,
    guid: u64,
    set_id: u32,
    ignore_mask: u32,
    pieces: [ObjectGuid; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
    appearances: [i32; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
    enchants: [i32; 2],
    assigned_spec_index: Option<i32>,
    name: &str,
    icon: &str,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(set_type);
    pkt.write_uint64(guid);
    pkt.write_uint32(set_id);
    pkt.write_uint32(ignore_mask);
    for i in 0..wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP {
        pkt.write_guid(&pieces[i]);
        pkt.write_int32(appearances[i]);
    }
    pkt.write_int32(enchants[0]);
    pkt.write_int32(enchants[1]);
    pkt.write_int32(0);
    pkt.write_int32(0);
    pkt.write_int32(0);
    pkt.write_int32(0);
    pkt.write_bit(assigned_spec_index.is_some());
    pkt.write_bits(name.len() as u32, 8);
    pkt.write_bits(icon.len() as u32, 9);
    if let Some(spec_index) = assigned_spec_index {
        pkt.write_int32(spec_index);
    }
    pkt.write_string(name);
    pkt.write_string(icon);
    pkt
}

pub(super) fn read_equipment_set_id(encoded: Vec<u8>) -> (u64, i32, u32) {
    let mut packet = WorldPacket::new_client(encoded.as_slice().into());
    assert_eq!(
        packet.server_opcode(),
        Some(wow_constants::ServerOpcodes::EquipmentSetId)
    );
    packet.skip_opcode();
    let guid = packet.read_uint64().unwrap();
    let set_type = packet.read_int32().unwrap();
    let set_id = packet.read_uint32().unwrap();
    assert_eq!(packet.remaining(), 0);
    (guid, set_type, set_id)
}

pub(super) fn read_use_equipment_set_result(encoded: Vec<u8>) -> (u64, u8) {
    let mut packet = WorldPacket::new_client(encoded.as_slice().into());
    assert_eq!(
        packet.server_opcode(),
        Some(wow_constants::ServerOpcodes::UseEquipmentSetResult)
    );
    packet.skip_opcode();
    let guid = packet.read_uint64().unwrap();
    let reason = packet.read_uint8().unwrap();
    assert_eq!(packet.remaining(), 0);
    (guid, reason)
}

pub(super) fn install_child_equipment_fixture(
    session: &mut WorldSession,
    parent_entry: u32,
    child_entry: u32,
    child_slot: u8,
) {
    session.set_item_child_equipment_store(Arc::new(ItemChildEquipmentStore::from_entries([
        ItemChildEquipmentEntry {
            id: 1,
            child_item_id: child_entry as i32,
            child_item_equip_slot: child_slot,
            parent_item_id: parent_entry,
        },
    ])));
}

pub(super) fn make_area_spirit_healer_session(
    capacity: usize,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    Arc<std::sync::Mutex<wow_map::MapManager>>,
) {
    let (mut session, send_rx) = make_session_with_send_capacity(capacity);
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 10)));
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(0.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_player_alive_like_cpp(false);
    (session, send_rx, canonical)
}

pub(super) fn make_bank_slot_session(
    capacity: usize,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    Arc<std::sync::Mutex<wow_map::MapManager>>,
) {
    let (mut session, send_rx) = make_session_with_send_capacity(capacity);
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 10)));
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(0.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_player_faction_template_like_cpp(1);
    session.set_bank_bag_slot_prices_store(Arc::new(
        wow_data::BankBagSlotPricesStore::from_entries([
            wow_data::BankBagSlotPricesEntry { id: 1, cost: 100 },
            wow_data::BankBagSlotPricesEntry { id: 2, cost: 200 },
        ]),
    ));
    session.set_player_gold_like_cpp(150);
    session.set_player_bank_bag_slot_count_like_cpp(0);
    (session, send_rx, canonical)
}

pub(super) fn insert_bank_test_player_in_world(
    session: &WorldSession,
    canonical: &Arc<std::sync::Mutex<wow_map::MapManager>>,
) {
    let player_guid = session.player_guid().expect("player guid");
    let mut player = wow_entities::Player::new(Some(1), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    player.unit_mut().world_mut().set_map(571, 0).unwrap();
    player
        .unit_mut()
        .world_mut()
        .relocate(Position::new(0.0, 0.0, 0.0, 0.0));
    player.unit_mut().world_mut().object_mut().add_to_world();
    canonical
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
        .unwrap();
}
