use super::*;

#[tokio::test]
async fn recovery_requests_share_the_frozen_unix_nanosecond_snapshot() {
    let registry = PlayerRegistry::new();
    let first = ObjectGuid::create_player(1, 91);
    let second = ObjectGuid::create_player(1, 92);
    let first_rail = register(&registry, first);
    let second_rail = register(&registry, second);
    let participants = one_map(vec![first, second]);
    let unix_time_ns = 1_799_999_999_123_456_789;

    let (summary, ()) = tokio::join!(
        run_player_recovery_passes(
            &participants,
            &registry,
            COORDINATOR,
            91,
            27,
            35,
            unix_time_ns,
            Duration::from_millis(20),
        ),
        async {
            let first_request = await_recovery_request(&first_rail).await;
            assert_eq!(first_request.admission.unix_time_ns, unix_time_ns);
            complete_recovery_request(first_request, PlayerRecoveryStatus::Completed);

            let second_request = await_recovery_request(&second_rail).await;
            assert_eq!(second_request.admission.unix_time_ns, unix_time_ns);
            complete_recovery_request(second_request, PlayerRecoveryStatus::Completed);
        }
    );

    assert_eq!(summary.requested, 2);
    assert_eq!(summary.completed, 2);
    assert!(summary.quiescent_like_cpp());
}
