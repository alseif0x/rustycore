//! Admission and identity before Player construction, with one owned writer gate.
use super::{
    CreateError, CreationLease, CreationReservation, ForeverSessionRepository, PreparedStatement,
    field, journal, next, writer,
};
use wow_persistence::{
    PersistenceFutureLikeCpp,
    forever::creation::save::{
        CharacterCreation, CreatedCharacter, CreationAdmission, CreationIdentities,
        CreationIdentity, NewCharacter,
    },
};

#[cfg(test)]
#[path = "../../../unit_tests/forever/creation_lease.rs"]
mod tests;

impl ForeverSessionRepository {
    pub(in crate::forever) async fn reserve_creation(
        &self,
        admission: CreationAdmission,
    ) -> Result<Box<dyn CharacterCreation>, CreateError> {
        if admission.account == 0 || admission.realm & 0xFFFF == 0 {
            return Err(CreateError::InvalidRow);
        }
        // The returned operation retains this same async gate through the
        // constructor and save. Other callers wait rather than racing counts.
        let mut ids = self.creation_ids.clone().lock_owned().await;
        if ids.recovery_required {
            return Err(CreateError::RecoveryRequired);
        }
        journal::ensure_schema(&self.characters, &self.auth).await?;
        let mut writer = writer::CreationWriter::acquire(&self.characters).await?;
        if let Err(error) = journal::ensure_empty(&mut writer).await {
            if error == CreateError::RecoveryRequired {
                ids.recovery_required = true;
            }
            return Err(error);
        }
        let mut auth_transaction =
            writer::lock_account(&self.auth, admission.account, admission.realm).await?;
        let mut collision = PreparedStatement::new(super::super::names::COLLISION_QUERY);
        collision.set_string(0, admission.name.clone());
        if !writer.query(&collision).await?.is_empty() {
            return Err(CreateError::NameInUse);
        }
        let mut counts =
            PreparedStatement::new("SELECT numchars FROM realmcharacters WHERE acctid=?");
        counts.set_u32(0, admission.account);
        let mut result = writer::auth_query(&mut auth_transaction, &counts).await?;
        let mut account_count = 0_u32;
        if !result.is_empty() {
            loop {
                account_count = account_count
                    .checked_add(field::<u32>(&result, 0).map_err(|_| CreateError::InvalidRow)?)
                    .ok_or(CreateError::AccountLimit)?;
                if !result.next_row() {
                    break;
                }
            }
        }
        if account_count >= admission.limits.account {
            return Err(CreateError::AccountLimit);
        }
        let mut realm_query = PreparedStatement::new(
            "SELECT CAST(COUNT(guid) AS UNSIGNED) FROM characters WHERE account=? AND deleteDate IS NULL",
        );
        realm_query.set_u32(0, admission.account);
        let realm_count: u64 =
            field(&writer.query(&realm_query).await?, 0).map_err(|_| CreateError::InvalidRow)?;
        let realm_count = u32::try_from(realm_count).map_err(|_| CreateError::InvalidRow)?;
        if realm_count >= admission.limits.realm {
            return Err(CreateError::RealmLimit);
        }
        // realmcharacters.numchars is TINYINT UNSIGNED in the pinned schema.
        if realm_count >= u32::from(u8::MAX) {
            return Err(CreateError::InvalidRow);
        }
        let maximum = field::<Option<u64>>(
            &writer
                .query(&PreparedStatement::new("SELECT MAX(guid) FROM characters"))
                .await?,
            0,
        )
        .map_err(|_| CreateError::InvalidRow)?;
        synchronize_counter(&mut ids.player, maximum)?;
        let guid = next(&mut ids.player, None)?;
        let maximum = field::<Option<u64>>(
            &writer
                .query(&PreparedStatement::new(
                    "SELECT MAX(guid) FROM item_instance",
                ))
                .await?,
            0,
        )
        .map_err(|_| CreateError::InvalidRow)?;
        synchronize_counter(&mut ids.item, maximum)?;
        let item_start = ids.item.ok_or(CreateError::InvalidRow)?;
        Ok(Box::new(CreationLease {
            reservation: CreationReservation {
                ids,
                identity: CreationIdentity {
                    guid,
                    account: admission.account,
                    realm: admission.realm,
                },
                item_start,
                name: admission.name,
                realm_count,
            },
            writer,
            auth_transaction,
        }))
    }
}

impl CreationIdentities for CreationReservation {
    fn identity(&self) -> CreationIdentity {
        self.identity
    }

    fn next_item_guid(&mut self) -> Result<u64, CreateError> {
        next(&mut self.ids.item, None)
    }
}

fn synchronize_counter(counter: &mut Option<u64>, maximum: Option<u64>) -> Result<(), CreateError> {
    let persisted_next = maximum
        .unwrap_or(0)
        .checked_add(1)
        .ok_or(CreateError::InvalidRow)?;
    *counter = Some(counter.unwrap_or(1).max(persisted_next));
    Ok(())
}

impl CreationIdentities for CreationLease {
    fn identity(&self) -> CreationIdentity {
        self.reservation.identity()
    }
    fn next_item_guid(&mut self) -> Result<u64, CreateError> {
        self.reservation.next_item_guid()
    }
}

impl CharacterCreation for CreationLease {
    fn save(
        self: Box<Self>,
        character: NewCharacter,
    ) -> PersistenceFutureLikeCpp<'static, Result<CreatedCharacter, CreateError>> {
        Box::pin(self.persist(character))
    }
}
