use super::*;

fn ip() -> IpAddr {
    "127.0.0.1".parse().unwrap()
}

#[test]
fn cookie_reconnect_preserves_one_state_and_rejects_wrong_ip_or_unknown_id() {
    let registry = RestSessions::default();
    let (first, new) = registry.acquire(None, ip());
    assert!(new);
    let id = first.id.clone();
    let state = Arc::clone(&first.state);
    let cookie = format!("ignored=x; JSESSIONID={id}");
    drop(first);
    let (second, new) = registry.acquire(Some(&cookie), ip());
    assert!(!new);
    assert!(Arc::ptr_eq(&state, &second.state));
    let (foreign, new) = registry.acquire(Some(&cookie), "127.0.0.2".parse().unwrap());
    assert!(new);
    assert!(!Arc::ptr_eq(&state, &foreign.state));
    let (unknown, new) = registry.acquire(Some("JSESSIONID=malformed"), ip());
    assert!(new);
    assert_ne!(unknown.id, "malformed");
    assert_eq!(registry.entries.lock()[&id].connections, 1);
}

#[test]
fn active_lease_pins_state_and_last_disconnect_starts_five_minute_expiry() {
    let registry = RestSessions::default();
    let (first, _) = registry.acquire(None, ip());
    let id = first.id.clone();
    let cookie = format!("JSESSIONID={id}");
    let later = Instant::now() + INACTIVE_TTL * 2;
    let (second, new) = registry.acquire_at(Some(&cookie), ip(), later);
    assert!(!new);
    drop(first);
    assert!(registry.entries.lock()[&id].inactive_since.is_none());
    drop(second);
    let since = registry.entries.lock()[&id].inactive_since.unwrap();
    let (replacement, new) = registry.acquire_at(
        Some(&cookie),
        ip(),
        since + INACTIVE_TTL + Duration::from_secs(1),
    );
    assert!(new);
    assert_ne!(replacement.id, id);
    assert!(!registry.entries.lock().contains_key(&id));
}

#[test]
fn cookie_attributes_match_modern_https_session_contract() {
    let registry = RestSessions::default();
    let (lease, _) = registry.acquire(None, ip());
    assert_eq!(
        lease.set_cookie("localhost:18081"),
        format!(
            "JSESSIONID={}; Path=/bnetserver; Domain=localhost; Secure; HttpOnly; SameSite=None",
            lease.id
        )
    );
    assert!(!lease.set_cookie("[::1]:18081").contains("Domain="));
    assert!(!lease.set_cookie("bad; injected=x").contains("Domain="));
}

#[tokio::test]
async fn operations_serialize_per_session_not_globally_and_cancel_safely() {
    let registry = RestSessions::default();
    let (first, _) = registry.acquire(None, ip());
    let (same, _) = registry.acquire(Some(&format!("JSESSIONID={}", first.id)), ip());
    let (other, _) = registry.acquire(None, ip());
    let guard = first.lock().await;
    assert!(same.state.try_lock().is_err());
    assert!(other.state.try_lock().is_ok());
    drop(guard);
    assert!(same.state.try_lock().is_ok());
    drop(first);
    assert_eq!(registry.entries.lock()[&same.id].connections, 1);
}
