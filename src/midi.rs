use keyboard_types::Code as PhysicalKeyCode;
use truce::prelude::{EventBody, EventList};

#[must_use]
pub const fn pad_for_note(note: u8) -> Option<usize> {
    match note {
        60..=75 => Some((note - 60) as usize),
        57..=59 => Some((note - 48) as usize),
        81..=83 => Some((note - 72) as usize),
        _ => None,
    }
}

#[must_use]
pub const fn pad_for_physical_key(code: PhysicalKeyCode) -> Option<usize> {
    use PhysicalKeyCode::{
        Digit1, Digit2, Digit3, Digit4, KeyA, KeyC, KeyD, KeyE, KeyF, KeyQ, KeyR, KeyS, KeyV, KeyW,
        KeyX, KeyZ,
    };

    match code {
        Digit1 => Some(0),
        Digit2 => Some(1),
        Digit3 => Some(2),
        Digit4 => Some(3),
        KeyQ => Some(4),
        KeyW => Some(5),
        KeyE => Some(6),
        KeyR => Some(7),
        KeyA => Some(8),
        KeyS => Some(9),
        KeyD => Some(10),
        KeyF => Some(11),
        KeyZ => Some(12),
        KeyX => Some(13),
        KeyC => Some(14),
        KeyV => Some(15),
        _ => None,
    }
}

pub fn apply_events(held_pads_by_channel: &mut [u16; 16], events: &EventList) -> Option<usize> {
    let mut last_pressed_pad = None;
    for event in events.iter() {
        let (channel, note, pressed) = match event.body {
            EventBody::NoteOn { channel, note, .. } | EventBody::NoteOn2 { channel, note, .. } => {
                (channel, note, true)
            }
            EventBody::NoteOff { channel, note, .. }
            | EventBody::NoteOff2 { channel, note, .. } => (channel, note, false),
            _ => continue,
        };

        let Some(pad) = pad_for_note(note) else {
            continue;
        };
        let channel = usize::from(channel.min(15));
        let bit = 1_u16 << pad;
        if pressed {
            held_pads_by_channel[channel] |= bit;
            last_pressed_pad = Some(pad);
        } else {
            held_pads_by_channel[channel] &= !bit;
        }
    }
    last_pressed_pad
}

#[cfg(test)]
mod tests {
    use super::{apply_events, pad_for_note, pad_for_physical_key};
    use keyboard_types::Code as PhysicalKeyCode;
    use truce::prelude::{Event, EventBody, EventList};

    #[test]
    fn maps_main_note_range_to_all_pads() {
        for note in 60..=75 {
            assert_eq!(pad_for_note(note), Some(usize::from(note - 60)));
        }
    }

    #[test]
    fn maps_pitch_alias_octaves() {
        assert_eq!(pad_for_note(57), Some(9));
        assert_eq!(pad_for_note(59), Some(11));
        assert_eq!(pad_for_note(81), Some(9));
        assert_eq!(pad_for_note(83), Some(11));
        assert_eq!(pad_for_note(56), None);
    }

    #[test]
    fn maps_physical_keyboard_grid_to_all_pads() {
        let codes = [
            PhysicalKeyCode::Digit1,
            PhysicalKeyCode::Digit2,
            PhysicalKeyCode::Digit3,
            PhysicalKeyCode::Digit4,
            PhysicalKeyCode::KeyQ,
            PhysicalKeyCode::KeyW,
            PhysicalKeyCode::KeyE,
            PhysicalKeyCode::KeyR,
            PhysicalKeyCode::KeyA,
            PhysicalKeyCode::KeyS,
            PhysicalKeyCode::KeyD,
            PhysicalKeyCode::KeyF,
            PhysicalKeyCode::KeyZ,
            PhysicalKeyCode::KeyX,
            PhysicalKeyCode::KeyC,
            PhysicalKeyCode::KeyV,
        ];
        for (pad, code) in codes.into_iter().enumerate() {
            assert_eq!(pad_for_physical_key(code), Some(pad));
        }
        assert_eq!(pad_for_physical_key(PhysicalKeyCode::Digit5), None);
    }

    #[test]
    fn reports_the_last_pad_pressed_in_the_block() {
        let mut events = EventList::with_capacity(3);
        events.push(Event::new(
            0,
            EventBody::NoteOn {
                group: 0,
                channel: 2,
                note: 64,
                velocity: 100,
            },
        ));
        events.push(Event::new(
            4,
            EventBody::NoteOff {
                group: 0,
                channel: 2,
                note: 64,
                velocity: 0,
            },
        ));
        events.push(Event::new(
            8,
            EventBody::NoteOn {
                group: 0,
                channel: 8,
                note: 71,
                velocity: 100,
            },
        ));

        let mut held = [0_u16; 16];
        assert_eq!(apply_events(&mut held, &events), Some(11));
        assert_eq!(held[2], 0);
        assert_eq!(held[8], 1 << 11);
    }
}
