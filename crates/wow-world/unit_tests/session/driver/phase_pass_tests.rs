//! #787 — phase-filtered selection regressions.
//!
//! The C++ contract these pin: `LockedQueue::next` reads the head and leaves it
//! queued when the filter refuses it (`common/Threading/LockedQueue.h:82-95`),
//! and `MapSessionFilter`/`WorldSessionFilter` split the queue by
//! `ProcessingPlace` and `IsInWorld` (`Server/WorldSession.cpp:64-108`).

use super::*;
use wow_handler::PacketProcessing;

#[test]
fn the_cpp_filter_table_is_what_allows_phase_answers() {
    use PacketUpdatePhase::{Map, World};
    use PlayerPacketResidence::{InWorld, Missing, OutsideWorld};

    // PROCESS_INPLACE: both filters accept it.
    for phase in [Map, World] {
        for residence in [Missing, OutsideWorld, InWorld] {
            assert!(PacketProcessing::Inplace.allows_phase(phase, residence));
        }
    }

    // PROCESS_THREADUNSAFE: world only, whatever the residence.
    for residence in [Missing, OutsideWorld, InWorld] {
        assert!(PacketProcessing::ThreadUnsafe.allows_phase(World, residence));
        assert!(!PacketProcessing::ThreadUnsafe.allows_phase(Map, residence));
    }

    // The rest: the map pass takes it only while the player is in world, and
    // the world pass takes it only while it is not.
    assert!(PacketProcessing::ThreadSafe.allows_phase(Map, InWorld));
    assert!(!PacketProcessing::ThreadSafe.allows_phase(Map, OutsideWorld));
    assert!(!PacketProcessing::ThreadSafe.allows_phase(Map, Missing));
    assert!(!PacketProcessing::ThreadSafe.allows_phase(World, InWorld));
    assert!(PacketProcessing::ThreadSafe.allows_phase(World, OutsideWorld));
    assert!(PacketProcessing::ThreadSafe.allows_phase(World, Missing));
}

#[test]
fn every_packet_belongs_to_exactly_one_pass_for_a_given_residence() {
    use PacketUpdatePhase::{Map, World};
    use PlayerPacketResidence::{InWorld, Missing, OutsideWorld};

    // `Inplace` is the C++ exception: it is eligible in both filters, and the
    // driver must therefore select it once, not run it twice. Every other
    // classification lands in exactly one pass.
    for residence in [Missing, OutsideWorld, InWorld] {
        for processing in [PacketProcessing::ThreadUnsafe, PacketProcessing::ThreadSafe] {
            let map = processing.allows_phase(Map, residence);
            let world = processing.allows_phase(World, residence);
            assert!(
                map ^ world,
                "{processing:?} at {residence:?} must belong to exactly one pass"
            );
        }
        assert!(PacketProcessing::Inplace.allows_phase(Map, residence));
        assert!(PacketProcessing::Inplace.allows_phase(World, residence));
    }
}
