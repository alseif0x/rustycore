//! Modern Logon admission and device timezone interpretation.
//!
//! TC 6ebe044c: Shared::Authentication::HandleLogon, ClientBuild::Platform::IsValid
//! and src/common/Time/Timezone.cpp::{InitTimezoneHashDb,GetOffsetByHash}.

use crate::rpc::session::RpcStatusError;
use wow_proto::status;

pub(super) const WOW_TITLE_ID: u32 = 0x0057_6F57;

pub(super) fn validate(title: u32, platform: &str, locale: &str) -> Result<(), RpcStatusError> {
    if title != WOW_TITLE_ID {
        return Err(RpcStatusError::new(status::ERROR_BAD_PROGRAM));
    }
    if !matches!(platform, "Win" | "Wn64" | "WinA" | "Mac" | "Mc64" | "MacA") {
        return Err(RpcStatusError::new(status::ERROR_BAD_PLATFORM));
    }
    if !super::super::authentication::is_valid_locale_like_cpp(locale) {
        return Err(RpcStatusError::new(status::ERROR_BAD_LOCALE));
    }
    Ok(())
}

// The Linux TC table's exact offset domain. Hashes are FNV-1a of signed decimal
// minutes, not the raw offset itself and not an OS-local timezone lookup.
const OFFSETS: &[i32] = &[
    -720, -690, -660, -640, -630, -600, -570, -540, -510, -480, -420, -360, -300, -240, -225, -210,
    -180, -120, -60, -44, 0, 60, 120, 180, 210, 240, 270, 300, 330, 360, 390, 420, 450, 480, 525,
    540, 570, 600, 660, 690, 720, 765, 780, 840,
];

fn offset_hash(offset: i32) -> u32 {
    offset
        .to_string()
        .bytes()
        .fold(0x811C_9DC5_u32, |hash, byte| {
            (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
        })
}

pub(super) fn timezone(device_id: Option<&str>) -> i32 {
    let hash = device_id
        .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok())
        .and_then(|value| value.get("UTCO").and_then(serde_json::Value::as_u64))
        .and_then(|value| u32::try_from(value).ok());
    OFFSETS
        .iter()
        .copied()
        .find(|&offset| Some(offset_hash(offset)) == hash)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admission_preserves_program_platform_locale_order() {
        for platform in ["Win", "Wn64", "WinA", "Mac", "Mc64", "MacA"] {
            assert!(validate(WOW_TITLE_ID, platform, "esES").is_ok());
        }
        assert_eq!(
            validate(0, "bad", "bad").unwrap_err().status(),
            status::ERROR_BAD_PROGRAM
        );
        assert_eq!(
            validate(WOW_TITLE_ID, "bad", "bad").unwrap_err().status(),
            status::ERROR_BAD_PLATFORM
        );
        assert_eq!(
            validate(WOW_TITLE_ID, "Wn64", "bad").unwrap_err().status(),
            status::ERROR_BAD_LOCALE
        );
    }

    #[test]
    fn modern_device_timezone_is_a_hash_not_minutes() {
        for (hash, offset) in [
            (0xAADC_2D37, -720),
            (0x47CE_5170, -44),
            (0x350C_A8AF, 0),
            (0x7338_64AE, 120),
            (0xC595_85BB, 840),
        ] {
            assert_eq!(offset_hash(offset), hash);
            assert_eq!(timezone(Some(&format!("{{\"UTCO\":{hash}}}"))), offset);
        }
        for device in [
            None,
            Some("bad"),
            Some("{}"),
            Some("[]"),
            Some("null"),
            Some("{\"UTCO\":120}"),
            Some("{\"UTCO\":-1}"),
            Some("{\"UTCO\":4294967296}"),
            Some("{\"UTCO\":\"1933075630\"}"),
            Some("{\"UTCO\":1.5}"),
        ] {
            assert_eq!(timezone(device), 0);
        }
    }
}
