//! Chat hyperlink structural validation.
//!
//! C++ reference:
//! `/src/server/game/Chat/Hyperlinks.cpp`
//! - `Trinity::Hyperlinks::CheckAllLinks`
//! - `Trinity::Hyperlinks::ParseSingleHyperlink`

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HyperlinkInfoLikeCpp<'a> {
    pub tail: &'a str,
    pub color: u32,
    pub tag: &'a str,
    pub data: &'a str,
    pub text: &'a str,
}

pub fn check_all_links_shape_like_cpp(mut text: &str) -> bool {
    let mut pos = 0;
    while let Some(relative) = text[pos..].find('|') {
        pos += relative + 1;
        let Some(next) = text.as_bytes().get(pos).copied() else {
            return false;
        };
        if matches!(next, b'H' | b'h' | b'c' | b'r' | b'|') {
            pos += 1;
        } else {
            return false;
        }
    }

    while let Some(pos) = text.find('|') {
        if text.as_bytes().get(pos + 1) == Some(&b'|') {
            text = &text[pos + 2..];
            continue;
        }

        let Some(info) = parse_single_hyperlink_like_cpp(&text[pos..]) else {
            return false;
        };
        if !is_known_link_tag_like_cpp(info.tag) {
            return false;
        }
        text = info.tail;
    }

    true
}

pub fn parse_single_hyperlink_like_cpp(text: &str) -> Option<HyperlinkInfoLikeCpp<'_>> {
    let rest = text.strip_prefix("|c")?;
    if rest.len() < 8 {
        return None;
    }

    let (color_text, rest) = rest.split_at(8);
    let mut color = 0u32;
    for byte in color_text.bytes() {
        let hex = to_hex_like_cpp(byte)?;
        color = (color << 4) | u32::from(hex & 0x0f);
    }

    let mut rest = rest.strip_prefix("|H")?;
    let delim = rest.find('|')?;
    let mut tag = &rest[..delim];
    rest = &rest[delim + 1..];

    let mut data = "";
    if let Some(data_start) = tag.find(':') {
        data = &tag[data_start + 1..];
        tag = &tag[..data_start];
    }

    let rest = rest.strip_prefix('h')?;
    let end = rest.find('|')?;
    if !rest[end..].starts_with("|h|r") {
        return None;
    }
    if !rest.starts_with('[') || !rest[..end].ends_with(']') {
        return None;
    }

    let text = &rest[1..end - 1];
    let tail = &rest[end + 4..];
    Some(HyperlinkInfoLikeCpp {
        tail,
        color,
        tag,
        data,
        text,
    })
}

fn to_hex_like_cpp(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0' + 0x10),
        b'a'..=b'f' => Some(byte - b'a' + 0x1a),
        _ => None,
    }
}

fn is_known_link_tag_like_cpp(tag: &str) -> bool {
    matches!(
        tag,
        "achievement"
            | "api"
            | "apower"
            | "area"
            | "areatrigger"
            | "azessence"
            | "battlepet"
            | "battlePetAbil"
            | "clubFinder"
            | "clubTicket"
            | "creature"
            | "creature_entry"
            | "currency"
            | "dungeonScore"
            | "enchant"
            | "gameevent"
            | "gameobject"
            | "gameobject_entry"
            | "garrfollower"
            | "garrfollowerability"
            | "garrmission"
            | "instancelock"
            | "item"
            | "itemset"
            | "journal"
            | "keystone"
            | "mount"
            | "outfit"
            | "player"
            | "pvptal"
            | "quest"
            | "skill"
            | "spell"
            | "talent"
            | "talentbuild"
            | "taxinode"
            | "tele"
            | "title"
            | "trade"
            | "transmogappearance"
            | "transmogset"
            | "worldmap"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_and_escaped_pipe_are_valid_like_cpp() {
        assert!(check_all_links_shape_like_cpp("hello"));
        assert!(check_all_links_shape_like_cpp("hello || world"));
    }

    #[test]
    fn disallows_unknown_control_sequences_like_cpp() {
        assert!(!check_all_links_shape_like_cpp("bad |x control"));
        assert!(!check_all_links_shape_like_cpp("bad trailing |"));
    }

    #[test]
    fn parses_well_formed_item_link_like_cpp() {
        let input = "|cff0070dd|Hitem:12345:0:0:0|h[Example Item]|h|r tail";
        let parsed = parse_single_hyperlink_like_cpp(input).expect("item link parsed");

        assert_eq!(parsed.color, 0xff0070dd);
        assert_eq!(parsed.tag, "item");
        assert_eq!(parsed.data, "12345:0:0:0");
        assert_eq!(parsed.text, "Example Item");
        assert_eq!(parsed.tail, " tail");
        assert!(check_all_links_shape_like_cpp(input));
    }

    #[test]
    fn rejects_malformed_link_shape_like_cpp() {
        assert!(!check_all_links_shape_like_cpp(
            "|cff0070dd|Hitem:12345|hExample Item|h|r"
        ));
        assert!(!check_all_links_shape_like_cpp(
            "|cff0070dd|Hitem:12345|h[Example Item]|r"
        ));
        assert!(!check_all_links_shape_like_cpp(
            "|cFF0070DD|Hitem:12345|h[Example Item]|h|r"
        ));
    }

    #[test]
    fn rejects_unknown_link_tags_until_semantic_validator_exists_like_cpp() {
        assert!(!check_all_links_shape_like_cpp(
            "|cff0070dd|Hnotaport:12345|h[Example]|h|r"
        ));
    }
}
