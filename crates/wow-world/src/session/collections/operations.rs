//! Represented mail, auction and achievement operations.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    #[cfg(test)]
    pub(crate) fn represented_auction_place_bids_like_cpp(
        &self,
    ) -> &[RepresentedAuctionPlaceBidLikeCpp] {
        self.inventory.represented_auction_place_bids_like_cpp()
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/collections/operations/f3_shims.rs"]
mod f3_shims;
