//! Framework-neutral Buffer Uppercut factory kits and native `.bupreset`
//! serialization.

use std::fmt;

use buffer_uppercut_dsp::{
    EffectType, FilterMode, NUM_MACROS, NUM_PADS, PadConfig, PerformanceState, PitchRole,
    clamp_macro, classic_state, default_pad_config, filter_config, grid_normalized,
    lookback_normalized, normalize_linear, pitch_config,
};

pub const KIT_VERSION: u32 = 1;
pub const FACTORY_KIT_COUNT: usize = 5;
pub const MAX_KIT_NAME_BYTES: usize = 63;
pub const MAX_KIT_FILE_BYTES: usize = 4096;
pub const KIT_EXTENSION: &str = "bupreset";
const MAGIC: &[u8; 8] = b"BUPRESET";

#[derive(Clone, Debug)]
pub struct Kit {
    pub name: String,
    pub state: PerformanceState,
}

impl Default for Kit {
    fn default() -> Self {
        classic_kit()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodeError {
    Truncated,
    WrongMagic,
    TruncatedHeader,
    UnsupportedVersion,
    UnsupportedLayout,
    InvalidName,
    InvalidUtf8,
    InvalidPitch,
    InvalidEffect,
    InvalidControl,
    TrailingData,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Truncated => "The preset file is truncated.",
            Self::WrongMagic => "This is not a Buffer Uppercut preset.",
            Self::TruncatedHeader => "The preset header is truncated.",
            Self::UnsupportedVersion => "This preset version is not supported.",
            Self::UnsupportedLayout => "The preset layout is not supported.",
            Self::InvalidName => "The preset name is invalid.",
            Self::InvalidUtf8 => "The preset name is not valid UTF-8.",
            Self::InvalidPitch => "The performance pitch is invalid.",
            Self::InvalidEffect => "A pad effect type is invalid.",
            Self::InvalidControl => "A pad control value is invalid.",
            Self::TrailingData => "The preset contains unexpected trailing data.",
        })
    }
}

impl std::error::Error for DecodeError {}

#[must_use]
pub fn encode(kit: &Kit) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(1024);
    bytes.extend_from_slice(MAGIC);
    put_u32(&mut bytes, KIT_VERSION);
    put_u32(&mut bytes, NUM_PADS as u32);
    put_u32(&mut bytes, NUM_MACROS as u32);

    let name = truncated_name_bytes(&kit.name);
    put_u32(&mut bytes, name.len() as u32);
    bytes.extend_from_slice(name);
    put_f64(
        &mut bytes,
        finite_or_zero(kit.state.performance_pitch).clamp(-24.0, 24.0),
    );

    for pad in &kit.state.pads {
        put_u32(&mut bytes, pad.effect_type as u32);
        for control in pad.macros {
            put_f64(&mut bytes, clamp_macro(control));
        }
    }
    bytes
}

/// # Errors
///
/// Returns a precise error for malformed, unsupported, or non-finite content.
pub fn decode(bytes: &[u8]) -> Result<Kit, DecodeError> {
    if bytes.len() < MAGIC.len() {
        return Err(DecodeError::Truncated);
    }
    if &bytes[..MAGIC.len()] != MAGIC {
        return Err(DecodeError::WrongMagic);
    }

    let mut cursor = MAGIC.len();
    let version = get_u32(bytes, &mut cursor).ok_or(DecodeError::TruncatedHeader)?;
    let pad_count = get_u32(bytes, &mut cursor).ok_or(DecodeError::TruncatedHeader)?;
    let macro_count = get_u32(bytes, &mut cursor).ok_or(DecodeError::TruncatedHeader)?;
    let name_length = get_u32(bytes, &mut cursor).ok_or(DecodeError::TruncatedHeader)? as usize;
    if version != KIT_VERSION {
        return Err(DecodeError::UnsupportedVersion);
    }
    if pad_count as usize != NUM_PADS || macro_count as usize != NUM_MACROS {
        return Err(DecodeError::UnsupportedLayout);
    }
    if name_length > MAX_KIT_NAME_BYTES || cursor + name_length > bytes.len() {
        return Err(DecodeError::InvalidName);
    }
    let name_bytes = &bytes[cursor..cursor + name_length];
    let name = std::str::from_utf8(name_bytes)
        .map_err(|_| DecodeError::InvalidUtf8)?
        .to_owned();
    cursor += name_length;

    let performance_pitch = get_f64(bytes, &mut cursor).ok_or(DecodeError::InvalidPitch)?;
    if !performance_pitch.is_finite() || !(-24.0..=24.0).contains(&performance_pitch) {
        return Err(DecodeError::InvalidPitch);
    }

    let mut state = PerformanceState {
        performance_pitch,
        ..PerformanceState::default()
    };
    for pad in &mut state.pads {
        let effect = get_u32(bytes, &mut cursor).ok_or(DecodeError::InvalidEffect)?;
        if effect > EffectType::Vinyl as u32 {
            return Err(DecodeError::InvalidEffect);
        }
        pad.effect_type = EffectType::from_index(effect as i32);
        for control in &mut pad.macros {
            let value = get_f64(bytes, &mut cursor).ok_or(DecodeError::InvalidControl)?;
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(DecodeError::InvalidControl);
            }
            *control = value;
        }
    }
    if cursor != bytes.len() {
        return Err(DecodeError::TrailingData);
    }

    Ok(Kit {
        name: if name.is_empty() {
            "Untitled Kit".to_owned()
        } else {
            name
        },
        state,
    })
}

#[must_use]
pub fn factory_kit(index: usize) -> Kit {
    match index {
        1 => glitch_grid_kit(),
        2 => tape_lab_kit(),
        3 => filter_pitch_kit(),
        4 => vinyl_cuts_kit(),
        _ => classic_kit(),
    }
}

/// Four record textures followed by the classic performance tools.
#[must_use]
pub fn vinyl_cuts_kit() -> Kit {
    let mut kit = classic_kit();
    kit.name = "Vinyl Cuts".to_owned();
    for pad in &mut kit.state.pads[..4] {
        *pad = default_pad_config(EffectType::Vinyl);
    }
    kit.state.pads[1].macros = [0.75, 0.4, 0.2, 0.1, 0.0, 0.0, 1.0];
    kit.state.pads[2].macros = [0.1, 0.1, 0.45, 0.2, 0.65, 0.3, 1.0];
    kit.state.pads[3].macros = [0.5, 0.65, 0.85, 0.65, 0.35, 0.2, 1.0];
    kit
}

#[must_use]
pub fn classic_kit() -> Kit {
    Kit {
        name: "Classic".to_owned(),
        state: classic_state(),
    }
}

#[must_use]
pub fn glitch_grid_kit() -> Kit {
    let mut kit = kit_with_types(
        "Glitch Grid",
        [
            EffectType::BeatRepeat,
            EffectType::BeatRepeat,
            EffectType::BeatRepeat,
            EffectType::BeatRepeat,
            EffectType::BeatRepeat,
            EffectType::BeatRepeat,
            EffectType::Reverse,
            EffectType::Gate,
            EffectType::Pitch,
            EffectType::Pitch,
            EffectType::Pitch,
            EffectType::LoFi,
            EffectType::Filter,
            EffectType::Filter,
            EffectType::Filter,
            EffectType::Off,
        ],
    );
    kit.state.pads[0] = beat_repeat_config(0, 0);
    kit.state.pads[1] = beat_repeat_config(1, 0);
    kit.state.pads[1].macros[4] = normalize_linear(12.0, -24.0, 24.0);
    kit.state.pads[2] = beat_repeat_config(2, 0);
    kit.state.pads[2].macros[4] = normalize_linear(-12.0, -24.0, 24.0);
    kit.state.pads[3] = beat_repeat_config(0, 0);
    kit.state.pads[4] = beat_repeat_config(3, 0);
    kit.state.pads[4].macros[4] = normalize_linear(7.0, -24.0, 24.0);
    kit.state.pads[5] = beat_repeat_config(4, 0);
    kit.state.pads[5].macros[4] = normalize_linear(-7.0, -24.0, 24.0);
    kit.state.pads[6].macros[0] = grid_normalized(4);
    kit.state.pads[7].macros[1] = normalize_linear(35.0, 5.0, 95.0);
    kit.state.pads[8] = pitch_config(PitchRole::Down, 2.0);
    kit.state.pads[9] = pitch_config(PitchRole::Trigger, 1.0);
    kit.state.pads[10] = pitch_config(PitchRole::Up, 2.0);
    kit.state.pads[11].macros[0] = normalize_linear(8000.0, 1000.0, 44100.0);
    kit.state.pads[11].macros[1] = normalize_linear(6.0, 2.0, 16.0);
    kit.state.pads[12] = filter_config(FilterMode::LowPass, 260.0);
    kit.state.pads[13] = filter_config(FilterMode::BandPass, 1_200.0);
    kit.state.pads[14] = filter_config(FilterMode::HighPass, 3_600.0);
    kit
}

#[must_use]
pub fn tape_lab_kit() -> Kit {
    let mut kit = kit_with_types(
        "Tape Lab",
        [
            EffectType::Reverse,
            EffectType::Reverse,
            EffectType::Reverse,
            EffectType::Reverse,
            EffectType::TapeStop,
            EffectType::TapeStop,
            EffectType::TapeStop,
            EffectType::TapeStop,
            EffectType::BeatRepeat,
            EffectType::BeatRepeat,
            EffectType::BeatRepeat,
            EffectType::BeatRepeat,
            EffectType::Gate,
            EffectType::LoFi,
            EffectType::Pitch,
            EffectType::Off,
        ],
    );
    for pad in 0..4 {
        kit.state.pads[pad].macros[0] = grid_normalized(3 + pad as i32);
    }
    for (pad, curve) in (4..8).zip([0.75, 1.25, 2.0, 3.0]) {
        kit.state.pads[pad].macros[0] = grid_normalized(pad as i32 - 1);
        kit.state.pads[pad].macros[1] = normalize_linear(curve, 0.25, 4.0);
    }
    kit.state.pads[8] = beat_repeat_config(3, 6);
    kit.state.pads[9] = beat_repeat_config(3, 7);
    kit.state.pads[10] = beat_repeat_config(3, 0);
    kit.state.pads[10].macros[4] = normalize_linear(-5.0, -24.0, 24.0);
    kit.state.pads[11] = beat_repeat_config(4, 0);
    kit.state.pads[11].macros[4] = normalize_linear(7.0, -24.0, 24.0);
    kit.state.pads[12].macros[0] = grid_normalized(3);
    kit.state.pads[14] = pitch_config(PitchRole::Trigger, 1.0);
    kit
}

#[must_use]
pub fn filter_pitch_kit() -> Kit {
    let mut kit = kit_with_types(
        "Filter & Pitch",
        [
            EffectType::Filter,
            EffectType::Filter,
            EffectType::Filter,
            EffectType::Filter,
            EffectType::LoFi,
            EffectType::LoFi,
            EffectType::Gate,
            EffectType::Gate,
            EffectType::Pitch,
            EffectType::Pitch,
            EffectType::Pitch,
            EffectType::Pitch,
            EffectType::Pitch,
            EffectType::BeatRepeat,
            EffectType::Reverse,
            EffectType::Off,
        ],
    );
    kit.state.pads[0] = filter_config(FilterMode::LowPass, 180.0);
    kit.state.pads[1] = filter_config(FilterMode::LowPass, 500.0);
    kit.state.pads[2] = filter_config(FilterMode::BandPass, 1_400.0);
    kit.state.pads[3] = filter_config(FilterMode::HighPass, 5_000.0);
    kit.state.pads[4].macros[0] = normalize_linear(16000.0, 1000.0, 44100.0);
    kit.state.pads[4].macros[1] = normalize_linear(12.0, 2.0, 16.0);
    kit.state.pads[5].macros[0] = normalize_linear(8000.0, 1000.0, 44100.0);
    kit.state.pads[5].macros[1] = normalize_linear(8.0, 2.0, 16.0);
    kit.state.pads[6].macros[1] = normalize_linear(50.0, 5.0, 95.0);
    kit.state.pads[7].macros[0] = grid_normalized(3);
    kit.state.pads[7].macros[1] = normalize_linear(25.0, 5.0, 95.0);
    kit.state.pads[8] = pitch_config(PitchRole::Down, 1.0);
    kit.state.pads[9] = pitch_config(PitchRole::Down, 7.0);
    kit.state.pads[10] = pitch_config(PitchRole::Trigger, 1.0);
    kit.state.pads[11] = pitch_config(PitchRole::Up, 1.0);
    kit.state.pads[12] = pitch_config(PitchRole::Up, 7.0);
    kit.state.pads[13] = beat_repeat_config(2, 0);
    kit.state.pads[14].macros[0] = grid_normalized(4);
    kit
}

fn kit_with_types(name: &str, effects: [EffectType; NUM_PADS]) -> Kit {
    let mut state = PerformanceState::default();
    for (pad, effect) in state.pads.iter_mut().zip(effects) {
        *pad = default_pad_config(effect);
    }
    Kit {
        name: name.to_owned(),
        state,
    }
}

fn beat_repeat_config(cell_grid_index: i32, lookback_index: i32) -> PadConfig {
    let mut config = default_pad_config(EffectType::BeatRepeat);
    config.macros[0] = grid_normalized(cell_grid_index);
    config.macros[1] = lookback_normalized(lookback_index);
    config
}

fn finite_or_zero(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}

fn truncated_name_bytes(name: &str) -> &[u8] {
    let mut length = name.len().min(MAX_KIT_NAME_BYTES);
    while !name.is_char_boundary(length) {
        length -= 1;
    }
    &name.as_bytes()[..length]
}

fn put_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn get_u32(bytes: &[u8], cursor: &mut usize) -> Option<u32> {
    let value = u32::from_le_bytes(bytes.get(*cursor..*cursor + 4)?.try_into().ok()?);
    *cursor += 4;
    Some(value)
}

fn put_f64(bytes: &mut Vec<u8>, value: f64) {
    bytes.extend_from_slice(&value.to_bits().to_le_bytes());
}

fn get_f64(bytes: &[u8], cursor: &mut usize) -> Option<f64> {
    let bits = u64::from_le_bytes(bytes.get(*cursor..*cursor + 8)?.try_into().ok()?);
    *cursor += 8;
    Some(f64::from_bits(bits))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_factory_kits_round_trip_without_losing_bits() {
        for index in 0..FACTORY_KIT_COUNT {
            let encoded = encode(&factory_kit(index));
            assert_eq!(&encoded[8..12], &KIT_VERSION.to_le_bytes());
            let decoded = decode(&encoded).expect("decode factory kit");
            assert_eq!(encoded, encode(&decoded));
            assert_eq!(decoded.name, factory_kit(index).name);
            assert!(!decoded.state.held.iter().any(|held| *held));
        }
    }

    #[test]
    fn version_one_preserves_sixteen_independent_same_type_slots() {
        assert_eq!(KIT_VERSION, 1);

        let mut kit = classic_kit();
        kit.state.pads[0] = default_pad_config(EffectType::Gate);
        kit.state.pads[1] = default_pad_config(EffectType::Gate);
        kit.state.pads[0].macros[0] = 0.125;
        kit.state.pads[1].macros[0] = 0.875;
        kit.state.held[0] = true;
        kit.state.held[15] = true;

        let encoded = encode(&kit);
        assert_eq!(u32::from_le_bytes(encoded[12..16].try_into().unwrap()), 16);

        let decoded = decode(&encoded).expect("decode independent serial slots");
        assert_eq!(decoded.state.pads[0].effect_type, EffectType::Gate);
        assert_eq!(decoded.state.pads[1].effect_type, EffectType::Gate);
        assert_eq!(decoded.state.pads[0].macros[0], 0.125);
        assert_eq!(decoded.state.pads[1].macros[0], 0.875);
        assert!(!decoded.state.held.iter().any(|held| *held));
    }

    #[test]
    fn vinyl_kit_round_trips_and_unknown_effects_are_rejected() {
        let kit = vinyl_cuts_kit();
        let mut bytes = encode(&kit);
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded.state.pads[0].effect_type, EffectType::Vinyl);
        assert_eq!(decoded.state.pads[3].macros, kit.state.pads[3].macros);
        let first_effect = 24 + kit.name.len() + 8;
        bytes[first_effect..first_effect + 4].copy_from_slice(&11_u32.to_le_bytes());
        assert!(matches!(decode(&bytes), Err(DecodeError::InvalidEffect)));
    }

    #[test]
    fn codec_rejects_wrong_versions_layouts_and_trailing_bytes() {
        let encoded = encode(&classic_kit());

        let mut wrong_version = encoded.clone();
        wrong_version[8..12].copy_from_slice(&(KIT_VERSION + 1).to_le_bytes());
        assert!(matches!(
            decode(&wrong_version),
            Err(DecodeError::UnsupportedVersion)
        ));

        let mut wrong_layout = encoded.clone();
        wrong_layout[12..16].copy_from_slice(&15_u32.to_le_bytes());
        assert!(matches!(
            decode(&wrong_layout),
            Err(DecodeError::UnsupportedLayout)
        ));

        let mut trailing = encoded;
        trailing.push(0);
        assert!(matches!(decode(&trailing), Err(DecodeError::TrailingData)));
    }

    #[test]
    fn encoder_truncates_utf8_safely_and_sanitizes_non_finite_values() {
        let mut kit = classic_kit();
        kit.name = format!("{}é", "x".repeat(62));
        kit.state.performance_pitch = f64::NAN;
        kit.state.pads[0].macros[0] = f64::INFINITY;
        let decoded = decode(&encode(&kit)).expect("decode sanitized kit");
        assert_eq!(decoded.name, "x".repeat(62));
        assert_eq!(decoded.state.performance_pitch, 0.0);
        assert_eq!(decoded.state.pads[0].macros[0], 0.0);
    }
}
