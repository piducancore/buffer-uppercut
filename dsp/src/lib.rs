//! Framework-independent Buffer Uppercut DSP.
//!
//! Behavior is protected by the repository's frozen regression corpus.
//! Allocation is confined to [`Engine::reset`]; [`Engine::process`] only
//! mutates already-prepared storage.

pub const NUM_PADS: usize = 16;
pub const NUM_MACROS: usize = 7;
pub const MAX_ACTIVE_PROCESSORS: usize = 6;
const HISTORY_SECONDS: f64 = 8.0;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum VisualizationMode {
    #[default]
    Rolling,
    Capture,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VisualizationBin {
    pub min_l: f32,
    pub max_l: f32,
    pub min_r: f32,
    pub max_r: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VisualizationMeta {
    pub mode: VisualizationMode,
    pub active_pad: Option<usize>,
    pub active_effect: EffectType,
    pub normalized_playhead: f32,
    pub reverse: bool,
    pub play_rate: f32,
    pub duration_seconds: f32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AdmissionMeta {
    pub held_mask: u16,
    pub active_mask: u16,
    pub suspended_mask: u16,
}

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
            Self::BandLow | Self::BandMid | Self::BandHigh => NUM_MACROS,
            Self::LoFi => 3,
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
            Self::BandLow | Self::BandHigh => [
                "CUTOFF",
                "RESONANCE",
                "DRIVE",
                "ENVELOPE",
                "MOTION",
                "FEEDBACK",
                "WET",
            ][control],
            Self::BandMid => [
                "CENTER",
                "WIDTH",
                "RESONANCE",
                "DRIVE",
                "ENVELOPE",
                "MOTION",
                "WET",
            ][control],
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
        EffectType::BandLow | EffectType::BandHigh => match control {
            0 => format!("{:.0} Hz", filter_frequency(normalized)),
            1 | 3 | 4 | 5 | 6 => percent(normalized),
            _ => format!("{:.1} dB", denormalize_linear(normalized, 0.0, 30.0)),
        },
        EffectType::BandMid => match control {
            0 => format!("{:.0} Hz", filter_frequency(normalized)),
            1 | 2 | 4 | 5 | 6 => percent(normalized),
            _ => format!("{:.1} dB", denormalize_linear(normalized, 0.0, 30.0)),
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
pub fn filter_frequency(normalized: f64) -> f64 {
    20.0 * (1000.0_f64).powf(clamp_macro(normalized))
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
                normalize_linear(260.0_f64.ln(), 20.0_f64.ln(), 20000.0_f64.ln()),
                0.35,
                0.2,
                0.0,
                0.0,
                0.15,
                1.0,
            ];
        }
        EffectType::BandMid => {
            config.macros = [
                normalize_linear(1200.0_f64.ln(), 20.0_f64.ln(), 20000.0_f64.ln()),
                0.5,
                0.35,
                0.2,
                0.0,
                0.0,
                1.0,
            ];
        }
        EffectType::BandHigh => {
            config.macros = [
                normalize_linear(3600.0_f64.ln(), 20.0_f64.ln(), 20000.0_f64.ln()),
                0.35,
                0.2,
                0.0,
                0.0,
                0.15,
                1.0,
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

struct BufferState {
    history_l: Vec<f32>,
    history_r: Vec<f32>,
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
    seed: u32,
}

impl BufferState {
    fn new(slot: usize) -> Self {
        let seed = 0x1234_5678 ^ (slot as u32).wrapping_mul(0x9e37_79b9);
        Self {
            history_l: Vec::new(),
            history_r: Vec::new(),
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
            rng: seed,
            seed,
        }
    }

    fn prepare(&mut self, length: usize) {
        self.history_l.clear();
        self.history_l.resize(length, 0.0);
        self.history_r.clear();
        self.history_r.resize(length, 0.0);
        self.clear_history();
    }

    fn clear_history(&mut self) {
        self.history_l.fill(0.0);
        self.history_r.fill(0.0);
        self.write_pos = 0;
        self.reset_capture();
    }

    fn reset_capture(&mut self) {
        self.repeat_active = false;
        self.repeat_phase = 0.0;
        self.repeat_count = 0;
        self.capture_start = self.write_pos;
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
        self.rng = self.seed;
    }

    fn wrap(&self, position: i64) -> usize {
        position.rem_euclid(self.history_l.len() as i64) as usize
    }

    fn record(&mut self, sample: [f64; 2]) {
        self.history_l[self.write_pos] = sample[0] as f32;
        self.history_r[self.write_pos] = sample[1] as f32;
    }

    fn advance(&mut self) {
        self.write_pos = self.wrap(self.write_pos as i64 + 1);
    }

    fn rand_01(&mut self) -> f64 {
        self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        f64::from((self.rng >> 8) & 0x00ff_ffff) / 16_777_216.0
    }
}

struct SlotRuntime {
    configured_type: EffectType,
    held: bool,
    active: bool,
    suspended: bool,
    request_order: u64,
    admission_order: u64,
    processor_ready: bool,
    buffer: BufferState,
    filter_a: [f64; 2],
    filter_b: [f64; 2],
    filter_envelope: f64,
    filter_phase: f64,
    filter_feedback: [f64; 2],
    lofi_held: [f64; 2],
    lofi_counter: i32,
    gate_phase: f64,
    gate_gain: f64,
}

impl SlotRuntime {
    fn new(slot: usize) -> Self {
        Self {
            configured_type: EffectType::Off,
            held: false,
            active: false,
            suspended: false,
            request_order: 0,
            admission_order: 0,
            processor_ready: false,
            buffer: BufferState::new(slot),
            filter_a: [0.0; 2],
            filter_b: [0.0; 2],
            filter_envelope: 0.0,
            filter_phase: 0.0,
            filter_feedback: [0.0; 2],
            lofi_held: [0.0; 2],
            lofi_counter: 0,
            gate_phase: 0.0,
            gate_gain: 1.0,
        }
    }

    fn reset_processor(&mut self) {
        self.buffer.reset_capture();
        self.filter_a = [0.0; 2];
        self.filter_b = [0.0; 2];
        self.filter_envelope = 0.0;
        self.filter_phase = 0.0;
        self.filter_feedback = [0.0; 2];
        self.lofi_held = [0.0; 2];
        self.lofi_counter = 0;
        self.gate_phase = 0.0;
        self.gate_gain = 1.0;
        self.processor_ready = false;
    }

    fn clear_runtime(&mut self) {
        self.configured_type = EffectType::Off;
        self.held = false;
        self.active = false;
        self.suspended = false;
        self.request_order = 0;
        self.admission_order = 0;
        self.reset_processor();
        self.buffer.clear_history();
    }
}

pub struct Engine {
    sample_rate: f64,
    slots: [SlotRuntime; NUM_PADS],
    rolling_l: Vec<f32>,
    rolling_r: Vec<f32>,
    rolling_write_pos: usize,
    request_counter: u64,
    admission_counter: u64,
    selected_slot: Option<usize>,
}

impl Default for Engine {
    fn default() -> Self {
        let mut engine = Self {
            sample_rate: 44_100.0,
            slots: std::array::from_fn(SlotRuntime::new),
            rolling_l: Vec::new(),
            rolling_r: Vec::new(),
            rolling_write_pos: 0,
            request_counter: 0,
            admission_counter: 0,
            selected_slot: None,
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
        self.sample_rate = if sample_rate.is_finite() {
            sample_rate.max(1.0)
        } else {
            44_100.0
        };
        let required = (self.sample_rate * HISTORY_SECONDS).ceil() as usize;
        let length = required.max(16).next_power_of_two();
        self.rolling_l.clear();
        self.rolling_l.resize(length, 0.0);
        self.rolling_r.clear();
        self.rolling_r.resize(length, 0.0);
        for slot in &mut self.slots {
            slot.buffer.prepare(length);
        }
        self.clear_transient();
    }

    /// Clears all runtime state and histories without resizing prepared storage.
    pub fn clear_transient(&mut self) {
        self.rolling_l.fill(0.0);
        self.rolling_r.fill(0.0);
        self.rolling_write_pos = 0;
        self.request_counter = 0;
        self.admission_counter = 0;
        self.selected_slot = None;
        for slot in &mut self.slots {
            slot.clear_runtime();
        }
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
        let tempo = if tempo.is_finite() {
            tempo.max(1.0)
        } else {
            120.0
        };
        let samples_per_beat = self.sample_rate * 60.0 / tempo;
        self.resolve_slots(state, samples_per_beat);

        for sample in 0..frames {
            let input = [
                Self::finite_f64(input_l[sample]),
                Self::finite_f64(input_r[sample]),
            ];
            self.rolling_l[self.rolling_write_pos] = input[0] as f32;
            self.rolling_r[self.rolling_write_pos] = input[1] as f32;
            self.rolling_write_pos =
                Self::wrap_length(self.rolling_write_pos as i64 + 1, self.rolling_l.len());

            let mut signal = input;
            for slot_index in 0..NUM_PADS {
                let config = state.pads[slot_index];
                let slot = &mut self.slots[slot_index];
                let stage_input = signal;
                if config.effect_type.is_buffer() {
                    slot.buffer.record(stage_input);
                }
                if slot.active {
                    signal = Self::process_slot(
                        slot,
                        stage_input,
                        &config,
                        self.sample_rate,
                        samples_per_beat,
                        state.performance_pitch,
                    );
                }
                if config.effect_type.is_buffer() {
                    slot.buffer.advance();
                }
            }
            output_l[sample] = Self::finite_f64(signal[0]);
            output_r[sample] = Self::finite_f64(signal[1]);
        }
    }

    fn resolve_slots(&mut self, state: &PerformanceState, samples_per_beat: f64) {
        let mut new_continuous_request = [false; NUM_PADS];

        for (slot_index, request) in new_continuous_request.iter_mut().enumerate() {
            let old_type = self.slots[slot_index].configured_type;
            let new_type = state.pads[slot_index].effect_type;
            if old_type == new_type {
                continue;
            }

            let old_continuous = Self::is_continuous(old_type);
            let new_continuous = Self::is_continuous(new_type);
            if !old_type.is_buffer() && new_type.is_buffer() {
                self.slots[slot_index].buffer.clear_history();
            } else {
                self.slots[slot_index].buffer.reset_capture();
            }
            self.slots[slot_index].filter_a = [0.0; 2];
            self.slots[slot_index].filter_b = [0.0; 2];
            self.slots[slot_index].filter_envelope = 0.0;
            self.slots[slot_index].filter_phase = 0.0;
            self.slots[slot_index].filter_feedback = [0.0; 2];
            self.slots[slot_index].lofi_held = [0.0; 2];
            self.slots[slot_index].lofi_counter = 0;
            self.slots[slot_index].gate_phase = 0.0;
            self.slots[slot_index].gate_gain = 1.0;
            self.slots[slot_index].processor_ready = false;
            self.slots[slot_index].configured_type = new_type;

            if old_continuous && !new_continuous {
                self.remove_from_admission(slot_index);
            } else if !old_continuous && new_continuous && state.held[slot_index] {
                *request = true;
            } else if old_continuous && new_continuous && self.slots[slot_index].active {
                self.start_processor(slot_index, &state.pads[slot_index], samples_per_beat);
            }
        }

        for slot_index in 0..NUM_PADS {
            if self.slots[slot_index].held && !state.held[slot_index] {
                self.remove_from_admission(slot_index);
                self.slots[slot_index].reset_processor();
            }
        }

        self.restore_suspended(state, samples_per_beat);

        for (slot_index, new_request) in new_continuous_request.into_iter().enumerate() {
            let rising = state.held[slot_index] && !self.slots[slot_index].held;
            if (rising || new_request) && Self::is_continuous(state.pads[slot_index].effect_type) {
                self.admit_new_request(slot_index, &state.pads[slot_index], samples_per_beat);
            }
            self.slots[slot_index].held = state.held[slot_index];
        }
    }

    fn admit_new_request(&mut self, slot_index: usize, config: &PadConfig, samples_per_beat: f64) {
        self.remove_from_admission(slot_index);
        self.slots[slot_index].reset_processor();
        self.request_counter = self.request_counter.wrapping_add(1);
        self.slots[slot_index].request_order = self.request_counter;
        if self.active_count() >= MAX_ACTIVE_PROCESSORS
            && let Some(oldest) = self.oldest_active_slot()
        {
            self.slots[oldest].active = false;
            self.slots[oldest].suspended = true;
            self.slots[oldest].admission_order = 0;
        }
        self.activate_slot(slot_index, config, samples_per_beat);
    }

    fn restore_suspended(&mut self, state: &PerformanceState, samples_per_beat: f64) {
        while self.active_count() < MAX_ACTIVE_PROCESSORS {
            let mut candidate = None;
            let mut newest_request = 0;
            for slot_index in 0..NUM_PADS {
                let slot = &self.slots[slot_index];
                if slot.suspended && state.held[slot_index] && slot.request_order >= newest_request
                {
                    candidate = Some(slot_index);
                    newest_request = slot.request_order;
                }
            }
            let Some(slot_index) = candidate else {
                break;
            };
            self.activate_slot(slot_index, &state.pads[slot_index], samples_per_beat);
        }
    }

    fn activate_slot(&mut self, slot_index: usize, config: &PadConfig, samples_per_beat: f64) {
        self.admission_counter = self.admission_counter.wrapping_add(1);
        let slot = &mut self.slots[slot_index];
        slot.active = true;
        slot.suspended = false;
        slot.admission_order = self.admission_counter;
        if !slot.processor_ready {
            self.start_processor(slot_index, config, samples_per_beat);
        }
    }

    fn start_processor(&mut self, slot_index: usize, config: &PadConfig, samples_per_beat: f64) {
        let slot = &mut self.slots[slot_index];
        match config.effect_type {
            EffectType::BeatRepeat | EffectType::Reverse | EffectType::TapeStop => {
                Self::start_buffer_effect(&mut slot.buffer, config, samples_per_beat);
            }
            EffectType::Gate => {
                let period = (grid_beats(config.macros[0]) * samples_per_beat).max(2.0);
                slot.gate_phase = clamp_macro(config.macros[5]) * period;
            }
            _ => {}
        }
        slot.processor_ready = true;
    }

    fn remove_from_admission(&mut self, slot_index: usize) {
        let slot = &mut self.slots[slot_index];
        slot.active = false;
        slot.suspended = false;
        slot.request_order = 0;
        slot.admission_order = 0;
    }

    fn active_count(&self) -> usize {
        self.slots.iter().filter(|slot| slot.active).count()
    }

    fn oldest_active_slot(&self) -> Option<usize> {
        let mut oldest = None;
        let mut oldest_order = u64::MAX;
        for (slot_index, slot) in self.slots.iter().enumerate() {
            if slot.active && slot.admission_order < oldest_order {
                oldest = Some(slot_index);
                oldest_order = slot.admission_order;
            }
        }
        oldest
    }

    const fn is_continuous(effect: EffectType) -> bool {
        !matches!(
            effect,
            EffectType::Off | EffectType::PitchDown | EffectType::PitchReset | EffectType::PitchUp
        )
    }

    fn process_slot(
        slot: &mut SlotRuntime,
        stage_input: [f64; 2],
        config: &PadConfig,
        sample_rate: f64,
        samples_per_beat: f64,
        performance_pitch: f64,
    ) -> [f64; 2] {
        match config.effect_type {
            EffectType::BeatRepeat | EffectType::Reverse | EffectType::TapeStop => {
                Self::process_buffer_sample(
                    &mut slot.buffer,
                    stage_input,
                    config,
                    performance_pitch,
                )
            }
            EffectType::Gate => {
                Self::apply_gate(slot, stage_input, config, sample_rate, samples_per_beat)
            }
            EffectType::BandLow | EffectType::BandMid | EffectType::BandHigh => {
                Self::apply_band(slot, stage_input, config, sample_rate)
            }
            EffectType::LoFi => Self::apply_lofi(slot, stage_input, config, sample_rate),
            _ => stage_input,
        }
    }

    #[must_use]
    pub fn active_buffer_slot(&self) -> Option<usize> {
        self.slots.iter().position(|slot| {
            slot.active && slot.configured_type.is_buffer() && slot.buffer.repeat_active
        })
    }

    #[must_use]
    pub fn repeat_active(&self) -> bool {
        self.visualization_buffer_slot()
            .is_some_and(|slot| self.slots[slot].buffer.repeat_active)
    }

    #[must_use]
    pub fn gate_gain(&self) -> f64 {
        self.slots
            .iter()
            .find(|slot| slot.active && slot.configured_type == EffectType::Gate)
            .map_or(1.0, |slot| slot.gate_gain)
    }

    #[must_use]
    pub fn slice_samples(&self) -> i32 {
        self.visualization_buffer_slot()
            .map_or(1, |slot| self.slots[slot].buffer.slice_samples)
    }

    #[must_use]
    pub fn lookback_samples(&self) -> i32 {
        self.visualization_buffer_slot()
            .map_or(1, |slot| self.slots[slot].buffer.lookback_samples)
    }

    #[must_use]
    pub fn play_rate(&self) -> f64 {
        self.visualization_buffer_slot()
            .map_or(1.0, |slot| self.slots[slot].buffer.play_rate)
    }

    #[must_use]
    pub fn history_bytes(&self) -> usize {
        let rolling = (self.rolling_l.len() + self.rolling_r.len()) * std::mem::size_of::<f32>();
        rolling
            + self
                .slots
                .iter()
                .map(|slot| {
                    (slot.buffer.history_l.len() + slot.buffer.history_r.len())
                        * std::mem::size_of::<f32>()
                })
                .sum::<usize>()
    }

    #[must_use]
    pub fn admission_meta(&self) -> AdmissionMeta {
        let mut meta = AdmissionMeta::default();
        for (slot_index, slot) in self.slots.iter().enumerate() {
            let bit = 1_u16 << slot_index;
            if slot.held {
                meta.held_mask |= bit;
            }
            if slot.active {
                meta.active_mask |= bit;
            }
            if slot.suspended {
                meta.suspended_mask |= bit;
            }
        }
        meta
    }

    pub fn set_visualization_selected_slot(&mut self, selected_slot: Option<usize>) {
        self.selected_slot = selected_slot.filter(|slot| *slot < NUM_PADS);
    }

    fn visualization_buffer_slot(&self) -> Option<usize> {
        if let Some(slot_index) = self.selected_slot {
            let slot = &self.slots[slot_index];
            if slot.held && slot.configured_type.is_buffer() && slot.buffer.repeat_active {
                return Some(slot_index);
            }
        }
        self.active_buffer_slot()
    }

    /// Downsample prepared stereo history into caller-owned bins without allocation.
    #[must_use]
    pub fn fill_visualization(
        &self,
        tempo: f64,
        bins: &mut [VisualizationBin],
    ) -> VisualizationMeta {
        if let Some(slot_index) = self.visualization_buffer_slot() {
            let slot = &self.slots[slot_index];
            let buffer = &slot.buffer;
            let history_len = buffer.history_l.len().min(buffer.history_r.len());
            let sample_count = (buffer.slice_samples.max(1) as usize).min(history_len);
            Self::fill_visualization_bins(
                &buffer.history_l,
                &buffer.history_r,
                buffer.capture_start,
                sample_count,
                bins,
            );
            let max_phase = (sample_count.saturating_sub(1)) as f64;
            let phase = Self::finite_f64(buffer.repeat_phase).clamp(0.0, max_phase);
            let read_phase = if buffer.repeat_reverse {
                max_phase - phase
            } else {
                phase
            };
            return VisualizationMeta {
                mode: VisualizationMode::Capture,
                active_pad: Some(slot_index),
                active_effect: slot.configured_type,
                normalized_playhead: if max_phase > 0.0 {
                    Self::finite_f32((read_phase / max_phase).clamp(0.0, 1.0))
                } else {
                    0.0
                },
                reverse: buffer.repeat_reverse,
                play_rate: Self::finite_f32(buffer.play_rate),
                duration_seconds: Self::finite_f32(sample_count as f64 / self.sample_rate.max(1.0)),
            };
        }

        let tempo = if tempo.is_finite() {
            tempo.max(1.0)
        } else {
            120.0
        };
        let history_len = self.rolling_l.len().min(self.rolling_r.len());
        let requested = (4.0 * self.sample_rate.max(1.0) * 60.0 / tempo).round() as usize;
        let sample_count = requested.max(1).min(history_len);
        let start = Self::wrap_length(
            self.rolling_write_pos as i64 - sample_count as i64,
            history_len,
        );
        Self::fill_visualization_bins(&self.rolling_l, &self.rolling_r, start, sample_count, bins);
        VisualizationMeta {
            mode: VisualizationMode::Rolling,
            active_pad: None,
            active_effect: EffectType::Off,
            normalized_playhead: 1.0,
            reverse: false,
            play_rate: 1.0,
            duration_seconds: Self::finite_f32(sample_count as f64 / self.sample_rate.max(1.0)),
        }
    }

    fn fill_visualization_bins(
        history_l: &[f32],
        history_r: &[f32],
        start: usize,
        sample_count: usize,
        bins: &mut [VisualizationBin],
    ) {
        if bins.is_empty() || sample_count == 0 || history_l.is_empty() {
            return;
        }

        let history_len = history_l.len().min(history_r.len());
        let bin_count = bins.len();
        for (bin_index, bin) in bins.iter_mut().enumerate() {
            let first_offset = bin_index * sample_count / bin_count;
            let mut end_offset = (bin_index + 1) * sample_count / bin_count;
            if end_offset <= first_offset {
                end_offset = (first_offset + 1).min(sample_count);
            }
            let first_index = Self::wrap_length(start as i64 + first_offset as i64, history_len);
            let first_l = Self::finite_f32(f64::from(history_l[first_index]));
            let first_r = Self::finite_f32(f64::from(history_r[first_index]));
            let mut result = VisualizationBin {
                min_l: first_l,
                max_l: first_l,
                min_r: first_r,
                max_r: first_r,
            };
            for offset in first_offset + 1..end_offset {
                let index = Self::wrap_length(start as i64 + offset as i64, history_len);
                let sample_l = Self::finite_f32(f64::from(history_l[index]));
                let sample_r = Self::finite_f32(f64::from(history_r[index]));
                result.min_l = result.min_l.min(sample_l);
                result.max_l = result.max_l.max(sample_l);
                result.min_r = result.min_r.min(sample_r);
                result.max_r = result.max_r.max(sample_r);
            }
            *bin = result;
        }
    }

    fn finite_f32(value: f64) -> f32 {
        let value = value as f32;
        if value.is_finite() { value } else { 0.0 }
    }

    fn finite_f64(value: f64) -> f64 {
        if value.is_finite() { value } else { 0.0 }
    }

    fn wrap_length(position: i64, length: usize) -> usize {
        position.rem_euclid(length as i64) as usize
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

    fn start_buffer_effect(buffer: &mut BufferState, config: &PadConfig, samples_per_beat: f64) {
        let slice_beats = grid_beats(config.macros[0]);
        let jitter = Self::common_jitter(config);
        let jitter_scale = 1.0 + (buffer.rand_01() * 2.0 - 1.0) * jitter * 0.25;
        let maximum_slice = buffer.history_l.len().saturating_sub(2).max(8);
        buffer.slice_samples = (slice_beats * samples_per_beat * jitter_scale)
            .round()
            .clamp(8.0, maximum_slice as f64) as i32;
        buffer.repeat_active = true;
        buffer.repeat_phase = 0.0;
        buffer.repeat_count = 0;
        buffer.repeat_reverse = config.effect_type == EffectType::Reverse;

        let mut lookback = buffer.slice_samples
            + (Self::common_offset(config) * f64::from(buffer.slice_samples)) as i32;
        if config.effect_type == EffectType::BeatRepeat {
            let configured = if lookback_index(config.macros[1]) == 0 {
                buffer.slice_samples
            } else {
                (lookback_beats(config.macros[1], slice_beats) * samples_per_beat).round() as i32
            };
            lookback = buffer.slice_samples.max(configured);
        } else if config.effect_type == EffectType::TapeStop {
            lookback = (samples_per_beat / 16.0).max(1.0) as i32
                + (Self::common_offset(config) * f64::from(buffer.slice_samples)) as i32;
        }
        let jitter_offset =
            ((buffer.rand_01() * 2.0 - 1.0) * jitter * f64::from(buffer.slice_samples)) as i32;
        buffer.lookback_samples = lookback.clamp(1, maximum_slice as i32);
        buffer.capture_start = buffer.wrap(
            buffer.write_pos as i64 - i64::from(buffer.lookback_samples) - i64::from(jitter_offset),
        );

        if config.effect_type == EffectType::TapeStop {
            buffer.stop_elapsed = 0;
            buffer.stop_total_samples = buffer.slice_samples.max(1);
            buffer.stop_curve = denormalize_linear(config.macros[1], 0.25, 4.0);
            buffer.stop_start_rate = denormalize_linear(config.macros[2], 0.5, 2.0);
            buffer.stop_fade = clamp_macro(config.macros[6]);
            buffer.play_rate = buffer.stop_start_rate;
            buffer.stop_amp = 1.0;
        }
    }

    fn process_buffer_sample(
        buffer: &mut BufferState,
        dry: [f64; 2],
        config: &PadConfig,
        performance_pitch: f64,
    ) -> [f64; 2] {
        let max_phase = f64::from((buffer.slice_samples - 1).max(0));
        let phase = buffer.repeat_phase.clamp(0.0, max_phase);
        let read_phase = if buffer.repeat_reverse {
            max_phase - phase
        } else {
            phase
        };
        let read_a = buffer.wrap(buffer.capture_start as i64 + read_phase as i64);
        let read_b = buffer.wrap(read_a as i64 + 1);
        let fraction = read_phase - read_phase.floor();
        let mut repeat = [
            f64::from(buffer.history_l[read_a]) * (1.0 - fraction)
                + f64::from(buffer.history_l[read_b]) * fraction,
            f64::from(buffer.history_r[read_a]) * (1.0 - fraction)
                + f64::from(buffer.history_r[read_b]) * fraction,
        ];
        let mut amplitude =
            10.0_f64.powf(-Self::common_decay(config) * f64::from(buffer.repeat_count) / 20.0);
        if config.effect_type == EffectType::TapeStop {
            let progress = (f64::from(buffer.stop_elapsed) / f64::from(buffer.stop_total_samples))
                .clamp(0.0, 1.0);
            buffer.play_rate =
                buffer.stop_start_rate * (1.0 - progress).max(0.0).powf(buffer.stop_curve);
            amplitude = 1.0 - buffer.stop_fade * progress;
            buffer.stop_amp = amplitude;
            buffer.stop_elapsed += 1;
        } else {
            let pitch = (Self::common_pitch(config) + performance_pitch).clamp(-48.0, 48.0);
            buffer.play_rate = 2.0_f64.powf(pitch / 12.0);
        }
        repeat[0] *= amplitude;
        repeat[1] *= amplitude;
        let wet = Self::common_wet(config);
        let mut output = dry;
        for channel in 0..2 {
            output[channel] = if Self::common_insert(config) {
                dry[channel] * (1.0 - wet) + repeat[channel] * wet
            } else {
                dry[channel] + repeat[channel] * wet
            };
        }
        buffer.repeat_phase += buffer.play_rate;
        while buffer.repeat_phase >= f64::from(buffer.slice_samples) && buffer.slice_samples > 0 {
            buffer.repeat_phase -= f64::from(buffer.slice_samples);
            buffer.repeat_count += 1;
        }
        output
    }

    fn apply_band(
        slot: &mut SlotRuntime,
        dry: [f64; 2],
        config: &PadConfig,
        sample_rate: f64,
    ) -> [f64; 2] {
        let input_level = dry[0].abs().max(dry[1].abs()).min(4.0);
        let env_coefficient = 1.0
            - (-1.0
                / (sample_rate
                    * if input_level > slot.filter_envelope {
                        0.003
                    } else {
                        0.080
                    }))
            .exp();
        slot.filter_envelope += env_coefficient * (input_level - slot.filter_envelope);

        let (base, resonance, drive, envelope, motion, feedback, wet, width) =
            match config.effect_type {
                EffectType::BandMid => (
                    filter_frequency(config.macros[0]),
                    config.macros[2],
                    config.macros[3],
                    config.macros[4],
                    config.macros[5],
                    0.18 * config.macros[2],
                    config.macros[6],
                    config.macros[1],
                ),
                _ => (
                    filter_frequency(config.macros[0]),
                    config.macros[1],
                    config.macros[2],
                    config.macros[3],
                    config.macros[4],
                    config.macros[5],
                    config.macros[6],
                    0.5,
                ),
            };
        let lfo = slot.filter_phase.sin();
        slot.filter_phase = (slot.filter_phase
            + std::f64::consts::TAU * (0.08 + 7.92 * motion * motion) / sample_rate)
            .rem_euclid(std::f64::consts::TAU);
        let octave_shift = envelope * slot.filter_envelope * 5.0 + motion * lfo * 2.0;
        let maximum_cutoff = (sample_rate * 0.45).max(0.001);
        let cutoff = (base * 2.0_f64.powf(octave_shift)).clamp(0.001, maximum_cutoff);
        let g = (std::f64::consts::PI * cutoff / sample_rate)
            .tan()
            .min(12.0);
        let q = 0.5 + 19.5 * clamp_macro(resonance).powi(2);
        let k = 1.0 / q;
        let gain = 10.0_f64.powf(denormalize_linear(drive, 0.0, 30.0) / 20.0);
        let feedback_gain = 1.15 * clamp_macro(feedback);
        let wet = clamp_macro(wet);
        let mut output = dry;
        for channel in 0..2 {
            let driven =
                ((dry[channel] + slot.filter_feedback[channel] * feedback_gain) * gain).tanh();
            let denominator = 1.0 + g * (g + k);
            let v1 = (slot.filter_a[channel] + g * (driven - slot.filter_b[channel])) / denominator;
            let v2 = slot.filter_b[channel] + g * v1;
            slot.filter_a[channel] = 2.0 * v1 - slot.filter_a[channel];
            slot.filter_b[channel] = 2.0 * v2 - slot.filter_b[channel];
            let low = v2;
            let band = v1 * (0.35 + 1.3 * width);
            let high = driven - k * v1 - v2;
            let filtered = match config.effect_type {
                EffectType::BandLow => low,
                EffectType::BandMid => band,
                EffectType::BandHigh => high,
                _ => driven,
            };
            let saturated = filtered.tanh();
            slot.filter_feedback[channel] = saturated;
            output[channel] = dry[channel] * (1.0 - wet) + saturated * wet;
            if !slot.filter_a[channel].is_finite() || !slot.filter_b[channel].is_finite() {
                slot.filter_a[channel] = 0.0;
                slot.filter_b[channel] = 0.0;
                slot.filter_feedback[channel] = 0.0;
                output[channel] = dry[channel] * (1.0 - wet);
            }
        }
        output
    }

    fn quantize(sample: f64, bits: i32) -> f64 {
        let safe_bits = bits.clamp(2, 16);
        let steps = f64::from((1_u32 << (safe_bits - 1)) - 1);
        (sample.clamp(-1.0, 1.0) * steps).round() / steps
    }

    fn apply_lofi(
        slot: &mut SlotRuntime,
        dry: [f64; 2],
        config: &PadConfig,
        sample_rate: f64,
    ) -> [f64; 2] {
        let target_rate = denormalize_linear(config.macros[0], 1000.0, 44100.0);
        let bits = denormalize_linear(config.macros[1], 2.0, 16.0).round() as i32;
        let wet = clamp_macro(config.macros[2]);
        let hold_samples = (sample_rate / target_rate).round().max(1.0) as i32;
        if slot.lofi_counter <= 0 {
            slot.lofi_held = [Self::quantize(dry[0], bits), Self::quantize(dry[1], bits)];
            slot.lofi_counter = hold_samples;
        }
        let mut output = dry;
        for channel in 0..2 {
            output[channel] = dry[channel] * (1.0 - wet) + slot.lofi_held[channel] * wet;
        }
        slot.lofi_counter -= 1;
        output
    }

    fn apply_gate(
        slot: &mut SlotRuntime,
        sample: [f64; 2],
        config: &PadConfig,
        sample_rate: f64,
        samples_per_beat: f64,
    ) -> [f64; 2] {
        let period = (grid_beats(config.macros[0]) * samples_per_beat).max(2.0);
        let duty = denormalize_linear(config.macros[1], 5.0, 95.0) * 0.01;
        let depth = clamp_macro(config.macros[2]);
        let attack_ms = denormalize_linear(config.macros[3], 0.0, 20.0);
        let release_ms = denormalize_linear(config.macros[4], 0.0, 100.0);
        let target = if slot.gate_phase < period * duty {
            1.0
        } else {
            1.0 - depth
        };
        slot.gate_phase += 1.0;
        if slot.gate_phase >= period {
            slot.gate_phase %= period;
        }
        let time_ms = if target > slot.gate_gain {
            attack_ms
        } else {
            release_ms
        };
        let slew = if time_ms <= 0.0 {
            1.0
        } else {
            1.0 / (sample_rate * time_ms * 0.001).max(1.0)
        };
        if slot.gate_gain < target {
            slot.gate_gain = (slot.gate_gain + slew).min(target);
        } else if slot.gate_gain > target {
            slot.gate_gain = (slot.gate_gain - slew).max(target);
        }
        [sample[0] * slot.gate_gain, sample[1] * slot.gate_gain]
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
    fn mapping_helpers_match_canonical_boundaries() {
        assert_eq!(grid_index(-1.0), 0);
        assert_eq!(grid_index(1.0), 8);
        assert_eq!(lookback_index(2.0), 9);
        assert_eq!(grid_beats(grid_normalized(6)), 4.0);
        assert_eq!(lookback_beats(lookback_normalized(0), 0.25), 0.25);
    }

    #[test]
    fn classic_state_has_all_effects_and_expected_repeat_presets() {
        let state = classic_state();
        assert_eq!(state.pads[0].effect_type, EffectType::BeatRepeat);
        assert_eq!(state.pads[15].effect_type, EffectType::LoFi);
        assert_eq!(state.pads[4].macros[1], lookback_normalized(6));
    }

    #[test]
    fn effect_control_metadata_matches_the_product_contract() {
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
                7,
                [
                    "CUTOFF",
                    "RESONANCE",
                    "DRIVE",
                    "ENVELOPE",
                    "MOTION",
                    "FEEDBACK",
                    "WET",
                ],
            ),
            (
                EffectType::BandMid,
                7,
                [
                    "CENTER",
                    "WIDTH",
                    "RESONANCE",
                    "DRIVE",
                    "ENVELOPE",
                    "MOTION",
                    "WET",
                ],
            ),
            (
                EffectType::BandHigh,
                7,
                [
                    "CUTOFF",
                    "RESONANCE",
                    "DRIVE",
                    "ENVELOPE",
                    "MOTION",
                    "FEEDBACK",
                    "WET",
                ],
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
    fn semantic_control_values_match_the_product_ui_contract() {
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
                0,
                normalize_linear(3600.0_f64.ln(), 20.0_f64.ln(), 20000.0_f64.ln()),
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
            "20 Hz"
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
    fn nonlinear_filters_remain_finite_at_macro_and_sample_rate_extremes() {
        for sample_rate in [1.0, 8_000.0, 44_100.0, 192_000.0] {
            for effect in [
                EffectType::BandLow,
                EffectType::BandMid,
                EffectType::BandHigh,
            ] {
                for macros in [[0.0; NUM_MACROS], [1.0; NUM_MACROS], [f64::NAN; NUM_MACROS]] {
                    let mut engine = Engine::new(sample_rate);
                    let mut state = PerformanceState::default();
                    state.pads[0] = PadConfig {
                        effect_type: effect,
                        macros,
                    };
                    state.held[0] = true;
                    let input: Vec<f64> = (0..4_096)
                        .map(|sample| if sample % 2 == 0 { 16.0 } else { -16.0 })
                        .collect();
                    let mut left = vec![0.0; input.len()];
                    let mut right = vec![0.0; input.len()];
                    engine.process(&input, &input, &mut left, &mut right, 999.0, &state);
                    assert!(left.iter().chain(&right).all(|sample| sample.is_finite()));
                }
            }
        }
        assert_eq!(filter_frequency(f64::NAN), 20.0);
        assert!((filter_frequency(1.0) - 20_000.0).abs() < 1.0e-9);
    }

    #[test]
    fn active_cap_suspends_oldest_and_restores_most_recent_held() {
        let mut engine = Engine::new(1_000.0);
        let mut state = PerformanceState::default();
        for slot in 0..8 {
            state.pads[slot] = default_pad_config(EffectType::Gate);
        }
        state.pads[8] = default_pad_config(EffectType::PitchUp);

        state.held[..6].fill(true);
        engine.process(&[], &[], &mut [], &mut [], 120.0, &state);
        assert_eq!(engine.admission_meta().active_mask, 0b00_111111);

        state.held[6] = true;
        engine.process(&[], &[], &mut [], &mut [], 120.0, &state);
        assert_eq!(engine.admission_meta().active_mask, 0b01_111110);
        assert_eq!(engine.admission_meta().suspended_mask, 0b00_000001);

        state.held[7] = true;
        engine.process(&[], &[], &mut [], &mut [], 120.0, &state);
        assert_eq!(engine.admission_meta().active_mask, 0b11_111100);
        assert_eq!(engine.admission_meta().suspended_mask, 0b00_000011);

        state.held[8] = true;
        engine.process(&[], &[], &mut [], &mut [], 120.0, &state);
        let meta = engine.admission_meta();
        assert_eq!(meta.active_mask, 0b11_111100);
        assert_eq!(meta.suspended_mask, 0b00_000011);
        assert_ne!(meta.held_mask & (1 << 8), 0);

        state.held[7] = false;
        engine.process(&[], &[], &mut [], &mut [], 120.0, &state);
        assert_eq!(engine.admission_meta().active_mask, 0b01_111110);
        assert_eq!(engine.admission_meta().suspended_mask, 0b00_000001);

        state.held[6] = false;
        engine.process(&[], &[], &mut [], &mut [], 120.0, &state);
        assert_eq!(engine.admission_meta().active_mask, 0b00_111111);
        assert_eq!(engine.admission_meta().suspended_mask, 0);
    }

    #[test]
    fn duplicate_exact_gate_stages_stack_serially() {
        let input = vec![0.75; 128];
        let mut one = PerformanceState::default();
        one.pads[0] = default_pad_config(EffectType::Gate);
        one.pads[0].macros[1] = 0.0;
        one.pads[0].macros[2] = 0.5;
        one.pads[0].macros[3] = 0.0;
        one.pads[0].macros[4] = 0.0;
        one.held[0] = true;

        let mut two = one;
        two.pads[1] = one.pads[0];
        two.held[1] = true;

        let mut one_engine = Engine::new(1_000.0);
        let mut two_engine = Engine::new(1_000.0);
        let mut one_output = vec![0.0; input.len()];
        let mut two_output = vec![0.0; input.len()];
        let mut scratch = vec![0.0; input.len()];
        one_engine.process(&input, &input, &mut one_output, &mut scratch, 120.0, &one);
        two_engine.process(&input, &input, &mut two_output, &mut scratch, 120.0, &two);

        assert_ne!(one_output, input);
        assert_ne!(two_output, one_output);
        assert!(
            two_output
                .iter()
                .zip(&one_output)
                .any(|(two, one)| two < one)
        );
    }

    #[test]
    fn chain_order_is_slot_order_not_press_order() {
        let input: Vec<f64> = (0..256)
            .map(|sample| (f64::from(sample) * 0.113).sin() * 0.8)
            .collect();
        let mut state = PerformanceState::default();
        state.pads[0] = default_pad_config(EffectType::LoFi);
        state.pads[1] = default_pad_config(EffectType::BandLow);

        let render_press_order = |first: usize, second: usize| {
            let mut engine = Engine::new(2_000.0);
            let mut staged = state;
            staged.held[first] = true;
            engine.process(&[], &[], &mut [], &mut [], 120.0, &staged);
            staged.held[second] = true;
            engine.process(&[], &[], &mut [], &mut [], 120.0, &staged);
            let mut output = vec![0.0; input.len()];
            let mut right = vec![0.0; input.len()];
            engine.process(&input, &input, &mut output, &mut right, 120.0, &staged);
            output
        };
        let forward_press = render_press_order(0, 1);
        let reverse_press = render_press_order(1, 0);
        assert_eq!(forward_press, reverse_press);

        let mut swapped = state;
        swapped.pads.swap(0, 1);
        swapped.held[0] = true;
        swapped.held[1] = true;
        let mut engine = Engine::new(2_000.0);
        let mut swapped_output = vec![0.0; input.len()];
        let mut right = vec![0.0; input.len()];
        engine.process(
            &input,
            &input,
            &mut swapped_output,
            &mut right,
            120.0,
            &swapped,
        );
        assert_ne!(forward_press, swapped_output);
    }

    #[test]
    fn each_stage_wet_dry_is_local_to_its_input() {
        let input: Vec<f64> = (0..192)
            .map(|sample| (f64::from(sample) * 0.17).sin() * 0.9)
            .collect();
        let mut serial = PerformanceState::default();
        serial.pads[0] = default_pad_config(EffectType::BandLow);
        serial.pads[0].macros[1] = 0.45;
        serial.pads[1] = default_pad_config(EffectType::LoFi);
        serial.pads[1].macros[2] = 0.35;
        serial.held[0] = true;
        serial.held[1] = true;

        let mut serial_engine = Engine::new(2_000.0);
        let mut serial_output = vec![0.0; input.len()];
        let mut scratch = vec![0.0; input.len()];
        serial_engine.process(
            &input,
            &input,
            &mut serial_output,
            &mut scratch,
            120.0,
            &serial,
        );

        let mut first_state = PerformanceState::default();
        first_state.pads[0] = serial.pads[0];
        first_state.held[0] = true;
        let mut first_engine = Engine::new(2_000.0);
        let mut first_output = vec![0.0; input.len()];
        first_engine.process(
            &input,
            &input,
            &mut first_output,
            &mut scratch,
            120.0,
            &first_state,
        );

        let mut second_state = PerformanceState::default();
        second_state.pads[0] = serial.pads[1];
        second_state.held[0] = true;
        let mut second_engine = Engine::new(2_000.0);
        let mut staged_output = vec![0.0; input.len()];
        second_engine.process(
            &first_output,
            &first_output,
            &mut staged_output,
            &mut scratch,
            120.0,
            &second_state,
        );

        assert_eq!(serial_output, staged_output);
    }

    #[test]
    fn configured_buffer_histories_capture_their_own_stage_inputs() {
        let input: Vec<f64> = (0..64)
            .map(|sample| (f64::from(sample) * 0.19).sin() * 0.77)
            .collect();
        let mut state = PerformanceState::default();
        state.pads[0] = default_pad_config(EffectType::BandLow);
        state.pads[1] = default_pad_config(EffectType::BeatRepeat);
        state.pads[2] = default_pad_config(EffectType::BeatRepeat);
        state.pads[3] = default_pad_config(EffectType::LoFi);
        state.held[0] = true;
        state.held[3] = true;

        let mut engine = Engine::new(1_000.0);
        let mut output = vec![0.0; input.len()];
        let mut right = vec![0.0; input.len()];
        engine.process(&input, &input, &mut output, &mut right, 120.0, &state);

        assert!(
            engine.slots[1].buffer.history_l[..input.len()]
                .iter()
                .zip(&input)
                .any(|(captured, input)| *captured != *input as f32)
        );
        assert_eq!(
            &engine.slots[1].buffer.history_l[..input.len()],
            &engine.slots[2].buffer.history_l[..input.len()]
        );
        assert_ne!(
            engine.slots[1].buffer.history_l.as_ptr(),
            engine.slots[2].buffer.history_l.as_ptr()
        );

        let mut upstream_buffer = PerformanceState::default();
        upstream_buffer.pads[0] = default_pad_config(EffectType::BeatRepeat);
        upstream_buffer.pads[1] = default_pad_config(EffectType::LoFi);
        upstream_buffer.held[1] = true;
        let mut engine = Engine::new(1_000.0);
        engine.process(
            &input,
            &input,
            &mut output,
            &mut right,
            120.0,
            &upstream_buffer,
        );
        assert_eq!(
            &engine.slots[0].buffer.history_l[..input.len()],
            input
                .iter()
                .map(|sample| *sample as f32)
                .collect::<Vec<_>>()
        );
        assert_ne!(output, input);
    }

    #[test]
    fn suspension_freezes_processor_state_while_buffer_history_keeps_recording() {
        let mut state = PerformanceState::default();
        state.pads[0] = default_pad_config(EffectType::BeatRepeat);
        state.held[0] = true;
        for slot in 1..=6 {
            state.pads[slot] = default_pad_config(EffectType::Gate);
        }
        state.held[1..=5].fill(true);

        let mut engine = Engine::new(1_000.0);
        let input = vec![0.25; 32];
        let mut output = vec![0.0; input.len()];
        let mut right = vec![0.0; input.len()];
        engine.process(&input, &input, &mut output, &mut right, 120.0, &state);
        let phase = engine.slots[0].buffer.repeat_phase;
        let write_pos = engine.slots[0].buffer.write_pos;

        state.held[6] = true;
        engine.process(&input, &input, &mut output, &mut right, 120.0, &state);
        assert_ne!(engine.admission_meta().suspended_mask & 1, 0);
        assert_eq!(engine.slots[0].buffer.repeat_phase, phase);
        assert_ne!(engine.slots[0].buffer.write_pos, write_pos);

        state.held[6] = false;
        engine.process(&[], &[], &mut [], &mut [], 120.0, &state);
        assert_ne!(engine.admission_meta().active_mask & 1, 0);
        assert_eq!(engine.slots[0].buffer.repeat_phase, phase);
    }

    #[test]
    fn release_resets_processor_but_configured_buffer_history_continues() {
        let mut state = PerformanceState::default();
        state.pads[0] = default_pad_config(EffectType::Reverse);
        let mut engine = Engine::new(1_000.0);
        let input = vec![0.5; 24];
        let mut output = vec![0.0; input.len()];
        let mut right = vec![0.0; input.len()];
        engine.process(&input, &input, &mut output, &mut right, 120.0, &state);
        state.held[0] = true;
        engine.process(&input, &input, &mut output, &mut right, 120.0, &state);
        assert!(engine.slots[0].buffer.repeat_phase > 0.0);

        state.held[0] = false;
        engine.process(&[], &[], &mut [], &mut [], 120.0, &state);
        assert!(!engine.slots[0].buffer.repeat_active);
        assert_eq!(engine.slots[0].buffer.repeat_phase, 0.0);
        let write_pos = engine.slots[0].buffer.write_pos;
        engine.process(&input, &input, &mut output, &mut right, 120.0, &state);
        assert_ne!(engine.slots[0].buffer.write_pos, write_pos);
        assert_eq!(engine.slots[0].buffer.history_l[write_pos], 0.5);
    }

    #[test]
    fn buffer_type_changes_preserve_history_and_nonbuffer_to_buffer_is_cold() {
        let mut state = PerformanceState::default();
        state.pads[0] = default_pad_config(EffectType::BeatRepeat);
        let mut engine = Engine::new(1_000.0);
        let input: Vec<f64> = (1..=32).map(f64::from).collect();
        let mut output = vec![0.0; input.len()];
        let mut right = vec![0.0; input.len()];
        engine.process(&input, &input, &mut output, &mut right, 120.0, &state);
        let history = engine.slots[0].buffer.history_l.clone();
        let write_pos = engine.slots[0].buffer.write_pos;

        state.pads[0] = default_pad_config(EffectType::Reverse);
        engine.process(&[], &[], &mut [], &mut [], 120.0, &state);
        assert_eq!(engine.slots[0].buffer.history_l, history);
        assert_eq!(engine.slots[0].buffer.write_pos, write_pos);
        assert!(!engine.slots[0].buffer.repeat_active);

        state.pads[0] = default_pad_config(EffectType::Gate);
        engine.process(&[], &[], &mut [], &mut [], 120.0, &state);
        state.pads[0] = default_pad_config(EffectType::TapeStop);
        engine.process(&[], &[], &mut [], &mut [], 120.0, &state);
        assert_eq!(engine.slots[0].buffer.write_pos, 0);
        assert!(
            engine.slots[0]
                .buffer
                .history_l
                .iter()
                .all(|sample| *sample == 0.0)
        );
    }

    #[test]
    fn reset_prepares_eight_seconds_of_independent_stereo_f32_history() {
        let engine = Engine::new(1_000.0);
        let history_len = engine.slots[0].buffer.history_l.len();
        assert!(history_len >= 8_000);
        assert_eq!(
            std::mem::size_of_val(&engine.slots[0].buffer.history_l[0]),
            4
        );
        assert!(engine.slots.iter().all(|slot| {
            slot.buffer.history_l.len() == history_len && slot.buffer.history_r.len() == history_len
        }));
        assert_eq!(
            engine.history_bytes(),
            (NUM_PADS + 1) * 2 * history_len * std::mem::size_of::<f32>()
        );
    }

    #[test]
    fn duplicate_buffer_processors_stack_with_local_wet_dry() {
        let pre_roll: Vec<f64> = (0..128)
            .map(|sample| (f64::from(sample) * 0.071).sin() * 0.8)
            .collect();
        let performance: Vec<f64> = (0..128)
            .map(|sample| (f64::from(sample) * 0.137).cos() * 0.6)
            .collect();
        let mut one = PerformanceState::default();
        one.pads[0] = default_pad_config(EffectType::BeatRepeat);
        one.pads[0].macros[2] = 0.5;
        one.pads[1] = one.pads[0];

        let render = |hold_second: bool| {
            let mut engine = Engine::new(1_000.0);
            let mut output = vec![0.0; pre_roll.len()];
            let mut right = vec![0.0; pre_roll.len()];
            engine.process(&pre_roll, &pre_roll, &mut output, &mut right, 120.0, &one);
            let mut held = one;
            held.held[0] = true;
            held.held[1] = hold_second;
            engine.process(&[], &[], &mut [], &mut [], 120.0, &held);
            engine.process(
                &performance,
                &performance,
                &mut output,
                &mut right,
                120.0,
                &held,
            );
            output
        };

        let one_stage = render(false);
        let two_stages = render(true);
        assert_ne!(one_stage, two_stages);
    }

    #[test]
    fn visualization_prefers_selected_held_buffer_then_lowest_active_buffer() {
        let mut engine = Engine::new(1_000.0);
        let mut state = PerformanceState::default();
        state.pads[0] = default_pad_config(EffectType::BeatRepeat);
        state.pads[1] = default_pad_config(EffectType::Reverse);
        let input: Vec<f64> = (0..256)
            .map(|sample| (f64::from(sample) * 0.03).sin())
            .collect();
        let mut output = vec![0.0; input.len()];
        let mut right = vec![0.0; input.len()];
        engine.process(&input, &input, &mut output, &mut right, 120.0, &state);
        state.held[0] = true;
        state.held[1] = true;
        engine.process(&[], &[], &mut [], &mut [], 120.0, &state);

        let mut bins = [VisualizationBin::default(); 8];
        assert_eq!(
            engine.fill_visualization(120.0, &mut bins).active_pad,
            Some(0)
        );
        engine.set_visualization_selected_slot(Some(1));
        assert_eq!(
            engine.fill_visualization(120.0, &mut bins).active_pad,
            Some(1)
        );
        engine.set_visualization_selected_slot(Some(2));
        assert_eq!(
            engine.fill_visualization(120.0, &mut bins).active_pad,
            Some(0)
        );
    }

    #[test]
    fn clear_transient_clears_runtime_masks_and_all_histories() {
        let mut engine = Engine::new(1_000.0);
        let mut state = PerformanceState::default();
        state.pads[0] = default_pad_config(EffectType::BeatRepeat);
        state.pads[1] = default_pad_config(EffectType::Gate);
        state.held[0] = true;
        state.held[1] = true;
        let input = vec![0.5; 32];
        let mut output = vec![0.0; input.len()];
        let mut right = vec![0.0; input.len()];
        engine.process(&input, &input, &mut output, &mut right, 120.0, &state);
        assert_ne!(engine.admission_meta().active_mask, 0);

        engine.clear_transient();
        assert_eq!(engine.admission_meta(), AdmissionMeta::default());
        assert!(engine.rolling_l.iter().all(|sample| *sample == 0.0));
        assert!(engine.slots.iter().all(|slot| {
            slot.buffer.history_l.iter().all(|sample| *sample == 0.0)
                && slot.buffer.history_r.iter().all(|sample| *sample == 0.0)
                && !slot.buffer.repeat_active
        }));
    }

    #[test]
    fn rolling_visualization_uses_the_newest_four_beats() {
        let mut engine = Engine::new(1.0);
        let input_l: Vec<f64> = (0..20).map(f64::from).collect();
        let input_r: Vec<f64> = input_l.iter().map(|sample| -*sample).collect();
        let mut output_l = vec![0.0; input_l.len()];
        let mut output_r = vec![0.0; input_l.len()];
        engine.process(
            &input_l,
            &input_r,
            &mut output_l,
            &mut output_r,
            60.0,
            &PerformanceState::default(),
        );

        let mut bins = [VisualizationBin::default(); 2];
        let meta = engine.fill_visualization(60.0, &mut bins);

        assert_eq!(
            meta,
            VisualizationMeta {
                mode: VisualizationMode::Rolling,
                active_pad: None,
                active_effect: EffectType::Off,
                normalized_playhead: 1.0,
                reverse: false,
                play_rate: 1.0,
                duration_seconds: 4.0,
            }
        );
        assert_eq!(
            bins,
            [
                VisualizationBin {
                    min_l: 16.0,
                    max_l: 17.0,
                    min_r: -17.0,
                    max_r: -16.0,
                },
                VisualizationBin {
                    min_l: 18.0,
                    max_l: 19.0,
                    min_r: -19.0,
                    max_r: -18.0,
                },
            ]
        );
    }

    #[test]
    fn capture_visualization_handles_circular_wrap_and_reverse_playhead() {
        let mut engine = Engine::new(16.0);
        let input_l: Vec<f64> = (0..260).map(f64::from).collect();
        let input_r: Vec<f64> = input_l.iter().map(|sample| *sample + 1_000.0).collect();
        let mut output_l = vec![0.0; input_l.len()];
        let mut output_r = vec![0.0; input_l.len()];
        let mut state = PerformanceState::default();
        state.pads[3] = default_pad_config(EffectType::Reverse);
        state.pads[3].macros[0] = grid_normalized(0);
        engine.process(
            &input_l,
            &input_r,
            &mut output_l,
            &mut output_r,
            60.0,
            &state,
        );

        state.held[3] = true;
        engine.process(&[], &[], &mut [], &mut [], 60.0, &state);

        let mut bins = [VisualizationBin::default(); 4];
        let meta = engine.fill_visualization(60.0, &mut bins);

        assert_eq!(meta.mode, VisualizationMode::Capture);
        assert_eq!(meta.active_pad, Some(3));
        assert_eq!(meta.active_effect, EffectType::Reverse);
        assert_eq!(meta.normalized_playhead, 1.0);
        assert!(meta.reverse);
        assert_eq!(meta.play_rate, 1.0);
        assert_eq!(meta.duration_seconds, 0.5);
        assert_eq!(
            bins,
            [
                VisualizationBin {
                    min_l: 252.0,
                    max_l: 253.0,
                    min_r: 1_252.0,
                    max_r: 1_253.0,
                },
                VisualizationBin {
                    min_l: 254.0,
                    max_l: 255.0,
                    min_r: 1_254.0,
                    max_r: 1_255.0,
                },
                VisualizationBin {
                    min_l: 256.0,
                    max_l: 257.0,
                    min_r: 1_256.0,
                    max_r: 1_257.0,
                },
                VisualizationBin {
                    min_l: 258.0,
                    max_l: 259.0,
                    min_r: 1_258.0,
                    max_r: 1_259.0,
                },
            ]
        );
    }

    #[test]
    fn visualization_sanitizes_non_finite_and_out_of_range_f32_samples() {
        let mut engine = Engine::new(1.0);
        let input_l = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, f64::MAX];
        let input_r = [f64::MAX, f64::NEG_INFINITY, f64::INFINITY, f64::NAN];
        let mut output_l = [0.0; 4];
        let mut output_r = [0.0; 4];
        engine.process(
            &input_l,
            &input_r,
            &mut output_l,
            &mut output_r,
            60.0,
            &PerformanceState::default(),
        );

        let mut bins = [VisualizationBin::default(); 8];
        let meta = engine.fill_visualization(60.0, &mut bins);
        assert!(meta.duration_seconds.is_finite());
        assert!(meta.normalized_playhead.is_finite());
        assert!(meta.play_rate.is_finite());
        assert!(bins.iter().all(|bin| {
            [bin.min_l, bin.max_l, bin.min_r, bin.max_r]
                .into_iter()
                .all(f32::is_finite)
        }));
        assert!(bins.iter().all(|bin| *bin == VisualizationBin::default()));
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
