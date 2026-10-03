//! All 153 five-field target entries from 02245dcd SpellInfo.cpp:246-401.
//! NYI rows and their non-NONE object/reference fields are retained verbatim.
use super::{
    TargetCheck as C, TargetDirection as D, TargetObject as O, TargetReference as R,
    TargetSelection as S,
};
pub(super) type Metadata = (O, R, S, C, D);
pub(super) const TARGETS: [Metadata; 153] = [
    (O::None, R::None, S::NotImplemented, C::Default, D::None), // 0
    (O::Unit, R::Caster, S::Default, C::Default, D::None),      // 1
    (O::Unit, R::Caster, S::Nearby, C::Enemy, D::None),         // 2
    (O::Unit, R::Caster, S::Nearby, C::Ally, D::None),          // 3
    (O::Unit, R::Caster, S::Nearby, C::Party, D::None),         // 4
    (O::Unit, R::Caster, S::Default, C::Default, D::None),      // 5
    (O::Unit, R::Target, S::Default, C::Enemy, D::None),        // 6
    (O::Unit, R::Source, S::Area, C::Entry, D::None),           // 7
    (O::Unit, R::Destination, S::Area, C::Entry, D::None),      // 8
    (O::Destination, R::Caster, S::Default, C::Default, D::None), // 9
    (O::None, R::None, S::NotImplemented, C::Default, D::None), // 10
    (O::Unit, R::Source, S::NotImplemented, C::Default, D::None), // 11
    (O::None, R::None, S::NotImplemented, C::Default, D::None), // 12
    (O::None, R::None, S::NotImplemented, C::Default, D::None), // 13
    (O::None, R::None, S::NotImplemented, C::Default, D::None), // 14
    (O::Unit, R::Source, S::Area, C::Enemy, D::None),           // 15
    (O::Unit, R::Destination, S::Area, C::Enemy, D::None),      // 16
    (O::Destination, R::Caster, S::Default, C::Default, D::None), // 17
    (O::Destination, R::Caster, S::Default, C::Default, D::None), // 18
    (O::None, R::None, S::NotImplemented, C::Default, D::None), // 19
    (O::Unit, R::Caster, S::Area, C::Party, D::None),           // 20
    (O::Unit, R::Target, S::Default, C::Ally, D::None),         // 21
    (O::Source, R::Caster, S::Default, C::Default, D::None),    // 22
    (O::GameObject, R::Target, S::Default, C::Default, D::None), // 23
    (O::Unit, R::Caster, S::Cone, C::Enemy, D::Front),          // 24
    (O::Unit, R::Target, S::Default, C::Default, D::None),      // 25
    (
        O::GameObjectItem,
        R::Target,
        S::Default,
        C::Default,
        D::None,
    ), // 26
    (O::Unit, R::Caster, S::Default, C::Default, D::None),      // 27
    (
        O::Destination,
        R::Destination,
        S::Default,
        C::Enemy,
        D::None,
    ), // 28
    (O::Destination, R::Destination, S::Default, C::Ally, D::None), // 29
    (O::Unit, R::Source, S::Area, C::Ally, D::None),            // 30
    (O::Unit, R::Destination, S::Area, C::Ally, D::None),       // 31
    (
        O::Destination,
        R::Caster,
        S::Default,
        C::Default,
        D::FrontLeft,
    ), // 32
    (O::Unit, R::Source, S::Area, C::Party, D::None),           // 33
    (O::Unit, R::Destination, S::Area, C::Party, D::None),      // 34
    (O::Unit, R::Target, S::Default, C::Party, D::None),        // 35
    (
        O::Destination,
        R::Caster,
        S::NotImplemented,
        C::Default,
        D::None,
    ), // 36
    (O::Unit, R::Last, S::Area, C::Party, D::None),             // 37
    (O::Unit, R::Caster, S::Nearby, C::Entry, D::None),         // 38
    (O::Destination, R::Caster, S::Default, C::Default, D::None), // 39
    (O::GameObject, R::Caster, S::Nearby, C::Entry, D::None),   // 40
    (
        O::Destination,
        R::Caster,
        S::Default,
        C::Default,
        D::FrontRight,
    ), // 41
    (
        O::Destination,
        R::Caster,
        S::Default,
        C::Default,
        D::BackRight,
    ), // 42
    (
        O::Destination,
        R::Caster,
        S::Default,
        C::Default,
        D::BackLeft,
    ), // 43
    (
        O::Destination,
        R::Caster,
        S::Default,
        C::Default,
        D::FrontLeft,
    ), // 44
    (O::Unit, R::Target, S::Default, C::Ally, D::None),         // 45
    (O::Destination, R::Caster, S::Nearby, C::Entry, D::None),  // 46
    (O::Destination, R::Caster, S::Default, C::Default, D::Front), // 47
    (O::Destination, R::Caster, S::Default, C::Default, D::Back), // 48
    (O::Destination, R::Caster, S::Default, C::Default, D::Right), // 49
    (O::Destination, R::Caster, S::Default, C::Default, D::Left), // 50
    (O::GameObject, R::Source, S::Area, C::Default, D::None),   // 51
    (O::GameObject, R::Destination, S::Area, C::Default, D::None), // 52
    (O::Destination, R::Target, S::Default, C::Enemy, D::None), // 53
    (O::Unit, R::Caster, S::Cone, C::Enemy, D::Front),          // 54
    (O::Destination, R::Caster, S::Default, C::Default, D::None), // 55
    (O::Unit, R::Caster, S::Area, C::Raid, D::None),            // 56
    (O::Unit, R::Target, S::Default, C::Raid, D::None),         // 57
    (O::Unit, R::Caster, S::Nearby, C::Raid, D::None),          // 58
    (O::Unit, R::Caster, S::Cone, C::Ally, D::Front),           // 59
    (O::Unit, R::Caster, S::Cone, C::Entry, D::Front),          // 60
    (O::Unit, R::Target, S::Area, C::RaidClass, D::None),       // 61
    (O::Destination, R::Caster, S::Default, C::Default, D::None), // 62
    (O::Destination, R::Target, S::Default, C::Default, D::None), // 63
    (O::Destination, R::Target, S::Default, C::Default, D::Front), // 64
    (O::Destination, R::Target, S::Default, C::Default, D::Back), // 65
    (O::Destination, R::Target, S::Default, C::Default, D::Right), // 66
    (O::Destination, R::Target, S::Default, C::Default, D::Left), // 67
    (
        O::Destination,
        R::Target,
        S::Default,
        C::Default,
        D::FrontRight,
    ), // 68
    (
        O::Destination,
        R::Target,
        S::Default,
        C::Default,
        D::BackRight,
    ), // 69
    (
        O::Destination,
        R::Target,
        S::Default,
        C::Default,
        D::BackLeft,
    ), // 70
    (
        O::Destination,
        R::Target,
        S::Default,
        C::Default,
        D::FrontLeft,
    ), // 71
    (O::Destination, R::Caster, S::Default, C::Default, D::Random), // 72
    (O::Destination, R::Caster, S::Default, C::Default, D::Random), // 73
    (O::Destination, R::Target, S::Default, C::Default, D::Random), // 74
    (O::Destination, R::Target, S::Default, C::Default, D::Random), // 75
    (O::Destination, R::Caster, S::Channel, C::Default, D::None), // 76
    (O::Unit, R::Caster, S::Channel, C::Default, D::None),      // 77
    (
        O::Destination,
        R::Destination,
        S::Default,
        C::Default,
        D::Front,
    ), // 78
    (
        O::Destination,
        R::Destination,
        S::Default,
        C::Default,
        D::Back,
    ), // 79
    (
        O::Destination,
        R::Destination,
        S::Default,
        C::Default,
        D::Right,
    ), // 80
    (
        O::Destination,
        R::Destination,
        S::Default,
        C::Default,
        D::Left,
    ), // 81
    (
        O::Destination,
        R::Destination,
        S::Default,
        C::Default,
        D::FrontRight,
    ), // 82
    (
        O::Destination,
        R::Destination,
        S::Default,
        C::Default,
        D::BackRight,
    ), // 83
    (
        O::Destination,
        R::Destination,
        S::Default,
        C::Default,
        D::BackLeft,
    ), // 84
    (
        O::Destination,
        R::Destination,
        S::Default,
        C::Default,
        D::FrontLeft,
    ), // 85
    (
        O::Destination,
        R::Destination,
        S::Default,
        C::Default,
        D::Random,
    ), // 86
    (
        O::Destination,
        R::Destination,
        S::Default,
        C::Default,
        D::None,
    ), // 87
    (
        O::Destination,
        R::Destination,
        S::Default,
        C::Default,
        D::None,
    ), // 88
    (
        O::Destination,
        R::Destination,
        S::Trajectory,
        C::Default,
        D::None,
    ), // 89
    (O::Unit, R::Target, S::Default, C::Default, D::None),      // 90
    (
        O::Destination,
        R::Destination,
        S::Default,
        C::Default,
        D::Random,
    ), // 91
    (O::Unit, R::Caster, S::Default, C::Default, D::None),      // 92
    (O::Corpse, R::Source, S::Area, C::Enemy, D::None),         // 93
    (O::Unit, R::Caster, S::Default, C::Default, D::None),      // 94
    (O::Unit, R::Target, S::Default, C::Passenger, D::None),    // 95
    (O::Unit, R::Caster, S::Default, C::Default, D::None),      // 96
    (O::Unit, R::Caster, S::Default, C::Default, D::None),      // 97
    (O::Unit, R::Caster, S::Default, C::Default, D::None),      // 98
    (O::Unit, R::Caster, S::Default, C::Default, D::None),      // 99
    (O::Unit, R::Caster, S::Default, C::Default, D::None),      // 100
    (O::Unit, R::Caster, S::Default, C::Default, D::None),      // 101
    (O::Unit, R::Caster, S::Default, C::Default, D::None),      // 102
    (O::Unit, R::Caster, S::Default, C::Default, D::None),      // 103
    (O::Unit, R::Caster, S::Cone, C::Enemy, D::Front),          // 104
    (O::Unit, R::Caster, S::Area, C::Default, D::None),         // 105
    (O::Destination, R::Caster, S::Default, C::Default, D::None), // 106
    (O::Destination, R::Caster, S::Nearby, C::Entry, D::None),  // 107
    (O::GameObject, R::Caster, S::Cone, C::Enemy, D::Front),    // 108
    (O::GameObject, R::Caster, S::Cone, C::Ally, D::Front),     // 109
    (O::Unit, R::Caster, S::Cone, C::Entry, D::Front),          // 110
    (O::None, R::None, S::NotImplemented, C::Default, D::None), // 111
    (O::Destination, R::Caster, S::Default, C::Default, D::None), // 112
    (O::Destination, R::Target, S::Default, C::Default, D::None), // 113
    (O::None, R::None, S::NotImplemented, C::Default, D::None), // 114
    (O::Unit, R::Source, S::Area, C::Enemy, D::None),           // 115
    (O::UnitAndDestination, R::Last, S::Area, C::Enemy, D::None), // 116
    (O::None, R::None, S::NotImplemented, C::Default, D::None), // 117
    (O::Unit, R::Target, S::Area, C::Raid, D::None),            // 118
    (O::Corpse, R::Caster, S::Area, C::Raid, D::None),          // 119
    (O::Unit, R::Caster, S::Area, C::Summoned, D::None),        // 120
    (O::Corpse, R::Target, S::Default, C::Ally, D::None),       // 121
    (O::Unit, R::Caster, S::Area, C::Default, D::None),         // 122
    (O::Unit, R::Caster, S::Area, C::Default, D::None),         // 123
    (O::Unit, R::Caster, S::Default, C::Default, D::None),      // 124
    (O::Destination, R::Caster, S::Default, C::Default, D::None), // 125
    (O::Unit, R::None, S::NotImplemented, C::Default, D::None), // 126
    (
        O::Destination,
        R::None,
        S::NotImplemented,
        C::Default,
        D::None,
    ), // 127
    (O::Unit, R::Caster, S::Cone, C::Ally, D::Front),           // 128
    (O::Unit, R::Caster, S::Cone, C::Enemy, D::Front),          // 129
    (O::Unit, R::Caster, S::Cone, C::Default, D::Front),        // 130
    (O::Destination, R::Caster, S::Default, C::Default, D::None), // 131
    (O::Destination, R::Target, S::Default, C::Ally, D::None),  // 132
    (O::Unit, R::Destination, S::Line, C::Ally, D::None),       // 133
    (O::Unit, R::Destination, S::Line, C::Enemy, D::None),      // 134
    (O::Unit, R::Destination, S::Line, C::Default, D::None),    // 135
    (O::Unit, R::Caster, S::Cone, C::Ally, D::Front),           // 136
    (O::Destination, R::Caster, S::Default, C::Default, D::None), // 137
    (
        O::Destination,
        R::Destination,
        S::Default,
        C::Default,
        D::None,
    ), // 138
    (O::None, R::None, S::NotImplemented, C::Default, D::None), // 139
    (
        O::Destination,
        R::None,
        S::NotImplemented,
        C::Default,
        D::None,
    ), // 140
    (O::None, R::None, S::NotImplemented, C::Default, D::None), // 141
    (
        O::Destination,
        R::Caster,
        S::Nearby,
        C::Entry,
        D::FrontRight,
    ), // 142
    (O::None, R::None, S::NotImplemented, C::Default, D::None), // 143
    (O::None, R::None, S::NotImplemented, C::Default, D::None), // 144
    (O::None, R::None, S::NotImplemented, C::Default, D::None), // 145
    (O::None, R::None, S::NotImplemented, C::Default, D::None), // 146
    (O::None, R::None, S::NotImplemented, C::Default, D::None), // 147
    (
        O::Destination,
        R::Destination,
        S::Default,
        C::Default,
        D::None,
    ), // 148
    (O::Destination, R::Caster, S::Default, C::Default, D::Random), // 149
    (O::Unit, R::Caster, S::Default, C::Default, D::None),      // 150
    (O::Unit, R::Caster, S::Area, C::Enemy, D::None),           // 151
    (O::None, R::None, S::NotImplemented, C::Default, D::None), // 152
];
