//! Battle-pet account regressions, part 2 of 2.
//!
//! Moved out of the battle_pet_account_tests.rs root under #683; every test is unchanged.

use super::*;

#[tokio::test]
async fn failed_update_delete_and_slot_persistence_publish_nothing_like_cpp() {
    let persistence = Arc::new(FakePersistenceLikeCpp::default());
    let registry = registry_like_cpp(Arc::clone(&persistence), 0, 800);
    let attachment = registry.attach_like_cpp(77).await.expect("attach");
    assert!(attachment.try_acquire_lease_like_cpp().await);
    let owner = attachment.owner_like_cpp();
    let lease = attachment.lease_id_like_cpp();
    let request = add_request_like_cpp(23, 11, 1);
    let pet_guid = match owner
        .try_add_pet_like_cpp(lease, request.clone())
        .await
        .expect("add")
    {
        BattlePetAddOutcomeLikeCpp::Added(pet) | BattlePetAddOutcomeLikeCpp::Replayed(pet) => {
            pet.guid
        }
    };
    let original = owner.pet_snapshot_like_cpp(pet_guid).expect("pet");

    persistence.fail_next_update.store(true, Ordering::Release);
    assert!(matches!(
        owner
            .try_mutate_pet_like_cpp(lease, pet_guid, |pet| pet.level = 9)
            .await,
        Err(BattlePetMutationFailureLikeCpp::DatabaseFailure(_))
    ));
    assert_eq!(owner.pet_snapshot_like_cpp(pet_guid), Some(original));

    persistence.fail_next_slots.store(true, Ordering::Release);
    assert!(matches!(
        owner.try_set_slot_like_cpp(lease, pet_guid, 0).await,
        Err(BattlePetMutationFailureLikeCpp::DatabaseFailure(_))
    ));
    assert_eq!(
        owner.journal_like_cpp(lease, request.owner_guid).slots[0].pet_guid,
        empty_battle_pet_guid_like_cpp()
    );

    persistence.fail_next_delete.store(true, Ordering::Release);
    assert!(matches!(
        owner.try_remove_pet_like_cpp(lease, pet_guid).await,
        Err(BattlePetMutationFailureLikeCpp::DatabaseFailure(_))
    ));
    assert!(owner.pet_snapshot_like_cpp(pet_guid).is_some());
}
