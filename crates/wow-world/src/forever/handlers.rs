//! Registry thunks for target account-phase operations.
use super::{CharacterCatalog, HandlerResult, Outgoing, Phase, Request, Session, SessionError};
use std::borrow::Cow;
use wow_packet::forever as packet;

pub(super) async fn check_name(session: &mut Session, request: Request) -> HandlerResult {
    use super::name_rules::Validation;
    use packet::name_availability::{
        CHECK_CHARACTER_NAME_AVAILABILITY_RESULT_OPCODE, CheckCharacterNameAvailability,
        CheckCharacterNameAvailabilityResult,
    };
    let request = CheckCharacterNameAvailability::decode(&request.payload)
        .map_err(|_| SessionError::Protocol)?;
    // CharacterHandler.cpp:1687-1719. Surname and unknown bits are source
    // fields, not extra admission rules. Preserve the caller's sequence.
    let result = match session
        .name_rules
        .check(
            request.name(),
            session.name_policy,
            session.identity.dbc_locale,
            session.identity.permissions.skip_sql_reserved_names(),
        )
        .map_err(SessionError::NameRules)?
    {
        Validation::Rejected(rejection) => rejection.wire_result(),
        Validation::Accepted(name) => {
            if session
                .repository
                .name_in_use(name.spelling())
                .await
                .map_err(SessionError::Persistence)?
            {
                27
            } else {
                0
            }
        }
    };
    // Read-only lookup: this is not a name reservation. The complete Create
    // operation must recheck admission/collision and commit independently.
    Ok(vec![Outgoing::new(
        CHECK_CHARACTER_NAME_AVAILABILITY_RESULT_OPCODE,
        CheckCharacterNameAvailabilityResult::new(request.sequence_index(), result)
            .encode_payload(),
    )])
}

pub(super) async fn create(
    session: &mut Session,
    catalog: &CharacterCatalog,
    request: Request,
) -> HandlerResult {
    use super::name_rules::Validation;
    use packet::character_result::{CREATE_CHAR_CLASSIC_OPCODE, CreateCharResponse};
    use wow_persistence::forever::creation::save::CreationAdmission;

    let request = packet::character_create::CharacterCreatePayload::decode(&request.payload)
        .map_err(|_| SessionError::Protocol)?;
    let normalized = match session
        .name_rules
        .check(
            request.name(),
            session.name_policy,
            session.identity.dbc_locale,
            session.identity.permissions.skip_sql_reserved_names(),
        )
        .map_err(SessionError::NameRules)?
    {
        Validation::Rejected(rejection) => {
            return Ok(vec![Outgoing::new(
                CREATE_CHAR_CLASSIC_OPCODE,
                CreateCharResponse {
                    code: rejection.wire_result(),
                    guid: wow_core::ObjectGuid::EMPTY,
                }
                .encode_payload(),
            )]);
        }
        Validation::Accepted(name) => name,
    };
    let Some(creator) = session.creator.clone() else {
        return Ok(vec![Outgoing::new(
            CREATE_CHAR_CLASSIC_OPCODE,
            CreateCharResponse {
                code: 25,
                guid: wow_core::ObjectGuid::EMPTY,
            }
            .encode_payload(),
        )]);
    };
    // The whole admitted operation, including its initial reads, is fenced
    // against re-entry if dispatch is cancelled.
    session.phase = super::Phase::Creating;
    session.enumerated = false;
    session.legitimate_characters.clear();
    let operation = session
        .repository
        .begin_character_creation(CreationAdmission {
            account: session.identity.account_id,
            realm: session.identity.realm_address,
            name: normalized.spelling().to_owned(),
            limits: creator.limits(),
        })
        .await;
    let mut owner = None;
    let (code, guid) =
        match operation {
            Err(error) => creation_error(error)?,
            Ok(mut operation) => {
                let reserved = operation.identity();
                // Source allocator excludes zero and the final two counter values;
                // validate before construction/save, never convert fallibly after commit.
                if reserved.account != session.identity.account_id
                    || reserved.realm != session.identity.realm_address
                    || reserved.guid == 0
                    || reserved.guid
                        >= (wow_core::ObjectGuid::max_counter(wow_core::guid::HighGuid::Player)
                            as u64)
                            - 1
                {
                    return Err(SessionError::Protocol);
                }
                // CharacterHandler generates the identity before Player::Create.
                // The constructor can allocate item identities from this same
                // operation, without seeing the database or its writer gate.
                let constructed = creator.construct(
                    operation.as_mut(),
                    &request,
                    normalized.spelling(),
                    catalog,
                    session.identity.account_expansion,
                    session.latency,
                );
                match constructed {
                    Err(failure) => {
                        drop(operation);
                        (failure.response_code, 0)
                    }
                    Ok(constructed) => {
                        let (character, guard) = constructed.into_parts();
                        owner = Some(guard);
                        if character.guid != reserved.guid
                            || character.account != reserved.account
                            || character.realm != reserved.realm
                            || character.name != normalized.spelling()
                        {
                            return Err(SessionError::Protocol);
                        }
                        match operation.save(character).await {
                        Ok(created) if created.guid == reserved.guid => {
                            owner.as_mut().expect("constructed owner retained").committed(&created);
                            (24, created.guid)
                        }
                        Ok(_) => return Err(SessionError::Creation(
                            wow_persistence::forever::creation::save::CreateError::RecoveryRequired,
                        )),
                        Err(error) => creation_error(error)?,
                    }
                    }
                }
            }
        };
    let guid = if guid == 0 {
        wow_core::ObjectGuid::EMPTY
    } else {
        super::guid::player_guid(session.identity.realm_address, guid)
    };
    let output = vec![Outgoing::new(
        CREATE_CHAR_CLASSIC_OPCODE,
        CreateCharResponse { code, guid }.encode_payload(),
    )];
    drop(owner);
    session.phase = super::Phase::Authenticated;
    Ok(output)
}

fn creation_error(
    error: wow_persistence::forever::creation::save::CreateError,
) -> Result<(u32, u64), SessionError> {
    use wow_persistence::forever::creation::save::CreateError;
    match error {
        CreateError::NameInUse => Ok((27, 0)),
        CreateError::AccountLimit => Ok((31, 0)),
        CreateError::RealmLimit => Ok((30, 0)),
        CreateError::Database | CreateError::InvalidRow => Ok((25, 0)),
        error @ (CreateError::CommitUnknown
        | CreateError::LoginCommitUnknown
        | CreateError::RecoveryRequired) => Err(SessionError::Creation(error)),
    }
}

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
    session.close();
    Ok(vec![])
}

pub(super) async fn battlenet(_session: &mut Session, request: Request) -> HandlerResult {
    use packet::battlenet::{
        BattlenetRequest, BattlenetResponse, ContentHandleRequest, GET_CONTENT_HANDLE_METHOD_ID,
        RESOURCES_SERVICE_HASH, official_beta_content_handle,
    };
    let request = BattlenetRequest::decode(&request.payload).ok_or(SessionError::Protocol)?;
    let response = if request.service_hash() == RESOURCES_SERVICE_HASH
        && request.method_id() == GET_CONTENT_HANDLE_METHOD_ID
    {
        let content = ContentHandleRequest::decode(request.data()).ok_or(SessionError::Protocol)?;
        if let Some(handle) = official_beta_content_handle(&content) {
            BattlenetResponse::success(
                request.service_hash(),
                request.method_id(),
                request.token(),
                handle,
            )
        } else {
            BattlenetResponse::not_implemented(
                request.service_hash(),
                request.method_id(),
                request.token(),
            )
        }
    } else if request.service_hash() == packet::battlenet::FRIENDS_SERVICE_HASH
        && matches!(
            request.method_id(),
            packet::battlenet::GET_FRIENDS_METHOD_ID
                | packet::battlenet::GET_RECEIVED_INVITATIONS_METHOD_ID
                | packet::battlenet::GET_SENT_INVITATIONS_METHOD_ID
        )
    {
        BattlenetResponse::success(
            request.service_hash(),
            request.method_id(),
            request.token(),
            vec![0x10, 0x00],
        )
    } else {
        BattlenetResponse::not_implemented(
            request.service_hash(),
            request.method_id(),
            request.token(),
        )
    };
    Ok(vec![Outgoing::new(
        packet::battlenet::BATTLENET_RESPONSE_OPCODE,
        response.encode_payload().map_err(|_| SessionError::Codec)?,
    )])
}

pub(super) async fn purchase_list(_session: &mut Session, request: Request) -> HandlerResult {
    if !request.payload.is_empty() {
        return Err(SessionError::Protocol);
    }
    Ok(vec![Outgoing::new(
        packet::BATTLE_PAY_GET_PURCHASE_LIST_RESPONSE_OPCODE,
        packet::EmptyBattlePayPurchaseListResponse { result: 0 }.encode_payload(),
    )])
}

pub(super) async fn enumerate(
    session: &mut Session,
    catalog: &CharacterCatalog,
    request: Request,
) -> HandlerResult {
    if !request.payload.is_empty() {
        return Err(SessionError::Protocol);
    }
    // Revoke the previous normal-list authority while this refresh is pending.
    // Cancellation cannot leave old characters admitted or restart this phase.
    session.phase = Phase::Selecting;
    session.legitimate_characters.clear();
    session.enumerated = false;
    let rows = session
        .repository
        .load_character_selection(
            session.identity.account_id,
            session.selection_policy.declined_names,
        )
        .await
        .map_err(SessionError::Persistence)?;
    let projection = super::selection::project(
        rows,
        session.identity.realm_address,
        &session.selection_policy,
    )?;
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
    let enumeration = packet::character_list::EnumCharactersResult {
        success: true,
        realmless: false,
        is_deleted_characters: false,
        ignore_new_player_restrictions: false,
        is_restricted_new_player: false,
        is_newcomer_chat_completed: false,
        is_restricted_trial: false,
        is_account_lapsed_player: false,
        force_character_list_sort: false,
        class_disable_mask: Some(session.selection_policy.class_disable_mask),
        // CharacterPackets.h::EnumCharactersResult defaults to 1 even if empty.
        max_character_level: projection.max_level,
        race_unlock_data: races,
        characters: projection.characters,
        unlocked_conditional_appearances: vec![],
        race_limit_disables: vec![],
    }
    .encode_payload()
    .map_err(|_| SessionError::Codec)?;
    let release = packet::EmptyRecentAllyDataResponse70009 {
        leading_value: 0x08ECBD10,
        mode: 7,
        entry_count: 0,
    }
    .encode_payload();
    // Intentional fence over the source's queued Execute: await each
    // idempotent OR before publishing. A lost ACK closes the session; no
    // success is inferred and a later enumeration can safely repeat it.
    for guid in projection.recustomize {
        session
            .repository
            .require_recustomization(session.identity.account_id, guid)
            .await
            .map_err(SessionError::Persistence)?;
    }
    session.legitimate_characters = projection.legitimate;
    session.enumerated = true;
    session.phase = Phase::Authenticated;
    // CollectionMgr::SendWarbandSceneCollectionData follows the release.
    // ClassicOpcodes.cpp deliberately replaces the retail collection layout
    // with four zero bytes. This 70009 contract still needs native 70170 QA.
    Ok(vec![
        Outgoing::new(0x460018, enumeration),
        Outgoing::new(
            packet::BATTLE_PAY_GET_DISTRIBUTION_LIST_RESPONSE_OPCODE,
            packet::EmptyBattlePayDistributionListResponse { result: 0 }.encode_payload(),
        ),
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
                if record.available_locales_mask & (1 << session.identity.dbc_locale) == 0 {
                    continue;
                }
                let mut status = record.status as u8;
                let blob = if status == 1 {
                    match session
                        .hotfixes
                        .write_record(
                            record.table_hash,
                            record.record_id as u32,
                            session.identity.dbc_locale,
                        )
                        .map_err(|_| SessionError::Protocol)?
                        .or_else(|| {
                            session
                                .hotfixes
                                .metadata()
                                .get_hotfix_blob(record.table_hash, record.record_id)
                                .map(Cow::Borrowed)
                        }) {
                        Some(blob) => blob,
                        None => {
                            status = if session.hotfixes.has_store(record.table_hash) {
                                2
                            } else {
                                3
                            };
                            Cow::Borrowed(&[] as &[u8])
                        }
                    }
                } else {
                    Cow::Borrowed(&[] as &[u8])
                };
                records.push(packet::hotfix::HotfixConnectRecord {
                    push_id: record.id.push_id,
                    unique_id: record.id.unique_id,
                    table_hash: record.table_hash,
                    record_id: record.record_id,
                    size: u32::try_from(blob.len()).map_err(|_| SessionError::Codec)?,
                    status: status.try_into().map_err(|_| SessionError::Protocol)?,
                });
                content.extend_from_slice(&blob);
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
    // Only explicitly composed typed stores have serializers here.
    // Other hashes may name target stores not yet loaded by Rust: do not
    // manufacture Invalid/absence merely because this milestone lacks them.
    if !session.hotfixes.has_serializer(query.table_hash) {
        return Err(SessionError::Protocol);
    }
    let mut output = Vec::with_capacity(query.record_ids.len());
    for id in query.record_ids {
        let data = session
            .hotfixes
            .write_record(query.table_hash, id, session.identity.dbc_locale)
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
            data: data.as_deref().unwrap_or(&[]),
        }
        .encode_payload()
        .map_err(|_| SessionError::Codec)?;
        output.push(Outgoing::new(packet::db_query::CLASSIC_REPLY_OPCODE, reply));
    }
    Ok(output)
}
