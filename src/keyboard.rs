//! Native physical-key identities translated to the portable kit protocol.
use buffer_uppercut_kit::KeyId;
use keyboard_types::Code;

pub(crate) fn key_for_code(code: Code) -> Option<KeyId> {
    let id = match code {
        Code::KeyA => 1,
        Code::KeyB => 2,
        Code::KeyC => 3,
        Code::KeyD => 4,
        Code::KeyE => 5,
        Code::KeyF => 6,
        Code::KeyG => 7,
        Code::KeyH => 8,
        Code::KeyI => 9,
        Code::KeyJ => 10,
        Code::KeyK => 11,
        Code::KeyL => 12,
        Code::KeyM => 13,
        Code::KeyN => 14,
        Code::KeyO => 15,
        Code::KeyP => 16,
        Code::KeyQ => 17,
        Code::KeyR => 18,
        Code::KeyS => 19,
        Code::KeyT => 20,
        Code::KeyU => 21,
        Code::KeyV => 22,
        Code::KeyW => 23,
        Code::KeyX => 24,
        Code::KeyY => 25,
        Code::KeyZ => 26,
        Code::Digit0 => 27,
        Code::Digit1 => 28,
        Code::Digit2 => 29,
        Code::Digit3 => 30,
        Code::Digit4 => 31,
        Code::Digit5 => 32,
        Code::Digit6 => 33,
        Code::Digit7 => 34,
        Code::Digit8 => 35,
        Code::Digit9 => 36,
        Code::Minus => 37,
        Code::Equal => 38,
        Code::BracketLeft => 39,
        Code::BracketRight => 40,
        Code::Backslash => 41,
        Code::Semicolon => 42,
        Code::Quote => 43,
        Code::Backquote => 44,
        Code::Comma => 45,
        Code::Period => 46,
        Code::Slash => 47,
        _ => return None,
    };
    KeyId::from_id(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn physical_positions_match_default_map() {
        let map = buffer_uppercut_kit::KeyMap::default();
        for (pad, code) in [
            Code::Digit1,
            Code::Digit2,
            Code::Digit3,
            Code::Digit4,
            Code::KeyQ,
            Code::KeyW,
            Code::KeyE,
            Code::KeyR,
            Code::KeyA,
            Code::KeyS,
            Code::KeyD,
            Code::KeyF,
            Code::KeyZ,
            Code::KeyX,
            Code::KeyC,
            Code::KeyV,
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(map.pad_for(key_for_code(code).unwrap()), Some(pad));
        }
        assert_eq!(key_for_code(Code::Space), None);
        assert_eq!(key_for_code(Code::ShiftLeft), None);
    }
}
