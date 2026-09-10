use std::cell::{Cell, RefCell};
use std::fmt::Write as _;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use buffer_uppercut_dsp::{
    EffectType, FilterMode, PitchRole, VisualizationBin, VisualizationMode, default_pad_config,
    format_control_value,
};
use buffer_uppercut_kit::{FACTORY_KIT_COUNT, Kit, factory_kit};
use truce::core::editor::PluginContextReadF32;
use truce::prelude::{Editor, Params, PluginContext};
use truce_slint::slint::{Model, ModelRc, SharedString, VecModel, include_modules};
use truce_slint::{PhysicalKeyboardEvent, SlintEditor};

use crate::params::{
    BufferUppercutParams, NUM_MACROS, NUM_PADS, PARAM_PERFORMANCE_PITCH_ID, WaveformSnapshot,
    pad_control_id, pad_trigger_id, pad_type_id,
};

include_modules!();

const EFFECT_COUNT: usize = EffectType::Vinyl as usize + 1;
const DEFAULT_EDITOR_SIZE: (u32, u32) = (1120, 700);
const WAVEFORM_VIEWBOX_WIDTH: f32 = 1000.0;
const WAVEFORM_VIEWBOX_HEIGHT: f32 = 32.0;
const WAVEFORM_AMPLITUDE: f32 = 14.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EditorPreview {
    Captured,
    Pitch,
    Vinyl,
}

#[derive(Clone, Copy)]
struct EffectGesture {
    pad: usize,
    effect: EffectType,
    controls_started: bool,
}

impl EditorPreview {
    const fn selected_pad(self) -> usize {
        match self {
            Self::Captured => 1,
            Self::Pitch => 9,
            Self::Vinyl => 0,
        }
    }

    const fn held_pad(self) -> Option<usize> {
        match self {
            Self::Captured => Some(1),
            Self::Pitch => None,
            Self::Vinyl => Some(0),
        }
    }
}

pub fn create(params: Arc<BufferUppercutParams>) -> Box<dyn Editor> {
    params.initialize_direct_keys_enabled(standalone_direct_keys_default(
        std::env::current_exe().ok().as_deref(),
    ));
    let editor = SlintEditor::new(params.clone(), configured_editor_size(), move |state| {
        let ui = BufferUppercutUi::new().expect("create Buffer Uppercut Slint editor");
        let editor_preview = configured_editor_preview();
        let selected_pad = Rc::new(Cell::new(
            editor_preview.map_or(0, EditorPreview::selected_pad),
        ));
        if editor_preview == Some(EditorPreview::Vinyl) {
            // Headless screenshots do not run the host automation queue.
            let kit = buffer_uppercut_kit::vinyl_cuts_kit();
            for (pad, config) in kit.state.pads.iter().enumerate() {
                state
                    .params()
                    .set_plain(pad_type_id(pad), config.effect_type as u8 as f64);
                for (control, value) in config.macros.iter().enumerate() {
                    state
                        .params()
                        .set_plain(pad_control_id(pad, control), *value);
                }
            }
            state.params().set_kit_name(&kit.name);
        }
        let auto_select_midi = Rc::new(Cell::new(true));
        let factory_kit_index = Rc::new(Cell::new(Some(
            if editor_preview == Some(EditorPreview::Vinyl) {
                4_usize
            } else {
                0
            },
        )));
        let last_midi_press_sequence = Rc::new(Cell::new(state.params().midi_pad_press_event().0));
        let last_direct_key_press_sequence =
            Rc::new(Cell::new(state.params().direct_key_press_event().0));
        let active_macro_gestures: Rc<[Cell<Option<u32>>; NUM_MACROS]> =
            Rc::new(std::array::from_fn(|_| Cell::new(None)));

        let pads = Rc::new(VecModel::from(
            (0..NUM_PADS).map(empty_pad_view).collect::<Vec<_>>(),
        ));
        let macros = Rc::new(VecModel::from(
            (0..NUM_MACROS).map(empty_macro_view).collect::<Vec<_>>(),
        ));
        let active_effect_gesture = Rc::new(Cell::new(None::<EffectGesture>));

        ui.set_pads(ModelRc::from(pads.clone()));
        ui.set_macros(ModelRc::from(macros.clone()));
        apply_editor_preview(&ui, editor_preview);

        {
            let state = state.clone();
            ui.on_pitch_edit_began(move || state.begin_edit(PARAM_PERFORMANCE_PITCH_ID));
        }
        {
            let state = state.clone();
            ui.on_pitch_value_changed(move |value| {
                state.set_param(PARAM_PERFORMANCE_PITCH_ID, f64::from(value));
            });
        }
        {
            let state = state.clone();
            ui.on_pitch_edit_ended(move || state.end_edit(PARAM_PERFORMANCE_PITCH_ID));
        }
        {
            let auto_select_midi = auto_select_midi.clone();
            ui.on_auto_select_toggled(move |enabled| auto_select_midi.set(enabled));
        }
        {
            let state = state.clone();
            ui.on_direct_keys_toggled(move |enabled| {
                state.params().set_direct_keys_enabled(enabled);
            });
        }
        {
            let state = state.clone();
            let selected_pad = selected_pad.clone();
            let factory_kit_index = factory_kit_index.clone();
            ui.on_previous_kit(move || {
                let index = factory_kit_index
                    .get()
                    .unwrap_or(0)
                    .wrapping_add(FACTORY_KIT_COUNT - 1)
                    % FACTORY_KIT_COUNT;
                apply_kit(&state, &factory_kit(index), &selected_pad);
                factory_kit_index.set(Some(index));
            });
        }
        {
            let state = state.clone();
            let selected_pad = selected_pad.clone();
            let factory_kit_index = factory_kit_index.clone();
            ui.on_next_kit(move || {
                let index = (factory_kit_index.get().unwrap_or(0) + 1) % FACTORY_KIT_COUNT;
                apply_kit(&state, &factory_kit(index), &selected_pad);
                factory_kit_index.set(Some(index));
            });
        }
        {
            let state = state.clone();
            let selected_pad = selected_pad.clone();
            ui.on_pad_pressed(move |pad| {
                let pad = valid_pad_index(pad);
                selected_pad.set(pad);
                state.params().set_visualization_selected_pad(pad);
                state.params().set_pointer_held(pad, true);
            });
        }
        {
            let state = state.clone();
            ui.on_pad_released(move |pad| {
                state.params().set_pointer_held(valid_pad_index(pad), false);
            });
        }
        {
            let state = state.clone();
            let selected_pad = selected_pad.clone();
            let active_effect_gesture = active_effect_gesture.clone();
            ui.on_effect_edit_began(move || {
                let pad = selected_pad.get();
                active_effect_gesture.set(Some(EffectGesture {
                    pad,
                    effect: selected_effect(&state, pad),
                    controls_started: false,
                }));
                state.begin_edit(pad_type_id(pad));
            });
        }
        {
            let state = state.clone();
            let active_effect_gesture = active_effect_gesture.clone();
            ui.on_effect_value_changed(move |value| {
                let Some(mut gesture) = active_effect_gesture.get() else {
                    return;
                };
                let effect_index = effect_index_from_normalized(value);
                let effect = EffectType::from_index(effect_index as i32);
                if effect == gesture.effect {
                    return;
                }

                if !gesture.controls_started {
                    for control in 0..NUM_MACROS {
                        state.begin_edit(pad_control_id(gesture.pad, control));
                    }
                    gesture.controls_started = true;
                }
                state.set_param(
                    pad_type_id(gesture.pad),
                    truce::core::cast::discrete_norm(effect_index, EFFECT_COUNT),
                );
                for (control, value) in default_pad_config(effect).macros.into_iter().enumerate() {
                    state.set_param(pad_control_id(gesture.pad, control), value);
                }
                gesture.effect = effect;
                active_effect_gesture.set(Some(gesture));
            });
        }
        {
            let state = state.clone();
            let active_effect_gesture = active_effect_gesture.clone();
            ui.on_effect_edit_ended(move || {
                if let Some(gesture) = active_effect_gesture.take() {
                    state.end_edit(pad_type_id(gesture.pad));
                    if gesture.controls_started {
                        for control in 0..NUM_MACROS {
                            state.end_edit(pad_control_id(gesture.pad, control));
                        }
                    }
                }
            });
        }
        {
            let state = state.clone();
            let selected_pad = selected_pad.clone();
            let active_macro_gestures = active_macro_gestures.clone();
            ui.on_macro_edit_began(move |control| {
                let Some(control) = valid_macro_index(control) else {
                    return;
                };
                let id = pad_control_id(selected_pad.get(), control);
                active_macro_gestures[control].set(Some(id));
                state.begin_edit(id);
            });
        }
        {
            let state = state.clone();
            let selected_pad = selected_pad.clone();
            let active_macro_gestures = active_macro_gestures.clone();
            ui.on_macro_value_changed(move |control, value| {
                let Some(control) = valid_macro_index(control) else {
                    return;
                };
                if let Some(id) = active_macro_gestures[control].get() {
                    state.set_param(
                        id,
                        macro_value_for_effect(
                            selected_effect(&state, selected_pad.get()),
                            control,
                            value,
                        ),
                    );
                }
            });
        }
        {
            let state = state.clone();
            let active_macro_gestures = active_macro_gestures.clone();
            ui.on_macro_edit_ended(move |control| {
                let Some(control) = valid_macro_index(control) else {
                    return;
                };
                if let Some(id) = active_macro_gestures[control].take() {
                    state.end_edit(id);
                }
            });
        }

        let waveform = Rc::new(RefCell::new(WaveformSnapshot::default()));
        Box::new(move |state: &PluginContext<BufferUppercutParams>| {
            let (midi_press_sequence, pressed_pad) = state.params().midi_pad_press_event();
            if midi_press_sequence != last_midi_press_sequence.get() {
                last_midi_press_sequence.set(midi_press_sequence);
                if auto_select_midi.get()
                    && let Some(pad) = pressed_pad
                {
                    selected_pad.set(pad);
                }
            }

            let (direct_press_sequence, direct_pressed_pad) =
                state.params().direct_key_press_event();
            if direct_press_sequence != last_direct_key_press_sequence.get() {
                last_direct_key_press_sequence.set(direct_press_sequence);
                if let Some(pad) = direct_pressed_pad {
                    selected_pad.set(pad);
                }
            }

            let selected = selected_pad.get().min(NUM_PADS - 1);
            state.params().set_visualization_selected_pad(selected);
            ui.set_selected_pad(selected as i32);
            ui.set_auto_select(auto_select_midi.get());
            ui.set_direct_keys(state.params().direct_keys_enabled());
            ui.set_kit_display(SharedString::from(state.params().kit_name()));

            let transport = state.transport();
            let tempo = transport.map_or_else(
                || SharedString::from("--.-"),
                |transport| SharedString::from(format!("{:.1}", transport.tempo)),
            );
            ui.set_tempo_text(tempo);

            let pitch = state.get_param(PARAM_PERFORMANCE_PITCH_ID);
            ui.set_pitch_value(pitch);
            ui.set_pitch_text(SharedString::from(
                state.format_param(PARAM_PERFORMANCE_PITCH_ID),
            ));

            let midi_held_bits = state.params().midi_held_pad_bits();
            let direct_key_held_bits = state.params().direct_key_held_pad_bits();
            let pointer_held_bits = state.params().pointer_held_pad_bits();
            let (active_pad_bits, suspended_pad_bits) = state.params().admission_pad_bits();
            for pad in 0..NUM_PADS {
                let effect = selected_effect(state, pad);
                let trigger_held = state.get_param(pad_trigger_id(pad)) >= 0.5;
                pads.set_row_data(
                    pad,
                    PadView {
                        number: SharedString::from(format!("{:02}", pad + 1)),
                        key: SharedString::from(physical_key_label(pad)),
                        note: SharedString::from(format!("{}", pad + 60)),
                        effect: SharedString::from(pad_effect_abbreviation(
                            effect,
                            state.get_param(pad_control_id(pad, 0)),
                        )),
                        held: trigger_held
                            || midi_held_bits & (1_u16 << pad) != 0
                            || direct_key_held_bits & (1_u16 << pad) != 0
                            || pointer_held_bits & (1_u16 << pad) != 0
                            || editor_preview.and_then(EditorPreview::held_pad) == Some(pad),
                        active: active_pad_bits & (1_u16 << pad) != 0,
                        suspended: suspended_pad_bits & (1_u16 << pad) != 0,
                        selected: pad == selected,
                    },
                );
            }

            let effect = selected_effect(state, selected);
            ui.set_effect_value(
                truce::core::cast::discrete_norm(effect as usize, EFFECT_COUNT) as f32,
            );
            ui.set_effect_name(SharedString::from(effect_name(effect)));
            ui.set_selected_pad_note(SharedString::from(format!("MIDI {}", selected + 60)));
            for control in 0..NUM_MACROS {
                let id = pad_control_id(selected, control);
                let value = state.get_param(id);
                macros.set_row_data(
                    control,
                    MacroView {
                        label: SharedString::from(effect.control_name(control)),
                        value,
                        value_text: SharedString::from(format_control_value(
                            effect,
                            control,
                            f64::from(value),
                        )),
                        enabled: effect.is_control_enabled(control),
                    },
                );
            }

            let mut waveform = waveform.borrow_mut();
            if state.params().waveform_snapshot(&mut waveform) {
                let (left, right) = waveform_paths(&waveform.bins);
                ui.set_waveform_left(SharedString::from(left));
                ui.set_waveform_right(SharedString::from(right));
                ui.set_waveform_playhead(waveform.meta.normalized_playhead.clamp(0.0, 1.0));
                ui.set_waveform_captured(waveform.meta.mode == VisualizationMode::Capture);
                ui.set_waveform_effect_index(waveform.meta.active_effect as i32);
                ui.set_waveform_mode(SharedString::from(match waveform.meta.mode {
                    VisualizationMode::Rolling => "ROLLING / 4 BEATS",
                    VisualizationMode::Capture => "CAPTURED CELL",
                }));
                ui.set_waveform_status(SharedString::from(waveform_status(&waveform)));
            }
        })
    })
    .resizable(true)
    .min_size((920, 620))
    .physical_keyboard_input({
        let params = params.clone();
        move |event| match event {
            PhysicalKeyboardEvent::Key {
                code,
                pressed,
                repeat,
            } => crate::midi::pad_for_physical_key(code)
                .is_some_and(|pad| params.apply_direct_key(pad, pressed, repeat)),
            PhysicalKeyboardEvent::FocusChanged(false) | PhysicalKeyboardEvent::EditorClosed => {
                params.clear_direct_key_holds();
                params.clear_pointer_holds();
                false
            }
            PhysicalKeyboardEvent::FocusChanged(true) => false,
        }
    })
    .keyboard_passthrough(true);

    Box::new(editor)
}

fn apply_kit(state: &PluginContext<BufferUppercutParams>, kit: &Kit, selected_pad: &Cell<usize>) {
    state.automate(
        PARAM_PERFORMANCE_PITCH_ID,
        (kit.state.performance_pitch.clamp(-24.0, 24.0) + 24.0) / 48.0,
    );
    for pad in 0..NUM_PADS {
        state.automate(pad_trigger_id(pad), 0.0);
        state.automate(
            pad_type_id(pad),
            truce::core::cast::discrete_norm(
                kit.state.pads[pad].effect_type as usize,
                EFFECT_COUNT,
            ),
        );
        for control in 0..NUM_MACROS {
            state.automate(
                pad_control_id(pad, control),
                kit.state.pads[pad].macros[control].clamp(0.0, 1.0),
            );
        }
    }
    state.params().set_kit_name(&kit.name);
    state.params().clear_direct_key_holds();
    state.params().clear_pointer_holds();
    state.params().request_kit_reset();
    selected_pad.set(0);
    state.params().set_visualization_selected_pad(0);
}

fn configured_editor_size() -> (u32, u32) {
    std::env::var("BUFFER_UPPERCUT_EDITOR_SIZE")
        .ok()
        .and_then(|value| parse_editor_size(&value))
        .unwrap_or(DEFAULT_EDITOR_SIZE)
}

fn configured_editor_preview() -> Option<EditorPreview> {
    std::env::var("BUFFER_UPPERCUT_EDITOR_PREVIEW")
        .ok()
        .and_then(|value| parse_editor_preview(&value))
}

fn parse_editor_preview(value: &str) -> Option<EditorPreview> {
    match value {
        "captured" => Some(EditorPreview::Captured),
        "pitch" => Some(EditorPreview::Pitch),
        "vinyl" => Some(EditorPreview::Vinyl),
        _ => None,
    }
}

fn apply_editor_preview(ui: &BufferUppercutUi, preview: Option<EditorPreview>) {
    match preview {
        Some(EditorPreview::Captured) => apply_captured_editor_preview(ui),
        Some(EditorPreview::Pitch | EditorPreview::Vinyl) | None => {}
    }
}

fn apply_captured_editor_preview(ui: &BufferUppercutUi) {
    let mut bins = [VisualizationBin::default(); crate::params::NUM_VISUALIZATION_BINS];
    let bin_count = bins.len() as f32;
    for (index, bin) in bins.iter_mut().enumerate() {
        let phase = index as f32 / (bin_count - 1.0);
        let envelope = (1.0 - phase).powf(0.72) * 0.78 + 0.08;
        let carrier = (phase * 83.0).sin() * 0.58 + (phase * 29.0).sin() * 0.24;
        let side = (phase * 67.0 + 0.9).sin() * 0.5 + (phase * 17.0).sin() * 0.22;
        let left = carrier * envelope;
        let right = side * envelope;
        bin.min_l = (left - envelope * 0.16).clamp(-1.0, 1.0);
        bin.max_l = (left + envelope * 0.16).clamp(-1.0, 1.0);
        bin.min_r = (right - envelope * 0.14).clamp(-1.0, 1.0);
        bin.max_r = (right + envelope * 0.14).clamp(-1.0, 1.0);
    }

    let (left, right) = waveform_paths(&bins);
    ui.set_waveform_left(SharedString::from(left));
    ui.set_waveform_right(SharedString::from(right));
    ui.set_waveform_playhead(0.64);
    ui.set_waveform_captured(true);
    ui.set_waveform_effect_index(EffectType::BeatRepeat as i32);
    ui.set_waveform_mode(SharedString::from("CAPTURED CELL"));
    ui.set_waveform_status(SharedString::from(
        "PAD 02 / REPEAT  ·  FWD 0.50x  ·  0.50s",
    ));
}

fn parse_editor_size(value: &str) -> Option<(u32, u32)> {
    let (width, height) = value.split_once('x')?;
    let width = width.parse().ok()?;
    let height = height.parse().ok()?;
    (width >= 920 && height >= 620).then_some((width, height))
}

fn selected_effect(state: &PluginContext<BufferUppercutParams>, pad: usize) -> EffectType {
    let index = state
        .params()
        .get_plain(pad_type_id(pad))
        .unwrap_or_default()
        .round() as i32;
    EffectType::from_index(index)
}

fn valid_pad_index(index: i32) -> usize {
    index.clamp(0, NUM_PADS as i32 - 1) as usize
}

fn valid_macro_index(index: i32) -> Option<usize> {
    usize::try_from(index)
        .ok()
        .filter(|control| *control < NUM_MACROS)
}

fn effect_index_from_normalized(value: f32) -> usize {
    (value.clamp(0.0, 1.0) * (EFFECT_COUNT - 1) as f32).round() as usize
}

fn macro_value_for_effect(effect: EffectType, control: usize, value: f32) -> f64 {
    if control == 0 {
        match effect {
            EffectType::Filter => {
                return FilterMode::from_normalized(f64::from(value)).normalized();
            }
            EffectType::Pitch => return PitchRole::from_normalized(f64::from(value)).normalized(),
            _ => {}
        }
    }
    f64::from(value.clamp(0.0, 1.0))
}

fn empty_pad_view(pad: usize) -> PadView {
    PadView {
        number: SharedString::from(format!("{:02}", pad + 1)),
        key: SharedString::from(physical_key_label(pad)),
        note: SharedString::from(format!("{}", pad + 60)),
        effect: SharedString::default(),
        held: false,
        active: false,
        suspended: false,
        selected: pad == 0,
    }
}

const fn physical_key_label(pad: usize) -> &'static str {
    let pad = if pad < NUM_PADS { pad } else { NUM_PADS - 1 };
    [
        "1", "2", "3", "4", "Q", "W", "E", "R", "A", "S", "D", "F", "Z", "X", "C", "V",
    ][pad]
}

fn standalone_direct_keys_default(executable: Option<&Path>) -> bool {
    executable
        .and_then(Path::file_stem)
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| stem.eq_ignore_ascii_case("buffer-uppercut-standalone"))
}

fn empty_macro_view(control: usize) -> MacroView {
    MacroView {
        label: SharedString::from(format!("{}", control + 1)),
        value: 0.0,
        value_text: SharedString::default(),
        enabled: false,
    }
}

fn effect_name(effect: EffectType) -> &'static str {
    match effect {
        EffectType::Off => "Off",
        EffectType::BeatRepeat => "Beat Repeat",
        EffectType::Reverse => "Reverse",
        EffectType::TapeStop => "Tape Stop",
        EffectType::Gate => "Gate",
        EffectType::Pitch => "Pitch",
        EffectType::Filter => "Filter",
        EffectType::LoFi => "LoFi",
        EffectType::Vinyl => "Vinyl",
    }
}

fn effect_abbreviation(effect: EffectType) -> &'static str {
    match effect {
        EffectType::Off => "OFF",
        EffectType::BeatRepeat => "REPEAT",
        EffectType::Reverse => "REV",
        EffectType::TapeStop => "STOP",
        EffectType::Gate => "GATE",
        EffectType::Pitch => "PITCH",
        EffectType::Filter => "FILTER",
        EffectType::LoFi => "LOFI",
        EffectType::Vinyl => "VINYL",
    }
}

fn pad_effect_abbreviation(effect: EffectType, first_macro: f32) -> &'static str {
    match effect {
        EffectType::Filter => match FilterMode::from_normalized(f64::from(first_macro)) {
            FilterMode::LowPass => "LP",
            FilterMode::BandPass => "BP",
            FilterMode::HighPass => "HP",
        },
        EffectType::Pitch => match PitchRole::from_normalized(f64::from(first_macro)) {
            PitchRole::Down => "PITCH -",
            PitchRole::Trigger => "PITCH",
            PitchRole::Up => "PITCH +",
        },
        _ => effect_abbreviation(effect),
    }
}

fn waveform_status(snapshot: &WaveformSnapshot) -> String {
    let meta = snapshot.meta;
    let duration = format!("{:.2}s", meta.duration_seconds.max(0.0));
    match meta.mode {
        VisualizationMode::Rolling => format!("STEREO HISTORY  ·  {duration}"),
        VisualizationMode::Capture => {
            let pad = meta
                .active_pad
                .map_or_else(|| "--".to_owned(), |pad| format!("{:02}", pad + 1));
            let direction = if meta.reverse { "REV" } else { "FWD" };
            format!(
                "PAD {pad} / {}  ·  {direction} {:.2}x  ·  {duration}",
                effect_abbreviation(meta.active_effect),
                meta.play_rate
            )
        }
    }
}

fn waveform_paths(bins: &[VisualizationBin]) -> (String, String) {
    (
        waveform_path(bins, |bin| (bin.min_l, bin.max_l)),
        waveform_path(bins, |bin| (bin.min_r, bin.max_r)),
    )
}

fn waveform_path(
    bins: &[VisualizationBin],
    channel: impl Fn(&VisualizationBin) -> (f32, f32),
) -> String {
    if bins.is_empty() {
        return String::new();
    }

    let mut path = String::with_capacity(bins.len() * 28);
    for (index, bin) in bins.iter().enumerate() {
        let (_, max) = channel(bin);
        let x = waveform_x(index, bins.len());
        let y = waveform_y(max);
        if index == 0 {
            let _ = write!(path, "M {x:.2} {y:.2}");
        } else {
            let _ = write!(path, " L {x:.2} {y:.2}");
        }
    }
    for (index, bin) in bins.iter().enumerate().rev() {
        let (min, _) = channel(bin);
        let _ = write!(
            path,
            " L {:.2} {:.2}",
            waveform_x(index, bins.len()),
            waveform_y(min)
        );
    }
    path.push_str(" Z");
    path
}

fn waveform_x(index: usize, count: usize) -> f32 {
    if count <= 1 {
        0.0
    } else {
        index as f32 * WAVEFORM_VIEWBOX_WIDTH / (count - 1) as f32
    }
}

fn waveform_y(sample: f32) -> f32 {
    WAVEFORM_VIEWBOX_HEIGHT * 0.5 - sample.clamp(-1.0, 1.0) * WAVEFORM_AMPLITUDE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_sizes_accept_supported_dimensions_only() {
        assert_eq!(parse_editor_size("920x620"), Some((920, 620)));
        assert_eq!(parse_editor_size("1440x760"), Some((1440, 760)));
        assert_eq!(parse_editor_size("919x620"), None);
        assert_eq!(parse_editor_size("wide"), None);
    }

    #[test]
    fn configured_preview_accepts_supported_states_only() {
        assert_eq!(
            parse_editor_preview("captured"),
            Some(EditorPreview::Captured)
        );
        assert_eq!(parse_editor_preview("pitch"), Some(EditorPreview::Pitch));
        assert_eq!(parse_editor_preview("rolling"), None);
        assert_eq!(parse_editor_preview("CAPTURED"), None);
    }

    #[test]
    fn direct_keys_default_only_for_the_dedicated_standalone_executable() {
        assert!(standalone_direct_keys_default(Some(Path::new(
            "/Applications/Buffer Uppercut.app/Contents/MacOS/buffer-uppercut-standalone"
        ))));
        assert!(!standalone_direct_keys_default(Some(Path::new(
            "/Applications/REAPER.app/Contents/MacOS/REAPER"
        ))));
        assert!(!standalone_direct_keys_default(None));
    }

    #[test]
    fn effect_knob_quantizes_to_all_discrete_effect_types() {
        for effect in 0..EFFECT_COUNT {
            let normalized = truce::core::cast::discrete_norm(effect, EFFECT_COUNT) as f32;
            assert_eq!(effect_index_from_normalized(normalized), effect);
        }
        assert_eq!(effect_index_from_normalized(-1.0), 0);
        assert_eq!(effect_index_from_normalized(2.0), EFFECT_COUNT - 1);
    }

    #[test]
    fn three_position_role_knobs_quantize() {
        assert_eq!(macro_value_for_effect(EffectType::Filter, 0, 0.1), 0.0);
        assert_eq!(macro_value_for_effect(EffectType::Filter, 0, 0.5), 0.5);
        assert_eq!(macro_value_for_effect(EffectType::Filter, 0, 0.9), 1.0);
        assert_eq!(macro_value_for_effect(EffectType::Pitch, 0, 0.1), 0.0);
        assert_eq!(macro_value_for_effect(EffectType::Pitch, 0, 0.5), 0.5);
        assert_eq!(macro_value_for_effect(EffectType::Pitch, 0, 0.9), 1.0);
        assert_eq!(
            macro_value_for_effect(EffectType::Gate, 0, 0.1),
            0.1_f32 as f64
        );
    }

    #[test]
    fn macro_drag_uses_current_pad_value_after_model_changes() {
        use slint::platform::{PointerEventButton, WindowEvent};
        use truce_slint::platform;

        platform::ensure_platform();
        let window = platform::create_slint_window();
        window.set_size(slint::PhysicalSize::new(1120, 700));
        let ui = BufferUppercutUi::new().unwrap();
        let macros = Rc::new(VecModel::from(
            (0..NUM_MACROS)
                .map(|_| MacroView {
                    label: "WET".into(),
                    value: 1.0,
                    value_text: "100%".into(),
                    enabled: true,
                })
                .collect::<Vec<_>>(),
        ));
        ui.set_macros(ModelRc::from(macros.clone()));
        let last_value = Rc::new(Cell::new(None));
        ui.on_macro_value_changed({
            let last_value = last_value.clone();
            move |control, value| last_value.set(Some((control, value)))
        });
        let mut pixels = Vec::new();
        let mut rgba = Vec::new();
        let mut render = || {
            window.request_redraw();
            platform::render_to_rgba(&window, 1120, 700, &mut pixels, &mut rgba);
        };
        render();
        let drag = || {
            let position = slint::LogicalPosition::new(1060.0, 520.0);
            window.dispatch_event(WindowEvent::PointerPressed {
                position,
                button: PointerEventButton::Left,
            });
            let position = slint::LogicalPosition::new(1060.0, 534.5);
            window.dispatch_event(WindowEvent::PointerMoved { position });
            window.dispatch_event(WindowEvent::PointerReleased {
                position,
                button: PointerEventButton::Left,
            });
        };
        drag();
        let (control, value) = last_value.get().expect("drag reaches Wet knob");
        assert_eq!(control, 6);
        assert!((value - 0.9).abs() < 1e-5);

        // Selecting another pad or recalling parameters changes this same row.
        let mut row = macros.row_data(6).unwrap();
        row.value = 0.4;
        row.value_text = "40%".into();
        macros.set_row_data(6, row);
        render();
        drag();
        let (_, value) = last_value.get().unwrap();
        assert!((value - 0.3).abs() < 1e-5, "drag used stale value: {value}");
    }

    #[test]
    fn waveform_geometry_is_closed_and_finite() {
        let bins = [
            VisualizationBin {
                min_l: -1.0,
                max_l: 1.0,
                min_r: -0.5,
                max_r: 0.5,
            },
            VisualizationBin::default(),
        ];
        let (left, right) = waveform_paths(&bins);
        assert!(left.starts_with("M 0.00 2.00"));
        assert!(left.ends_with(" Z"));
        assert!(right.ends_with(" Z"));
        assert!(!left.contains("NaN"));
        assert!(!right.contains("NaN"));
    }
}
