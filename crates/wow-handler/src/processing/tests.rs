// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::{PacketProcessing, PacketUpdatePhase, PlayerPacketResidence};

#[test]
fn both_cpp_filters_match_all_processing_and_residence_combinations() {
    use PacketProcessing::{Inplace, ThreadSafe, ThreadUnsafe};
    use PlayerPacketResidence::{InWorld, Missing, OutsideWorld};

    // Expected columns are literal transcriptions of the independent C++
    // WorldSessionFilter / MapSessionFilter branches, not computed by the API.
    let cases = [
        (Inplace, Missing, true, true),
        (Inplace, OutsideWorld, true, true),
        (Inplace, InWorld, true, true),
        (ThreadUnsafe, Missing, true, false),
        (ThreadUnsafe, OutsideWorld, true, false),
        (ThreadUnsafe, InWorld, true, false),
        (ThreadSafe, Missing, true, false),
        (ThreadSafe, OutsideWorld, true, false),
        (ThreadSafe, InWorld, false, true),
    ];
    for (processing, residence, world, map) in cases {
        assert_eq!(
            processing.allows_phase(PacketUpdatePhase::World, residence),
            world,
            "world filter: {processing:?}, {residence:?}"
        );
        assert_eq!(
            processing.allows_phase(PacketUpdatePhase::Map, residence),
            map,
            "map filter: {processing:?}, {residence:?}"
        );
    }
}

#[test]
fn thread_safe_eligibility_follows_attach_transfer_and_retirement() {
    use PlayerPacketResidence::{InWorld, Missing, OutsideWorld};

    // Observations supplied by the lifetime owner, not a simulation of that owner.
    // Detach transfers eligibility to World; successful reattach returns it to Map.
    let observations = [
        Missing,
        OutsideWorld,
        InWorld,
        OutsideWorld,
        InWorld,
        Missing,
    ];
    let expected_phases = [
        PacketUpdatePhase::World,
        PacketUpdatePhase::World,
        PacketUpdatePhase::Map,
        PacketUpdatePhase::World,
        PacketUpdatePhase::Map,
        PacketUpdatePhase::World,
    ];
    for (residence, expected) in observations.into_iter().zip(expected_phases) {
        for phase in [PacketUpdatePhase::World, PacketUpdatePhase::Map] {
            assert_eq!(
                PacketProcessing::ThreadSafe.allows_phase(phase, residence),
                phase == expected
            );
        }
    }
}
