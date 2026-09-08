//! Action-specific #587 QA. No provisioning, fixture SQL writes or service control.
//! Wire authorities: NPCPackets.cpp TrainerBuySpell::Read; SpellPackets.cpp
//! SpellCastRequest/SpellTargetData readers and LearnedSpellInfo/LearnedSpells writers.
use super::*;
use serde::Deserialize;

mod persistence;
use persistence::{read_state, Expectation};
pub(super) use persistence::{verify_saved, Evidence};

#[derive(Debug, Deserialize)]
pub(super) struct Plan {
    expected_spell: u32,
    #[serde(default)]
    persistence: Expectation,
    #[serde(flatten)]
    action: Action,
    #[serde(skip)]
    live_trainer: Option<(u64, u64)>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Spawn {
    entry: u32,
    map: u16,
    position: [f32; 3],
}

impl Plan {
    pub(super) async fn observe_login(
        &mut self,
        opcode: u16,
        payload: &[u8],
        stream: &mut TcpStream,
        crypt: &mut WorldCrypt,
    ) -> Result<()> {
        // C++ Player::CanNeverSee gates NPC visibility on this client ACK.
        // MovementHandler::HandleMoveInitActiveMoverComplete then refreshes it.
        if opcode == 0x2597 {
            send_encrypted_packet(
                stream,
                crypt,
                CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE,
                &build_move_init_active_mover_complete_payload(0),
            )
            .await?;
        }
        if let Action::Trainer {
            spawn: Some(spawn), ..
        } = &self.action
        {
            if opcode == SMSG_UPDATE_OBJECT {
                for candidate in find_creature_guids_near_position_in_update_object(
                    payload,
                    spawn.map,
                    spawn.entry,
                    spawn.position[0],
                    spawn.position[1],
                    spawn.position[2],
                    3.0,
                    None,
                ) {
                    let guid = (candidate.low, candidate.high);
                    if self.live_trainer.is_some_and(|previous| previous != guid) {
                        bail!("multiple live trainer candidates match the pinned spawn position");
                    }
                    self.live_trainer = Some(guid);
                }
            }
        }
        Ok(())
    }

    pub(super) fn login_ready(&self) -> bool {
        !matches!(&self.action, Action::Trainer { spawn: Some(_), .. })
            || self.live_trainer.is_some()
    }

    fn trainer_guid(&self) -> Result<(u64, u64)> {
        match &self.action {
            Action::Trainer { spawn: Some(_), .. } => self
                .live_trainer
                .context("trainer CREATE_OBJECT was not observed"),
            Action::Trainer {
                guid_low,
                guid_high,
                ..
            } => Ok((*guid_low, *guid_high)),
            _ => bail!("not a trainer plan"),
        }
    }
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
        gossip_option: Option<i32>,
        spawn: Option<Spawn>,
    },
    Cast {
        spell: i32,
        cast_low: u64,
        cast_high: u64,
    },
    Verify {},
}

pub(super) fn load() -> Result<Option<Plan>> {
    let Some(path) = std::env::var_os("WOW_BOT_ACQUISITION_PLAN") else {
        return Ok(None);
    };
    let plan: Plan = serde_json::from_slice(&std::fs::read(path)?)?;
    plan.persistence.validate()?;
    if plan.expected_spell == 0 || plan.expected_spell > i32::MAX as u32 {
        bail!("acquisition expected spell must be a positive signed spell ID");
    }
    match &plan.action {
        Action::Trainer {
            guid_low,
            guid_high,
            trainer_id,
            offer_spell,
            spawn,
            ..
        } if (spawn.is_none() && (*guid_low == 0 || *guid_high == 0))
            || *trainer_id <= 0
            || *offer_spell <= 0
            || spawn
                .as_ref()
                .is_some_and(|s| s.entry == 0 || s.position.iter().any(|v| !v.is_finite())) =>
        {
            bail!("trainer plan requires a live NPC GUID, trainer and offer")
        }
        Action::Cast { spell, .. } if *spell <= 0 => bail!("cast spell must be positive"),
        _ => {}
    }
    if !std::env::var("WOW_BOT_LOGIN_SAVE_CHECK").is_ok_and(|v| is_truthy(&v))
        || std::env::var("WOW_BOT_LOGIN_DISCONNECT_CHECK").is_ok_and(|v| is_truthy(&v))
    {
        bail!("acquisition requires normal login-save mode");
    }
    Ok(Some(plan))
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
    bytes.extend([0; 12]); // SpellXSpellVisualID, trajectory pitch/speed
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
    let spell = plan.expected_spell;
    let before = read_state(bot, spell, plan.persistence).await?;
    let money_before = before.money;
    let verified_at_login = matches!(plan.action, Action::Verify {});
    let mut receipt = Evidence {
        expected_spell: spell,
        trainer_guid: if matches!(&plan.action, Action::Trainer { .. }) {
            Some(plan.trainer_guid()?)
        } else {
            None
        },
        learned_on_instance: false,
        verified_at_login,
        money_before,
        observed_db_money_after_action: money_before,
        expected_saved_money: money_before,
        repeated_purchase_rejected: false,
        saved_spell: false,
        saved_money: 0,
        persistence: plan.persistence,
        observed_skill_root: None,
        persistence_verified: false,
    };
    if verified_at_login {
        plan.persistence
            .verify_login(&before, known.known_spells.contains(&spell))?;
        return Ok(receipt);
    }
    if known.known_spells.contains(&spell) || before.saved_spell || before.skill_root.is_some() {
        bail!("acquisition fixture already knows expected spell");
    }
    match &plan.action {
        Action::Trainer {
            trainer_id,
            offer_spell,
            gossip_option,
            ..
        } => {
            let (guid_low, guid_high) = plan.trainer_guid()?;
            let guid = build_packed_guid(guid_low, guid_high);
            send_encrypted_packet(
                stream,
                crypt,
                if gossip_option.is_some() {
                    0x3492
                } else {
                    0x34AD
                },
                &guid,
            )
            .await?;
            let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
            let mut admitted = false;
            let mut selected = false;
            while tokio::time::Instant::now() < deadline {
                if let Some((_, op, payload)) =
                    next(stream, crypt, inflater, realm, deadline).await?
                {
                    if op == 0x2A98 && !selected {
                        if let Some(option) = gossip_option {
                            let (_, low, high) = packet_parser::parse_packed_guid(&payload)
                                .context("invalid gossip GUID")?;
                            if (low, high) != (guid_low, guid_high) {
                                bail!("gossip NPC does not match fixture");
                            }
                            let menu = packet_parser::parse_gossip_id(&payload)
                                .context("invalid gossip menu")?;
                            send_encrypted_packet(
                                stream,
                                crypt,
                                0x3494,
                                &build_gossip_select_option(&guid, menu, *option),
                            )
                            .await?;
                            selected = true;
                        }
                    }
                    if op == 0x26DF {
                        let summary = packet_parser::parse_trainer_list_summary(&payload)
                            .context("invalid trainer list")?;
                        let (size, low, high) = packet_parser::parse_packed_guid(&payload)
                            .context("invalid trainer GUID")?;
                        let _ = size;
                        if (low, high) != (guid_low, guid_high) || summary.trainer_id != *trainer_id
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
                &trainer_buy(&guid, *trainer_id, *offer_spell),
            )
            .await?;
        }
        Action::Cast {
            spell,
            cast_low,
            cast_high,
        } => {
            if !known.known_spells.contains(&(*spell as u32)) {
                bail!("fixture does not know the requested cast");
            }
            let payload = cast_self(
                *spell,
                (*cast_low, *cast_high),
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
        trainer_id,
        offer_spell,
        ..
    } = &plan.action
    {
        let (guid_low, guid_high) = plan.trainer_guid()?;
        let guid = build_packed_guid(guid_low, guid_high);
        send_encrypted_packet(
            stream,
            crypt,
            0x34AE,
            &trainer_buy(&guid, *trainer_id, *offer_spell),
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
                        || i32::from_le_bytes(tail[..4].try_into().unwrap()) != *offer_spell
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
    receipt.observed_db_money_after_action = read_state(bot, spell, plan.persistence).await?.money;
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

#[cfg(test)]
mod tests;
