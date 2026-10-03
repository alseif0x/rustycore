//! Payload-only Forever character-create request decoding.
//!
//! The field order is pinned to `CharacterPackets.cpp:487-516` at
//! `02245dcd245e7433e524577656177723d3e4992e`.  That source labels the
//! captured bit layout Classic 1.60.1.70009; a native 1.60.1.70170 capture
//! confirms the same 106-byte shape for a 9-byte name, 7-byte surname, and
//! nine customization pairs.  This module deliberately does not register an
//! opcode, admit a character, or write a response.

use std::convert::TryFrom;

use thiserror::Error;

use crate::{PacketError, WorldPacket};

const NAME_LENGTH_BITS: u32 = 6;
const UNKNOWN_FLAGS_BITS: u32 = 2;
const CUSTOMIZATION_PAIR_BYTES: usize = 8;
// CharacterPackets.h:57-65 uses Array<ChrCustomizationChoice, 250>.
const MAX_CUSTOMIZATIONS: usize = 250;

/// One source `UF::ChrCustomizationChoice` pair.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CustomizationChoice {
    pub option_id: u32,
    pub choice_id: u32,
}

/// Payload fields from `WorldPackets::Character::CreateCharacter`.
///
/// `name` and `surname` intentionally have no public fields or `Debug`
/// implementation: they are user-provided personal data.  The remaining
/// fields are transport data only; admission and DB validation belong to the
/// world session owner.
#[derive(Clone, PartialEq, Eq)]
pub struct CharacterCreatePayload {
    name: String,
    surname: String,
    pub template_set: Option<i32>,
    pub is_trial_boost: bool,
    pub use_npe: bool,
    pub hardcore_self_found: bool,
    /// The two source bits consumed by `ReadBits(2)` and otherwise ignored.
    pub unknown_flags: u8,
    pub race: u8,
    pub class: u8,
    pub sex: u8,
    pub timerunning_season_id: i32,
    /// The unannotated int32 skipped by the source reader's old layout.
    pub unknown_i32: i32,
    pub customizations: Vec<CustomizationChoice>,
}

impl CharacterCreatePayload {
    /// Decode a payload only; no opcode or Forever transport header is read.
    pub fn decode(payload: &[u8]) -> Result<Self, CharacterCreateError> {
        let mut packet = WorldPacket::from_bytes(payload);

        // CharacterPackets.cpp:491-500.  ResetBitPos aligns the following
        // byte fields; the two unknown bits are retained without validation.
        let name_length = packet.read_bits(NAME_LENGTH_BITS)? as usize;
        let has_template_set = packet.read_bit()?;
        let is_trial_boost = packet.read_bit()?;
        let use_npe = packet.read_bit()?;
        let hardcore_self_found = packet.read_bit()?;
        let unknown_flags = packet.read_bits(UNKNOWN_FLAGS_BITS)? as u8;
        let surname_length = packet.read_bits(NAME_LENGTH_BITS)? as usize;
        packet.reset_bits();

        let race = packet.read_uint8()?;
        let class = packet.read_uint8()?;
        let sex = packet.read_uint8()?;
        let customization_count = usize::try_from(packet.read_uint32()?)
            .map_err(|_| CharacterCreateError::CustomizationCountTooLarge)?;
        let unknown_i32 = packet.read_int32()?;
        let timerunning_season_id = packet.read_int32()?;

        // WorldPacket checks remaining bytes before allocating each String;
        // the six-bit lengths also cap each allocation at 63 bytes.
        let name = packet.read_string(name_length)?;
        let surname = packet.read_string(surname_length)?;
        let template_set = if has_template_set {
            Some(packet.read_int32()?)
        } else {
            None
        };

        // Check the complete pair region before reserving.  This gives the
        // caller a deterministic truncation/trailing error and prevents a
        // hostile uint32 count from driving allocation.
        if customization_count > MAX_CUSTOMIZATIONS {
            return Err(CharacterCreateError::CustomizationCountExceedsSourceLimit {
                count: customization_count,
                max: MAX_CUSTOMIZATIONS,
            });
        }
        let required_pair_bytes = customization_count
            .checked_mul(CUSTOMIZATION_PAIR_BYTES)
            .ok_or(CharacterCreateError::CustomizationCountTooLarge)?;
        let available_pair_bytes = packet.remaining();
        if available_pair_bytes < required_pair_bytes {
            return Err(CharacterCreateError::CustomizationBytes {
                count: customization_count,
                required: required_pair_bytes,
                available: available_pair_bytes,
            });
        }
        if available_pair_bytes > required_pair_bytes {
            return Err(CharacterCreateError::TrailingBytes {
                count: available_pair_bytes - required_pair_bytes,
            });
        }

        let mut customizations = Vec::new();
        customizations
            .try_reserve_exact(customization_count)
            .map_err(|_| CharacterCreateError::CustomizationCountTooLarge)?;
        for _ in 0..customization_count {
            customizations.push(CustomizationChoice {
                option_id: packet.read_uint32()?,
                choice_id: packet.read_uint32()?,
            });
        }

        // CharacterPackets.cpp:515 uses std::ranges::sort by option ID.  The
        // stable Rust sort preserves duplicate options for later admission
        // validation instead of silently discarding them here.
        customizations.sort_by_key(|choice| choice.option_id);

        Ok(Self {
            name,
            surname,
            template_set,
            is_trial_boost,
            use_npe,
            hardcore_self_found,
            unknown_flags,
            race,
            class,
            sex,
            timerunning_season_id,
            unknown_i32,
            customizations,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn surname(&self) -> &str {
        &self.surname
    }
}

/// Errors raised while decoding the bounded transport payload.
#[derive(Debug, Error)]
pub enum CharacterCreateError {
    #[error(transparent)]
    Packet(#[from] PacketError),
    #[error("character customization count cannot be represented safely")]
    CustomizationCountTooLarge,
    #[error("character customization count {count} exceeds source limit {max}")]
    CustomizationCountExceedsSourceLimit { count: usize, max: usize },
    #[error(
        "character customization count {count} requires {required} bytes, but only {available} remain"
    )]
    CustomizationBytes {
        count: usize,
        required: usize,
        available: usize,
    },
    #[error("character-create payload has {count} trailing bytes")]
    TrailingBytes { count: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(
        name: &str,
        surname: &str,
        template_set: Option<i32>,
        trial: bool,
        npe: bool,
        hardcore: bool,
        unknown_flags: u8,
        unknown_i32: i32,
        season: i32,
        pairs: &[(u32, u32)],
    ) -> Vec<u8> {
        let mut packet = WorldPacket::new_empty();
        packet.write_bits(name.len() as u32, NAME_LENGTH_BITS);
        packet.write_bit(template_set.is_some());
        packet.write_bit(trial);
        packet.write_bit(npe);
        packet.write_bit(hardcore);
        packet.write_bits(u32::from(unknown_flags & 0b11), UNKNOWN_FLAGS_BITS);
        packet.write_bits(surname.len() as u32, NAME_LENGTH_BITS);
        packet.flush_bits();
        packet.write_uint8(1);
        packet.write_uint8(2);
        packet.write_uint8(0);
        packet.write_uint32(pairs.len() as u32);
        packet.write_int32(unknown_i32);
        packet.write_int32(season);
        packet.write_string(name);
        packet.write_string(surname);
        if let Some(template) = template_set {
            packet.write_int32(template);
        }
        for (option_id, choice_id) in pairs {
            packet.write_uint32(*option_id);
            packet.write_uint32(*choice_id);
        }
        packet.into_data()
    }

    #[test]
    fn native_shape_sorts_pairs_and_preserves_duplicate_options() {
        let bytes = payload(
            "Character",
            "Surname",
            None,
            false,
            false,
            false,
            0,
            -1,
            0,
            &[
                (2, 20),
                (1, 10),
                (2, 21),
                (9, 90),
                (3, 30),
                (8, 80),
                (4, 40),
                (7, 70),
                (6, 60),
            ],
        );
        assert_eq!(bytes.len(), 106);

        let decoded = CharacterCreatePayload::decode(&bytes).unwrap();
        assert_eq!(decoded.name(), "Character");
        assert_eq!(decoded.surname(), "Surname");
        assert_eq!(decoded.unknown_i32, -1);
        assert_eq!(decoded.customizations[0].option_id, 1);
        assert_eq!(decoded.customizations[1].option_id, 2);
        assert_eq!(decoded.customizations[1].choice_id, 20);
        assert_eq!(decoded.customizations[2].option_id, 2);
        assert_eq!(decoded.customizations[2].choice_id, 21);
    }

    #[test]
    fn source_70009_default_example_has_no_template_count() {
        // CharacterPackets.cpp:491-492 documents this captured prefix:
        // `08 00 80 | 5f 0b 01 | 12000000 ffffffff 00000000 | Df Df`.
        // Complete its declared 18 pairs with synthetic zero IDs.
        let mut bytes = vec![
            0x08, 0x00, 0x80, 0x5f, 0x0b, 0x01, 0x12, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff,
            0x00, 0x00, 0x00, 0x00, b'D', b'f', b'D', b'f',
        ];
        bytes.extend(std::iter::repeat_n(0, 18 * CUSTOMIZATION_PAIR_BYTES));

        let decoded = CharacterCreatePayload::decode(&bytes).unwrap();
        assert_eq!(decoded.name(), "Df");
        assert_eq!(decoded.surname(), "Df");
        assert_eq!(decoded.template_set, None);
        assert_eq!((decoded.race, decoded.class, decoded.sex), (95, 11, 1));
        assert_eq!(decoded.customizations.len(), 18);
        assert_eq!(decoded.unknown_i32, -1);
    }

    #[test]
    fn template_flags_unknown_bits_and_utf8_names_are_retained() {
        let decoded = CharacterCreatePayload::decode(&payload(
            "Málaga",
            "Łódź",
            Some(-7),
            true,
            true,
            true,
            0b11,
            -1,
            17,
            &[(4, 5)],
        ))
        .unwrap();
        assert_eq!(decoded.name(), "Málaga");
        assert_eq!(decoded.surname(), "Łódź");
        assert_eq!(decoded.template_set, Some(-7));
        assert!(decoded.is_trial_boost && decoded.use_npe && decoded.hardcore_self_found);
        assert_eq!(decoded.unknown_flags, 0b11);
        assert_eq!(decoded.timerunning_season_id, 17);
    }

    #[test]
    fn every_truncated_prefix_is_rejected() {
        let valid = payload(
            "Character",
            "Surname",
            None,
            false,
            false,
            false,
            0,
            -1,
            0,
            &[(2, 20), (1, 10)],
        );
        for length in 0..valid.len() {
            assert!(
                CharacterCreatePayload::decode(&valid[..length]).is_err(),
                "truncated prefix {length} unexpectedly decoded"
            );
        }
    }

    #[test]
    fn rejects_trailing_bytes_and_huge_count_before_reserve() {
        let mut trailing = payload(
            "Character",
            "Surname",
            None,
            false,
            false,
            false,
            0,
            -1,
            0,
            &[],
        );
        trailing.push(0xEE);
        assert!(matches!(
            CharacterCreatePayload::decode(&trailing),
            Err(CharacterCreateError::TrailingBytes { count: 1 })
        ));

        let mut huge = payload(
            "Character",
            "Surname",
            None,
            false,
            false,
            false,
            0,
            -1,
            0,
            &[],
        );
        huge[6..10].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(
            CharacterCreatePayload::decode(&huge),
            Err(CharacterCreateError::CustomizationCountExceedsSourceLimit { .. })
        ));
    }

    #[test]
    fn enforces_source_customization_capacity_after_complete_payload_check() {
        let pairs = (0..MAX_CUSTOMIZATIONS as u32)
            .map(|id| (id, id + 1))
            .collect::<Vec<_>>();
        let bytes = payload(
            "Character",
            "Surname",
            None,
            false,
            false,
            false,
            0,
            -1,
            0,
            &pairs,
        );
        let decoded = CharacterCreatePayload::decode(&bytes).unwrap();
        assert_eq!(decoded.customizations.len(), MAX_CUSTOMIZATIONS);

        let mut too_many = pairs;
        too_many.push((MAX_CUSTOMIZATIONS as u32, 0));
        let bytes = payload(
            "Character",
            "Surname",
            None,
            false,
            false,
            false,
            0,
            -1,
            0,
            &too_many,
        );
        assert!(matches!(
            CharacterCreatePayload::decode(&bytes),
            Err(CharacterCreateError::CustomizationCountExceedsSourceLimit {
                count: 251,
                max: 250,
            })
        ));
    }

    #[test]
    fn rejects_invalid_utf8_and_declared_name_without_bytes() {
        let mut invalid = payload(
            "Character",
            "Surname",
            None,
            false,
            false,
            false,
            0,
            -1,
            0,
            &[],
        );
        invalid[18] = 0xFF;
        assert!(matches!(
            CharacterCreatePayload::decode(&invalid),
            Err(CharacterCreateError::Packet(PacketError::StringError(_)))
        ));

        let mut overlong = payload("", "", None, false, false, false, 0, -1, 0, &[]);
        // Name length is the first six MSB-first bits.  No 63-byte name is
        // present, so read_string must reject before allocating it.
        overlong[0] = 63 << 2;
        assert!(matches!(
            CharacterCreatePayload::decode(&overlong),
            Err(CharacterCreateError::Packet(
                PacketError::ReadPastEnd { .. }
            ))
        ));
    }
}
