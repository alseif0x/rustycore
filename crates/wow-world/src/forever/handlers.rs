//! Registry thunks for target account-phase operations.
use super::{CharacterCatalog, HandlerResult, Outgoing, Phase, Request, Session, SessionError};
use wow_packet::forever as packet;

pub(super) async fn ping(session: &mut Session, request: Request) -> HandlerResult {
    if request.payload.len() != 8 {
        return Err(SessionError::Protocol);
    }
    session.latency =
        u32::from_le_bytes(request.payload[4..].try_into().expect("checked ping size"));
    Ok(vec![Outgoing::new(
        wow_network::forever::PONG,
        request.payload[..4].to_vec(),
    )])
}

pub(super) async fn disconnect(session: &mut Session, request: Request) -> HandlerResult {
    if request.payload.len() != 4 {
        return Err(SessionError::Protocol);
    }
    session.phase = Phase::Closed;
    Ok(vec![])
}

pub(super) async fn enumerate(
    session: &mut Session,
    catalog: &CharacterCatalog,
    request: Request,
) -> HandlerResult {
    if !request.payload.is_empty() {
        return Err(SessionError::Protocol);
    }
    session
        .repository
        .enumerate_empty(session.identity.account_id)
        .await
        .map_err(SessionError::Persistence)?;
    let account_expansion = session.identity.account_expansion;
    let races = catalog
        .races()
        .iter()
        .map(|race| {
            let class_unlocks: Vec<_> = race
                .classes
                .iter()
                .map(|class| packet::ClassUnlock {
                    class_id: class.id as i8,
                    achievement_id: 0,
                    has_expansion: class.active_expansion == 0
                        && account_expansion >= class.account_expansion,
                    has_unlocked_achievement: true,
                    has_entitlement: true,
                })
                .collect();
            packet::RaceUnlock {
                race_id: race.id as i8,
                has_unlocked_license: account_expansion >= race.unlock_expansion,
                has_unlocked_achievement: false,
                has_heritage_armor_unlock_achievement: false,
                has_entitlement: true,
                hide_race_on_client: false,
                faction_balance_disabled: false,
                does_not_have_available_classes: class_unlocks
                    .iter()
                    .all(|class| !class.has_expansion),
                class_unlocks,
            }
        })
        .collect();
    let enumeration = packet::EmptyEnumCharactersResult {
        success: true,
        realmless: false,
        is_deleted_characters: false,
        ignore_new_player_restrictions: false,
        is_restricted_new_player: false,
        is_newcomer_chat_completed: false,
        is_restricted_trial: false,
        is_account_lapsed_player: false,
        force_character_list_sort: false,
        class_disable_mask: Some(0),
        // CharacterPackets.h::EnumCharactersResult defaults to 1 even if empty.
        max_character_level: 1,
        race_unlock_data: races,
    }
    .encode_payload()
    .map_err(|_| SessionError::Codec)?;
    let release = packet::EmptyRecentAllyDataResponse70009 {
        leading_value: 0,
        mode: 0,
        entry_count: 0,
    }
    .encode_payload();
    session.enumerated = true;
    // CollectionMgr::SendWarbandSceneCollectionData follows the release.
    // ClassicOpcodes.cpp deliberately replaces the retail collection layout
    // with four zero bytes. This 70009 contract still needs native 70170 QA.
    Ok(vec![
        Outgoing::new(0x460018, enumeration),
        Outgoing::new(0x460362, release),
        Outgoing::new(0x460360, vec![0; 4]),
    ])
}

pub(super) async fn hotfix(session: &mut Session, request: Request) -> HandlerResult {
    let query = packet::hotfix::HotfixRequest::decode(
        &request.payload,
        session.hotfixes.metadata().hotfix_count(),
    )
    .map_err(|_| SessionError::Protocol)?;
    let mut records = vec![];
    let mut content = vec![];
    for push in query.push_ids {
        if let Some(push) = session.hotfixes.metadata().hotfix_push(push) {
            for record in &push.records {
                if record.available_locales_mask & (1 << 6) == 0 {
                    continue;
                }
                let mut status = record.status as u8;
                let blob = if status == 1 {
                    match session
                        .hotfixes
                        .record(record.table_hash, record.record_id as u32)
                        .map_err(|_| SessionError::Protocol)?
                        .or_else(|| {
                            session
                                .hotfixes
                                .metadata()
                                .get_hotfix_blob(record.table_hash, record.record_id)
                        }) {
                        Some(blob) => blob,
                        None => {
                            status = if session.hotfixes.has_store(record.table_hash) {
                                2
                            } else {
                                3
                            };
                            &[]
                        }
                    }
                } else {
                    &[]
                };
                records.push(packet::hotfix::HotfixConnectRecord {
                    push_id: record.id.push_id,
                    unique_id: record.id.unique_id,
                    table_hash: record.table_hash,
                    record_id: record.record_id,
                    size: u32::try_from(blob.len()).map_err(|_| SessionError::Codec)?,
                    status: status.try_into().map_err(|_| SessionError::Protocol)?,
                });
                content.extend_from_slice(blob);
            }
        }
    }
    let payload = packet::hotfix::HotfixConnectPayload { records, content }
        .encode_payload()
        .map_err(|_| SessionError::Codec)?;
    Ok(vec![Outgoing::new(0x4A0003, payload)])
}

pub(super) async fn db_query(session: &mut Session, request: Request) -> HandlerResult {
    let query = packet::db_query::DBQueryBulk::decode(&request.payload)
        .map_err(|_| SessionError::Protocol)?;
    // Only TactKey has a complete target baseline/typed serializer here.
    // Other hashes may name target stores not yet loaded by Rust: do not
    // manufacture Invalid/absence merely because this milestone lacks them.
    if query.table_hash != wow_data::forever_hotfix::TACT_KEY_TABLE_HASH {
        return Err(SessionError::Protocol);
    }
    let mut output = Vec::with_capacity(query.record_ids.len());
    for id in query.record_ids {
        let data = session
            .hotfixes
            .record(query.table_hash, id)
            .map_err(|_| SessionError::Protocol)?;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|time| u32::try_from(time.as_secs()).ok())
            .ok_or(SessionError::Protocol)?;
        let reply = packet::db_query::DBReply {
            table_hash: query.table_hash,
            record_id: id,
            timestamp,
            status: if data.is_some() {
                packet::hotfix::HotfixStatus::Valid
            } else {
                packet::hotfix::HotfixStatus::Invalid
            },
            data: data.unwrap_or(&[]),
        }
        .encode_payload()
        .map_err(|_| SessionError::Codec)?;
        output.push(Outgoing::new(packet::db_query::CLASSIC_REPLY_OPCODE, reply));
    }
    Ok(output)
}
