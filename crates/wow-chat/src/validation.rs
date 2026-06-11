//! Chat text validation shared by world-session handlers.
//!
//! C++ reference: `WorldSession::HandleChatMessage` uses the local
//! `ValidateMessage(Player const*, std::string&)` helper in
//! `/src/server/game/Handlers/ChatHandler.cpp`.

pub fn validate_message_like_cpp(text: &mut String, collapse_multiple_spaces: bool) -> bool {
    if let Some(pos) = text.find(['\n', '\r']) {
        if pos == 0 {
            return false;
        }
        text.truncate(pos);
    }

    if text.bytes().any(is_nasty_like_cpp) {
        return false;
    }

    if collapse_multiple_spaces {
        collapse_spaces_like_cpp(text);
    }

    true
}

fn is_nasty_like_cpp(byte: u8) -> bool {
    byte != b'\t' && byte <= 0x1f
}

fn collapse_spaces_like_cpp(text: &mut String) {
    let mut collapsed = String::with_capacity(text.len());
    let mut previous_space = false;
    for ch in text.chars() {
        if ch == ' ' {
            if previous_space {
                continue;
            }
            previous_space = true;
        } else {
            previous_space = false;
        }
        collapsed.push(ch);
    }
    *text = collapsed;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newline_at_start_rejects_like_cpp() {
        let mut text = "\nhidden".to_string();
        assert!(!validate_message_like_cpp(&mut text, false));
    }

    #[test]
    fn newline_after_text_truncates_like_cpp() {
        let mut text = "visible\nhidden".to_string();
        assert!(validate_message_like_cpp(&mut text, false));
        assert_eq!(text, "visible");
    }

    #[test]
    fn rejects_ascii_control_except_tab_like_cpp() {
        let mut bad = "bad\u{1f}".to_string();
        assert!(!validate_message_like_cpp(&mut bad, false));

        let mut tab = "ok\ttext".to_string();
        assert!(validate_message_like_cpp(&mut tab, false));
        assert_eq!(tab, "ok\ttext");
    }

    #[test]
    fn optionally_collapses_multiple_spaces_like_cpp() {
        let mut text = "a  b   c".to_string();
        assert!(validate_message_like_cpp(&mut text, true));
        assert_eq!(text, "a b c");
    }
}
