//! Login/realm-list adapter for modern GameUtilities, TC 6ebe044c.
//! V2 has different Variant tags and first-match parameter semantics. Reuse the
//! existing realm readers, not V1 bytes on the wire. World join remains outside
//! this branch's login-only acceptance and is explicitly not implemented here.

use super::*;
use types::variant::Type;
use wow_proto::bgs::protocol::game_utilities::v2::client as wire;
use wow_proto::bgs::protocol::v2 as types;

fn decode<M: Message + Default>(payload: &[u8]) -> Result<M> {
    M::decode(payload).map_err(|_| RpcStatusError::new(status::ERROR_RPC_MALFORMED_REQUEST).into())
}

pub async fn handle<S: AsyncRead + AsyncWrite + Unpin>(
    session: &mut RpcSession<S>,
    method: u32,
    payload: &[u8],
) -> Result<Option<Vec<u8>>> {
    match method & 0x3FFF_FFFF {
        1 => {
            let request: wire::ProcessTaskRequest = decode(payload)?;
            if !session.authed {
                return Err(RpcStatusError::new(status::ERROR_DENIED).into());
            }
            let request = project_attributes(request.attribute);
            let command = find_command_attr_like_cpp(&request.attribute)
                .map(|attr| remove_suffix(&attr.name))
                .ok_or_else(|| RpcStatusError::new(status::ERROR_RPC_MALFORMED_REQUEST))?;
            tracing::debug!(command, "GameUtilities V2 command");
            let response = match command {
                "Command_SuperDistrictListRequest_v1" => {
                    if session.build != 70170 {
                        return Err(RpcStatusError::new(status::ERROR_RPC_NOT_IMPLEMENTED).into());
                    }
                    // Discovery is admitted only after a game-account ticket.
                    let account = session.account_info.as_ref().ok_or_else(|| {
                        RpcStatusError::new(status::ERROR_USER_SERVER_BAD_WOW_ACCOUNT)
                    })?;
                    selected_game_account_like_cpp(account, session.selected_game_account_id)?;
                    let data = session.state().forever_catalog().compressed_list()?;
                    Some(
                        ClientResponse {
                            attribute: vec![make_blob_attribute("Param_SuperDistrictList", &data)],
                        }
                        .encode_to_vec(),
                    )
                }
                "Command_FetchBleepProxiesRequest_v1" => {
                    let data = crate::realm::forever::compressed_empty_bleep_proxies();
                    Some(
                        ClientResponse {
                            attribute: vec![make_blob_attribute("Param_BleepProxyList", &data)],
                        }
                        .encode_to_vec(),
                    )
                }
                "Command_RealmListTicketRequest_v1" => realm_list_ticket(session, &request).await?,
                "Command_LastCharPlayedRequest_v1" => {
                    let attr = find_command_param_like_cpp(&request.attribute, command)
                        .expect("selected command");
                    if attr.value.string_value.is_none() {
                        return Err(
                            RpcStatusError::new(status::ERROR_UTIL_SERVER_UNKNOWN_REALM).into()
                        );
                    }
                    get_last_char_played(session, &request).await?
                }
                "Command_RealmListRequest_v1" => get_realm_list(session, &request).await?,
                _ => return Err(RpcStatusError::new(status::ERROR_RPC_NOT_IMPLEMENTED).into()),
            };
            let response = ClientResponse::decode(response.unwrap_or_default().as_slice())?;
            Ok(Some(
                wire::ProcessTaskResponse {
                    result: response
                        .attribute
                        .into_iter()
                        .map(|attr| types::Attribute {
                            name: Some(attr.name),
                            value: Some(to_v2(attr.value)),
                        })
                        .collect(),
                }
                .encode_to_vec(),
            ))
        }
        2 => {
            let request: wire::GetAllValuesForAttributeRequest = decode(payload)?;
            if !session.authed {
                return Err(RpcStatusError::new(status::ERROR_DENIED).into());
            }
            let key = request.attribute_key.as_deref().unwrap_or_default();
            if !key.starts_with("Command_") || remove_suffix(key) != "Command_RealmListRequest_v1" {
                return Err(RpcStatusError::new(status::ERROR_RPC_NOT_IMPLEMENTED).into());
            }
            let values = session
                .state()
                .realm_mgr
                .read()
                .write_sub_regions_like_cpp();
            Ok(Some(
                wire::GetAllValuesForAttributeResponse {
                    attribute_value: values.into_iter().map(to_v2).collect(),
                }
                .encode_to_vec(),
            ))
        }
        _ => Err(RpcStatusError::new(status::ERROR_RPC_INVALID_METHOD).into()),
    }
}

async fn realm_list_ticket<S: AsyncRead + AsyncWrite + Unpin>(
    session: &mut RpcSession<S>,
    request: &ClientRequest,
) -> Result<Option<Vec<u8>>> {
    // Modern Shared::GetRealmListTicket admits every input before SetClientInfo.
    let id = parse_realm_list_ticket_game_account_id_like_cpp(&request.attribute)
        .ok_or_else(|| RpcStatusError::new(status::ERROR_UTIL_SERVER_INVALID_IDENTITY_ARGS))?;
    let account = session
        .account_info
        .as_ref()
        .and_then(|account| account.game_accounts.get(&id))
        .ok_or_else(|| RpcStatusError::new(status::ERROR_UTIL_SERVER_INVALID_IDENTITY_ARGS))?;
    if account.is_permanently_banned {
        return Err(RpcStatusError::new(status::ERROR_GAME_ACCOUNT_BANNED).into());
    }
    if account.is_banned {
        return Err(RpcStatusError::new(status::ERROR_GAME_ACCOUNT_SUSPENDED).into());
    }
    let secret = parse_realm_list_ticket_client_secret_like_cpp(&request.attribute)
        .ok_or_else(|| RpcStatusError::new(status::ERROR_WOW_SERVICES_DENIED_REALM_LIST_TICKET))?;
    session.selected_game_account_id = Some(id);
    session.client_secret = secret;
    let update = BnetLastLoginInfoUpdateLikeCpp {
        client_ip: session.addr().ip().to_string(),
        locale: locale_string_to_id_like_cpp(&session.locale),
        os: session.os.clone(),
        account_id: session.account_info.as_ref().expect("admitted account").id,
    };
    let mut stmt = session
        .state()
        .login_db
        .prepare(LoginStatements::UPD_BNET_LAST_LOGIN_INFO);
    apply_bnet_last_login_info_update_like_cpp(&mut stmt, &update);
    session.state().login_db.execute(&stmt).await?;
    Ok(Some(
        ClientResponse {
            attribute: vec![make_blob_attribute(
                "Param_RealmListTicket",
                b"AuthRealmListTicket\0",
            )],
        }
        .encode_to_vec(),
    ))
}

fn project_attributes(attributes: Vec<types::Attribute>) -> ClientRequest {
    // The existing readers select the last match. Reverse only this projection
    // to reproduce modern Shared::FindParamValue's FIRST match, without changing V1.
    ClientRequest {
        attribute: attributes
            .into_iter()
            .rev()
            .filter_map(|attr| {
                Some(Attribute {
                    name: attr.name?,
                    value: from_v2(attr.value?),
                })
            })
            .collect(),
        ..Default::default()
    }
}

fn from_v2(value: types::Variant) -> Variant {
    let mut out = Variant::default();
    match value.r#type.unwrap_or(Type::BoolValue(false)) {
        Type::BoolValue(v) => out.bool_value = Some(v),
        Type::IntValue(v) => out.int_value = Some(v),
        Type::FloatValue(v) => out.float_value = Some(v),
        Type::StringValue(v) => out.string_value = Some(v),
        Type::BlobValue(v) => out.blob_value = Some(v),
        Type::UintValue(v) => out.uint_value = Some(v),
    }
    out
}

fn to_v2(value: Variant) -> types::Variant {
    let value = if let Some(v) = value.bool_value {
        Type::BoolValue(v)
    } else if let Some(v) = value.int_value {
        Type::IntValue(v)
    } else if let Some(v) = value.float_value {
        Type::FloatValue(v)
    } else if let Some(v) = value.string_value {
        Type::StringValue(v)
    } else if let Some(v) = value.blob_value {
        Type::BlobValue(v)
    } else {
        Type::UintValue(value.uint_value.unwrap_or_default())
    };
    types::Variant {
        r#type: Some(value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn v2_variant_tags_are_not_v1_tags() {
        let value = types::Variant {
            r#type: Some(Type::StringValue("x".into())),
        };
        assert_eq!(value.encode_to_vec(), b"\x22\x01x");
        assert_eq!(to_v2(from_v2(value.clone())), value);
        let blob = types::Variant {
            r#type: Some(Type::BlobValue(vec![1, 2])),
        };
        assert_eq!(blob.encode_to_vec(), [0x2a, 2, 1, 2]);
        assert_eq!(to_v2(from_v2(blob.clone())), blob);
        let value = types::Variant {
            r#type: Some(Type::UintValue(1)),
        };
        assert_eq!(value.encode_to_vec(), [0x30, 1]);
        assert_eq!(to_v2(from_v2(value.clone())), value);
    }
    #[test]
    fn projection_skips_missing_attributes_and_keeps_modern_first_match() {
        let attribute = |name: &str, text: &str| types::Attribute {
            name: Some(name.into()),
            value: Some(types::Variant {
                r#type: Some(Type::StringValue(text.into())),
            }),
        };
        let request = project_attributes(vec![
            types::Attribute::default(),
            attribute("Command_RealmListRequest_v1_classic", "first"),
            attribute("Command_RealmListRequest_v1_classic", "second"),
        ]);
        assert_eq!(request.attribute.len(), 2);
        assert_eq!(
            find_command_attr_like_cpp(&request.attribute)
                .unwrap()
                .value
                .string_value
                .as_deref(),
            Some("first")
        );
    }
}
