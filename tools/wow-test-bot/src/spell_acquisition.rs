//! Action-specific #587 QA. No provisioning, fixture SQL writes or service control.
//! Wire authorities: NPCPackets.cpp TrainerBuySpell::Read; SpellPackets.cpp
//! SpellCastRequest/SpellTargetData readers and LearnedSpellInfo/LearnedSpells writers.
use super::*;
use mysql::prelude::Queryable;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(super) struct Plan {
    expected_spell: u32,
    #[serde(flatten)]
    action: Action,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Action {
    Trainer {
        guid_low: u64,
        guid_high: u64,
        trainer_id: i32,
        offer_spell: i32,
        fee: u64,
    },
    Cast {
        spell: i32,
        cast_low: u64,
        cast_high: u64,
    },
    Verify {},
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct Evidence {
    expected_spell: u32,
    learned_on_instance: bool,
    verified_at_login: bool,
    money_before: u64,
    observed_db_money_after_action: u64,
    expected_saved_money: u64,
    repeated_purchase_rejected: bool,
    /// Read from the DB after confirmed logout, not inferred from packets.
    saved_spell: bool,
    saved_money: u64,
}

pub(super) fn load() -> Result<Option<Plan>> {
    let Some(path) = std::env::var_os("WOW_BOT_ACQUISITION_PLAN") else {
        return Ok(None);
    };
    let plan: Plan = serde_json::from_slice(&std::fs::read(path)?)?;
    if plan.expected_spell == 0 || plan.expected_spell > i32::MAX as u32 {
        bail!("acquisition expected spell must be a positive signed spell ID");
    }
    match plan.action {
        Action::Trainer {
            guid_low,
            guid_high,
            trainer_id,
            offer_spell,
            ..
        } if guid_low == 0 || guid_high == 0 || trainer_id <= 0 || offer_spell <= 0 => {
            bail!("trainer plan requires a live NPC GUID, trainer and offer")
        }
        Action::Cast { spell, .. } if spell <= 0 => bail!("cast spell must be positive"),
        _ => {}
    }
    if !std::env::var("WOW_BOT_LOGIN_SAVE_CHECK").is_ok_and(|v| is_truthy(&v))
        || std::env::var("WOW_BOT_LOGIN_DISCONNECT_CHECK").is_ok_and(|v| is_truthy(&v))
    {
        bail!("acquisition requires normal login-save mode");
    }
    Ok(Some(plan))
}

fn state(bot: &config::BotConfig, spell: u32) -> Result<(u64, bool)> {
    let url = characters_db_url()?;
    let opts = mysql::Opts::from_url(&url).map_err(|_| anyhow!("invalid QA DB options"))?;
    let mut conn = mysql::Conn::new(opts).map_err(|_| anyhow!("QA DB connection failed"))?;
    let money: Option<u64> = conn
        .exec_first(
            "SELECT money FROM characters WHERE guid=? AND account=?",
            (bot.character_guid, bot.account_id),
        )
        .map_err(|_| anyhow!("acquisition character read failed"))?;
    let learned: Option<u8> = conn
        .exec_first(
            "SELECT active FROM character_spell WHERE guid=? AND spell=? AND disabled=0",
            (bot.character_guid, spell),
        )
        .map_err(|_| anyhow!("acquisition spell read failed"))?;
    Ok((
        money.context("acquisition character is missing")?,
        learned == Some(1),
    ))
}

fn trainer_buy(guid: &[u8], trainer: i32, spell: i32) -> Vec<u8> {
    let mut bytes = guid.to_vec();
    bytes.extend(trainer.to_le_bytes());
    bytes.extend(spell.to_le_bytes());
    bytes
}

fn cast_self(spell: i32, cast: (u64, u64), player: (u64, u64)) -> Vec<u8> {
    let mut bytes = build_packed_guid(cast.0, cast.1);
    bytes.extend([0; 8]); // Misc[2]
    bytes.extend(spell.to_le_bytes());
    bytes.extend([0; 16]); // SpellCastVisual[2], trajectory pitch/speed
    bytes.extend(build_packed_guid(0, 0)); // CraftingNPC
    bytes.extend([0; 12]); // Currency/reagent/removal counts
    bytes.extend([0; 2]); // 5 flags, move bit, 2 weight bits, order bit
                          // Target.Flags=UNIT (28 MSB-first bits), four absent optional fields,
                          // then seven zero name-length bits flushed before PackedGuid.
    bytes.extend([0, 0, 0, 0x20, 0]);
    bytes.extend(build_packed_guid(player.0, player.1));
    bytes.extend(build_packed_guid(0, 0)); // Item
    bytes
}

fn learned(payload: &[u8]) -> Result<Vec<(u32, bool)>> {
    if payload.len() < 9 {
        bail!("truncated LearnedSpells header");
    }
    let count = u32::from_le_bytes(payload[..4].try_into().unwrap()) as usize;
    if count > (payload.len() - 9) / 5 {
        bail!("invalid LearnedSpells count");
    }
    let mut offset = 9;
    let mut spells = Vec::with_capacity(count);
    for _ in 0..count {
        let row = payload
            .get(offset..offset + 5)
            .context("truncated learned row")?;
        let spell = i32::from_le_bytes(row[..4].try_into().unwrap());
        if spell <= 0 || row[4] & 0x0f != 0 {
            bail!("invalid learned row");
        }
        offset += 5;
        for bit in [0x40, 0x20, 0x10] {
            if row[4] & bit != 0 {
                payload
                    .get(offset..offset + 4)
                    .context("truncated learned optional field")?;
                offset += 4;
            }
        }
        spells.push((spell as u32, row[4] & 0x80 != 0));
    }
    if offset != payload.len() {
        bail!("trailing LearnedSpells bytes");
    }
    Ok(spells)
}

async fn next(
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut Option<EncryptedWorldConnection>,
    deadline: tokio::time::Instant,
) -> Result<Option<(bool, u16, Vec<u8>)>> {
    let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
    if remaining.is_zero() {
        return Ok(None);
    }
    if let Some(realm) = realm {
        if let Some((op, data)) = read_encrypted_packet_if_ready(
            &mut realm.stream,
            &mut realm.crypt,
            &mut realm.inflater,
            remaining.min(Duration::from_millis(10)),
            remaining,
            "acquisition realm",
        )
        .await?
        {
            return Ok(Some((false, op, data)));
        }
    }
    let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
    if remaining.is_zero() {
        return Ok(None);
    }
    let packet = read_encrypted_packet_if_ready(
        stream,
        crypt,
        inflater,
        remaining.min(Duration::from_millis(10)),
        remaining,
        "acquisition instance",
    )
    .await?;
    if let Some((SMSG_TIME_SYNC_REQUEST, payload)) = packet.as_ref() {
        let sequence = parse_time_sync_request_sequence(payload)?;
        send_encrypted_packet(
            stream,
            crypt,
            CMSG_TIME_SYNC_RESPONSE,
            &build_time_sync_response_payload(sequence, 0),
        )
        .await?;
        return Ok(None);
    }
    Ok(packet.map(|(op, data)| (true, op, data)))
}

pub(super) async fn execute(
    plan: &Plan,
    bot: &config::BotConfig,
    known: &mut LoginKnownSpellsLikeCpp,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut Option<EncryptedWorldConnection>,
) -> Result<Evidence> {
    if !bot.account.eq_ignore_ascii_case("TESTBOT1@bot.local") {
        bail!("acquisition QA requires the existing isolated TESTBOT1 identity");
    }
    let selected = bot.clone();
    let spell = plan.expected_spell;
    let (money_before, saved_before) =
        tokio::task::spawn_blocking(move || state(&selected, spell)).await??;
    let verified_at_login = matches!(plan.action, Action::Verify {});
    let mut receipt = Evidence {
        expected_spell: spell,
        learned_on_instance: false,
        verified_at_login,
        money_before,
        observed_db_money_after_action: money_before,
        expected_saved_money: money_before,
        repeated_purchase_rejected: false,
        saved_spell: false,
        saved_money: 0,
    };
    if verified_at_login {
        if !known.known_spells.contains(&spell) || !saved_before {
            bail!("relogin lost acquired spell");
        }
        return Ok(receipt);
    }
    if known.known_spells.contains(&spell) || saved_before {
        bail!("acquisition fixture already knows expected spell");
    }
    match plan.action {
        Action::Trainer {
            guid_low,
            guid_high,
            trainer_id,
            offer_spell,
            ..
        } => {
            let guid = build_packed_guid(guid_low, guid_high);
            send_encrypted_packet(stream, crypt, 0x34AD, &guid).await?;
            let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
            let mut admitted = false;
            while tokio::time::Instant::now() < deadline {
                if let Some((_, op, payload)) =
                    next(stream, crypt, inflater, realm, deadline).await?
                {
                    if op == 0x26DF {
                        let summary = packet_parser::parse_trainer_list_summary(&payload)
                            .context("invalid trainer list")?;
                        let (size, low, high) = packet_parser::parse_packed_guid(&payload)
                            .context("invalid trainer GUID")?;
                        let _ = size;
                        if (low, high) != (guid_low, guid_high) || summary.trainer_id != trainer_id
                        {
                            bail!("trainer admission response does not match selected fixture");
                        }
                        admitted = true;
                        break;
                    }
                }
            }
            if !admitted {
                bail!("trainer list timed out");
            }
            send_encrypted_packet(
                stream,
                crypt,
                0x34AE,
                &trainer_buy(&guid, trainer_id, offer_spell),
            )
            .await?;
        }
        Action::Cast {
            spell,
            cast_low,
            cast_high,
        } => {
            if !known.known_spells.contains(&(spell as u32)) {
                bail!("fixture does not know the requested cast");
            }
            let payload = cast_self(
                spell,
                (cast_low, cast_high),
                create_player_guid_raw(bot.character_guid, realm_id()),
            );
            send_encrypted_packet(stream, crypt, 0x329C, &payload).await?;
        }
        Action::Verify {} => unreachable!(),
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    while tokio::time::Instant::now() < deadline {
        if let Some((instance, op, payload)) =
            next(stream, crypt, inflater, realm, deadline).await?
        {
            if op == 0x2C4A {
                if !instance {
                    bail!("LearnedSpells arrived on realm instead of instance");
                }
                for (id, favorite) in learned(&payload)? {
                    if !known.known_spells.contains(&id) {
                        known.known_spells.push(id);
                    }
                    if favorite && !known.favorite_spells.contains(&id) {
                        known.favorite_spells.push(id);
                    }
                    receipt.learned_on_instance |= id == plan.expected_spell;
                }
                if receipt.learned_on_instance {
                    break;
                }
            }
        }
    }
    if !receipt.learned_on_instance {
        bail!("expected learning publication timed out");
    }
    if let Action::Trainer {
        guid_low,
        guid_high,
        trainer_id,
        offer_spell,
        ..
    } = plan.action
    {
        let guid = build_packed_guid(guid_low, guid_high);
        send_encrypted_packet(
            stream,
            crypt,
            0x34AE,
            &trainer_buy(&guid, trainer_id, offer_spell),
        )
        .await?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        while tokio::time::Instant::now() < deadline {
            if let Some((_, op, payload)) = next(stream, crypt, inflater, realm, deadline).await? {
                if op == 0x2C4A && learned(&payload)?.iter().any(|(id, _)| *id == spell) {
                    bail!("repeated purchase published duplicate learning");
                }
                if op == 0x26E0 {
                    let (offset, low, high) = packet_parser::parse_packed_guid(&payload)
                        .context("invalid trainer failure GUID")?;
                    let tail = payload.get(offset..).context("invalid trainer failure")?;
                    if (low, high) != (guid_low, guid_high)
                        || tail.len() != 8
                        || i32::from_le_bytes(tail[..4].try_into().unwrap()) != offer_spell
                        || u32::from_le_bytes(tail[4..].try_into().unwrap()) != 0
                    {
                        bail!("unexpected repeated trainer purchase failure");
                    }
                    receipt.repeated_purchase_rejected = true;
                    break;
                }
            }
        }
        if !receipt.repeated_purchase_rejected {
            bail!("repeated trainer purchase rejection timed out");
        }
    }
    known.known_spells.sort_unstable();
    known.favorite_spells.sort_unstable();
    let selected = bot.clone();
    receipt.observed_db_money_after_action =
        tokio::task::spawn_blocking(move || state(&selected, spell))
            .await??
            .0;
    receipt.expected_saved_money = match plan.action {
        Action::Trainer { fee, .. } => money_before
            .checked_sub(fee)
            .context("fixture cannot afford trainer fee")?,
        _ => money_before,
    };
    // C++ persists ordinary Player money during SaveToDB. Observe this value,
    // but compare the fee only after confirmed logout, for both implementations.
    Ok(receipt)
}

pub(super) async fn verify_saved(bot: &config::BotConfig, receipt: &mut Evidence) -> Result<()> {
    let selected = bot.clone();
    let spell = receipt.expected_spell;
    let (money, saved) = tokio::task::spawn_blocking(move || state(&selected, spell)).await??;
    if !saved || money != receipt.expected_saved_money {
        bail!("confirmed logout did not retain acquisition and money");
    }
    receipt.saved_money = money;
    receipt.saved_spell = saved;
    Ok(())
}

#[cfg(test)]
mod tests;
