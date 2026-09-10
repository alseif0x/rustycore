//! Tests for the handlers module.
//!
//! Separated from handlers.rs under #687.

use super::*;

#[test]
fn wrong_password_remote_ip_uses_forwarded_for_first_hop_like_cpp() {
    let headers = HashMap::from([(
        "x-forwarded-for".to_string(),
        "198.51.100.7, 198.51.100.8".to_string(),
    )]);

    assert_eq!(
        wrong_password_remote_ip_from_headers_like_cpp(&headers, "203.0.113.10"),
        "198.51.100.7"
    );
}

#[test]
fn wrong_password_remote_ip_falls_back_to_external_address_like_cpp() {
    let headers = HashMap::new();

    assert_eq!(
        wrong_password_remote_ip_from_headers_like_cpp(&headers, "203.0.113.10"),
        "203.0.113.10"
    );
}

#[test]
fn login_form_headers_do_not_set_cookie_like_cpp() {
    let headers = login_form_headers_like_cpp();

    assert_eq!(
        headers,
        vec![("Content-Type", "application/json;charset=utf-8".to_string())]
    );
    assert!(
        !headers
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("Set-Cookie"))
    );
}

#[test]
fn srp_challenge_headers_do_not_set_cookie_like_cpp() {
    let headers = srp_challenge_headers_like_cpp();

    assert_eq!(
        headers,
        vec![("Content-Type", "application/json;charset=utf-8".to_string())]
    );
    assert!(
        !headers
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("Set-Cookie"))
    );
}

#[test]
fn hex_encode_uses_cpp_uppercase() {
    assert_eq!(hex_encode(&[0x00, 0x0a, 0xbc, 0xff]), "000ABCFF");
}

#[test]
fn make_login_ticket_uses_cpp_uppercase_hex() {
    let ticket = make_login_ticket();
    let Some(hex) = ticket.strip_prefix("TC-") else {
        panic!("ticket must use TC- prefix");
    };

    assert_eq!(hex.len(), 40);
    assert!(hex.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert!(!hex.bytes().any(|byte| matches!(byte, b'a'..=b'f')));
}

#[test]
fn srp_challenge_missing_account_returns_done_like_cpp() {
    let response = srp_challenge_missing_account_response_like_cpp();

    assert_eq!(response.status_code, 200);
    assert_eq!(
        response.headers,
        vec![("Content-Type", "application/json;charset=utf-8".to_string())]
    );

    let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(body["authentication_state"], "DONE");
    assert!(body["error_code"].is_null());
    assert!(body["error_message"].is_null());
    assert!(body["login_ticket"].is_null());
    assert!(body["server_evidence_M2"].is_null());
}

#[test]
fn bot_srp_challenge_rejects_malformed_or_missing_inputs_like_cpp() {
    let mut connection_state = RestConnectionState::default();

    let bad_json = post_bot_srp_challenge(&mut connection_state, Some(b"not-json"));
    assert_eq!(bad_json.status_code, 400);
    assert!(bad_json.body.is_empty());

    let missing_password =
        post_bot_srp_challenge(&mut connection_state, Some(br#"{"username":"user"}"#));
    assert_eq!(missing_password.status_code, 400);
    assert!(missing_password.body.is_empty());
}

#[test]
fn bot_srp_challenge_returns_cpp_shape_and_connection_state() {
    let mut connection_state = RestConnectionState::default();

    let response = post_bot_srp_challenge(
        &mut connection_state,
        Some(br#"{"username":"user@example.com","password":"secret"}"#),
    );

    assert_eq!(response.status_code, 200);
    assert_eq!(
        response.headers,
        vec![("Content-Type", "application/json".to_string())]
    );
    let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
    assert!(body.get("salt").and_then(|value| value.as_str()).is_some());
    assert!(
        body.get("public_B")
            .and_then(|value| value.as_str())
            .is_some()
    );
    assert!(connection_state.bot_srp.is_some());
}

#[test]
fn bot_srp_fixed_32_and_broken_vectors_match_cpp_lengths() {
    assert_eq!(bot_fixed_32_be_like_cpp(&BigUint::from(2u32)).len(), 32);
    assert_eq!(
        hex_encode_upper(&bot_fixed_32_be_like_cpp(&BigUint::from(2u32))),
        "0000000000000000000000000000000000000000000000000000000000000002"
    );

    assert_eq!(
        bot_broken_evidence_vector_like_cpp(&BigUint::from(0x1234u32)),
        vec![0x12, 0x34]
    );
    assert_eq!(
        bot_broken_evidence_vector_like_cpp(&BigUint::from(0x80u32)),
        vec![0x00, 0x80]
    );
}

#[test]
fn bot_srp_evidence_verifies_matching_client_proof_like_cpp() {
    let n = bot_srp_n_like_cpp();
    let g = BigUint::from(2u32);
    let k = bot_srp_k_like_cpp(&n, &g);
    let x = BigUint::from(11u32);
    let a = BigUint::from(17u32);
    let b = BigUint::from(19u32);
    let verifier = g.modpow(&x, &n);
    let public_a = g.modpow(&a, &n);
    let public_b = (g.modpow(&b, &n) + (&verifier * &k)) % &n;

    let u = BigUint::from_bytes_be(&Sha256::digest(
        [
            bot_fixed_32_be_like_cpp(&public_a).as_slice(),
            bot_fixed_32_be_like_cpp(&public_b).as_slice(),
        ]
        .concat(),
    ));
    let gx = g.modpow(&x, &n);
    let base = (&public_b + &n - ((&k * &gx) % &n)) % &n;
    let client_s = base.modpow(&(&a + (&u * &x)), &n);
    let client_m1 = bot_srp_evidence_hash_like_cpp(&[&public_a, &public_b, &client_s]);

    let bot_srp = BotSrpState {
        username: "user@example.com".to_string(),
        verifier,
        b,
        public_b,
    };
    let proof = verify_bot_srp_evidence_like_cpp(
        &bot_srp,
        &public_a.to_str_radix(16),
        &client_m1.to_str_radix(16),
    )
    .expect("matching bot proof");

    assert_eq!(proof.session_key.len(), 32);
    assert!(
        verify_bot_srp_evidence_like_cpp(&bot_srp, &public_a.to_str_radix(16), "deadbeef")
            .is_none()
    );
}

#[test]
fn extract_auth_ticket_decodes_basic_and_truncates_at_colon_like_cpp() {
    let headers = HashMap::from([(
        "authorization".to_string(),
        "Basic VElDS0VUOnNlY3JldA==".to_string(),
    )]);

    assert_eq!(extract_auth_ticket(&headers).as_deref(), Some("TICKET"));
}

#[test]
fn extract_auth_ticket_decodes_without_basic_prefix_like_cpp() {
    let headers = HashMap::from([("authorization".to_string(), "VEMtYWJjMTIzOg==".to_string())]);

    assert_eq!(extract_auth_ticket(&headers).as_deref(), Some("TC-abc123"));
}

#[test]
fn extract_auth_ticket_accepts_decoded_value_without_colon_like_cpp() {
    let headers = HashMap::from([("authorization".to_string(), "VEMtcmF3".to_string())]);

    assert_eq!(extract_auth_ticket(&headers).as_deref(), Some("TC-raw"));
}

#[test]
fn extract_auth_ticket_rejects_invalid_or_empty_ticket_like_cpp() {
    let invalid = HashMap::from([("authorization".to_string(), "Basic not base64".to_string())]);
    let empty = HashMap::from([("authorization".to_string(), "Og==".to_string())]);

    assert_eq!(extract_auth_ticket(&invalid), None);
    assert_eq!(extract_auth_ticket(&empty), None);
}

#[test]
fn login_refresh_result_serializes_extended_ticket_shape_like_cpp() {
    let body = serde_json::to_string(&LoginRefreshResult {
        login_ticket_expiry: Some(1_700_000_600),
        is_expired: None,
    })
    .unwrap();

    assert_eq!(body, r#"{"login_ticket_expiry":1700000600}"#);
}

#[test]
fn login_refresh_result_serializes_expired_ticket_shape_like_cpp() {
    let body = serde_json::to_string(&LoginRefreshResult {
        login_ticket_expiry: None,
        is_expired: Some(true),
    })
    .unwrap();

    assert_eq!(body, r#"{"is_expired":true}"#);
}
