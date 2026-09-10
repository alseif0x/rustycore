//! Responses packets.
//!
//! Separated from handlers.rs under #693.

use super::*;

// ── Helpers ─────────────────────────────────────────────────────────────────

pub(super) fn json_response<T: serde::Serialize>(value: T) -> HttpResponse {
    json_response_with_content_type(value, "application/json;charset=utf-8")
}

pub(super) fn json_response_with_content_type<T: serde::Serialize>(
    value: T,
    content_type: &'static str,
) -> HttpResponse {
    let body = serde_json::to_string(&value).unwrap_or_default();
    HttpResponse {
        status_code: 200,
        status_text: "OK",
        headers: vec![("Content-Type", content_type.to_string())],
        body,
    }
}

pub(super) fn json_error_response(
    status_code: u16,
    status_text: &'static str,
    error: &str,
) -> HttpResponse {
    json_error_response_with_content_type(
        status_code,
        status_text,
        error,
        "application/json;charset=utf-8",
    )
}

pub(super) fn json_error_response_with_content_type(
    status_code: u16,
    status_text: &'static str,
    error: &str,
    content_type: &'static str,
) -> HttpResponse {
    let body = serde_json::to_string(&serde_json::json!({"error": error})).unwrap_or_default();
    HttpResponse {
        status_code,
        status_text,
        headers: vec![("Content-Type", content_type.to_string())],
        body,
    }
}

pub(super) fn empty_response(status_code: u16, status_text: &'static str) -> HttpResponse {
    HttpResponse {
        status_code,
        status_text,
        headers: vec![],
        body: String::new(),
    }
}

/// C++ returns "DONE" with no other fields for wrong password / account not found
/// to prevent account enumeration.
pub(super) fn error_result(_msg: &str) -> AuthResult {
    AuthResult {
        authentication_state: "DONE",
        error_code: None,
        error_message: None,
        url: None,
        login_ticket: None,
        server_evidence_m2: None,
    }
}
