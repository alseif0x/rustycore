//! Chest money collection, release and response checks for the QA restart scenario.

use super::*;

pub(super) async fn claim_recovery_chest_money(
    action_index: usize,
    expected_coins: u32,
    deadline: tokio::time::Instant,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut Option<EncryptedWorldConnection>,
    controller: &mut Controller,
) -> Result<()> {
    wait_for_packet(
        deadline,
        Connection::Instance,
        crate::loot_race::SMSG_LOOT_RESPONSE,
        stream,
        crypt,
        inflater,
        realm,
        controller,
    )
    .await?;
    let selected_guid = recovery_chest_guid(&controller.evidence)?;
    let open_send = controller
        .evidence
        .sent_packets
        .iter()
        .rev()
        .find(|sent| sent.action == "open_loot_recovery_chest")
        .context("recovery chest was not opened by this session")?;
    let response_event = controller
        .evidence
        .events
        .iter()
        .rev()
        .find(|event| {
            event.connection == Connection::Instance
                && event.opcode == crate::loot_race::SMSG_LOOT_RESPONSE
                && event.observation_sequence > open_send.observation_sequence
        })
        .context("server did not return the recovery chest loot window")?;
    let response_payload = hex::decode(&response_event.body_hex)?;
    let loot_object_guid = validate_recovery_chest_loot_response(
        &response_payload,
        selected_guid,
        expected_coins,
        realm_id(),
    )?;

    // The item row remains untouched: this workflow sends only CMSG_LOOT_MONEY.
    let (opcode, body) = recovery_chest_money_claim_packet();
    send_traced_packet(
        stream,
        crypt,
        &mut controller.evidence,
        Connection::Instance,
        Some(action_index),
        "claim_loot_recovery_money",
        opcode,
        &body,
    )
    .await?;
    wait_for_packet(
        deadline,
        Connection::Instance,
        crate::loot_race::SMSG_COIN_REMOVED,
        stream,
        crypt,
        inflater,
        realm,
        controller,
    )
    .await?;
    wait_for_packet(
        deadline,
        Connection::Instance,
        crate::loot_race::SMSG_LOOT_MONEY_NOTIFY,
        stream,
        crypt,
        inflater,
        realm,
        controller,
    )
    .await?;
    let coin_removed = controller
        .evidence
        .events
        .iter()
        .rev()
        .find(|event| {
            event.connection == Connection::Instance
                && event.opcode == crate::loot_race::SMSG_COIN_REMOVED
        })
        .context("server did not send CoinRemoved after the money claim")?;
    let money_notify = controller
        .evidence
        .events
        .iter()
        .rev()
        .find(|event| {
            event.connection == Connection::Instance
                && event.opcode == crate::loot_race::SMSG_LOOT_MONEY_NOTIFY
        })
        .context("server did not send LootMoneyNotify after the money claim")?;
    validate_recovery_chest_money_outcome(
        &hex::decode(&coin_removed.body_hex)?,
        &hex::decode(&money_notify.body_hex)?,
        loot_object_guid,
        expected_coins,
    )?;
    Ok(())
}

pub(super) fn validate_recovery_chest_loot_response(
    payload: &[u8],
    selected_guid: Guid,
    expected_coins: u32,
    expected_realm_id: u32,
) -> Result<Guid> {
    let response = crate::loot_race::parse_loot_response(payload)?;
    if (response.owner_low, response.owner_high) != (selected_guid.low, selected_guid.high) {
        bail!("LootResponse owner did not match the discovered recovery chest GUID");
    }
    let loot_object_guid = Guid {
        low: response.loot_low,
        high: response.loot_high,
    };
    crate::loot_race::validate_loot_object_guid_like_cpp(
        loot_object_guid.low,
        loot_object_guid.high,
        crate::loot_race::RACE_GAMEOBJECT_MAP_ID,
        expected_realm_id,
    )?;
    if response.failure_reason != CXX_DEFAULT_LOOT_RESPONSE_FAILURE_REASON
        || !response.acquired
        || !matches!(
            response.loot_method,
            0 | crate::loot_race::PERSONAL_LOOT_METHOD_LIKE_CPP
        )
        || response.coins != expected_coins
        || response.items.len() != 1
        || response.items[0].item_entry != crate::loot_race::DEFAULT_ITEM_ENTRY
        || response.items[0].quantity != 1
    {
        bail!(
            "recovery chest LootResponse differed from the fixture contract: failure={} acquired={} method={} coins={} rows={:?}",
            response.failure_reason,
            response.acquired,
            response.loot_method,
            response.coins,
            response
                .items
                .iter()
                .map(|item| (item.item_entry, item.quantity))
                .collect::<Vec<_>>(),
        );
    }
    Ok(loot_object_guid)
}

pub(super) fn validate_recovery_chest_money_outcome(
    coin_removed_payload: &[u8],
    money_notify_payload: &[u8],
    loot_object_guid: Guid,
    expected_coins: u32,
) -> Result<()> {
    let coin_guid = wire::parse_loot_coin_removed(coin_removed_payload)?;
    if coin_guid != loot_object_guid {
        bail!("CoinRemoved GUID did not match the LootObject from LootResponse");
    }
    let notification = wire::parse_loot_money_notify(money_notify_payload)?;
    if notification.money != u64::from(expected_coins)
        || notification.money_mod != 0
        || !notification.sole_looter
    {
        bail!("LootMoneyNotify did not match the expected chest money: {notification:?}");
    }
    Ok(())
}

pub(super) fn recovery_chest_money_claim_packet() -> (u16, [u8; 1]) {
    (crate::loot_race::CMSG_LOOT_MONEY, [0])
}

pub(super) async fn release_recovery_chest(
    action_index: usize,
    deadline: tokio::time::Instant,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut Option<EncryptedWorldConnection>,
    controller: &mut Controller,
) -> Result<()> {
    let chest_guid = recovery_chest_guid(&controller.evidence)?;
    send_traced_packet(
        stream,
        crypt,
        &mut controller.evidence,
        Connection::Instance,
        Some(action_index),
        "release_loot_recovery_chest",
        CMSG_LOOT_RELEASE,
        &chest_guid.packed(),
    )
    .await?;
    wait_for_packet(
        deadline,
        Connection::Instance,
        SMSG_LOOT_RELEASE,
        stream,
        crypt,
        inflater,
        realm,
        controller,
    )
    .await?;
    let event = controller
        .evidence
        .events
        .iter()
        .rev()
        .find(|event| event.connection == Connection::Instance && event.opcode == SMSG_LOOT_RELEASE)
        .context("server did not acknowledge the recovery chest release")?;
    let (loot_guid, owner_guid) = wire::parse_loot_release(&hex::decode(&event.body_hex)?)?;
    if loot_guid != chest_guid || owner_guid != controller.mover_guid() {
        bail!("LootRelease acknowledgement did not match this chest and player session");
    }
    Ok(())
}
