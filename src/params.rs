use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU16, AtomicU32, Ordering};
use std::sync::{Arc, RwLock};

use buffer_uppercut_dsp::{
    EffectType as DspEffectType, VisualizationBin, VisualizationMeta, VisualizationMode,
};
use truce::prelude::*;

pub const NUM_PADS: usize = 16;
pub const NUM_MACROS: usize = 7;
pub const NUM_VISUALIZATION_BINS: usize = 256;
pub const PAD_PARAMETER_STRIDE: u32 = 9;
pub const PARAM_PERFORMANCE_PITCH_ID: u32 = 0;

pub(crate) struct WaveformSnapshot {
    pub bins: [VisualizationBin; NUM_VISUALIZATION_BINS],
    pub meta: VisualizationMeta,
    sequence: u32,
}

impl Default for WaveformSnapshot {
    fn default() -> Self {
        Self {
            bins: [VisualizationBin::default(); NUM_VISUALIZATION_BINS],
            meta: VisualizationMeta::default(),
            sequence: 0,
        }
    }
}

struct WaveformBridge {
    sequence: AtomicU32,
    bins: [[AtomicU32; 4]; NUM_VISUALIZATION_BINS],
    capture_mode: AtomicBool,
    active_pad: AtomicU8,
    active_effect: AtomicU8,
    normalized_playhead: AtomicU32,
    reverse: AtomicBool,
    play_rate: AtomicU32,
    duration_seconds: AtomicU32,
}

impl Default for WaveformBridge {
    fn default() -> Self {
        Self {
            sequence: AtomicU32::new(0),
            bins: std::array::from_fn(|_| std::array::from_fn(|_| AtomicU32::new(0))),
            capture_mode: AtomicBool::new(false),
            active_pad: AtomicU8::new(u8::MAX),
            active_effect: AtomicU8::new(DspEffectType::Off as u8),
            normalized_playhead: AtomicU32::new(0.0_f32.to_bits()),
            reverse: AtomicBool::new(false),
            play_rate: AtomicU32::new(1.0_f32.to_bits()),
            duration_seconds: AtomicU32::new(0.0_f32.to_bits()),
        }
    }
}

impl WaveformBridge {
    fn publish(&self, bins: &[VisualizationBin; NUM_VISUALIZATION_BINS], meta: VisualizationMeta) {
        self.sequence.fetch_add(1, Ordering::Release);
        for (target, source) in self.bins.iter().zip(bins) {
            target[0].store(source.min_l.to_bits(), Ordering::Relaxed);
            target[1].store(source.max_l.to_bits(), Ordering::Relaxed);
            target[2].store(source.min_r.to_bits(), Ordering::Relaxed);
            target[3].store(source.max_r.to_bits(), Ordering::Relaxed);
        }
        self.capture_mode
            .store(meta.mode == VisualizationMode::Capture, Ordering::Relaxed);
        self.active_pad.store(
            meta.active_pad
                .and_then(|pad| u8::try_from(pad).ok())
                .unwrap_or(u8::MAX),
            Ordering::Relaxed,
        );
        self.active_effect
            .store(meta.active_effect as u8, Ordering::Relaxed);
        self.normalized_playhead
            .store(meta.normalized_playhead.to_bits(), Ordering::Relaxed);
        self.reverse.store(meta.reverse, Ordering::Relaxed);
        self.play_rate
            .store(meta.play_rate.to_bits(), Ordering::Relaxed);
        self.duration_seconds
            .store(meta.duration_seconds.to_bits(), Ordering::Relaxed);
        self.sequence.fetch_add(1, Ordering::Release);
    }

    fn read(&self, snapshot: &mut WaveformSnapshot) -> bool {
        for _ in 0..3 {
            let before = self.sequence.load(Ordering::Acquire);
            if before & 1 != 0 {
                continue;
            }
            if before == snapshot.sequence {
                return false;
            }
            for (target, source) in snapshot.bins.iter_mut().zip(&self.bins) {
                *target = VisualizationBin {
                    min_l: f32::from_bits(source[0].load(Ordering::Relaxed)),
                    max_l: f32::from_bits(source[1].load(Ordering::Relaxed)),
                    min_r: f32::from_bits(source[2].load(Ordering::Relaxed)),
                    max_r: f32::from_bits(source[3].load(Ordering::Relaxed)),
                };
            }
            snapshot.meta = VisualizationMeta {
                mode: if self.capture_mode.load(Ordering::Relaxed) {
                    VisualizationMode::Capture
                } else {
                    VisualizationMode::Rolling
                },
                active_pad: match self.active_pad.load(Ordering::Relaxed) {
                    u8::MAX => None,
                    pad => Some(usize::from(pad)),
                },
                active_effect: DspEffectType::from_index(i32::from(
                    self.active_effect.load(Ordering::Relaxed),
                )),
                normalized_playhead: f32::from_bits(
                    self.normalized_playhead.load(Ordering::Relaxed),
                ),
                reverse: self.reverse.load(Ordering::Relaxed),
                play_rate: f32::from_bits(self.play_rate.load(Ordering::Relaxed)),
                duration_seconds: f32::from_bits(self.duration_seconds.load(Ordering::Relaxed)),
            };
            let after = self.sequence.load(Ordering::Acquire);
            if before == after {
                snapshot.sequence = after;
                return true;
            }
        }
        false
    }
}

#[derive(ParamEnum)]
pub enum EffectType {
    Off,
    #[name = "Beat Repeat"]
    BeatRepeat,
    Reverse,
    #[name = "Tape Stop"]
    TapeStop,
    Gate,
    Pitch,
    Filter,
    LoFi,
    Vinyl,
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
    [0.25, 0.5, 1.0, 0.05, 0.01, 0.0, 0.0]
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
    [
        0.0,
        0.0,
        0.3333333333333333,
        0.2,
        0.13559322033898305,
        0.0,
        1.0
    ]
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
    5,
    [
        0.5,
        0.0,
        0.3333333333333333,
        0.2,
        0.13559322033898305,
        0.0,
        1.0
    ]
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
    5,
    [
        1.0,
        0.0,
        0.3333333333333333,
        0.2,
        0.13559322033898305,
        0.0,
        1.0
    ]
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
    6,
    [0.0, 0.3713144507689456, 0.35, 0.2, 0.0, 0.0, 1.0]
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
    6,
    [0.5, 0.5927170834612147, 0.35, 0.2, 0.0, 0.0, 1.0]
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
    6,
    [1.0, 0.751757501701102, 0.35, 0.2, 0.0, 0.0, 1.0]
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
    7,
    [
        0.2325986078886311,
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
    #[persist]
    kit_name: RwLock<String>,
    #[skip]
    midi_held_pad_bits: AtomicU16,
    #[skip]
    midi_pad_press_event: AtomicU32,
    #[skip]
    direct_keys_initialized: AtomicBool,
    #[skip]
    direct_keys_enabled: AtomicBool,
    #[skip]
    direct_key_held_pad_bits: AtomicU16,
    #[skip]
    direct_key_press_event: AtomicU32,
    #[skip]
    pointer_held_pad_bits: AtomicU16,
    #[skip]
    active_pad_bits: AtomicU16,
    #[skip]
    suspended_pad_bits: AtomicU16,
    #[skip]
    visualization_selected_pad: AtomicU8,
    #[skip]
    kit_reset_sequence: AtomicU32,
    #[skip]
    waveform: Arc<WaveformBridge>,
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
    pub(crate) fn kit_name(&self) -> String {
        self.kit_name
            .read()
            .ok()
            .map(|name| {
                if name.is_empty() {
                    "Classic".to_owned()
                } else {
                    name.clone()
                }
            })
            .unwrap_or_else(|| "Classic".to_owned())
    }

    pub(crate) fn set_kit_name(&self, name: &str) {
        if let Ok(mut kit_name) = self.kit_name.write() {
            *kit_name = if name.is_empty() {
                "Untitled Kit".to_owned()
            } else {
                name.to_owned()
            };
        }
    }

    pub(crate) fn request_kit_reset(&self) {
        self.kit_reset_sequence.fetch_add(1, Ordering::Release);
    }

    pub(crate) fn kit_reset_sequence(&self) -> u32 {
        self.kit_reset_sequence.load(Ordering::Acquire)
    }

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

    pub(crate) fn initialize_direct_keys_enabled(&self, enabled: bool) {
        if self
            .direct_keys_initialized
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            self.direct_keys_enabled.store(enabled, Ordering::Release);
        }
    }

    pub(crate) fn set_direct_keys_enabled(&self, enabled: bool) {
        self.direct_keys_initialized.store(true, Ordering::Release);
        self.direct_keys_enabled.store(enabled, Ordering::Release);
        if !enabled {
            self.clear_direct_key_holds();
        }
    }

    pub(crate) fn direct_keys_enabled(&self) -> bool {
        self.direct_keys_enabled.load(Ordering::Acquire)
    }

    pub(crate) fn apply_direct_key(&self, pad: usize, pressed: bool, repeat: bool) -> bool {
        debug_assert!(pad < NUM_PADS);
        if !self.direct_keys_enabled() {
            return false;
        }
        if repeat {
            return true;
        }

        let bit = 1_u16 << pad;
        if pressed {
            let previous = self
                .direct_key_held_pad_bits
                .fetch_or(bit, Ordering::AcqRel);
            if previous & bit == 0 {
                self.record_direct_key_press(pad);
            }
            if !self.direct_keys_enabled() {
                self.direct_key_held_pad_bits
                    .fetch_and(!bit, Ordering::Release);
            }
        } else {
            self.direct_key_held_pad_bits
                .fetch_and(!bit, Ordering::Release);
        }
        true
    }

    pub(crate) fn clear_direct_key_holds(&self) {
        self.direct_key_held_pad_bits.store(0, Ordering::Release);
    }

    pub(crate) fn direct_key_held_pad_bits(&self) -> u16 {
        self.direct_key_held_pad_bits.load(Ordering::Acquire)
    }

    fn record_direct_key_press(&self, pad: usize) {
        let previous = self.direct_key_press_event.load(Ordering::Relaxed);
        let sequence = ((previous >> 8).wrapping_add(1)) & 0x00ff_ffff;
        self.direct_key_press_event
            .store((sequence << 8) | (pad as u32 + 1), Ordering::Release);
    }

    pub(crate) fn direct_key_press_event(&self) -> (u32, Option<usize>) {
        let event = self.direct_key_press_event.load(Ordering::Acquire);
        let encoded_pad = event & 0xff;
        (
            event >> 8,
            (encoded_pad != 0).then(|| encoded_pad as usize - 1),
        )
    }

    pub(crate) fn set_pointer_held(&self, pad: usize, held: bool) {
        debug_assert!(pad < NUM_PADS);
        let bit = 1_u16 << pad;
        if held {
            self.pointer_held_pad_bits.fetch_or(bit, Ordering::Release);
        } else {
            self.pointer_held_pad_bits
                .fetch_and(!bit, Ordering::Release);
        }
    }

    pub(crate) fn clear_pointer_holds(&self) {
        self.pointer_held_pad_bits.store(0, Ordering::Release);
    }

    pub(crate) fn pointer_held_pad_bits(&self) -> u16 {
        self.pointer_held_pad_bits.load(Ordering::Acquire)
    }

    pub(crate) fn set_admission_pad_bits(&self, active: u16, suspended: u16) {
        self.active_pad_bits.store(active, Ordering::Release);
        self.suspended_pad_bits.store(suspended, Ordering::Release);
    }

    pub(crate) fn admission_pad_bits(&self) -> (u16, u16) {
        (
            self.active_pad_bits.load(Ordering::Acquire),
            self.suspended_pad_bits.load(Ordering::Acquire),
        )
    }

    pub(crate) fn set_visualization_selected_pad(&self, pad: usize) {
        self.visualization_selected_pad
            .store(pad.min(NUM_PADS - 1) as u8, Ordering::Release);
    }

    pub(crate) fn visualization_selected_pad(&self) -> usize {
        usize::from(self.visualization_selected_pad.load(Ordering::Acquire)).min(NUM_PADS - 1)
    }

    pub(crate) fn publish_waveform(
        &self,
        bins: &[VisualizationBin; NUM_VISUALIZATION_BINS],
        meta: VisualizationMeta,
    ) {
        self.waveform.publish(bins, meta);
    }

    pub(crate) fn waveform_snapshot(&self, snapshot: &mut WaveformSnapshot) -> bool {
        self.waveform.read(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_schema_has_145_stable_parameters() {
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
    fn direct_key_state_is_separate_repeat_safe_and_ephemeral() {
        let params = BufferUppercutParams::default();
        assert!(!params.direct_keys_enabled());
        assert!(!params.apply_direct_key(3, true, false));
        params.initialize_direct_keys_enabled(true);
        assert!(params.apply_direct_key(3, true, false));
        let first = params.direct_key_press_event();
        assert_eq!(params.direct_key_held_pad_bits(), 1 << 3);
        assert!(params.apply_direct_key(3, true, true));
        assert!(params.apply_direct_key(3, true, false));
        assert_eq!(params.direct_key_press_event(), first);
        assert!(params.apply_direct_key(3, false, false));
        assert_eq!(params.direct_key_held_pad_bits(), 0);
        assert_eq!(params.count(), 145);
    }

    #[test]
    fn disabling_or_clearing_direct_keys_releases_only_direct_holds() {
        let params = BufferUppercutParams::default();
        params.set_midi_held_pad_bits(1 << 5);
        params.set_direct_keys_enabled(true);
        assert!(params.apply_direct_key(5, true, false));
        params.set_direct_keys_enabled(false);
        assert_eq!(params.direct_key_held_pad_bits(), 0);
        assert_eq!(params.midi_held_pad_bits(), 1 << 5);
    }

    #[test]
    fn pointer_holds_do_not_mutate_automatable_trigger_parameters() {
        let params = BufferUppercutParams::default();
        assert_eq!(params.get_plain(pad_trigger_id(4)), Some(0.0));
        params.set_pointer_held(4, true);
        assert_eq!(params.pointer_held_pad_bits(), 1 << 4);
        assert_eq!(params.get_plain(pad_trigger_id(4)), Some(0.0));
        params.clear_pointer_holds();
        assert_eq!(params.pointer_held_pad_bits(), 0);
    }

    #[test]
    fn admission_and_visualization_selection_are_atomic_runtime_state() {
        let params = BufferUppercutParams::default();
        params.set_admission_pad_bits(0x003f, 0x0040);
        params.set_visualization_selected_pad(12);
        assert_eq!(params.admission_pad_bits(), (0x003f, 0x0040));
        assert_eq!(params.visualization_selected_pad(), 12);
        assert_eq!(params.count(), 145);
    }

    #[test]
    fn kit_name_is_persisted_without_becoming_a_parameter() {
        let params = BufferUppercutParams::default();
        params.set_kit_name("Tape Lab");
        let persist = params.serialize_persist();

        let restored = BufferUppercutParams::default();
        restored.load_persist(&persist);
        assert_eq!(restored.kit_name(), "Tape Lab");
        assert_eq!(restored.count(), 145);
    }

    #[test]
    fn waveform_bridge_publishes_complete_snapshots_without_parameters() {
        let params = BufferUppercutParams::default();
        let bins = [VisualizationBin {
            min_l: -0.75,
            max_l: 0.5,
            min_r: -0.25,
            max_r: 0.875,
        }; NUM_VISUALIZATION_BINS];
        let meta = VisualizationMeta {
            mode: VisualizationMode::Capture,
            active_pad: Some(6),
            active_effect: DspEffectType::Reverse,
            normalized_playhead: 0.625,
            reverse: true,
            play_rate: 0.5,
            duration_seconds: 1.25,
        };
        params.publish_waveform(&bins, meta);

        let mut snapshot = WaveformSnapshot::default();
        assert!(params.waveform_snapshot(&mut snapshot));
        assert_eq!(snapshot.bins, bins);
        assert_eq!(snapshot.meta, meta);
        assert!(!params.waveform_snapshot(&mut snapshot));
        assert_eq!(params.count(), 145);
    }

    #[test]
    fn vinyl_is_the_ninth_host_choice_on_every_slot() {
        let params = BufferUppercutParams::default();
        for pad in 0..NUM_PADS {
            let id = pad_type_id(pad);
            for effect in 0..=8 {
                params.set_normalized(id, effect as f64 / 8.0);
                assert_eq!(params.get_plain(id), Some(effect as f64));
            }
        }
        assert_eq!(params.count(), 145);
    }

    #[test]
    fn classic_defaults_are_preserved() {
        let params = BufferUppercutParams::default();
        assert_eq!(params.get_plain(pad_type_id(0)), Some(1.0));
        assert_eq!(params.get_plain(pad_type_id(15)), Some(7.0));
        assert_eq!(params.get_plain(pad_control_id(0, 0)), Some(0.25));
        assert_eq!(
            params.get_plain(pad_control_id(4, 1)),
            Some(0.6666666666666666)
        );
    }

    #[test]
    fn fresh_host_parameters_match_every_classic_kit_control() {
        let params = BufferUppercutParams::default();
        let kit = buffer_uppercut_kit::classic_kit();
        for (pad, config) in kit.state.pads.iter().enumerate() {
            assert_eq!(
                params.get_plain(pad_type_id(pad)),
                Some(config.effect_type as u8 as f64)
            );
            for (control, expected) in config.macros.iter().enumerate() {
                let actual = params.get_plain(pad_control_id(pad, control)).unwrap();
                assert!(
                    (actual - expected).abs() < 1e-12,
                    "pad {} control {}: host={actual}, kit={expected}",
                    pad + 1,
                    control + 1
                );
            }
        }
    }
}
