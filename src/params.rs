use std::sync::atomic::{AtomicU16, AtomicU32, Ordering};

use truce::prelude::*;

pub const NUM_PADS: usize = 16;
pub const NUM_MACROS: usize = 7;
pub const PAD_PARAMETER_STRIDE: u32 = 9;
pub const PARAM_PERFORMANCE_PITCH_ID: u32 = 0;

#[derive(ParamEnum)]
pub enum EffectType {
    Off,
    #[name = "Beat Repeat"]
    BeatRepeat,
    Reverse,
    #[name = "Tape Stop"]
    TapeStop,
    Gate,
    #[name = "Pitch Down"]
    PitchDown,
    #[name = "Pitch Reset"]
    PitchReset,
    #[name = "Pitch Up"]
    PitchUp,
    #[name = "Low Band"]
    LowBand,
    #[name = "Mid Band"]
    MidBand,
    #[name = "High Band"]
    HighBand,
    LoFi,
}

macro_rules! pad_params {
    (
        $struct_name:ident, $group:literal,
        $trigger_name:literal, $type_name:literal,
        [$control_1:literal, $control_2:literal, $control_3:literal,
         $control_4:literal, $control_5:literal, $control_6:literal, $control_7:literal],
        $type_default:tt,
        [$default_1:tt, $default_2:tt, $default_3:tt,
         $default_4:tt, $default_5:tt, $default_6:tt, $default_7:tt]
    ) => {
        #[derive(Params)]
        pub struct $struct_name {
            #[param(id = 0, name = $trigger_name, group = $group, default = false)]
            trigger: BoolParam,
            #[param(id = 1, name = $type_name, group = $group, default = $type_default)]
            effect_type: EnumParam<EffectType>,
            #[param(id = 2, name = $control_1, group = $group, range = "linear(0, 1)", default = $default_1)]
            control_1: FloatParam,
            #[param(id = 3, name = $control_2, group = $group, range = "linear(0, 1)", default = $default_2)]
            control_2: FloatParam,
            #[param(id = 4, name = $control_3, group = $group, range = "linear(0, 1)", default = $default_3)]
            control_3: FloatParam,
            #[param(id = 5, name = $control_4, group = $group, range = "linear(0, 1)", default = $default_4)]
            control_4: FloatParam,
            #[param(id = 6, name = $control_5, group = $group, range = "linear(0, 1)", default = $default_5)]
            control_5: FloatParam,
            #[param(id = 7, name = $control_6, group = $group, range = "linear(0, 1)", default = $default_6)]
            control_6: FloatParam,
            #[param(id = 8, name = $control_7, group = $group, range = "linear(0, 1)", default = $default_7)]
            control_7: FloatParam,
        }
    };
}

pad_params!(
    Pad01Params,
    "Pad 1",
    "Pad 1 Trigger",
    "Pad 1 Type",
    [
        "Pad 1 Control 1",
        "Pad 1 Control 2",
        "Pad 1 Control 3",
        "Pad 1 Control 4",
        "Pad 1 Control 5",
        "Pad 1 Control 6",
        "Pad 1 Control 7"
    ],
    1,
    [0.25, 0.0, 1.0, 1.0, 0.5, 0.0, 0.0]
);
pad_params!(
    Pad02Params,
    "Pad 2",
    "Pad 2 Trigger",
    "Pad 2 Type",
    [
        "Pad 2 Control 1",
        "Pad 2 Control 2",
        "Pad 2 Control 3",
        "Pad 2 Control 4",
        "Pad 2 Control 5",
        "Pad 2 Control 6",
        "Pad 2 Control 7"
    ],
    1,
    [0.375, 0.0, 1.0, 1.0, 0.5, 0.0, 0.0]
);
pad_params!(
    Pad03Params,
    "Pad 3",
    "Pad 3 Trigger",
    "Pad 3 Type",
    [
        "Pad 3 Control 1",
        "Pad 3 Control 2",
        "Pad 3 Control 3",
        "Pad 3 Control 4",
        "Pad 3 Control 5",
        "Pad 3 Control 6",
        "Pad 3 Control 7"
    ],
    1,
    [0.5, 0.0, 1.0, 1.0, 0.5, 0.0, 0.0]
);
pad_params!(
    Pad04Params,
    "Pad 4",
    "Pad 4 Trigger",
    "Pad 4 Type",
    [
        "Pad 4 Control 1",
        "Pad 4 Control 2",
        "Pad 4 Control 3",
        "Pad 4 Control 4",
        "Pad 4 Control 5",
        "Pad 4 Control 6",
        "Pad 4 Control 7"
    ],
    1,
    [0.125, 0.0, 1.0, 1.0, 0.5, 0.0, 0.0]
);
pad_params!(
    Pad05Params,
    "Pad 5",
    "Pad 5 Trigger",
    "Pad 5 Type",
    [
        "Pad 5 Control 1",
        "Pad 5 Control 2",
        "Pad 5 Control 3",
        "Pad 5 Control 4",
        "Pad 5 Control 5",
        "Pad 5 Control 6",
        "Pad 5 Control 7"
    ],
    1,
    [0.25, 0.6666666666666666, 1.0, 1.0, 0.5, 0.0, 0.0]
);
pad_params!(
    Pad06Params,
    "Pad 6",
    "Pad 6 Trigger",
    "Pad 6 Type",
    [
        "Pad 6 Control 1",
        "Pad 6 Control 2",
        "Pad 6 Control 3",
        "Pad 6 Control 4",
        "Pad 6 Control 5",
        "Pad 6 Control 6",
        "Pad 6 Control 7"
    ],
    1,
    [0.25, 0.7777777777777778, 1.0, 1.0, 0.5, 0.0, 0.0]
);
pad_params!(
    Pad07Params,
    "Pad 7",
    "Pad 7 Trigger",
    "Pad 7 Type",
    [
        "Pad 7 Control 1",
        "Pad 7 Control 2",
        "Pad 7 Control 3",
        "Pad 7 Control 4",
        "Pad 7 Control 5",
        "Pad 7 Control 6",
        "Pad 7 Control 7"
    ],
    2,
    [0.75, 1.0, 1.0, 0.5, 0.0, 0.0, 0.0]
);
pad_params!(
    Pad08Params,
    "Pad 8",
    "Pad 8 Trigger",
    "Pad 8 Type",
    [
        "Pad 8 Control 1",
        "Pad 8 Control 2",
        "Pad 8 Control 3",
        "Pad 8 Control 4",
        "Pad 8 Control 5",
        "Pad 8 Control 6",
        "Pad 8 Control 7"
    ],
    3,
    [0.75, 0.5, 0.3333333333333333, 1.0, 1.0, 0.0, 1.0]
);
pad_params!(
    Pad09Params,
    "Pad 9",
    "Pad 9 Trigger",
    "Pad 9 Type",
    [
        "Pad 9 Control 1",
        "Pad 9 Control 2",
        "Pad 9 Control 3",
        "Pad 9 Control 4",
        "Pad 9 Control 5",
        "Pad 9 Control 6",
        "Pad 9 Control 7"
    ],
    4,
    [0.25, 0.47368421052631576, 1.0, 0.05, 0.01, 0.0, 0.0]
);
pad_params!(
    Pad10Params,
    "Pad 10",
    "Pad 10 Trigger",
    "Pad 10 Type",
    [
        "Pad 10 Control 1",
        "Pad 10 Control 2",
        "Pad 10 Control 3",
        "Pad 10 Control 4",
        "Pad 10 Control 5",
        "Pad 10 Control 6",
        "Pad 10 Control 7"
    ],
    5,
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
);
pad_params!(
    Pad11Params,
    "Pad 11",
    "Pad 11 Trigger",
    "Pad 11 Type",
    [
        "Pad 11 Control 1",
        "Pad 11 Control 2",
        "Pad 11 Control 3",
        "Pad 11 Control 4",
        "Pad 11 Control 5",
        "Pad 11 Control 6",
        "Pad 11 Control 7"
    ],
    6,
    [0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
);
pad_params!(
    Pad12Params,
    "Pad 12",
    "Pad 12 Trigger",
    "Pad 12 Type",
    [
        "Pad 12 Control 1",
        "Pad 12 Control 2",
        "Pad 12 Control 3",
        "Pad 12 Control 4",
        "Pad 12 Control 5",
        "Pad 12 Control 6",
        "Pad 12 Control 7"
    ],
    7,
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
);
pad_params!(
    Pad13Params,
    "Pad 13",
    "Pad 13 Trigger",
    "Pad 13 Type",
    [
        "Pad 13 Control 1",
        "Pad 13 Control 2",
        "Pad 13 Control 3",
        "Pad 13 Control 4",
        "Pad 13 Control 5",
        "Pad 13 Control 6",
        "Pad 13 Control 7"
    ],
    8,
    [0.09375, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0]
);
pad_params!(
    Pad14Params,
    "Pad 14",
    "Pad 14 Trigger",
    "Pad 14 Type",
    [
        "Pad 14 Control 1",
        "Pad 14 Control 2",
        "Pad 14 Control 3",
        "Pad 14 Control 4",
        "Pad 14 Control 5",
        "Pad 14 Control 6",
        "Pad 14 Control 7"
    ],
    9,
    [
        0.04591836734693878,
        0.2032258064516129,
        1.0,
        0.0,
        0.0,
        0.0,
        0.0
    ]
);
pad_params!(
    Pad15Params,
    "Pad 15",
    "Pad 15 Trigger",
    "Pad 15 Type",
    [
        "Pad 15 Control 1",
        "Pad 15 Control 2",
        "Pad 15 Control 3",
        "Pad 15 Control 4",
        "Pad 15 Control 5",
        "Pad 15 Control 6",
        "Pad 15 Control 7"
    ],
    10,
    [0.17333333333333334, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0]
);
pad_params!(
    Pad16Params,
    "Pad 16",
    "Pad 16 Trigger",
    "Pad 16 Type",
    [
        "Pad 16 Control 1",
        "Pad 16 Control 2",
        "Pad 16 Control 3",
        "Pad 16 Control 4",
        "Pad 16 Control 5",
        "Pad 16 Control 6",
        "Pad 16 Control 7"
    ],
    11,
    [
        0.23356009070294784,
        0.42857142857142855,
        1.0,
        0.0,
        0.0,
        0.0,
        0.0
    ]
);

#[derive(Params)]
pub struct BufferUppercutParams {
    #[param(
        id = 0,
        name = "Performance Pitch",
        range = "discrete(-24, 24)",
        default = 0,
        unit = "st"
    )]
    performance_pitch: IntParam,
    #[nested(base = 1)]
    pad_01: Pad01Params,
    #[nested(base = 10)]
    pad_02: Pad02Params,
    #[nested(base = 19)]
    pad_03: Pad03Params,
    #[nested(base = 28)]
    pad_04: Pad04Params,
    #[nested(base = 37)]
    pad_05: Pad05Params,
    #[nested(base = 46)]
    pad_06: Pad06Params,
    #[nested(base = 55)]
    pad_07: Pad07Params,
    #[nested(base = 64)]
    pad_08: Pad08Params,
    #[nested(base = 73)]
    pad_09: Pad09Params,
    #[nested(base = 82)]
    pad_10: Pad10Params,
    #[nested(base = 91)]
    pad_11: Pad11Params,
    #[nested(base = 100)]
    pad_12: Pad12Params,
    #[nested(base = 109)]
    pad_13: Pad13Params,
    #[nested(base = 118)]
    pad_14: Pad14Params,
    #[nested(base = 127)]
    pad_15: Pad15Params,
    #[nested(base = 136)]
    pad_16: Pad16Params,
    #[skip]
    midi_held_pad_bits: AtomicU16,
    #[skip]
    midi_pad_press_event: AtomicU32,
}

#[must_use]
pub const fn pad_trigger_id(pad: usize) -> u32 {
    1 + pad as u32 * PAD_PARAMETER_STRIDE
}

#[must_use]
pub const fn pad_type_id(pad: usize) -> u32 {
    pad_trigger_id(pad) + 1
}

#[must_use]
pub const fn pad_control_id(pad: usize, control: usize) -> u32 {
    pad_trigger_id(pad) + 2 + control as u32
}

impl BufferUppercutParams {
    pub(crate) fn set_midi_held_pad_bits(&self, bits: u16) {
        self.midi_held_pad_bits.store(bits, Ordering::Release);
    }

    pub(crate) fn midi_held_pad_bits(&self) -> u16 {
        self.midi_held_pad_bits.load(Ordering::Acquire)
    }

    pub(crate) fn record_midi_pad_press(&self, pad: usize) {
        debug_assert!(pad < NUM_PADS);
        let previous = self.midi_pad_press_event.load(Ordering::Relaxed);
        let sequence = ((previous >> 8).wrapping_add(1)) & 0x00ff_ffff;
        let encoded_pad = pad as u32 + 1;
        self.midi_pad_press_event
            .store((sequence << 8) | encoded_pad, Ordering::Release);
    }

    pub(crate) fn midi_pad_press_event(&self) -> (u32, Option<usize>) {
        let event = self.midi_pad_press_event.load(Ordering::Acquire);
        let encoded_pad = event & 0xff;
        let pad = (encoded_pad != 0).then(|| encoded_pad as usize - 1);
        (event >> 8, pad)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_matches_the_wrac_comparison_target() {
        let params = BufferUppercutParams::default();
        let infos = params.param_infos();
        assert_eq!(params.count(), 145);
        assert_eq!(infos[0].id, 0);
        assert_eq!(infos[0].name, "Performance Pitch");
        assert_eq!(infos[1].name, "Pad 1 Trigger");
        assert_eq!(infos[144].id, 144);
        assert_eq!(infos[144].name, "Pad 16 Control 7");
    }

    #[test]
    fn midi_ui_state_is_ephemeral_and_not_a_parameter() {
        let params = BufferUppercutParams::default();
        params.set_midi_held_pad_bits((1 << 0) | (1 << 15));
        assert_eq!(params.midi_held_pad_bits(), (1 << 0) | (1 << 15));
        assert_eq!(params.midi_pad_press_event(), (0, None));
        params.record_midi_pad_press(5);
        assert_eq!(params.midi_pad_press_event(), (1, Some(5)));
        params.record_midi_pad_press(15);
        assert_eq!(params.midi_pad_press_event(), (2, Some(15)));
        assert_eq!(params.count(), 145);
    }

    #[test]
    fn classic_defaults_are_preserved() {
        let params = BufferUppercutParams::default();
        assert_eq!(params.get_plain(pad_type_id(0)), Some(1.0));
        assert_eq!(params.get_plain(pad_type_id(15)), Some(11.0));
        assert_eq!(params.get_plain(pad_control_id(0, 0)), Some(0.25));
        assert_eq!(
            params.get_plain(pad_control_id(4, 1)),
            Some(0.6666666666666666)
        );
    }
}
