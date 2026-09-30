use super::*;

impl WorldSession {
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_sign_petition_like_cpp(
        &mut self,
        petition_guid: ObjectGuid,
        choice: u8,
    ) {
        #[cfg(test)]
        self.represented_sign_petitions_like_cpp
            .push(RepresentedSignPetitionLikeCpp {
                petition_guid,
                choice,
            });
    }
    #[cfg(test)]
    pub(crate) fn represented_sign_petitions_like_cpp(&self) -> &[RepresentedSignPetitionLikeCpp] {
        &self.represented_sign_petitions_like_cpp
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_decline_petition_like_cpp(
        &mut self,
        petition_guid: ObjectGuid,
    ) {
        #[cfg(test)]
        self.represented_decline_petitions_like_cpp
            .push(RepresentedDeclinePetitionLikeCpp { petition_guid });
    }
    #[cfg(test)]
    pub(crate) fn represented_decline_petitions_like_cpp(
        &self,
    ) -> &[RepresentedDeclinePetitionLikeCpp] {
        &self.represented_decline_petitions_like_cpp
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_query_petition_like_cpp(
        &mut self,
        petition_id: u32,
        item_guid: ObjectGuid,
    ) {
        #[cfg(test)]
        self.represented_query_petitions_like_cpp
            .push(RepresentedQueryPetitionLikeCpp {
                petition_id,
                item_guid,
            });
    }
    #[cfg(test)]
    pub(crate) fn represented_query_petitions_like_cpp(
        &self,
    ) -> &[RepresentedQueryPetitionLikeCpp] {
        &self.represented_query_petitions_like_cpp
    }
}
