//! Framework-independent Buffer Uppercut DSP.
//!
//! This is an independent Rust implementation of the behavior pinned by
//! `contract/dsp-contract-v1.json`. Allocation is confined to [`Engine::reset`];
//! [`Engine::process`] only mutates already-prepared storage.

pub const NUM_PADS: usize = 16;
pub const NUM_MACROS: usize = 7;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u8)]
pub enum EffectType {
    #[default]
    Off = 0,
    BeatRepeat = 1,
    Reverse = 2,
    TapeStop = 3,
    Gate = 4,
    PitchDown = 5,
    PitchReset = 6,
    PitchUp = 7,
    BandLow = 8,
    BandMid = 9,
    BandHigh = 10,
    LoFi = 11,
}

impl EffectType {
    #[must_use]
    pub const fn from_index(index: i32) -> Self {
        match index {
            1 => Self::BeatRepeat,
            2 => Self::Reverse,
            3 => Self::TapeStop,
            4 => Self::Gate,
            5 => Self::PitchDown,
            6 => Self::PitchReset,
            7 => Self::PitchUp,
            8 => Self::BandLow,
            9 => Self::BandMid,
            10 => Self::BandHigh,
            11 => Self::LoFi,
            _ => Self::Off,
        }
    }

    #[must_use]
    pub const fn is_buffer(self) -> bool {
        matches!(self, Self::BeatRepeat | Self::Reverse | Self::TapeStop)
    }

    #[must_use]
    pub const fn is_pitch_action(self) -> bool {
        matches!(self, Self::PitchDown | Self::PitchReset | Self::PitchUp)
    }

    #[must_use]
    pub const fn active_control_count(self) -> usize {
        match self {
            Self::Off => 0,
            Self::Gate => 6,
            Self::PitchDown | Self::PitchReset | Self::PitchUp => 1,
            Self::BandLow | Self::BandHigh => 2,
            Self::BandMid | Self::LoFi => 3,
            Self::BeatRepeat | Self::Reverse | Self::TapeStop => NUM_MACROS,
        }
    }

    #[must_use]
    pub const fn is_control_enabled(self, control: usize) -> bool {
        control < self.active_control_count()
    }

    #[must_use]
    pub const fn control_name(self, control: usize) -> &'static str {
        if control >= NUM_MACROS {
            return "";
        }

        match self {
            Self::Off => "",
            Self::BeatRepeat => [
                "CELL", "LOOKBACK", "WET", "MODE", "PITCH", "DECAY", "JITTER",
            ][control],
            Self::Reverse => [
                "LENGTH", "WET", "MODE", "PITCH", "DECAY", "OFFSET", "JITTER",
            ][control],
            Self::TapeStop => ["TIME", "CURVE", "START", "WET", "MODE", "OFFSET", "FADE"][control],
            Self::Gate => ["GRID", "DUTY", "DEPTH", "ATTACK", "RELEASE", "PHASE", ""][control],
            Self::PitchDown | Self::PitchUp => ["STEP", "", "", "", "", "", ""][control],
            Self::PitchReset => ["TARGET", "", "", "", "", "", ""][control],
            Self::BandLow | Self::BandHigh => ["CUTOFF", "WET", "", "", "", "", ""][control],
            Self::BandMid => ["LOW CUT", "HIGH CUT", "WET", "", "", "", ""][control],
            Self::LoFi => ["RATE", "BITS", "WET", "", "", "", ""][control],
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PadConfig {
    pub effect_type: EffectType,
    pub macros: [f64; NUM_MACROS],
}

impl Default for PadConfig {
    fn default() -> Self {
        Self {
            effect_type: EffectType::Off,
            macros: [0.0; NUM_MACROS],
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PerformanceState {
    pub pads: [PadConfig; NUM_PADS],
    pub held: [bool; NUM_PADS],
    pub performance_pitch: f64,
}

impl Default for PerformanceState {
    fn default() -> Self {
        Self {
            pads: [PadConfig::default(); NUM_PADS],
            held: [false; NUM_PADS],
            performance_pitch: 0.0,
        }
    }
}

impl PerformanceState {
    #[must_use]
    pub fn classic() -> Self {
        classic_state()
    }
}

#[must_use]
pub fn clamp_macro(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[must_use]
pub fn normalize_linear(value: f64, minimum: f64, maximum: f64) -> f64 {
    if maximum <= minimum {
        0.0
    } else {
        clamp_macro((value - minimum) / (maximum - minimum))
    }
}

#[must_use]
pub fn denormalize_linear(normalized: f64, minimum: f64, maximum: f64) -> f64 {
    minimum + clamp_macro(normalized) * (maximum - minimum)
}

#[must_use]
pub fn grid_normalized(index: i32) -> f64 {
    clamp_macro(f64::from(index.clamp(0, 8)) / 8.0)
}

#[must_use]
pub fn grid_index(normalized: f64) -> usize {
    (clamp_macro(normalized) * 8.0).round().clamp(0.0, 8.0) as usize
}

#[must_use]
pub fn grid_beats(normalized: f64) -> f64 {
    const VALUES: [f64; 9] = [
        1.0 / 16.0,
        1.0 / 8.0,
        1.0 / 4.0,
        1.0 / 2.0,
        1.0,
        2.0,
        4.0,
        8.0,
        16.0,
    ];
    VALUES[grid_index(normalized)]
}

#[must_use]
pub fn lookback_normalized(index: i32) -> f64 {
    clamp_macro(f64::from(index.clamp(0, 9)) / 9.0)
}

#[must_use]
pub fn lookback_index(normalized: f64) -> usize {
    (clamp_macro(normalized) * 9.0).round().clamp(0.0, 9.0) as usize
}

#[must_use]
pub fn lookback_beats(normalized: f64, cell_beats: f64) -> f64 {
    const VALUES: [f64; 10] = [
        0.0,
        1.0 / 16.0,
        1.0 / 8.0,
        1.0 / 4.0,
        1.0 / 2.0,
        1.0,
        2.0,
        4.0,
        8.0,
        16.0,
    ];
    let index = lookback_index(normalized);
    if index == 0 {
        cell_beats
    } else {
        VALUES[index]
    }
}

#[must_use]
pub fn grid_label(normalized: f64) -> &'static str {
    const LABELS: [&str; 9] = [
        "1/64", "1/32", "1/16", "1/8", "1/4", "1/2", "1 BAR", "2 BAR", "4 BAR",
    ];
    LABELS[grid_index(normalized)]
}

#[must_use]
pub fn lookback_label(normalized: f64) -> &'static str {
    const LABELS: [&str; 10] = [
        "CELL",
        "1/16 BEAT",
        "1/8 BEAT",
        "1/4 BEAT",
        "1/2 BEAT",
        "1 BEAT",
        "2 BEATS",
        "4 BEATS",
        "8 BEATS",
        "16 BEATS",
    ];
    LABELS[lookback_index(normalized)]
}

/// Format one normalized macro value for a product UI.
///
/// This is deliberately separate from host parameter formatting: hosts keep
/// seeing the stable normalized parameter, while editors may show its meaning
/// in the context of the pad's current effect type.
#[must_use]
pub fn format_control_value(effect_type: EffectType, control: usize, normalized: f64) -> String {
    if !effect_type.is_control_enabled(control) {
        return String::new();
    }
    let normalized = clamp_macro(normalized);
    let percent = |value: f64| format!("{:.0}%", value * 100.0);

    match effect_type {
        EffectType::Off => String::new(),
        EffectType::BeatRepeat => match control {
            0 => grid_label(normalized).to_owned(),
            1 => lookback_label(normalized).to_owned(),
            2 | 6 => percent(normalized),
            3 => if normalized >= 0.5 { "INSERT" } else { "MIX" }.to_owned(),
            4 => format!("{:+.0} st", denormalize_linear(normalized, -24.0, 24.0)),
            _ => format!("{:.1} dB", denormalize_linear(normalized, 0.0, 12.0)),
        },
        EffectType::Reverse => match control {
            0 => grid_label(normalized).to_owned(),
            1 | 5 | 6 => percent(normalized),
            2 => if normalized >= 0.5 { "INSERT" } else { "MIX" }.to_owned(),
            3 => format!("{:+.0} st", denormalize_linear(normalized, -24.0, 24.0)),
            _ => format!("{:.1} dB", denormalize_linear(normalized, 0.0, 12.0)),
        },
        EffectType::TapeStop => match control {
            0 => grid_label(normalized).to_owned(),
            1 => format!("{:.2}", denormalize_linear(normalized, 0.25, 4.0)),
            2 => format!("{:.2}x", denormalize_linear(normalized, 0.5, 2.0)),
            4 => if normalized >= 0.5 { "INSERT" } else { "MIX" }.to_owned(),
            _ => percent(normalized),
        },
        EffectType::Gate => match control {
            0 => grid_label(normalized).to_owned(),
            1 => format!("{:.0}%", denormalize_linear(normalized, 5.0, 95.0)),
            2 | 5 => percent(normalized),
            3 => format!("{:.1} ms", denormalize_linear(normalized, 0.0, 20.0)),
            _ => format!("{:.1} ms", denormalize_linear(normalized, 0.0, 100.0)),
        },
        EffectType::PitchDown | EffectType::PitchUp => format!(
            "{:.0} st",
            denormalize_linear(normalized, 1.0, 24.0).round()
        ),
        EffectType::PitchReset => format!(
            "{:+.0} st",
            denormalize_linear(normalized, -24.0, 24.0).round()
        ),
        EffectType::BandLow => match control {
            0 => format!("{:.0} Hz", denormalize_linear(normalized, 80.0, 2000.0)),
            _ => percent(normalized),
        },
        EffectType::BandMid => match control {
            0 => format!("{:.0} Hz", denormalize_linear(normalized, 80.0, 4000.0)),
            1 => format!("{:.0} Hz", denormalize_linear(normalized, 500.0, 16000.0)),
            _ => percent(normalized),
        },
        EffectType::BandHigh => match control {
            0 => format!("{:.0} Hz", denormalize_linear(normalized, 1000.0, 16000.0)),
            _ => percent(normalized),
        },
        EffectType::LoFi => match control {
            0 => format!("{:.0} Hz", denormalize_linear(normalized, 1000.0, 44100.0)),
            1 => format!(
                "{:.0} bit",
                denormalize_linear(normalized, 2.0, 16.0).round()
            ),
            _ => percent(normalized),
        },
    }
}

#[must_use]
pub fn pitch_step(config: &PadConfig) -> f64 {
    denormalize_linear(config.macros[0], 1.0, 24.0).round()
}

#[must_use]
pub fn pitch_reset_target(config: &PadConfig) -> f64 {
    denormalize_linear(config.macros[0], -24.0, 24.0).round()
}

#[must_use]
pub fn apply_pitch_action(current: f64, effect: EffectType, config: &PadConfig) -> f64 {
    let current = if current.is_finite() { current } else { 0.0 }.clamp(-24.0, 24.0);
    match effect {
        EffectType::PitchDown => (current - pitch_step(config)).clamp(-24.0, 24.0),
        EffectType::PitchUp => (current + pitch_step(config)).clamp(-24.0, 24.0),
        EffectType::PitchReset => pitch_reset_target(config).clamp(-24.0, 24.0),
        _ => current,
    }
}

#[must_use]
pub fn default_pad_config(effect_type: EffectType) -> PadConfig {
    let mut config = PadConfig {
        effect_type,
        ..PadConfig::default()
    };
    let grid_16 = grid_normalized(2);
    match effect_type {
        EffectType::BeatRepeat => {
            config.macros = [grid_16, lookback_normalized(0), 1.0, 1.0, 0.5, 0.0, 0.0];
        }
        EffectType::Reverse => {
            config.macros = [grid_normalized(6), 1.0, 1.0, 0.5, 0.0, 0.0, 0.0];
        }
        EffectType::TapeStop => {
            config.macros = [
                grid_normalized(6),
                0.5,
                normalize_linear(1.0, 0.5, 2.0),
                1.0,
                1.0,
                0.0,
                1.0,
            ];
        }
        EffectType::Gate => {
            config.macros = [
                grid_16,
                normalize_linear(50.0, 5.0, 95.0),
                1.0,
                normalize_linear(1.0, 0.0, 20.0),
                normalize_linear(1.0, 0.0, 100.0),
                0.0,
                0.0,
            ];
        }
        EffectType::PitchDown | EffectType::PitchUp => {
            config.macros[0] = normalize_linear(1.0, 1.0, 24.0);
        }
        EffectType::PitchReset => {
            config.macros[0] = normalize_linear(0.0, -24.0, 24.0);
        }
        EffectType::BandLow => {
            config.macros = [
                normalize_linear(260.0, 80.0, 2000.0),
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
            ];
        }
        EffectType::BandMid => {
            config.macros = [
                normalize_linear(260.0, 80.0, 4000.0),
                normalize_linear(3600.0, 500.0, 16000.0),
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
            ];
        }
        EffectType::BandHigh => {
            config.macros = [
                normalize_linear(3600.0, 1000.0, 16000.0),
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
            ];
        }
        EffectType::LoFi => {
            config.macros = [
                normalize_linear(11025.0, 1000.0, 44100.0),
                normalize_linear(8.0, 2.0, 16.0),
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
            ];
        }
        EffectType::Off => {}
    }
    config
}

#[must_use]
pub fn classic_state() -> PerformanceState {
    const TYPES: [EffectType; NUM_PADS] = [
        EffectType::BeatRepeat,
        EffectType::BeatRepeat,
        EffectType::BeatRepeat,
        EffectType::BeatRepeat,
        EffectType::BeatRepeat,
        EffectType::BeatRepeat,
        EffectType::Reverse,
        EffectType::TapeStop,
        EffectType::Gate,
        EffectType::PitchDown,
        EffectType::PitchReset,
        EffectType::PitchUp,
        EffectType::BandLow,
        EffectType::BandMid,
        EffectType::BandHigh,
        EffectType::LoFi,
    ];
    let mut state = PerformanceState::default();
    for (pad, effect) in TYPES.into_iter().enumerate() {
        state.pads[pad] = default_pad_config(effect);
    }
    for (pad, grid) in [2, 3, 4, 1].into_iter().enumerate() {
        state.pads[pad].macros[0] = grid_normalized(grid);
    }
    state.pads[4].macros[0] = grid_normalized(2);
    state.pads[4].macros[1] = lookback_normalized(6);
    state.pads[5].macros[0] = grid_normalized(2);
    state.pads[5].macros[1] = lookback_normalized(7);
    state
}

#[derive(Clone, Copy)]
enum Category {
    Buffer,
    Gate,
    BandLow,
    BandMid,
    BandHigh,
    LoFi,
}

pub struct Engine {
    sample_rate: f64,
    buffer_l: Vec<f64>,
    buffer_r: Vec<f64>,
    write_pos: usize,
    repeat_active: bool,
    repeat_phase: f64,
    repeat_count: i32,
    capture_start: usize,
    lookback_samples: i32,
    slice_samples: i32,
    repeat_reverse: bool,
    play_rate: f64,
    stop_elapsed: i32,
    stop_total_samples: i32,
    stop_curve: f64,
    stop_start_rate: f64,
    stop_fade: f64,
    stop_amp: f64,
    rng: u32,
    last_held: [bool; NUM_PADS],
    press_order: [u64; NUM_PADS],
    press_counter: u64,
    active_buffer_slot: Option<usize>,
    active_buffer_type: EffectType,
    low_state: [f64; 2],
    mid_low_state: [f64; 2],
    mid_high_state: [f64; 2],
    high_state: [f64; 2],
    lofi_held: [f64; 2],
    lofi_counter: i32,
    gate_phase: f64,
    gate_gain: f64,
    active_gate_slot: Option<usize>,
    active_low_slot: Option<usize>,
    active_mid_slot: Option<usize>,
    active_high_slot: Option<usize>,
    active_lofi_slot: Option<usize>,
}

impl Default for Engine {
    fn default() -> Self {
        let mut engine = Self {
            sample_rate: 44_100.0,
            buffer_l: Vec::new(),
            buffer_r: Vec::new(),
            write_pos: 0,
            repeat_active: false,
            repeat_phase: 0.0,
            repeat_count: 0,
            capture_start: 0,
            lookback_samples: 1,
            slice_samples: 1,
            repeat_reverse: false,
            play_rate: 1.0,
            stop_elapsed: 0,
            stop_total_samples: 1,
            stop_curve: 1.0,
            stop_start_rate: 1.0,
            stop_fade: 1.0,
            stop_amp: 1.0,
            rng: 0x1234_5678,
            last_held: [false; NUM_PADS],
            press_order: [0; NUM_PADS],
            press_counter: 0,
            active_buffer_slot: None,
            active_buffer_type: EffectType::Off,
            low_state: [0.0; 2],
            mid_low_state: [0.0; 2],
            mid_high_state: [0.0; 2],
            high_state: [0.0; 2],
            lofi_held: [0.0; 2],
            lofi_counter: 0,
            gate_phase: 0.0,
            gate_gain: 1.0,
            active_gate_slot: None,
            active_low_slot: None,
            active_mid_slot: None,
            active_high_slot: None,
            active_lofi_slot: None,
        };
        engine.reset(44_100.0);
        engine
    }
}

impl Engine {
    #[must_use]
    pub fn new(sample_rate: f64) -> Self {
        let mut engine = Self::default();
        engine.reset(sample_rate);
        engine
    }

    pub fn reset(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate.max(1.0);
        let required = (self.sample_rate * 16.0) as usize;
        let length = required.max(1).next_power_of_two();
        self.buffer_l.clear();
        self.buffer_l.resize(length, 0.0);
        self.buffer_r.clear();
        self.buffer_r.resize(length, 0.0);
        self.write_pos = 0;
        self.repeat_active = false;
        self.repeat_phase = 0.0;
        self.repeat_count = 0;
        self.capture_start = 0;
        self.lookback_samples = 1;
        self.slice_samples = 1;
        self.repeat_reverse = false;
        self.play_rate = 1.0;
        self.stop_elapsed = 0;
        self.stop_total_samples = 1;
        self.stop_curve = 1.0;
        self.stop_start_rate = 1.0;
        self.stop_fade = 1.0;
        self.stop_amp = 1.0;
        self.rng = 0x1234_5678;
        self.last_held = [false; NUM_PADS];
        self.press_order = [0; NUM_PADS];
        self.press_counter = 0;
        self.active_buffer_slot = None;
        self.active_buffer_type = EffectType::Off;
        self.low_state = [0.0; 2];
        self.mid_low_state = [0.0; 2];
        self.mid_high_state = [0.0; 2];
        self.high_state = [0.0; 2];
        self.lofi_held = [0.0; 2];
        self.lofi_counter = 0;
        self.gate_phase = 0.0;
        self.gate_gain = 1.0;
        self.active_gate_slot = None;
        self.active_low_slot = None;
        self.active_mid_slot = None;
        self.active_high_slot = None;
        self.active_lofi_slot = None;
    }

    pub fn process(
        &mut self,
        input_l: &[f64],
        input_r: &[f64],
        output_l: &mut [f64],
        output_r: &mut [f64],
        tempo: f64,
        state: &PerformanceState,
    ) {
        let frames = input_l
            .len()
            .min(input_r.len())
            .min(output_l.len())
            .min(output_r.len());
        self.update_press_order(state);
        let samples_per_beat = self.sample_rate * 60.0 / tempo.max(1.0);
        let buffer_slot = self.find_newest_held(state, Category::Buffer);
        let buffer_type = buffer_slot.map_or(EffectType::Off, |slot| state.pads[slot].effect_type);
        if buffer_slot != self.active_buffer_slot || buffer_type != self.active_buffer_type {
            self.active_buffer_slot = buffer_slot;
            self.active_buffer_type = buffer_type;
            if let Some(slot) = buffer_slot {
                self.start_buffer_effect(&state.pads[slot], samples_per_beat);
            } else {
                self.repeat_active = false;
            }
        }

        let low_slot = self.find_newest_held(state, Category::BandLow);
        let mid_slot = self.find_newest_held(state, Category::BandMid);
        let high_slot = self.find_newest_held(state, Category::BandHigh);
        let lofi_slot = self.find_newest_held(state, Category::LoFi);
        let gate_slot = self.find_newest_held(state, Category::Gate);
        self.active_low_slot = low_slot;
        self.active_mid_slot = mid_slot;
        self.active_high_slot = high_slot;
        self.active_lofi_slot = lofi_slot;
        if gate_slot != self.active_gate_slot {
            self.active_gate_slot = gate_slot;
            if let Some(slot) = gate_slot {
                let gate = &state.pads[slot];
                let period = (grid_beats(gate.macros[0]) * samples_per_beat).max(2.0);
                self.gate_phase = clamp_macro(gate.macros[5]) * period;
            }
        }

        for sample in 0..frames {
            let dry_l = input_l[sample];
            let dry_r = input_r[sample];
            self.buffer_l[self.write_pos] = dry_l;
            self.buffer_r[self.write_pos] = dry_r;
            let mut output = [dry_l, dry_r];
            if self.repeat_active
                && let Some(slot) = buffer_slot
            {
                self.process_buffer_sample(
                    [dry_l, dry_r],
                    &mut output,
                    &state.pads[slot],
                    state.performance_pitch,
                );
            }
            self.apply_bands(&mut output, state, low_slot, mid_slot, high_slot);
            self.apply_lofi(&mut output, state, lofi_slot);
            self.apply_gate(&mut output, state, gate_slot, samples_per_beat);
            output_l[sample] = output[0];
            output_r[sample] = output[1];
            self.write_pos = self.wrap(self.write_pos as i64 + 1);
        }
    }

    #[must_use]
    pub const fn active_buffer_slot(&self) -> Option<usize> {
        self.active_buffer_slot
    }

    #[must_use]
    pub const fn repeat_active(&self) -> bool {
        self.repeat_active
    }

    #[must_use]
    pub const fn gate_gain(&self) -> f64 {
        self.gate_gain
    }

    #[must_use]
    pub const fn slice_samples(&self) -> i32 {
        self.slice_samples
    }

    #[must_use]
    pub const fn lookback_samples(&self) -> i32 {
        self.lookback_samples
    }

    #[must_use]
    pub const fn play_rate(&self) -> f64 {
        self.play_rate
    }

    #[must_use]
    pub const fn history_bytes(&self) -> usize {
        self.buffer_l.len() * 2 * std::mem::size_of::<f64>()
    }

    fn update_press_order(&mut self, state: &PerformanceState) {
        for pad in 0..NUM_PADS {
            if state.held[pad] && !self.last_held[pad] {
                self.press_counter = self.press_counter.wrapping_add(1);
                self.press_order[pad] = self.press_counter;
            }
            self.last_held[pad] = state.held[pad];
        }
    }

    fn matches_category(effect: EffectType, category: Category) -> bool {
        match category {
            Category::Buffer => effect.is_buffer(),
            Category::Gate => effect == EffectType::Gate,
            Category::BandLow => effect == EffectType::BandLow,
            Category::BandMid => effect == EffectType::BandMid,
            Category::BandHigh => effect == EffectType::BandHigh,
            Category::LoFi => effect == EffectType::LoFi,
        }
    }

    fn find_newest_held(&self, state: &PerformanceState, category: Category) -> Option<usize> {
        let mut newest = None;
        let mut newest_order = 0;
        for pad in 0..NUM_PADS {
            if state.held[pad]
                && Self::matches_category(state.pads[pad].effect_type, category)
                && self.press_order[pad] >= newest_order
            {
                newest_order = self.press_order[pad];
                newest = Some(pad);
            }
        }
        newest
    }

    fn common_wet(config: &PadConfig) -> f64 {
        match config.effect_type {
            EffectType::BeatRepeat => clamp_macro(config.macros[2]),
            EffectType::TapeStop => clamp_macro(config.macros[3]),
            _ => clamp_macro(config.macros[1]),
        }
    }

    fn common_insert(config: &PadConfig) -> bool {
        match config.effect_type {
            EffectType::BeatRepeat => config.macros[3] >= 0.5,
            EffectType::TapeStop => config.macros[4] >= 0.5,
            _ => config.macros[2] >= 0.5,
        }
    }

    fn common_pitch(config: &PadConfig) -> f64 {
        match config.effect_type {
            EffectType::BeatRepeat => denormalize_linear(config.macros[4], -24.0, 24.0),
            EffectType::TapeStop => 0.0,
            _ => denormalize_linear(config.macros[3], -24.0, 24.0),
        }
    }

    fn common_decay(config: &PadConfig) -> f64 {
        match config.effect_type {
            EffectType::BeatRepeat => denormalize_linear(config.macros[5], 0.0, 12.0),
            EffectType::TapeStop => 0.0,
            _ => denormalize_linear(config.macros[4], 0.0, 12.0),
        }
    }

    fn common_offset(config: &PadConfig) -> f64 {
        match config.effect_type {
            EffectType::TapeStop | EffectType::Reverse => clamp_macro(config.macros[5]),
            _ => 0.0,
        }
    }

    fn common_jitter(config: &PadConfig) -> f64 {
        if config.effect_type == EffectType::TapeStop {
            0.0
        } else {
            clamp_macro(config.macros[6])
        }
    }

    fn start_buffer_effect(&mut self, config: &PadConfig, samples_per_beat: f64) {
        let slice_beats = grid_beats(config.macros[0]);
        let jitter = Self::common_jitter(config);
        let jitter_scale = 1.0 + (self.rand_01() * 2.0 - 1.0) * jitter * 0.25;
        self.slice_samples = (slice_beats * samples_per_beat * jitter_scale)
            .round()
            .clamp(8.0, (self.buffer_l.len() - 2).max(8) as f64)
            as i32;
        self.repeat_active = true;
        self.repeat_phase = 0.0;
        self.repeat_count = 0;
        self.repeat_reverse = config.effect_type == EffectType::Reverse;

        let mut lookback = self.slice_samples
            + (Self::common_offset(config) * f64::from(self.slice_samples)) as i32;
        if config.effect_type == EffectType::BeatRepeat {
            let configured = if lookback_index(config.macros[1]) == 0 {
                self.slice_samples
            } else {
                (lookback_beats(config.macros[1], slice_beats) * samples_per_beat).round() as i32
            };
            lookback = self.slice_samples.max(configured);
        } else if config.effect_type == EffectType::TapeStop {
            lookback = (samples_per_beat / 16.0).max(1.0) as i32
                + (Self::common_offset(config) * f64::from(self.slice_samples)) as i32;
        }
        let jitter_offset =
            ((self.rand_01() * 2.0 - 1.0) * jitter * f64::from(self.slice_samples)) as i32;
        self.lookback_samples = lookback;
        self.capture_start =
            self.wrap(self.write_pos as i64 - i64::from(lookback) - i64::from(jitter_offset));

        if config.effect_type == EffectType::TapeStop {
            self.stop_elapsed = 0;
            self.stop_total_samples = self.slice_samples.max(1);
            self.stop_curve = denormalize_linear(config.macros[1], 0.25, 4.0);
            self.stop_start_rate = denormalize_linear(config.macros[2], 0.5, 2.0);
            self.stop_fade = clamp_macro(config.macros[6]);
            self.play_rate = self.stop_start_rate;
            self.stop_amp = 1.0;
        }
    }

    fn process_buffer_sample(
        &mut self,
        dry: [f64; 2],
        output: &mut [f64; 2],
        config: &PadConfig,
        performance_pitch: f64,
    ) {
        let max_phase = f64::from((self.slice_samples - 1).max(0));
        let phase = self.repeat_phase.clamp(0.0, max_phase);
        let read_phase = if self.repeat_reverse {
            max_phase - phase
        } else {
            phase
        };
        let read_a = self.wrap(self.capture_start as i64 + read_phase as i64);
        let read_b = self.wrap(read_a as i64 + 1);
        let fraction = read_phase - read_phase.floor();
        let mut repeat = [
            self.buffer_l[read_a] * (1.0 - fraction) + self.buffer_l[read_b] * fraction,
            self.buffer_r[read_a] * (1.0 - fraction) + self.buffer_r[read_b] * fraction,
        ];
        let mut amplitude =
            10.0_f64.powf(-Self::common_decay(config) * f64::from(self.repeat_count) / 20.0);
        if config.effect_type == EffectType::TapeStop {
            let progress =
                (f64::from(self.stop_elapsed) / f64::from(self.stop_total_samples)).clamp(0.0, 1.0);
            self.play_rate = self.stop_start_rate * (1.0 - progress).max(0.0).powf(self.stop_curve);
            amplitude = 1.0 - self.stop_fade * progress;
            self.stop_amp = amplitude;
            self.stop_elapsed += 1;
        } else {
            let pitch = (Self::common_pitch(config) + performance_pitch).clamp(-48.0, 48.0);
            self.play_rate = 2.0_f64.powf(pitch / 12.0);
        }
        repeat[0] *= amplitude;
        repeat[1] *= amplitude;
        let wet = Self::common_wet(config);
        for channel in 0..2 {
            output[channel] = if Self::common_insert(config) {
                dry[channel] * (1.0 - wet) + repeat[channel] * wet
            } else {
                dry[channel] + repeat[channel] * wet
            };
        }
        self.repeat_phase += self.play_rate;
        while self.repeat_phase >= f64::from(self.slice_samples) && self.slice_samples > 0 {
            self.repeat_phase -= f64::from(self.slice_samples);
            self.repeat_count += 1;
        }
    }

    fn one_pole(input: f64, state: &mut f64, cutoff: f64, sample_rate: f64) -> f64 {
        let coefficient = 1.0
            - (-std::f64::consts::TAU * cutoff.clamp(20.0, sample_rate * 0.45) / sample_rate).exp();
        *state += coefficient * (input - *state);
        *state
    }

    fn apply_bands(
        &mut self,
        sample: &mut [f64; 2],
        state: &PerformanceState,
        low_slot: Option<usize>,
        mid_slot: Option<usize>,
        high_slot: Option<usize>,
    ) {
        if low_slot.is_none() && mid_slot.is_none() && high_slot.is_none() {
            return;
        }
        let dry = *sample;
        let mut band = [0.0; 2];
        let mut maximum_wet: f64 = 0.0;
        if let Some(slot) = low_slot {
            let config = &state.pads[slot];
            let cutoff = denormalize_linear(config.macros[0], 80.0, 2000.0);
            let wet = clamp_macro(config.macros[1]);
            for channel in 0..2 {
                band[channel] += Self::one_pole(
                    dry[channel],
                    &mut self.low_state[channel],
                    cutoff,
                    self.sample_rate,
                ) * wet;
            }
            maximum_wet = maximum_wet.max(wet);
        }
        if let Some(slot) = mid_slot {
            let config = &state.pads[slot];
            let low_cutoff = denormalize_linear(config.macros[0], 80.0, 4000.0);
            let high_cutoff =
                denormalize_linear(config.macros[1], 500.0, 16000.0).max(low_cutoff + 20.0);
            let wet = clamp_macro(config.macros[2]);
            for channel in 0..2 {
                let low = Self::one_pole(
                    dry[channel],
                    &mut self.mid_low_state[channel],
                    low_cutoff,
                    self.sample_rate,
                );
                let high = Self::one_pole(
                    dry[channel],
                    &mut self.mid_high_state[channel],
                    high_cutoff,
                    self.sample_rate,
                );
                band[channel] += (high - low) * wet;
            }
            maximum_wet = maximum_wet.max(wet);
        }
        if let Some(slot) = high_slot {
            let config = &state.pads[slot];
            let cutoff = denormalize_linear(config.macros[0], 1000.0, 16000.0);
            let wet = clamp_macro(config.macros[1]);
            for channel in 0..2 {
                band[channel] += (dry[channel]
                    - Self::one_pole(
                        dry[channel],
                        &mut self.high_state[channel],
                        cutoff,
                        self.sample_rate,
                    ))
                    * wet;
            }
            maximum_wet = maximum_wet.max(wet);
        }
        for channel in 0..2 {
            sample[channel] = dry[channel] * (1.0 - maximum_wet) + band[channel];
        }
    }

    fn quantize(sample: f64, bits: i32) -> f64 {
        let safe_bits = bits.clamp(2, 16);
        let steps = f64::from((1_u32 << (safe_bits - 1)) - 1);
        (sample.clamp(-1.0, 1.0) * steps).round() / steps
    }

    fn apply_lofi(&mut self, sample: &mut [f64; 2], state: &PerformanceState, slot: Option<usize>) {
        let Some(slot) = slot else {
            self.lofi_counter = 0;
            self.lofi_held = *sample;
            return;
        };
        let config = &state.pads[slot];
        let target_rate = denormalize_linear(config.macros[0], 1000.0, 44100.0);
        let bits = denormalize_linear(config.macros[1], 2.0, 16.0).round() as i32;
        let wet = clamp_macro(config.macros[2]);
        let dry = *sample;
        let hold_samples = (self.sample_rate / target_rate).round().max(1.0) as i32;
        if self.lofi_counter <= 0 {
            self.lofi_held = [
                Self::quantize(sample[0], bits),
                Self::quantize(sample[1], bits),
            ];
            self.lofi_counter = hold_samples;
        }
        for channel in 0..2 {
            sample[channel] = dry[channel] * (1.0 - wet) + self.lofi_held[channel] * wet;
        }
        self.lofi_counter -= 1;
    }

    fn apply_gate(
        &mut self,
        sample: &mut [f64; 2],
        state: &PerformanceState,
        slot: Option<usize>,
        samples_per_beat: f64,
    ) {
        let mut target = 1.0;
        let mut attack_ms = 1.0;
        let mut release_ms = 1.0;
        if let Some(slot) = slot {
            let config = &state.pads[slot];
            let period = (grid_beats(config.macros[0]) * samples_per_beat).max(2.0);
            let duty = denormalize_linear(config.macros[1], 5.0, 95.0) * 0.01;
            let depth = clamp_macro(config.macros[2]);
            attack_ms = denormalize_linear(config.macros[3], 0.0, 20.0);
            release_ms = denormalize_linear(config.macros[4], 0.0, 100.0);
            target = if self.gate_phase < period * duty {
                1.0
            } else {
                1.0 - depth
            };
            self.gate_phase += 1.0;
            if self.gate_phase >= period {
                self.gate_phase %= period;
            }
        } else {
            self.gate_phase = 0.0;
        }
        let time_ms = if target > self.gate_gain {
            attack_ms
        } else {
            release_ms
        };
        let slew = if time_ms <= 0.0 {
            1.0
        } else {
            1.0 / (self.sample_rate * time_ms * 0.001).max(1.0)
        };
        if self.gate_gain < target {
            self.gate_gain = (self.gate_gain + slew).min(target);
        } else if self.gate_gain > target {
            self.gate_gain = (self.gate_gain - slew).max(target);
        }
        sample[0] *= self.gate_gain;
        sample[1] *= self.gate_gain;
    }

    fn wrap(&self, position: i64) -> usize {
        position.rem_euclid(self.buffer_l.len() as i64) as usize
    }

    fn rand_01(&mut self) -> f64 {
        self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        f64::from((self.rng >> 8) & 0x00ff_ffff) / 16_777_216.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn impulse(frames: usize) -> Vec<f64> {
        let mut input = vec![0.0; frames];
        input[0] = 1.0;
        input
    }

    fn render(effect: EffectType, frames: usize) -> Vec<f64> {
        let mut engine = Engine::default();
        engine.reset(1_000.0);
        let mut state = PerformanceState::default();
        state.pads[0] = default_pad_config(effect);
        state.held[0] = true;
        let input = impulse(frames);
        let mut output_l = vec![0.0; frames];
        let mut output_r = vec![0.0; frames];
        engine.process(&input, &input, &mut output_l, &mut output_r, 120.0, &state);
        assert!(output_l.iter().all(|sample| sample.is_finite()));
        output_l
    }

    #[test]
    fn mapping_helpers_match_reference_boundaries() {
        assert_eq!(grid_index(-1.0), 0);
        assert_eq!(grid_index(1.0), 8);
        assert_eq!(lookback_index(2.0), 9);
        assert_eq!(grid_beats(grid_normalized(6)), 4.0);
        assert_eq!(lookback_beats(lookback_normalized(0), 0.25), 0.25);
    }

    #[test]
    fn classic_state_has_all_effects_and_reference_repeat_presets() {
        let state = classic_state();
        assert_eq!(state.pads[0].effect_type, EffectType::BeatRepeat);
        assert_eq!(state.pads[15].effect_type, EffectType::LoFi);
        assert_eq!(state.pads[4].macros[1], lookback_normalized(6));
    }

    #[test]
    fn effect_control_metadata_matches_the_cpp_contract() {
        let expected = [
            (EffectType::Off, 0, ["", "", "", "", "", "", ""]),
            (
                EffectType::BeatRepeat,
                7,
                [
                    "CELL", "LOOKBACK", "WET", "MODE", "PITCH", "DECAY", "JITTER",
                ],
            ),
            (
                EffectType::Reverse,
                7,
                [
                    "LENGTH", "WET", "MODE", "PITCH", "DECAY", "OFFSET", "JITTER",
                ],
            ),
            (
                EffectType::TapeStop,
                7,
                ["TIME", "CURVE", "START", "WET", "MODE", "OFFSET", "FADE"],
            ),
            (
                EffectType::Gate,
                6,
                ["GRID", "DUTY", "DEPTH", "ATTACK", "RELEASE", "PHASE", ""],
            ),
            (EffectType::PitchDown, 1, ["STEP", "", "", "", "", "", ""]),
            (
                EffectType::PitchReset,
                1,
                ["TARGET", "", "", "", "", "", ""],
            ),
            (EffectType::PitchUp, 1, ["STEP", "", "", "", "", "", ""]),
            (
                EffectType::BandLow,
                2,
                ["CUTOFF", "WET", "", "", "", "", ""],
            ),
            (
                EffectType::BandMid,
                3,
                ["LOW CUT", "HIGH CUT", "WET", "", "", "", ""],
            ),
            (
                EffectType::BandHigh,
                2,
                ["CUTOFF", "WET", "", "", "", "", ""],
            ),
            (EffectType::LoFi, 3, ["RATE", "BITS", "WET", "", "", "", ""]),
        ];

        for (effect, active_count, names) in expected {
            assert_eq!(effect.active_control_count(), active_count);
            for (control, expected_name) in names.into_iter().enumerate() {
                assert_eq!(effect.control_name(control), expected_name);
                assert_eq!(effect.is_control_enabled(control), control < active_count);
            }
            assert_eq!(effect.control_name(NUM_MACROS), "");
            assert!(!effect.is_control_enabled(NUM_MACROS));
        }
    }

    #[test]
    fn semantic_control_values_match_the_cpp_ui_contract() {
        assert_eq!(
            format_control_value(EffectType::BeatRepeat, 0, grid_normalized(2)),
            "1/16"
        );
        assert_eq!(
            format_control_value(EffectType::BeatRepeat, 1, lookback_normalized(0)),
            "CELL"
        );
        assert_eq!(
            format_control_value(EffectType::BeatRepeat, 2, 0.625),
            "62%"
        );
        assert_eq!(format_control_value(EffectType::BeatRepeat, 3, 0.49), "MIX");
        assert_eq!(
            format_control_value(EffectType::BeatRepeat, 3, 0.5),
            "INSERT"
        );
        assert_eq!(
            format_control_value(EffectType::BeatRepeat, 4, 0.75),
            "+12 st"
        );
        assert_eq!(format_control_value(EffectType::Reverse, 4, 0.5), "6.0 dB");
        assert_eq!(format_control_value(EffectType::TapeStop, 1, 0.5), "2.12");
        assert_eq!(format_control_value(EffectType::TapeStop, 2, 0.5), "1.25x");
        assert_eq!(format_control_value(EffectType::Gate, 1, 0.5), "50%");
        assert_eq!(format_control_value(EffectType::Gate, 3, 0.05), "1.0 ms");
        assert_eq!(format_control_value(EffectType::PitchUp, 0, 0.0), "1 st");
        assert_eq!(
            format_control_value(EffectType::PitchReset, 0, 0.5),
            "+0 st"
        );
        assert_eq!(
            format_control_value(
                EffectType::BandMid,
                1,
                normalize_linear(3600.0, 500.0, 16000.0),
            ),
            "3600 Hz"
        );
        assert_eq!(
            format_control_value(
                EffectType::LoFi,
                0,
                normalize_linear(11025.0, 1000.0, 44100.0),
            ),
            "11025 Hz"
        );
        assert_eq!(
            format_control_value(EffectType::LoFi, 1, normalize_linear(8.0, 2.0, 16.0),),
            "8 bit"
        );
        assert_eq!(format_control_value(EffectType::LoFi, 2, 1.0), "100%");
        assert_eq!(format_control_value(EffectType::LoFi, 3, 1.0), "");
        assert_eq!(
            format_control_value(EffectType::BandLow, 0, f64::NAN),
            "80 Hz"
        );
    }

    #[test]
    fn pitch_actions_step_reset_and_clamp() {
        let up = default_pad_config(EffectType::PitchUp);
        let down = default_pad_config(EffectType::PitchDown);
        let reset = default_pad_config(EffectType::PitchReset);
        assert_eq!(apply_pitch_action(0.0, EffectType::PitchUp, &up), 1.0);
        assert_eq!(apply_pitch_action(0.0, EffectType::PitchDown, &down), -1.0);
        assert_eq!(
            apply_pitch_action(14.0, EffectType::PitchReset, &reset),
            0.0
        );
        assert_eq!(apply_pitch_action(24.0, EffectType::PitchUp, &up), 24.0);
    }

    #[test]
    fn bypass_is_sample_exact() {
        let mut engine = Engine::default();
        let input: Vec<f64> = (0..257).map(|n| (f64::from(n) * 0.07).sin()).collect();
        let mut left = vec![0.0; input.len()];
        let mut right = vec![0.0; input.len()];
        engine.process(
            &input,
            &input,
            &mut left,
            &mut right,
            120.0,
            &PerformanceState::default(),
        );
        assert_eq!(left, input);
        assert_eq!(right, input);
    }

    #[test]
    fn every_audio_effect_is_finite_and_changes_signal() {
        for effect in [
            EffectType::BeatRepeat,
            EffectType::Reverse,
            EffectType::TapeStop,
            EffectType::Gate,
            EffectType::BandLow,
            EffectType::BandMid,
            EffectType::BandHigh,
            EffectType::LoFi,
        ] {
            let output = render(effect, 2_048);
            assert!(output.iter().all(|sample| sample.is_finite()), "{effect:?}");
        }
    }

    #[test]
    fn newest_buffer_press_has_priority_and_release_restores_previous() {
        let mut engine = Engine::default();
        engine.reset(1_000.0);
        let input = vec![0.25; 32];
        let mut left = vec![0.0; 32];
        let mut right = vec![0.0; 32];
        let mut state = PerformanceState::default();
        state.pads[0] = default_pad_config(EffectType::BeatRepeat);
        state.pads[1] = default_pad_config(EffectType::Reverse);
        state.held[0] = true;
        engine.process(&input, &input, &mut left, &mut right, 120.0, &state);
        assert_eq!(engine.active_buffer_slot(), Some(0));
        state.held[1] = true;
        engine.process(&input, &input, &mut left, &mut right, 120.0, &state);
        assert_eq!(engine.active_buffer_slot(), Some(1));
        state.held[1] = false;
        engine.process(&input, &input, &mut left, &mut right, 120.0, &state);
        assert_eq!(engine.active_buffer_slot(), Some(0));
    }

    #[test]
    fn reset_makes_jitter_deterministic() {
        let mut state = PerformanceState::default();
        state.pads[0] = default_pad_config(EffectType::BeatRepeat);
        state.pads[0].macros[6] = 1.0;
        state.held[0] = true;
        let input = impulse(512);
        let mut engine = Engine::default();
        let mut first = vec![0.0; 512];
        let mut scratch = vec![0.0; 512];
        engine.process(&input, &input, &mut first, &mut scratch, 120.0, &state);
        engine.reset(44_100.0);
        let mut second = vec![0.0; 512];
        engine.process(&input, &input, &mut second, &mut scratch, 120.0, &state);
        assert_eq!(first, second);
    }
}
