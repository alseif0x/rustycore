//! Characters-DB fixture, durable recovery record, persistence checks and the
//! two-session workflow for the player TRADE smoke (#38).
//!
//! The fixture only touches the two disposable `@bot.local` characters while
//! they are offline: one isolated item row for the initiator, both money
//! columns, and the receiver's position (moved beside the initiator). A
//! recovery record is written (0600, fsync'd, create-new) before the first DB
//! write and consumed by a verified restore on every exit path, or later by
//! `--recover-trade-fixture`.

use super::*;
use mysql::prelude::Queryable;
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) const TRADE_ACK_FLAG: &str = "--ack-disposable-trade";
/// TESTBOT2 and TESTBOT3 are both Alliance (Human); TESTBOT1 is Horde, and C++
/// answers TRADE_STATUS_WRONG_FACTION across teams.
pub(crate) const DEFAULT_TRADE_ACCOUNT_A: &str = "TESTBOT2@bot.local";
pub(crate) const DEFAULT_TRADE_ACCOUNT_B: &str = "TESTBOT3@bot.local";
/// Linen Cloth: common, unbound, tradeable (same default as bank/vendor-sell).
pub(crate) const DEFAULT_TRADE_ITEM_ENTRY: u32 = 2589;
pub(crate) const DEFAULT_TRADE_GOLD: u64 = 3_000;
pub(crate) const DEFAULT_TRADE_MONEY_A: u64 = 10_000;
pub(crate) const DEFAULT_TRADE_MONEY_B: u64 = 500;
pub(crate) const DEFAULT_TRADE_TIMEOUT_SECS: u64 = 30;
pub(crate) const DEFAULT_TRADE_WORKFLOW_DEADLINE_SECS: u64 = 600;
/// Receiver offset from the initiator, well inside `TRADE_DISTANCE` (11.11).
pub(crate) const TRADE_RECEIVER_OFFSET_YARDS: f64 = 1.5;
pub(crate) const TRADE_FIXTURE_RECORD_VERSION: u32 = 1;
pub(crate) const TRADE_FIXTURE_RECORD_KIND: &str = "trade-smoke";
/// Player.h `INVENTORY_SLOT_ITEM_START` + default 16-slot backpack.
pub(crate) const TRADE_BACKPACK_SLOTS: std::ops::Range<u8> =
    INVENTORY_SLOT_ITEM_START..INVENTORY_SLOT_ITEM_START + 16;
/// Open-world continents (Eastern Kingdoms, Kalimdor, Outland, Northrend).
pub(crate) const TRADE_CONTINENT_MAPS: [u32; 4] = [0, 1, 530, 571];
pub(crate) const TRADE_POSITION_TOLERANCE: f64 = 0.01;
/// Player.h `INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END` in 3.4.3.
pub(crate) const TRADE_BAG_SLOTS: std::ops::Range<u8> = 30..34;
/// Small Brown Pouch: a plain 6-slot container.
pub(crate) const TRADE_BAG_ENTRY: u32 = 4496;
/// Worn Shortsword: non-stackable, so a traded stack can never merge into it.
pub(crate) const TRADE_FILLER_ENTRY: u32 = 25;

// ─── CLI ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub(crate) struct TradeSmokeCli {
    pub(crate) smoke: bool,
    pub(crate) ack: bool,
    pub(crate) recover: bool,
    pub(crate) account_a: String,
    pub(crate) account_b: String,
    pub(crate) item_entry: u32,
    pub(crate) gold: u64,
    pub(crate) money_a: u64,
    pub(crate) money_b: u64,
    pub(crate) timeout_secs: u64,
    pub(crate) workflow_deadline_secs: u64,
    /// Fill the receiver's free backpack slots and give it an empty bag, so
    /// the traded item must be stored inside that bag (#1480 review).
    pub(crate) bag_destination: bool,
}

fn env_parse<T: std::str::FromStr>(name: &str, default: T) -> Result<T>
where
    T::Err: std::fmt::Display,
{
    match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => value
            .trim()
            .parse()
            .map_err(|error| anyhow!("{name}: {error}")),
        _ => Ok(default),
    }
}

impl TradeSmokeCli {
    pub(crate) fn from_env() -> Result<Self> {
        Ok(Self {
            smoke: std::env::var("WOW_BOT_TRADE_SMOKE").is_ok_and(|value| is_truthy(&value)),
            ack: std::env::var("WOW_BOT_ACK_DISPOSABLE_TRADE").is_ok_and(|value| is_truthy(&value)),
            recover: false,
            bag_destination: std::env::var("WOW_BOT_TRADE_BAG_DESTINATION")
                .is_ok_and(|value| is_truthy(&value)),
            account_a: std::env::var("WOW_BOT_TRADE_ACCOUNT_A")
                .unwrap_or_else(|_| DEFAULT_TRADE_ACCOUNT_A.to_string()),
            account_b: std::env::var("WOW_BOT_TRADE_ACCOUNT_B")
                .unwrap_or_else(|_| DEFAULT_TRADE_ACCOUNT_B.to_string()),
            item_entry: env_parse("WOW_BOT_TRADE_ITEM_ENTRY", DEFAULT_TRADE_ITEM_ENTRY)?,
            gold: env_parse("WOW_BOT_TRADE_GOLD", DEFAULT_TRADE_GOLD)?,
            money_a: env_parse("WOW_BOT_TRADE_MONEY_A", DEFAULT_TRADE_MONEY_A)?,
            money_b: env_parse("WOW_BOT_TRADE_MONEY_B", DEFAULT_TRADE_MONEY_B)?,
            timeout_secs: env_parse("WOW_BOT_TRADE_TIMEOUT_SECS", DEFAULT_TRADE_TIMEOUT_SECS)?,
            workflow_deadline_secs: env_parse(
                "WOW_BOT_TRADE_WORKFLOW_DEADLINE_SECS",
                DEFAULT_TRADE_WORKFLOW_DEADLINE_SECS,
            )?,
        })
    }

    /// Consume one trade flag (and its value). Returns false for other flags.
    pub(crate) fn parse_arg(
        &mut self,
        arg: &str,
        args: &mut impl Iterator<Item = String>,
    ) -> Result<bool> {
        match arg {
            "--trade-smoke" => self.smoke = true,
            TRADE_ACK_FLAG => self.ack = true,
            "--recover-trade-fixture" => self.recover = true,
            "--trade-account-a" => self.account_a = next_arg(args, arg)?,
            "--trade-account-b" => self.account_b = next_arg(args, arg)?,
            "--trade-item-entry" => self.item_entry = next_arg(args, arg)?.parse()?,
            "--trade-gold" => self.gold = next_arg(args, arg)?.parse()?,
            "--trade-money-a" => self.money_a = next_arg(args, arg)?.parse()?,
            "--trade-money-b" => self.money_b = next_arg(args, arg)?.parse()?,
            "--trade-timeout" => self.timeout_secs = next_arg(args, arg)?.parse()?,
            "--trade-workflow-deadline" => {
                self.workflow_deadline_secs = next_arg(args, arg)?.parse()?
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    pub(crate) fn validate(&self, bots: &[config::BotConfig]) -> Result<()> {
        if !self.smoke {
            if self.ack {
                bail!("{TRADE_ACK_FLAG} is only valid with --trade-smoke");
            }
            return Ok(());
        }
        if !self.ack {
            bail!("--trade-smoke mutates two disposable characters; rerun with {TRADE_ACK_FLAG}");
        }
        if self.account_a.eq_ignore_ascii_case(&self.account_b) {
            bail!("trade accounts must be distinct");
        }
        for account in [&self.account_a, &self.account_b] {
            require_bot_local_account(account, "trade")?;
        }
        if bots.len() != 2 {
            bail!(
                "--trade-smoke requires exactly the two configured trade bots {} and {}, found {}",
                self.account_a,
                self.account_b,
                bots.len()
            );
        }
        if self.timeout_secs == 0 || self.workflow_deadline_secs == 0 {
            bail!("trade timeouts must be greater than zero");
        }
        if self.item_entry == 0 || self.item_entry > i32::MAX as u32 {
            bail!("trade item entry does not fit the 3.4.3 ItemInstance.ItemID wire field");
        }
        if self.gold == 0 || self.gold > self.money_a {
            bail!(
                "trade gold {} must be nonzero and covered by the initiator's fixture money {}",
                self.gold,
                self.money_a
            );
        }
        if self.money_b.saturating_add(self.gold) > loot_race::MAX_PLAYER_MONEY_LIKE_CPP {
            bail!("receiver money plus trade gold exceeds MAX_MONEY_AMOUNT");
        }
        Ok(())
    }
}

// ─── Fixture plan ───────────────────────────────────────────────────────────

/// Offline snapshot of one fixture character, recorded for exact restoration.
#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq)]
pub(crate) struct TradeCharacterSnapshot {
    pub(crate) account: String,
    pub(crate) account_id: u32,
    pub(crate) character_guid: u64,
    pub(crate) race: u8,
    pub(crate) health: u32,
    pub(crate) online: u8,
    pub(crate) money: u64,
    pub(crate) position: CharacterPositionSnapshot,
}

/// Inventory facts the plan needs, per character.
#[derive(Debug, Clone, Default)]
pub(crate) struct TradeInventoryFacts {
    pub(crate) occupied_backpack_slots: Vec<u8>,
    pub(crate) owned_same_entry: u64,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq)]
pub(crate) struct TradeFixturePlan {
    pub(crate) item_guid: u64,
    pub(crate) item_entry: u32,
    pub(crate) item_slot: u8,
    pub(crate) gold: u64,
    pub(crate) initiator_money: u64,
    pub(crate) receiver_money: u64,
    pub(crate) receiver_position: CharacterPositionSnapshot,
    /// Originals, restored verbatim by cleanup/recovery.
    pub(crate) initiator: TradeCharacterSnapshot,
    pub(crate) receiver: TradeCharacterSnapshot,
    /// The bag-destination fixture, when requested.
    #[serde(default)]
    pub(crate) bag_destination: Option<TradeBagFixture>,
}

/// Receiver-side rows of the bag-destination fixture.
#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq)]
pub(crate) struct TradeBagFixture {
    pub(crate) bag_guid: u64,
    pub(crate) bag_slot: u8,
    /// (item guid, backpack slot) of each filler.
    pub(crate) fillers: Vec<(u64, u8)>,
}

impl TradeBagFixture {
    fn item_guids(&self) -> Vec<u64> {
        std::iter::once(self.bag_guid)
            .chain(self.fillers.iter().map(|(guid, _)| *guid))
            .collect()
    }
}

impl TradeFixturePlan {
    pub(crate) fn expectation(&self) -> TradeDbExpectation {
        TradeDbExpectation {
            initiator_guid: self.initiator.character_guid,
            receiver_guid: self.receiver.character_guid,
            item_guid: self.item_guid,
            item_entry: self.item_entry,
            initiator_money_after: self.initiator_money - self.gold,
            receiver_money_after: self.receiver_money + self.gold,
            receiver_bag_guid: self
                .bag_destination
                .as_ref()
                .map(|fixture| fixture.bag_guid),
        }
    }
}

/// Pure precondition check and fixture choice (unit-tested).
pub(crate) fn plan_trade_fixture(
    cli: &TradeSmokeCli,
    initiator: TradeCharacterSnapshot,
    receiver: TradeCharacterSnapshot,
    initiator_inventory: &TradeInventoryFacts,
    receiver_inventory: &TradeInventoryFacts,
    max_item_guid: u64,
) -> Result<TradeFixturePlan> {
    for character in [&initiator, &receiver] {
        if character.online != 0 {
            bail!(
                "trade character {} is online; log it out before fixture setup",
                character.character_guid
            );
        }
        if character.health == 0 {
            bail!(
                "trade character {} is dead (HandleInitiateTradeOpcode refuses TRADE_STATUS_DEAD)",
                character.character_guid
            );
        }
    }
    if initiator.character_guid == receiver.character_guid {
        bail!("trade accounts resolve to the same character");
    }
    if loot_race::faction_for_race(initiator.race) != loot_race::faction_for_race(receiver.race) {
        bail!(
            "trade characters are on different teams (races {} and {}); C++ answers TRADE_STATUS_WRONG_FACTION",
            initiator.race,
            receiver.race
        );
    }
    let position = initiator.position;
    if position.instance_id != 0 || !TRADE_CONTINENT_MAPS.contains(&position.map_id) {
        bail!(
            "trade initiator stands on map {} instance {}; the fixture only co-locates players on an open-world continent",
            position.map_id,
            position.instance_id
        );
    }
    for (label, facts) in [
        ("initiator", initiator_inventory),
        ("receiver", receiver_inventory),
    ] {
        if facts.owned_same_entry != 0 {
            bail!(
                "trade {label} already owns {} item_instance row(s) of entry {}; a stack merge would hide the traded item, choose an isolated --trade-item-entry",
                facts.owned_same_entry,
                cli.item_entry
            );
        }
    }
    let item_slot = TRADE_BACKPACK_SLOTS
        .clone()
        .find(|slot| !initiator_inventory.occupied_backpack_slots.contains(slot))
        .ok_or_else(|| anyhow!("trade initiator has no empty default backpack slot"))?;
    if TRADE_BACKPACK_SLOTS
        .clone()
        .all(|slot| receiver_inventory.occupied_backpack_slots.contains(&slot))
    {
        bail!("trade receiver has no empty default backpack slot for the traded item");
    }
    let item_guid = max_item_guid
        .checked_add(10_000)
        .ok_or_else(|| anyhow!("item guid overflow while reserving the trade fixture"))?;
    let bag_destination = if cli.bag_destination {
        let bag_slot = TRADE_BAG_SLOTS
            .clone()
            .find(|slot| !receiver_inventory.occupied_backpack_slots.contains(slot))
            .ok_or_else(|| anyhow!("trade receiver has no empty bag slot for the bag fixture"))?;
        let fillers = TRADE_BACKPACK_SLOTS
            .clone()
            .filter(|slot| !receiver_inventory.occupied_backpack_slots.contains(slot))
            .zip(item_guid + 2..)
            .map(|(slot, guid)| (guid, slot))
            .collect();
        Some(TradeBagFixture {
            bag_guid: item_guid + 1,
            bag_slot,
            fillers,
        })
    } else {
        None
    };
    let receiver_position = CharacterPositionSnapshot {
        x: position.x + TRADE_RECEIVER_OFFSET_YARDS,
        ..position
    };
    Ok(TradeFixturePlan {
        item_guid,
        item_entry: cli.item_entry,
        item_slot,
        gold: cli.gold,
        initiator_money: cli.money_a,
        receiver_money: cli.money_b,
        receiver_position,
        initiator,
        receiver,
        bag_destination,
    })
}

// ─── SQL builders (unit-tested) ─────────────────────────────────────────────

pub(crate) type SqlStatement = (&'static str, mysql::Params);

fn position_params(
    money: u64,
    position: &CharacterPositionSnapshot,
    guid: u64,
) -> Vec<mysql::Value> {
    vec![
        money.into(),
        position.map_id.into(),
        position.zone_id.into(),
        position.instance_id.into(),
        position.x.into(),
        position.y.into(),
        position.z.into(),
        position.orientation.into(),
        guid.into(),
    ]
}

const RESTORE_CHARACTER_SQL: &str =
    "UPDATE characters SET money = ?, map = ?, zone = ?, instance_id = ?, \
     position_x = ?, position_y = ?, position_z = ?, orientation = ? WHERE guid = ? AND online = 0";

/// Statements applied in one transaction after the recovery record exists.
pub(crate) fn trade_fixture_setup_statements(plan: &TradeFixturePlan) -> Vec<SqlStatement> {
    let a = plan.initiator.character_guid;
    let b = plan.receiver.character_guid;
    let mut statements = vec![
        (
            "INSERT INTO item_instance \
             (guid, itemEntry, owner_guid, creatorGuid, giftCreatorGuid, count, durability, \
              enchantments, charges, flags, randomPropertiesId, randomPropertiesSeed, context) \
             VALUES (?, ?, ?, 0, 0, 1, 0, '', '', 0, 0, 0, 0)",
            mysql::Params::Positional(vec![
                plan.item_guid.into(),
                plan.item_entry.into(),
                a.into(),
            ]),
        ),
        (
            "INSERT INTO character_inventory (guid, bag, slot, item) VALUES (?, 0, ?, ?)",
            mysql::Params::Positional(vec![a.into(), plan.item_slot.into(), plan.item_guid.into()]),
        ),
        (
            "UPDATE characters SET money = ? WHERE guid = ? AND online = 0",
            mysql::Params::Positional(vec![plan.initiator_money.into(), a.into()]),
        ),
        (
            RESTORE_CHARACTER_SQL,
            mysql::Params::Positional(position_params(
                plan.receiver_money,
                &plan.receiver_position,
                b,
            )),
        ),
    ];
    if let Some(fixture) = &plan.bag_destination {
        let rows = std::iter::once((fixture.bag_guid, fixture.bag_slot, TRADE_BAG_ENTRY)).chain(
            fixture
                .fillers
                .iter()
                .map(|(guid, slot)| (*guid, *slot, TRADE_FILLER_ENTRY)),
        );
        for (guid, slot, entry) in rows {
            statements.push((
                "INSERT INTO item_instance \
                 (guid, itemEntry, owner_guid, creatorGuid, giftCreatorGuid, count, durability, \
                  enchantments, charges, flags, randomPropertiesId, randomPropertiesSeed, context) \
                 VALUES (?, ?, ?, 0, 0, 1, 0, '', '', 0, 0, 0, 0)",
                mysql::Params::Positional(vec![guid.into(), entry.into(), b.into()]),
            ));
            statements.push((
                "INSERT INTO character_inventory (guid, bag, slot, item) VALUES (?, 0, ?, ?)",
                mysql::Params::Positional(vec![b.into(), slot.into(), guid.into()]),
            ));
        }
    }
    statements
}

/// Idempotent restore: removes the fixture item wherever the trade left it
/// (only under the two fixture characters) and writes back both originals.
pub(crate) fn trade_fixture_cleanup_statements(plan: &TradeFixturePlan) -> Vec<SqlStatement> {
    let a = plan.initiator.character_guid;
    let b = plan.receiver.character_guid;
    let mut statements = Vec::new();
    if let Some(fixture) = &plan.bag_destination {
        // The traded item may sit inside the bag: its row goes first, then
        // the bag and fillers, wherever they ended up under the two players.
        for guid in fixture.item_guids() {
            statements.push((
                "DELETE FROM character_inventory WHERE item = ? AND guid IN (?, ?)",
                mysql::Params::Positional(vec![guid.into(), a.into(), b.into()]),
            ));
            statements.push((
                "DELETE FROM item_instance WHERE guid = ? AND owner_guid IN (?, ?)",
                mysql::Params::Positional(vec![guid.into(), a.into(), b.into()]),
            ));
        }
    }
    statements.extend([
        (
            "DELETE FROM character_inventory WHERE item = ? AND guid IN (?, ?)",
            mysql::Params::Positional(vec![plan.item_guid.into(), a.into(), b.into()]),
        ),
        (
            "DELETE FROM item_instance WHERE guid = ? AND owner_guid IN (?, ?)",
            mysql::Params::Positional(vec![plan.item_guid.into(), a.into(), b.into()]),
        ),
        (
            RESTORE_CHARACTER_SQL,
            mysql::Params::Positional(position_params(
                plan.initiator.money,
                &plan.initiator.position,
                a,
            )),
        ),
        (
            RESTORE_CHARACTER_SQL,
            mysql::Params::Positional(position_params(
                plan.receiver.money,
                &plan.receiver.position,
                b,
            )),
        ),
    ]);
    statements
}

// ─── DB access ──────────────────────────────────────────────────────────────

fn trade_characters_conn() -> Result<mysql::Conn> {
    let opts = qa_mysql_opts(&characters_db_url()?, "characters")?;
    mysql::Conn::new(opts).map_err(|error| anyhow!("Connect to characters DB failed: {error}"))
}

fn load_trade_snapshot(
    conn: &mut mysql::Conn,
    bot: &config::BotConfig,
) -> Result<TradeCharacterSnapshot> {
    type Row = (u32, u8, u8, u32, u64, u32, u32, u32, f64, f64, f64, f32);
    let row: Option<Row> = conn
        .exec_first(
            "SELECT account, online, race, health, money, map, zone, instance_id, \
             position_x, position_y, position_z, orientation FROM characters WHERE guid = ?",
            (bot.character_guid,),
        )
        .map_err(|error| anyhow!("Load trade character {}: {error}", bot.character_guid))?;
    let (owner, online, race, health, money, map_id, zone_id, instance_id, x, y, z, orientation) =
        row.ok_or_else(|| anyhow!("No characters row for guid {}", bot.character_guid))?;
    if owner != bot.account_id {
        bail!(
            "trade character {} belongs to account {owner}, expected {}",
            bot.character_guid,
            bot.account_id
        );
    }
    Ok(TradeCharacterSnapshot {
        account: bot.account.clone(),
        account_id: bot.account_id,
        character_guid: bot.character_guid,
        race,
        health,
        online,
        money,
        position: CharacterPositionSnapshot {
            map_id,
            zone_id,
            instance_id,
            x,
            y,
            z,
            orientation,
        },
    })
}

fn load_trade_inventory_facts(
    conn: &mut mysql::Conn,
    character_guid: u64,
    item_entry: u32,
) -> Result<TradeInventoryFacts> {
    let occupied_backpack_slots = conn
        .exec_map(
            "SELECT slot FROM character_inventory WHERE guid = ? AND bag = 0",
            (character_guid,),
            |slot: u8| slot,
        )
        .map_err(|error| anyhow!("Load trade backpack slots: {error}"))?;
    let owned_same_entry: u64 = conn
        .exec_first(
            "SELECT COUNT(*) FROM item_instance WHERE owner_guid = ? AND itemEntry = ?",
            (character_guid, item_entry),
        )
        .map_err(|error| anyhow!("Check existing trade fixture items: {error}"))?
        .unwrap_or(0);
    Ok(TradeInventoryFacts {
        occupied_backpack_slots,
        owned_same_entry,
    })
}

fn exec_statements(
    transaction: &mut mysql::Transaction<'_>,
    statements: Vec<SqlStatement>,
    label: &str,
) -> Result<()> {
    for (sql, params) in statements {
        transaction
            .exec_drop(sql, params)
            .map_err(|error| anyhow!("{label}: {error} ({sql})"))?;
    }
    Ok(())
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TradeDbState {
    pub(crate) item_row: Option<(u64, u32, u32)>,
    pub(crate) inventory_owners: Vec<u64>,
    /// (bag, slot) of each `character_inventory` row of the traded item.
    pub(crate) inventory_positions: Vec<(u64, u8)>,
    pub(crate) initiator_money: u64,
    pub(crate) receiver_money: u64,
}

pub(crate) fn load_trade_db_state(expected: &TradeDbExpectation) -> Result<TradeDbState> {
    let mut conn = trade_characters_conn()?;
    let item_row: Option<(u64, u32, u32)> = conn
        .exec_first(
            "SELECT owner_guid, itemEntry, count FROM item_instance WHERE guid = ?",
            (expected.item_guid,),
        )
        .map_err(|error| anyhow!("Load traded item_instance row: {error}"))?;
    let inventory_owners = conn
        .exec_map(
            "SELECT guid FROM character_inventory WHERE item = ? ORDER BY guid",
            (expected.item_guid,),
            |guid: u64| guid,
        )
        .map_err(|error| anyhow!("Load traded character_inventory rows: {error}"))?;
    let inventory_positions = conn
        .exec_map(
            "SELECT bag, slot FROM character_inventory WHERE item = ? ORDER BY guid",
            (expected.item_guid,),
            |(bag, slot): (u64, u8)| (bag, slot),
        )
        .map_err(|error| anyhow!("Load traded character_inventory positions: {error}"))?;
    let mut money = |guid: u64| -> Result<u64> {
        conn.exec_first("SELECT money FROM characters WHERE guid = ?", (guid,))
            .map_err(|error| anyhow!("Load trade money for {guid}: {error}"))?
            .ok_or_else(|| anyhow!("No characters row for guid {guid}"))
    };
    let initiator_money = money(expected.initiator_guid)?;
    let receiver_money = money(expected.receiver_guid)?;
    Ok(TradeDbState {
        item_row,
        inventory_owners,
        inventory_positions,
        initiator_money,
        receiver_money,
    })
}

/// The persisted outcome of `HandleAcceptTradeOpcode`'s
/// `SaveInventoryAndGoldToDB` for both players.
pub(crate) fn check_trade_db_state(
    state: &TradeDbState,
    expected: &TradeDbExpectation,
) -> Result<()> {
    match state.item_row {
        Some((owner, entry, count))
            if owner == expected.receiver_guid && entry == expected.item_entry && count == 1 => {}
        other => bail!(
            "traded item {} item_instance (owner, entry, count) is {other:?}, expected ({}, {}, 1)",
            expected.item_guid,
            expected.receiver_guid,
            expected.item_entry
        ),
    }
    if state.inventory_owners != [expected.receiver_guid] {
        bail!(
            "traded item {} character_inventory owners are {:?}, expected only the receiver {}",
            expected.item_guid,
            state.inventory_owners,
            expected.receiver_guid
        );
    }
    if let Some(bag_guid) = expected.receiver_bag_guid {
        if state.inventory_positions != [(bag_guid, 0)] {
            bail!(
                "traded item {} is stored at (bag, slot) {:?}, expected inside bag {bag_guid} slot 0",
                expected.item_guid,
                state.inventory_positions
            );
        }
    }
    if state.initiator_money != expected.initiator_money_after
        || state.receiver_money != expected.receiver_money_after
    {
        bail!(
            "trade money is initiator={} receiver={}, expected {} and {}",
            state.initiator_money,
            state.receiver_money,
            expected.initiator_money_after,
            expected.receiver_money_after
        );
    }
    Ok(())
}

fn wait_trade_characters_offline(
    conn: &mut mysql::Conn,
    guids: [u64; 2],
    wait: Duration,
) -> Result<()> {
    let deadline = std::time::Instant::now() + wait;
    loop {
        let online: Vec<u64> = conn
            .exec_map(
                "SELECT guid FROM characters WHERE guid IN (?, ?) AND online <> 0",
                (guids[0], guids[1]),
                |guid: u64| guid,
            )
            .map_err(|error| anyhow!("Check trade characters offline: {error}"))?;
        if online.is_empty() {
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            bail!(
                "trade characters {online:?} remained online for {}s; refusing restore before their disconnect save",
                wait.as_secs()
            );
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// Restore both characters and remove the fixture item, then prove it.
pub(crate) fn cleanup_trade_fixture(plan: &TradeFixturePlan, offline_wait: Duration) -> Result<()> {
    let mut conn = trade_characters_conn()?;
    let guids = [plan.initiator.character_guid, plan.receiver.character_guid];
    wait_trade_characters_offline(&mut conn, guids, offline_wait)?;
    let mut transaction = conn
        .start_transaction(mysql::TxOpts::default())
        .map_err(|error| anyhow!("Start trade cleanup transaction: {error}"))?;
    exec_statements(
        &mut transaction,
        trade_fixture_cleanup_statements(plan),
        "Trade fixture cleanup",
    )?;
    transaction
        .commit()
        .map_err(|error| anyhow!("Commit trade cleanup: {error}"))?;

    let item_rows: u64 = conn
        .exec_first(
            "SELECT (SELECT COUNT(*) FROM item_instance WHERE guid = ?) + \
                    (SELECT COUNT(*) FROM character_inventory WHERE item = ?)",
            (plan.item_guid, plan.item_guid),
        )
        .map_err(|error| anyhow!("Verify trade item removal: {error}"))?
        .unwrap_or(0);
    if item_rows != 0 {
        bail!(
            "trade cleanup left {item_rows} row(s) for item {} (owned outside the fixture characters?)",
            plan.item_guid
        );
    }
    for original in [&plan.initiator, &plan.receiver] {
        let bot = config::BotConfig {
            account: original.account.clone(),
            password: String::new(),
            character_guid: original.character_guid,
            account_id: original.account_id,
            lfg_role: 0,
            class: String::new(),
            enabled: false,
            session_key_bnet: String::new(),
        };
        let now = load_trade_snapshot(&mut conn, &bot)?;
        let position_ok = now.position.map_id == original.position.map_id
            && now.position.zone_id == original.position.zone_id
            && now.position.instance_id == original.position.instance_id
            && (now.position.x - original.position.x).abs() <= TRADE_POSITION_TOLERANCE
            && (now.position.y - original.position.y).abs() <= TRADE_POSITION_TOLERANCE
            && (now.position.z - original.position.z).abs() <= TRADE_POSITION_TOLERANCE;
        if now.money != original.money || !position_ok {
            bail!(
                "trade cleanup did not restore character {}: money {} position {:?}, expected {} {:?}",
                original.character_guid,
                now.money,
                now.position,
                original.money,
                original.position
            );
        }
    }
    info!(
        "Trade fixture restored: item {} removed; characters {} and {} money/position restored",
        plan.item_guid, guids[0], guids[1]
    );
    Ok(())
}

// ─── Durable recovery record ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub(crate) struct TradeFixtureRecord {
    pub(crate) version: u32,
    pub(crate) kind: String,
    pub(crate) created_by_pid: u32,
    pub(crate) plan: TradeFixturePlan,
}

pub(crate) fn validate_trade_journal_contract() -> Result<PathBuf> {
    let path = loot_race::configured_fixture_journal_path()?;
    if fs::symlink_metadata(&path).is_ok() {
        bail!(
            "pending fixture journal {} exists; run --recover-trade-fixture before another QA run",
            path.display()
        );
    }
    let marker = loot_race::cleanup_marker_path(&path);
    if fs::symlink_metadata(&marker).is_ok() {
        bail!(
            "stale cleanup marker {} exists; each QA run must use a fresh journal path",
            marker.display()
        );
    }
    Ok(path)
}

fn persist_trade_record(path: &std::path::Path, plan: &TradeFixturePlan) -> Result<()> {
    let record = TradeFixtureRecord {
        version: TRADE_FIXTURE_RECORD_VERSION,
        kind: TRADE_FIXTURE_RECORD_KIND.to_string(),
        created_by_pid: std::process::id(),
        plan: plan.clone(),
    };
    let payload = serde_json::to_vec_pretty(&record).context("Serialize trade fixture record")?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .with_context(|| format!("Create trade fixture record {}", path.display()))?;
    file.write_all(&payload)
        .and_then(|()| file.write_all(b"\n"))
        .context("Write trade fixture record")?;
    file.sync_all().context("fsync trade fixture record")?;
    loot_race::sync_parent_directory(path)
}

fn load_trade_record(path: &std::path::Path) -> Result<TradeFixturePlan> {
    use std::os::unix::fs::MetadataExt;
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("Inspect pending fixture journal {}", path.display()))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.mode() & 0o777 != 0o600
    {
        bail!("pending fixture journal must be a regular non-symlink 0600 file");
    }
    let record: TradeFixtureRecord = serde_json::from_slice(
        &fs::read(path).with_context(|| format!("Read fixture journal {}", path.display()))?,
    )
    .context("Parse trade fixture record (is this a loot journal? use --recover-loot-fixture)")?;
    parse_trade_record(record)
}

pub(crate) fn parse_trade_record(record: TradeFixtureRecord) -> Result<TradeFixturePlan> {
    if record.version != TRADE_FIXTURE_RECORD_VERSION || record.kind != TRADE_FIXTURE_RECORD_KIND {
        bail!(
            "fixture journal is {} v{}, expected {TRADE_FIXTURE_RECORD_KIND} v{TRADE_FIXTURE_RECORD_VERSION}",
            record.kind,
            record.version
        );
    }
    Ok(record.plan)
}

/// Restore from a pending record (if any), then publish the cleanup marker.
pub(crate) async fn recover_pending_trade_fixture_if_present() -> Result<bool> {
    let path = loot_race::configured_fixture_journal_path()?;
    if fs::symlink_metadata(&path).is_err() {
        return Ok(false);
    }
    tokio::task::spawn_blocking(move || -> Result<()> {
        let plan = load_trade_record(&path)?;
        cleanup_trade_fixture(&plan, Duration::from_secs(90))?;
        loot_race::FixtureJournal { path }.complete()
    })
    .await
    .map_err(|error| anyhow!("trade fixture recovery worker join failed: {error}"))??;
    Ok(true)
}

pub(crate) async fn finish_guarded_trade_result(
    result: Result<Vec<BotRunResult>>,
) -> Result<Vec<BotRunResult>> {
    match result {
        Ok(results) => Ok(results),
        Err(primary) => match recover_pending_trade_fixture_if_present().await {
            Ok(true) => Err(primary
                .context("trade workflow failed; the pending trade fixture was restored before exit")),
            Ok(false) => Err(primary),
            Err(recovery) => bail!(
                "trade workflow failed ({primary:#}) and fixture recovery also failed ({recovery:#}); run --recover-trade-fixture with the same WOW_BOT_FIXTURE_JOURNAL"
            ),
        },
    }
}

// ─── Fixture setup ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub(crate) struct TradeFixture {
    pub(crate) plan: TradeFixturePlan,
    pub(crate) journal: PathBuf,
}

pub(crate) fn prepare_trade_fixture(
    bots: &[config::BotConfig; 2],
    cli: &TradeSmokeCli,
    journal: PathBuf,
) -> Result<TradeFixture> {
    let mut conn = trade_characters_conn()?;
    for bot in bots {
        validate_local_bot_character_owner(&mut conn, bot)?;
    }
    let initiator = load_trade_snapshot(&mut conn, &bots[0])?;
    let receiver = load_trade_snapshot(&mut conn, &bots[1])?;
    let initiator_inventory =
        load_trade_inventory_facts(&mut conn, initiator.character_guid, cli.item_entry)?;
    let receiver_inventory =
        load_trade_inventory_facts(&mut conn, receiver.character_guid, cli.item_entry)?;
    let max_item_guid: u64 = conn
        .query_first("SELECT COALESCE(MAX(guid), 0) FROM item_instance")
        .map_err(|error| anyhow!("Load max item guid: {error}"))?
        .unwrap_or(0);
    let plan = plan_trade_fixture(
        cli,
        initiator,
        receiver,
        &initiator_inventory,
        &receiver_inventory,
        max_item_guid,
    )?;

    // Record first: from here on every exit path can restore exactly.
    persist_trade_record(&journal, &plan)?;
    let applied = (|| -> Result<()> {
        let mut transaction = conn
            .start_transaction(mysql::TxOpts::default())
            .map_err(|error| anyhow!("Start trade fixture transaction: {error}"))?;
        let online: Vec<u64> = transaction
            .exec_map(
                "SELECT guid FROM characters WHERE guid IN (?, ?) AND online <> 0 FOR UPDATE",
                (plan.initiator.character_guid, plan.receiver.character_guid),
                |guid: u64| guid,
            )
            .map_err(|error| anyhow!("Lock trade characters: {error}"))?;
        if !online.is_empty() {
            bail!("trade characters {online:?} came online during fixture setup");
        }
        exec_statements(
            &mut transaction,
            trade_fixture_setup_statements(&plan),
            "Trade fixture setup",
        )?;
        transaction
            .commit()
            .map_err(|error| anyhow!("Commit trade fixture: {error}"))
    })();
    if let Err(error) = applied {
        let restore = cleanup_trade_fixture(&plan, Duration::from_secs(10))
            .and_then(|()| loot_race::FixtureJournal { path: journal }.complete());
        return match restore {
            Ok(()) => Err(error.context("trade fixture setup failed and was rolled back")),
            Err(restore_error) => bail!(
                "trade fixture setup failed ({error:#}) and restore failed ({restore_error:#})"
            ),
        };
    }
    info!(
        "Trade fixture: item {}/entry {} in initiator {} backpack slot {}; money {}->{} / {}->{}; receiver {} at ({:.2},{:.2},{:.2}) map {}",
        plan.item_guid,
        plan.item_entry,
        plan.initiator.character_guid,
        plan.item_slot,
        plan.initiator.money,
        plan.initiator_money,
        plan.receiver.money,
        plan.receiver_money,
        plan.receiver.character_guid,
        plan.receiver_position.x,
        plan.receiver_position.y,
        plan.receiver_position.z,
        plan.receiver_position.map_id
    );
    if let Some(fixture) = &plan.bag_destination {
        info!(
            "Trade bag-destination fixture: receiver {} bag {} (entry {}) in slot {}; {} filler(s) fill the free backpack slots; the traded item must persist inside bag {} slot 0",
            plan.receiver.character_guid,
            fixture.bag_guid,
            TRADE_BAG_ENTRY,
            fixture.bag_slot,
            fixture.fillers.len(),
            fixture.bag_guid
        );
    }
    Ok(TradeFixture { plan, journal })
}

fn cleanup_and_complete(fixture: &TradeFixture) -> Result<()> {
    cleanup_trade_fixture(&fixture.plan, Duration::from_secs(30))?;
    loot_race::FixtureJournal {
        path: fixture.journal.clone(),
    }
    .complete()
}

// ─── Workflow ───────────────────────────────────────────────────────────────

fn trade_options(
    fixture: &TradeFixture,
    cli: &TradeSmokeCli,
    role: TradeRole,
    phase: TradeSmokePhase,
    sync: &Arc<TradeSmokeSync>,
) -> TradeSmokeOptions {
    let plan = &fixture.plan;
    let (me, partner) = match role {
        TradeRole::Initiator => (&plan.initiator, &plan.receiver),
        TradeRole::Receiver => (&plan.receiver, &plan.initiator),
    };
    TradeSmokeOptions {
        phase,
        role,
        character_guid: me.character_guid,
        partner_guid: partner.character_guid,
        item_guid: plan.item_guid,
        item_entry: plan.item_entry,
        item_slot: plan.item_slot,
        gold: plan.gold,
        expected: plan.expectation(),
        timeout_secs: cli.timeout_secs,
        sync: Arc::clone(sync),
    }
}

async fn verify_trade_db(fixture: &TradeFixture) -> Result<TradeDbState> {
    let expected = fixture.plan.expectation();
    let state = tokio::task::spawn_blocking(move || load_trade_db_state(&expected))
        .await
        .map_err(|error| anyhow!("trade DB worker join failed: {error}"))??;
    check_trade_db_state(&state, &fixture.plan.expectation())?;
    Ok(state)
}

fn fail_all(results: &mut [BotRunResult], failure: &str) {
    for result in results {
        result.trade.trade_smoke_passed = Some(false);
        if result.trade.trade_failure.is_none() {
            result.trade.trade_failure = Some(failure.to_string());
        }
    }
}

async fn run_trade_sessions(
    bots: &[config::BotConfig; 2],
    fixture: &TradeFixture,
    cli: &TradeSmokeCli,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    shutdown: &CancellationToken,
    deadline: tokio::time::Instant,
) -> (Vec<BotRunResult>, Option<String>) {
    let sync = Arc::new(TradeSmokeSync::new());
    let mut handles = tokio::task::JoinSet::new();
    for (index, role) in [TradeRole::Initiator, TradeRole::Receiver]
        .into_iter()
        .enumerate()
    {
        let bot = bots[index].clone();
        let options = trade_options(fixture, cli, role, TradeSmokePhase::Trade, &sync);
        let task_sync = Arc::clone(&sync);
        handles.spawn(async move {
            let run = run_bot_with_post_login_smoke(
                bot,
                dungeon_id,
                lfg_secs,
                auto_teleport,
                PostLoginSmokeOptions::Trade(options),
            )
            .await;
            match &run {
                Err(error) => task_sync.cancel(format!("{role:?} login failed: {error:#}")),
                Ok(result) if result.trade.trade_smoke_passed != Some(true) => {
                    task_sync.cancel(format!(
                        "{role:?} failed: {}",
                        result
                            .trade
                            .trade_failure
                            .as_deref()
                            .unwrap_or("unknown trade failure")
                    ))
                }
                Ok(_) => {}
            }
            run
        });
    }
    let mut results = Vec::with_capacity(2);
    let mut task_error = None;
    loop {
        let joined = tokio::select! {
            joined = handles.join_next() => joined,
            _ = shutdown.cancelled() => {
                sync.cancel("trade smoke received SIGINT/SIGTERM");
                handles.abort_all();
                while handles.join_next().await.is_some() {}
                task_error = Some("trade smoke received SIGINT/SIGTERM".to_string());
                break;
            }
            _ = tokio::time::sleep_until(deadline) => {
                sync.cancel("trade smoke exceeded its end-to-end deadline");
                handles.abort_all();
                while handles.join_next().await.is_some() {}
                task_error = Some(format!(
                    "trade smoke exceeded the {}s end-to-end deadline",
                    cli.workflow_deadline_secs
                ));
                break;
            }
        };
        let Some(joined) = joined else { break };
        match joined {
            Ok(Ok(result)) => results.push(result),
            Ok(Err(error)) => {
                task_error.get_or_insert(format!("{error:#}"));
            }
            Err(error) => {
                task_error.get_or_insert(format!("trade task join failed: {error}"));
            }
        }
    }
    results.sort_by_key(|result| result.account_id);
    (results, task_error)
}

async fn relog_and_verify(
    bots: &[config::BotConfig; 2],
    fixture: &TradeFixture,
    cli: &TradeSmokeCli,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    shutdown: &CancellationToken,
    deadline: tokio::time::Instant,
    results: &mut [BotRunResult],
) -> Result<()> {
    let sync = Arc::new(TradeSmokeSync::new());
    for (index, role) in [TradeRole::Initiator, TradeRole::Receiver]
        .into_iter()
        .enumerate()
    {
        let options = trade_options(fixture, cli, role, TradeSmokePhase::VerifyRelog, &sync);
        let relog = tokio::select! {
            _ = shutdown.cancelled() => bail!("trade relog cancelled by SIGINT/SIGTERM"),
            relog = tokio::time::timeout_at(deadline, run_bot_with_post_login_smoke(
                bots[index].clone(),
                dungeon_id,
                lfg_secs,
                auto_teleport,
                PostLoginSmokeOptions::Trade(options),
            )) => relog.map_err(|_| anyhow!("trade relog exceeded the end-to-end deadline"))??,
        };
        if !relog.trade.trade_relog_verified {
            bail!(
                "{role:?} relog did not complete a clean login/logout: {}",
                relog.trade.trade_failure.as_deref().unwrap_or("unknown")
            );
        }
        let result = results
            .iter_mut()
            .find(|result| result.account_id == bots[index].account_id)
            .context("relogged bot has no first-phase result")?;
        result.world_auth &= relog.world_auth;
        result.enum_characters &= relog.enum_characters;
        result.player_login_verified &= relog.player_login_verified;
        result.seen_opcodes.extend(relog.seen_opcodes);
        result.trade.trade_relog_verified = true;
    }
    verify_trade_db(fixture)
        .await
        .context("traded item/money did not survive a clean logout/relogin")?;
    for result in results.iter_mut() {
        result.trade.trade_db_verified_after_relog = true;
    }
    Ok(())
}

pub(crate) async fn run_trade_workflow(
    mut bots: Vec<config::BotConfig>,
    cli: TradeSmokeCli,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    shutdown: CancellationToken,
) -> Result<Vec<BotRunResult>> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(cli.workflow_deadline_secs);
    bots.sort_by_key(|bot| !bot.account.eq_ignore_ascii_case(&cli.account_a));
    let bots: [config::BotConfig; 2] = bots
        .try_into()
        .map_err(|_| anyhow!("trade smoke requires exactly two bots"))?;
    if !bots[0].account.eq_ignore_ascii_case(&cli.account_a)
        || !bots[1].account.eq_ignore_ascii_case(&cli.account_b)
    {
        bail!("trade bots do not match --trade-account-a/--trade-account-b");
    }
    if shutdown.is_cancelled() {
        bail!("trade smoke cancelled before fixture setup");
    }
    let journal = validate_trade_journal_contract()?;
    let fixture = {
        let bots = bots.clone();
        let cli = cli.clone();
        tokio::task::spawn_blocking(move || prepare_trade_fixture(&bots, &cli, journal))
            .await
            .map_err(|error| anyhow!("trade fixture DB worker join failed: {error}"))??
    };

    let (mut results, task_error) = run_trade_sessions(
        &bots,
        &fixture,
        &cli,
        dungeon_id,
        lfg_secs,
        auto_teleport,
        &shutdown,
        deadline,
    )
    .await;
    if let Some(error) = &task_error {
        fail_all(&mut results, error);
    }

    let wire_passed = task_error.is_none()
        && results.len() == 2
        && results
            .iter()
            .all(|result| result.trade.trade_smoke_passed == Some(true));
    if wire_passed {
        match verify_trade_db(&fixture).await {
            Ok(state) => {
                for result in &mut results {
                    result.trade.trade_db_verified_after_logout = true;
                    result.trade.trade_money_after = Some(
                        if result.character_guid == fixture.plan.initiator.character_guid {
                            state.initiator_money
                        } else {
                            state.receiver_money
                        },
                    );
                }
                if let Err(error) = relog_and_verify(
                    &bots,
                    &fixture,
                    &cli,
                    dungeon_id,
                    lfg_secs,
                    auto_teleport,
                    &shutdown,
                    deadline,
                    &mut results,
                )
                .await
                {
                    fail_all(&mut results, &format!("{error:#}"));
                }
            }
            Err(error) => fail_all(
                &mut results,
                &format!("trade not persisted after logout: {error:#}"),
            ),
        }
    }

    let cleanup = {
        let fixture = fixture.clone();
        tokio::task::spawn_blocking(move || cleanup_and_complete(&fixture))
            .await
            .map_err(|error| anyhow!("trade cleanup DB worker join failed: {error}"))?
    };
    if let Err(error) = cleanup {
        bail!("trade fixture restore failed: {error:#}");
    }
    if results.len() != 2 {
        bail!(
            "trade smoke produced {} results instead of 2{}",
            results.len(),
            task_error
                .as_deref()
                .map(|error| format!(": {error}"))
                .unwrap_or_default()
        );
    }
    Ok(results)
}
