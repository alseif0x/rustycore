//! Atomic object discovery and snapshot-fenced visibility publication.

use super::{HashSet, ObjectGuid, SharedClientVisibleGuidsLikeCpp};

impl SharedClientVisibleGuidsLikeCpp {
    /// Forget a known object only after its nonblocking destroy enqueue succeeds.
    /// A replacement birth may then discover the same GUID without a delayed
    /// old destroy removing that discovery.
    pub fn publish_known_destroy_like_cpp(
        &self,
        guid: ObjectGuid,
        enqueue: impl FnOnce() -> bool,
    ) -> bool {
        let mut current = self.write_like_cpp();
        if !current.contains(&guid) {
            return true;
        }
        if !enqueue() {
            return false;
        }
        current.remove(&guid);
        true
    }

    /// Enqueue an unseen CREATE before making discovery observable. A full or
    /// stale endpoint leaves the ledger intact; a known object needs no packet.
    /// `enqueue` must not re-enter this ledger and must not block.
    pub fn publish_unseen_create_like_cpp(
        &self,
        guid: ObjectGuid,
        enqueue: impl FnOnce() -> bool,
    ) -> bool {
        let mut current = self.write_like_cpp();
        if current.contains(&guid) {
            return true;
        }
        if !enqueue() {
            return false;
        }
        current.insert(guid);
        true
    }

    /// A scanner may only replace the membership it actually observed before
    /// querying Map. Concurrent birth discovery requires a fresh scan instead.
    pub fn publish_transition_if_snapshot_like_cpp<R>(
        &self,
        expected: &HashSet<ObjectGuid>,
        mut keep: impl FnMut(&ObjectGuid) -> bool,
        added: impl IntoIterator<Item = ObjectGuid>,
        publish: impl FnOnce() -> R,
    ) -> Option<R> {
        let mut current = self.write_like_cpp();
        if *current != *expected {
            return None;
        }
        current.retain(|guid| keep(guid));
        current.extend(added);
        Some(publish())
    }
}

#[cfg(test)]
#[path = "../../../../unit_tests/session/mailbox/visibility.rs"]
mod tests;
