use super::*;
use std::os::unix::fs::PermissionsExt;

const ITEM_TEST_REALM: u32 = 7;
const ITEM_TEST_ENTRY: u32 = 18_610;
const ITEM_TEST_CHARACTERS: [u64; 2] = [15, 16];

fn write_test_journal(path: &Path, payload: &[u8]) {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .unwrap();
    file.write_all(payload).unwrap();
    file.sync_all().unwrap();
}

fn valid_atomic_item_push() -> ItemPush {
    let (player_low, player_high) =
        create_player_guid_raw(ITEM_TEST_CHARACTERS[0], ITEM_TEST_REALM);
    ItemPush {
        player_low,
        player_high,
        slot: INVENTORY_SLOT_BAG_0,
        slot_in_bag: i32::from(INVENTORY_SLOT_ITEM_START),
        quest_log_item_id: 0,
        quantity: 1,
        quantity_in_inventory: 1,
        dungeon_encounter_id: 0,
        item_guid_low: 0x1234,
        item_guid_high: (HIGH_GUID_ITEM << 58) | (u64::from(ITEM_TEST_REALM) << 42),
        pushed: false,
        created: false,
        display_text: 1,
        is_bonus_roll: false,
        is_encounter_loot: false,
        item_entry: ITEM_TEST_ENTRY,
    }
}

fn valid_atomic_item_outcome(group_broadcast: bool) -> ([WireEvidence; 2], LootRemovedEvidence) {
    let push = valid_atomic_item_push();
    let removal = LootRemovedEvidence {
        owner_low: 268,
        owner_high: 0x2000_0442_4015_44C0,
        loot_low: 41,
        loot_high: (HIGH_GUID_LOOT_OBJECT << 58) | (u64::from(ITEM_TEST_REALM) << 42),
        loot_list_id: 3,
    };
    let loot_gone = InventoryFailure {
        result: 50,
        item_0_low: 0,
        item_0_high: 0,
        item_1_low: 0,
        item_1_high: 0,
        container_b_slot: 0,
    };
    (
        [
            WireEvidence {
                item_pushes: vec![push],
                loot_removed: vec![removal],
                ..Default::default()
            },
            WireEvidence {
                item_pushes: group_broadcast.then_some(push).into_iter().collect(),
                loot_removed: vec![removal],
                inventory_failures: vec![loot_gone],
                ..Default::default()
            },
        ],
        removal,
    )
}

fn runtime_discovery_options(override_counter: u64) -> LootRaceOptions {
    LootRaceOptions {
        phase: LootRacePhase::CaptureItem,
        participant: 0,
        character_guid: 15,
        peer_name: "Peer".to_string(),
        peer_character_guid: 16,
        killer_character_guid: 15,
        target: LootRaceTarget {
            kind: LootRaceTargetKind::Creature,
            entry: 21_779,
            spawn_guid: 1_117,
            runtime_counter_override: override_counter,
            map_id: 530,
            x: -2_695.57,
            y: 2_633.82,
            z: 74.6837,
            item_entry: 30_712,
        },
        timeout_secs: 30,
        sync: Arc::new(LootRaceSync::new()),
    }
}

fn gameobject_discovery_options(override_counter: u64) -> LootRaceOptions {
    LootRaceOptions {
        phase: LootRacePhase::Race,
        participant: 0,
        character_guid: 15,
        peer_name: "Peer".to_string(),
        peer_character_guid: 16,
        killer_character_guid: 15,
        target: LootRaceTarget {
            kind: LootRaceTargetKind::GameObject,
            entry: DEFAULT_CREATURE_ENTRY,
            spawn_guid: DEFAULT_CREATURE_SPAWN_GUID,
            runtime_counter_override: override_counter,
            map_id: RACE_GAMEOBJECT_MAP_ID,
            x: RACE_GAMEOBJECT_X,
            y: RACE_GAMEOBJECT_Y,
            z: RACE_GAMEOBJECT_Z,
            item_entry: DEFAULT_ITEM_ENTRY,
        },
        timeout_secs: 30,
        sync: Arc::new(LootRaceSync::new()),
    }
}

fn gameobject_runtime_high(map_id: u16, entry: u32) -> u64 {
    (HIGH_GUID_GAMEOBJECT << 58) | (u64::from(map_id) << 29) | (u64::from(entry) << 6)
}

fn update_object_with_guid(low: u64, high: u64) -> Vec<u8> {
    let mut payload = vec![1];
    payload.extend_from_slice(&build_packed_guid(low, high));
    payload
}

fn append_party_player_info_for_test(payload: &mut Vec<u8>, guid: (u64, u64), name: &str) {
    let name_len = u16::try_from(name.len()).unwrap();
    let voice_len_plus_one = 1u16;
    // C++ PartyPackets.cpp writes 15 MSB-first bits and the following
    // packed-GUID write pads the last low bit to the next byte.
    let info_bits = ((name_len << 9) | (voice_len_plus_one << 3) | 0x04) << 1;
    payload.extend_from_slice(&info_bits.to_be_bytes());
    payload.extend_from_slice(&build_packed_guid(guid.0, guid.1));
    payload.extend_from_slice(&[0, 0, 0, 1, 0]);
    payload.extend_from_slice(name.as_bytes());
}

fn party_update_for_test(
    party_flags: u16,
    party_index: u8,
    party_type: u8,
    my_index: i32,
    party_guid: (u64, u64),
    leader_guid: (u64, u64),
    roster: [(u64, u64); 2],
    loot_method: u8,
) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&party_flags.to_le_bytes());
    payload.push(party_index);
    payload.push(party_type);
    payload.extend_from_slice(&my_index.to_le_bytes());
    payload.extend_from_slice(&build_packed_guid(party_guid.0, party_guid.1));
    payload.extend_from_slice(&1u32.to_le_bytes());
    payload.extend_from_slice(&build_packed_guid(leader_guid.0, leader_guid.1));
    payload.push(0);
    payload.extend_from_slice(&2u32.to_le_bytes());
    payload.push(0x60); // no LFG, with loot and difficulty settings
    append_party_player_info_for_test(&mut payload, roster[0], "Leader");
    append_party_player_info_for_test(&mut payload, roster[1], "Peer");
    payload.push(loot_method); // PartyLootSettings.Method
    payload.extend_from_slice(&build_packed_guid(0, 0));
    payload.push(2); // PartyLootSettings.Threshold
    payload.extend_from_slice(&[0; 12]); // PartyDifficultySettings
    payload
}

fn group_capacity_party_update_for_test(
    receiver_index: i32,
    leader_guid: (u64, u64),
    roster: &[(u64, u64)],
) -> Vec<u8> {
    group_capacity_party_update_with_optional_bits_for_test(
        receiver_index,
        leader_guid,
        roster,
        0x60,
    )
}

fn group_capacity_party_update_with_optional_bits_for_test(
    receiver_index: i32,
    leader_guid: (u64, u64),
    roster: &[(u64, u64)],
    optional_bits: u8,
) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&0u16.to_le_bytes());
    payload.push(0);
    payload.push(1);
    payload.extend_from_slice(&receiver_index.to_le_bytes());
    payload.extend_from_slice(&build_packed_guid(77, 88));
    payload.extend_from_slice(&1u32.to_le_bytes());
    payload.extend_from_slice(&build_packed_guid(leader_guid.0, leader_guid.1));
    payload.push(0);
    payload.extend_from_slice(&(roster.len() as u32).to_le_bytes());
    payload.push(optional_bits);
    for (index, guid) in roster.iter().copied().enumerate() {
        append_party_player_info_for_test(&mut payload, guid, &format!("Member{index}"));
    }
    let settings = group_capacity_party_settings_for_test();
    payload.push(settings.loot_method);
    payload.extend_from_slice(&build_packed_guid(0, 0));
    payload.push(settings.loot_threshold);
    payload.extend_from_slice(&settings.dungeon_difficulty_id.to_le_bytes());
    payload.extend_from_slice(&settings.raid_difficulty_id.to_le_bytes());
    payload.extend_from_slice(&settings.legacy_raid_difficulty_id.to_le_bytes());
    payload
}

fn group_capacity_party_settings_for_test() -> GroupCapacityPartySettings {
    GroupCapacityPartySettings {
        loot_method: 0,
        loot_threshold: 2,
        master_looter_guid: 0,
        dungeon_difficulty_id: 1,
        raid_difficulty_id: 14,
        legacy_raid_difficulty_id: 3,
    }
}

fn group_capacity_fixture_for_test() -> GroupCapacityFixture {
    GroupCapacityFixture {
        leader_guid: 14,
        candidate_names: ["CandidateA".into(), "CandidateB".into()],
        candidate_guids: [15, 16],
        initial_member_guids: [13, 14, 17, 18],
        party_settings: group_capacity_party_settings_for_test(),
    }
}

fn group_capacity_options_for_test(candidate_guid: u64) -> GroupCapacityRaceOptions {
    GroupCapacityRaceOptions {
        role: GroupCapacityRaceRole::CandidateA,
        character_guid: candidate_guid,
        leader_guid: 14,
        candidate_names: ["CandidateA".into(), "CandidateB".into()],
        candidate_guids: [15, 16],
        initial_member_guids: [13, 14, 17, 18],
        party_settings: group_capacity_party_settings_for_test(),
        group_db_store_id: 99,
        timeout_secs: 1,
        auth_serial: Arc::new(Mutex::new(())),
        sync: Arc::new(GroupCapacityRaceSync::new()),
    }
}

mod scenarios_1;
mod scenarios_2;
