//! Offline, byte-exact tests for the #38 trade smoke packets and fixture SQL.

use super::*;
use mysql::Value;

const INITIATOR: u64 = 14;
const RECEIVER: u64 = 15;

fn player_guid(counter: u64) -> (u64, u64) {
    create_player_guid_raw(counter, 1)
}

// ─── Client builders ────────────────────────────────────────────────────────

#[test]
fn initiate_trade_is_one_packed_player_guid() {
    let (low, high) = player_guid(RECEIVER);
    // low 15 -> mask 0x01 [0x0F]; high 0x0800_0400_0000_0000 -> mask 0xA0 [0x04, 0x08].
    assert_eq!(
        build_initiate_trade(low, high),
        vec![0x01, 0xA0, 0x0F, 0x04, 0x08]
    );
}

#[test]
fn begin_set_item_set_gold_and_accept_match_cpp_read_layouts() {
    assert!(build_begin_trade().is_empty());
    assert_eq!(
        build_set_trade_item(0, TRADE_PACK_SLOT_BACKPACK, 35),
        [0x00, 0xFF, 0x23]
    );
    assert_eq!(
        build_set_trade_gold(3_000),
        [0xB8, 0x0B, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    );
    assert_eq!(build_accept_trade(1), [0x01, 0x00, 0x00, 0x00]);
    assert_eq!(build_accept_trade(0xDEAD_BEEF), [0xEF, 0xBE, 0xAD, 0xDE]);
}

// ─── SMSG_TRADE_STATUS ──────────────────────────────────────────────────────

#[test]
fn trade_status_proposed_carries_partner_and_empty_account() {
    let packet =
        parse_trade_status_like_cpp(&[0x04, 0x01, 0xA0, 0x0E, 0x04, 0x08, 0x00, 0x00]).unwrap();
    assert_eq!(packet.status, TRADE_STATUS_PROPOSED);
    assert!(!packet.partner_is_same_bnet_account);
    assert_eq!(packet.partner, Some(player_guid(INITIATOR)));
    assert_eq!(packet.partner_account, Some((0, 0)));
}

#[test]
fn trade_status_initiated_accepted_complete_shapes() {
    let initiated = parse_trade_status_like_cpp(&[0x08, 0x2A, 0x00, 0x00, 0x00]).unwrap();
    assert_eq!(initiated.status, TRADE_STATUS_INITIATED);
    assert_eq!(initiated.id, Some(42));

    let accepted = parse_trade_status_like_cpp(&[0x10]).unwrap();
    assert_eq!(accepted.status, TRADE_STATUS_ACCEPTED);
    assert_eq!(trade_status_name(accepted.status), "ACCEPTED");

    let unaccepted = parse_trade_status_like_cpp(&[0x1C]).unwrap();
    assert_eq!(unaccepted.status, TRADE_STATUS_UNACCEPTED);

    // bit7 PartnerIsSameBnetAccount + COMPLETE(8) << 2.
    let complete = parse_trade_status_like_cpp(&[0xA0]).unwrap();
    assert_eq!(complete.status, TRADE_STATUS_COMPLETE);
    assert!(complete.partner_is_same_bnet_account);
    assert_eq!(trade_status_name(complete.status), "COMPLETE");
}

#[test]
fn trade_status_failed_reads_failure_bit_bag_result_and_item() {
    // FAILED(12) << 2 = 0x30, FailureForYou bit1 set.
    let packet =
        parse_trade_status_like_cpp(&[0x32, 0x51, 0x00, 0x00, 0x00, 0x1D, 0x0A, 0x00, 0x00])
            .unwrap();
    assert_eq!(packet.status, TRADE_STATUS_FAILED);
    assert_eq!(packet.failure_for_you, Some(true));
    assert_eq!(packet.bag_result, Some(0x51));
    assert_eq!(packet.item_id, Some(2589));
}

#[test]
fn trade_status_taplist_and_currency_tails() {
    let taplist = parse_trade_status_like_cpp(&[0x5C, 0x03]).unwrap();
    assert_eq!(taplist.status, TRADE_STATUS_NOT_ON_TAPLIST);
    assert_eq!(taplist.trade_slot, Some(3));
    let currency =
        parse_trade_status_like_cpp(&[0x64, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00])
            .unwrap();
    assert_eq!(currency.status, TRADE_STATUS_NOT_ENOUGH_CURRENCY);
    assert_eq!(currency.currency, Some((1, 2)));
}

#[test]
fn trade_status_rejects_padding_truncation_and_trailing_bytes() {
    assert!(parse_trade_status_like_cpp(&[]).is_err());
    assert!(parse_trade_status_like_cpp(&[0x21]).is_err());
    assert!(parse_trade_status_like_cpp(&[0x20, 0x00]).is_err());
    assert!(parse_trade_status_like_cpp(&[0x08, 0x01]).is_err());
    assert!(parse_trade_status_like_cpp(&[0x04, 0x01]).is_err());
}

// ─── SMSG_TRADE_UPDATED ─────────────────────────────────────────────────────

fn plain_offer_bytes(gold: u64, current_state_index: u32) -> Vec<u8> {
    let mut bytes = vec![0x01]; // WhichPlayer
    bytes.extend_from_slice(&0u32.to_le_bytes()); // ID
    bytes.extend_from_slice(&2u32.to_le_bytes()); // ClientStateIndex
    bytes.extend_from_slice(&current_state_index.to_le_bytes());
    bytes.extend_from_slice(&gold.to_le_bytes());
    bytes.extend_from_slice(&[0; 12]); // CurrencyType, CurrencyQuantity, ProposedEnchantment
    bytes.extend_from_slice(&1u32.to_le_bytes()); // Items.size
    bytes.push(0x00); // Slot
    bytes.extend_from_slice(&1u32.to_le_bytes()); // StackCount
    bytes.extend_from_slice(&[0x00, 0x00]); // GiftCreator (empty)
    bytes.extend_from_slice(&2589i32.to_le_bytes()); // ItemID
    bytes.extend_from_slice(&[0; 8]); // RandomPropertiesSeed, RandomPropertiesID
    bytes.push(0x00); // hasItemBonus
    bytes.push(0x00); // ItemModList size 0
    bytes.push(0x80); // hasUnwrapped
    bytes.extend_from_slice(&[0; 8]); // EnchantID, OnUseEnchantmentID
    bytes.extend_from_slice(&[0x00, 0x00]); // Creator (empty)
    bytes.extend_from_slice(&[0; 12]); // Charges, MaxDurability, Durability
    bytes.push(0x00); // Gems.size(2 bits) + Lock
    bytes
}

#[test]
fn trade_updated_plain_offer_decodes_and_matches() {
    let bytes = plain_offer_bytes(3_000, 0x1234_5678);
    assert_eq!(bytes.len(), 82);
    let update = parse_trade_updated_like_cpp(&bytes).unwrap();
    assert_eq!(update.which_player, 1);
    assert_eq!(update.client_state_index, 2);
    assert_eq!(update.current_state_index, 0x1234_5678);
    assert_eq!(update.gold, 3_000);
    assert_eq!(update.items.len(), 1);
    let item = &update.items[0];
    assert_eq!(
        (item.slot, item.stack_count, item.gift_creator),
        (0, 1, (0, 0))
    );
    assert_eq!(item.item.item_id, 2589);
    assert!(item.item.bonus.is_none() && item.item.modifications.is_empty());
    let unwrapped = item.unwrapped.as_ref().expect("unwrapped item");
    assert!(!unwrapped.lock && unwrapped.gems.is_empty());

    assert!(trade_update_matches_offer(&update, 2589, 3_000));
    assert!(!trade_update_matches_offer(&update, 2589, 2_999));
    assert!(!trade_update_matches_offer(&update, 2590, 3_000));
    let mut own_side = update.clone();
    own_side.which_player = 0;
    assert!(!trade_update_matches_offer(&own_side, 2589, 3_000));
}

#[test]
fn trade_updated_gold_only_update_has_no_items() {
    let mut bytes = vec![0x01];
    bytes.extend_from_slice(&[0; 12]);
    bytes.extend_from_slice(&500u64.to_le_bytes());
    bytes.extend_from_slice(&[0; 12]);
    bytes.extend_from_slice(&0u32.to_le_bytes());
    let update = parse_trade_updated_like_cpp(&bytes).unwrap();
    assert_eq!(update.gold, 500);
    assert!(update.items.is_empty());
    assert!(!trade_update_matches_offer(&update, 2589, 500));
}

#[test]
fn trade_updated_decodes_mods_bonus_lock_and_gems() {
    let mut bytes = vec![0x00];
    bytes.extend_from_slice(&7u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&9u32.to_le_bytes());
    bytes.extend_from_slice(&0u64.to_le_bytes());
    bytes.extend_from_slice(&[0; 12]);
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.push(0x06); // Slot 6 (non-traded)
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&[0x01, 0xA0, 0x0E, 0x04, 0x08]); // GiftCreator = initiator
    bytes.extend_from_slice(&100i32.to_le_bytes());
    bytes.extend_from_slice(&[0; 8]);
    bytes.push(0x80); // hasItemBonus
    bytes.push(0x04); // one ItemMod
    bytes.extend_from_slice(&5i32.to_le_bytes());
    bytes.push(0x03);
    bytes.push(0x01); // ItemBonuses.Context
    bytes.extend_from_slice(&2u32.to_le_bytes());
    bytes.extend_from_slice(&10u32.to_le_bytes());
    bytes.extend_from_slice(&20u32.to_le_bytes());
    bytes.push(0x80); // hasUnwrapped
    bytes.extend_from_slice(&[0; 8]);
    bytes.extend_from_slice(&[0x00, 0x00]);
    bytes.extend_from_slice(&[0; 4]);
    bytes.extend_from_slice(&50u32.to_le_bytes());
    bytes.extend_from_slice(&40u32.to_le_bytes());
    bytes.push(0x60); // one gem, Lock
    bytes.push(0x02); // ItemGemData.Slot
    bytes.extend_from_slice(&200i32.to_le_bytes());
    bytes.extend_from_slice(&[0; 8]);
    bytes.extend_from_slice(&[0x00, 0x00]);
    let update = parse_trade_updated_like_cpp(&bytes).unwrap();
    assert_eq!((update.id, update.current_state_index), (7, 9));
    let item = &update.items[0];
    assert_eq!(item.slot, 6);
    assert_eq!(item.gift_creator, player_guid(INITIATOR));
    assert_eq!(item.item.modifications, vec![(5, 3)]);
    assert_eq!(item.item.bonus, Some((1, vec![10, 20])));
    let unwrapped = item.unwrapped.as_ref().unwrap();
    assert_eq!((unwrapped.max_durability, unwrapped.durability), (50, 40));
    assert!(unwrapped.lock);
    assert_eq!(unwrapped.gems.len(), 1);
    assert_eq!(unwrapped.gems[0].0, 2);
    assert_eq!(unwrapped.gems[0].1.item_id, 200);
}

#[test]
fn trade_updated_rejects_bad_padding_counts_and_trailing_bytes() {
    let good = plain_offer_bytes(3_000, 1);
    let mut trailing = good.clone();
    trailing.push(0);
    assert!(parse_trade_updated_like_cpp(&trailing).is_err());
    assert!(parse_trade_updated_like_cpp(&good[..good.len() - 1]).is_err());
    let mut bad_mod_padding = good.clone();
    // hasItemBonus byte at 56, ItemModList byte at 57.
    bad_mod_padding[57] = 0x01;
    assert!(parse_trade_updated_like_cpp(&bad_mod_padding).is_err());
    let mut bad_bonus_padding = good.clone();
    bad_bonus_padding[56] = 0x40;
    assert!(parse_trade_updated_like_cpp(&bad_bonus_padding).is_err());
    let mut too_many = good;
    too_many[33..37].copy_from_slice(&8u32.to_le_bytes());
    assert!(parse_trade_updated_like_cpp(&too_many).is_err());
}

// ─── Fixture plan, SQL, record, DB checks ───────────────────────────────────

fn cli() -> TradeSmokeCli {
    TradeSmokeCli {
        smoke: true,
        ack: true,
        recover: false,
        account_a: DEFAULT_TRADE_ACCOUNT_A.into(),
        account_b: DEFAULT_TRADE_ACCOUNT_B.into(),
        item_entry: DEFAULT_TRADE_ITEM_ENTRY,
        gold: DEFAULT_TRADE_GOLD,
        money_a: DEFAULT_TRADE_MONEY_A,
        money_b: DEFAULT_TRADE_MONEY_B,
        timeout_secs: DEFAULT_TRADE_TIMEOUT_SECS,
        workflow_deadline_secs: DEFAULT_TRADE_WORKFLOW_DEADLINE_SECS,
        bag_destination: false,
    }
}

fn snapshot(guid: u64, money: u64, x: f64) -> TradeCharacterSnapshot {
    TradeCharacterSnapshot {
        account: format!("TESTBOT{}@bot.local", guid - 13),
        account_id: (guid - 6) as u32,
        character_guid: guid,
        race: 1,
        health: 100,
        online: 0,
        money,
        position: CharacterPositionSnapshot {
            map_id: 0,
            zone_id: 12,
            instance_id: 0,
            x,
            y: -132.5,
            z: 83.5,
            orientation: 1.25,
        },
    }
}

fn facts(slots: &[u8]) -> TradeInventoryFacts {
    TradeInventoryFacts {
        occupied_backpack_slots: slots.to_vec(),
        owned_same_entry: 0,
    }
}

fn plan() -> TradeFixturePlan {
    plan_trade_fixture(
        &cli(),
        snapshot(INITIATOR, 77, -8_946.0),
        snapshot(RECEIVER, 88, 100.0),
        &facts(&[35, 37]),
        &facts(&[]),
        1_000,
    )
    .unwrap()
}

#[test]
fn fixture_plan_picks_first_free_slot_and_co_locates_receiver() {
    let plan = plan();
    assert_eq!(plan.item_slot, 36);
    assert_eq!(plan.item_guid, 11_000);
    assert_eq!(plan.item_entry, 2589);
    assert_eq!((plan.initiator_money, plan.receiver_money), (10_000, 500));
    assert_eq!(plan.receiver_position.map_id, 0);
    assert_eq!(plan.receiver_position.x, -8_944.5);
    assert_eq!(plan.receiver_position.y, -132.5);
    assert_eq!(plan.receiver.position.x, 100.0, "original kept for restore");
    let expected = plan.expectation();
    assert_eq!(expected.initiator_money_after, 7_000);
    assert_eq!(expected.receiver_money_after, 3_500);
    assert_eq!(
        (expected.initiator_guid, expected.receiver_guid),
        (INITIATOR, RECEIVER)
    );
}

#[test]
fn fixture_plan_rejects_unsafe_preconditions() {
    fn attempt(
        mutate: impl Fn(
            &mut TradeCharacterSnapshot,
            &mut TradeCharacterSnapshot,
            &mut TradeInventoryFacts,
            &mut TradeInventoryFacts,
        ),
    ) -> Result<TradeFixturePlan> {
        let mut a = snapshot(INITIATOR, 0, 0.0);
        let mut b = snapshot(RECEIVER, 0, 0.0);
        let mut fa = facts(&[]);
        let mut fb = facts(&[]);
        mutate(&mut a, &mut b, &mut fa, &mut fb);
        plan_trade_fixture(&cli(), a, b, &fa, &fb, 0)
    }
    assert!(attempt(|_, _, _, _| {}).is_ok());
    assert!(attempt(|a, _, _, _| a.online = 1).is_err());
    assert!(attempt(|_, b, _, _| b.health = 0).is_err());
    assert!(attempt(|_, b, _, _| b.race = 2).is_err());
    assert!(attempt(|a, _, _, _| a.position.instance_id = 5).is_err());
    assert!(attempt(|a, _, _, _| a.position.map_id = 36).is_err());
    assert!(attempt(|_, _, _, fb| fb.owned_same_entry = 1).is_err());
    assert!(attempt(|_, _, fa, _| fa.occupied_backpack_slots = (35..51).collect()).is_err());
    assert!(attempt(|_, _, _, fb| fb.occupied_backpack_slots = (35..51).collect()).is_err());
    assert!(attempt(|_, b, _, _| b.character_guid = INITIATOR).is_err());
}

fn uints(values: &[u64]) -> Vec<Value> {
    values.iter().map(|value| Value::UInt(*value)).collect()
}

#[test]
fn fixture_setup_sql_inserts_item_and_sets_money_and_position() {
    let plan = plan();
    let statements = trade_fixture_setup_statements(&plan);
    assert_eq!(statements.len(), 4);
    assert!(statements[0].0.starts_with("INSERT INTO item_instance"));
    assert!(statements[0]
        .0
        .contains("VALUES (?, ?, ?, 0, 0, 1, 0, '', '', 0, 0, 0, 0)"));
    assert_eq!(
        statements[0].1,
        mysql::Params::Positional(uints(&[11_000, 2589, INITIATOR]))
    );
    assert_eq!(
        statements[1].0,
        "INSERT INTO character_inventory (guid, bag, slot, item) VALUES (?, 0, ?, ?)"
    );
    assert_eq!(
        statements[1].1,
        mysql::Params::Positional(uints(&[INITIATOR, 36, 11_000]))
    );
    assert_eq!(
        statements[2].0,
        "UPDATE characters SET money = ? WHERE guid = ? AND online = 0"
    );
    assert_eq!(
        statements[2].1,
        mysql::Params::Positional(uints(&[10_000, INITIATOR]))
    );
    assert!(statements[3].0.ends_with("WHERE guid = ? AND online = 0"));
    let mut expected = uints(&[500, 0, 12, 0]);
    expected.extend([
        Value::Double(-8_944.5),
        Value::Double(-132.5),
        Value::Double(83.5),
        Value::Float(1.25),
        Value::UInt(RECEIVER),
    ]);
    assert_eq!(statements[3].1, mysql::Params::Positional(expected));
}

#[test]
fn fixture_cleanup_sql_removes_item_from_either_character_and_restores_originals() {
    let plan = plan();
    let statements = trade_fixture_cleanup_statements(&plan);
    assert_eq!(statements.len(), 4);
    assert_eq!(
        statements[0].0,
        "DELETE FROM character_inventory WHERE item = ? AND guid IN (?, ?)"
    );
    assert_eq!(
        statements[1].0,
        "DELETE FROM item_instance WHERE guid = ? AND owner_guid IN (?, ?)"
    );
    for statement in &statements[..2] {
        assert_eq!(
            statement.1,
            mysql::Params::Positional(uints(&[11_000, INITIATOR, RECEIVER]))
        );
    }
    let restore = |money: u64, x: f64, guid: u64| {
        let mut values = uints(&[money, 0, 12, 0]);
        values.extend([
            Value::Double(x),
            Value::Double(-132.5),
            Value::Double(83.5),
            Value::Float(1.25),
            Value::UInt(guid),
        ]);
        mysql::Params::Positional(values)
    };
    assert_eq!(statements[2].1, restore(77, -8_946.0, INITIATOR));
    assert_eq!(statements[3].1, restore(88, 100.0, RECEIVER));
    assert_eq!(statements[2].0, statements[3].0);
}

#[test]
fn fixture_record_round_trips_and_rejects_other_journals() {
    let plan = plan();
    let record = TradeFixtureRecord {
        version: TRADE_FIXTURE_RECORD_VERSION,
        kind: TRADE_FIXTURE_RECORD_KIND.into(),
        created_by_pid: 1,
        plan: plan.clone(),
    };
    let json = serde_json::to_vec(&record).unwrap();
    let parsed: TradeFixtureRecord = serde_json::from_slice(&json).unwrap();
    assert_eq!(parse_trade_record(parsed).unwrap(), plan);
    let mut loot = record.clone();
    loot.kind = "loot-race".into();
    assert!(parse_trade_record(loot).is_err());
    let mut future = record;
    future.version = 2;
    assert!(parse_trade_record(future).is_err());
}

#[test]
fn db_state_check_requires_receiver_ownership_and_exact_money() {
    let expected = plan().expectation();
    let good = TradeDbState {
        item_row: Some((RECEIVER, 2589, 1)),
        inventory_owners: vec![RECEIVER],
        inventory_positions: vec![(0, 36)],
        initiator_money: 7_000,
        receiver_money: 3_500,
    };
    assert!(check_trade_db_state(&good, &expected).is_ok());
    let variants: Vec<Box<dyn Fn(&mut TradeDbState)>> = vec![
        Box::new(|state: &mut TradeDbState| state.item_row = Some((INITIATOR, 2589, 1))),
        Box::new(|state: &mut TradeDbState| state.item_row = None),
        Box::new(|state: &mut TradeDbState| state.item_row = Some((RECEIVER, 2589, 2))),
        Box::new(|state: &mut TradeDbState| state.inventory_owners = vec![INITIATOR]),
        Box::new(|state: &mut TradeDbState| state.inventory_owners = vec![INITIATOR, RECEIVER]),
        Box::new(|state: &mut TradeDbState| state.inventory_owners.clear()),
        Box::new(|state: &mut TradeDbState| state.initiator_money = 10_000),
        Box::new(|state: &mut TradeDbState| state.receiver_money = 500),
    ];
    for mutate in variants {
        let mut state = good.clone();
        mutate(&mut state);
        assert!(
            check_trade_db_state(&state, &expected).is_err(),
            "{state:?}"
        );
    }
}

#[test]
fn trade_cli_parses_flags_and_validates_guards() {
    let mut options = cli();
    options.smoke = false;
    options.ack = false;
    let mut rest = vec![
        "TESTBOT3@bot.local".to_string(),
        "4000".to_string(),
        "9".to_string(),
    ]
    .into_iter();
    assert!(options.parse_arg("--trade-smoke", &mut rest).unwrap());
    assert!(options.parse_arg(TRADE_ACK_FLAG, &mut rest).unwrap());
    assert!(options.parse_arg("--trade-account-b", &mut rest).unwrap());
    assert!(options.parse_arg("--trade-gold", &mut rest).unwrap());
    assert!(options.parse_arg("--trade-timeout", &mut rest).unwrap());
    assert!(!options.parse_arg("--movement-smoke", &mut rest).unwrap());
    assert_eq!(options.account_b, "TESTBOT3@bot.local");
    assert_eq!((options.gold, options.timeout_secs), (4_000, 9));

    let bot = |account: &str| config::BotConfig {
        account: account.into(),
        password: String::new(),
        character_guid: 1,
        account_id: 1,
        lfg_role: 0,
        class: String::new(),
        enabled: true,
        session_key_bnet: String::new(),
    };
    let pair = [bot(DEFAULT_TRADE_ACCOUNT_A), bot("TESTBOT3@bot.local")];
    assert!(options.validate(&pair).is_ok());
    assert!(options.validate(&pair[..1]).is_err());
    let mut no_ack = options.clone();
    no_ack.ack = false;
    assert!(no_ack.validate(&pair).is_err());
    let mut same = options.clone();
    same.account_b = same.account_a.clone();
    assert!(same.validate(&pair).is_err());
    let mut broke = options.clone();
    broke.gold = broke.money_a + 1;
    assert!(broke.validate(&pair).is_err());
    let mut ack_only = cli();
    ack_only.smoke = false;
    assert!(ack_only.validate(&[]).is_err());
}

/// #1480 review: the bag-destination fixture fills every free backpack slot
/// of the receiver with non-stackable fillers and gives it an empty bag, and
/// the DB check then demands the traded item inside that bag at slot 0.
#[test]
fn bag_destination_fixture_forces_and_checks_an_in_bag_store() {
    let mut options = cli();
    options.bag_destination = true;
    let plan = plan_trade_fixture(
        &options,
        snapshot(INITIATOR, 77, -8_946.0),
        snapshot(RECEIVER, 88, 100.0),
        &facts(&[35, 37]),
        &facts(&[36, 30]),
        1_000,
    )
    .unwrap();
    let fixture = plan.bag_destination.clone().expect("bag fixture planned");
    assert_eq!(fixture.bag_slot, 31, "first free bag slot");
    assert_eq!(fixture.bag_guid, 11_001);
    let filler_slots: Vec<u8> = fixture.fillers.iter().map(|(_, slot)| *slot).collect();
    let expected_slots: Vec<u8> = TRADE_BACKPACK_SLOTS.filter(|slot| *slot != 36).collect();
    assert_eq!(filler_slots, expected_slots);
    assert_eq!(fixture.fillers[0].0, 11_002);
    // Setup inserts the item, the bag and every filler; cleanup removes them.
    let setup = trade_fixture_setup_statements(&plan);
    assert_eq!(setup.len(), 4 + 2 * (1 + fixture.fillers.len()));
    let cleanup = trade_fixture_cleanup_statements(&plan);
    assert_eq!(cleanup.len(), 4 + 2 * (1 + fixture.fillers.len()));
    // A record without the field (older journal) still parses.
    let json = serde_json::to_value(&plan).unwrap();
    let mut legacy = json.clone();
    legacy.as_object_mut().unwrap().remove("bag_destination");
    let legacy: TradeFixturePlan = serde_json::from_value(legacy).unwrap();
    assert_eq!(legacy.bag_destination, None);

    let expected = plan.expectation();
    assert_eq!(expected.receiver_bag_guid, Some(11_001));
    let in_bag = TradeDbState {
        item_row: Some((RECEIVER, 2589, 1)),
        inventory_owners: vec![RECEIVER],
        inventory_positions: vec![(11_001, 0)],
        initiator_money: 7_000,
        receiver_money: 3_500,
    };
    assert!(check_trade_db_state(&in_bag, &expected).is_ok());
    let mut top_level = in_bag.clone();
    top_level.inventory_positions = vec![(0, 0)];
    assert!(check_trade_db_state(&top_level, &expected).is_err());
}
