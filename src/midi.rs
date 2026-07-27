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

pub fn apply_events(held_pads_by_channel: &mut [u16; 16], events: &EventList) {
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
        } else {
            held_pads_by_channel[channel] &= !bit;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::pad_for_note;

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
}
