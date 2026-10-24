use buffer_uppercut_dsp::StageVisualization;
use std::cell::{Cell, RefCell};
use std::fmt::Write as _;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use buffer_uppercut_dsp::{
    EffectType, FilterMode, PitchRole, VisualizationBin, default_pad_config, format_control_value,
};
use buffer_uppercut_kit::{FACTORY_KIT_COUNT, Kit, factory_kit};
use truce::core::editor::PluginContextReadF32;
use truce::prelude::{Editor, Params, PluginContext};
use truce_slint::slint::{Model, ModelRc, SharedString, VecModel, include_modules};
use truce_slint::{PhysicalKeyboardEvent, SlintEditor};

use crate::params::{
    BufferUppercutParams, NUM_MACROS, NUM_PADS, WaveformSnapshot, pad_control_id, pad_trigger_id,
    pad_type_id,
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
    Suspended,
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
            Self::Captured | Self::Suspended => 1,
            Self::Pitch => 9,
            Self::Vinyl => 0,
        }
    }

    const fn held_pad(self) -> Option<usize> {
        match self {
            Self::Captured | Self::Suspended => Some(1),
            Self::Pitch => None,
            Self::Vinyl => Some(0),
        }
    }
    fn active(self, pad: usize) -> bool {
        match self {
            Self::Captured => [1, 6, 8, 12].contains(&pad),
            Self::Suspended => [1, 6, 7, 8, 10, 15].contains(&pad),
            Self::Vinyl => pad == 0,
            Self::Pitch => false,
        }
    }

    fn suspended(self, pad: usize) -> bool {
        self == Self::Suspended && [12, 14].contains(&pad)
    }
}

pub fn create(params: Arc<BufferUppercutParams>) -> Box<dyn Editor> {
    params.initialize_direct_keys_enabled(standalone_direct_keys_default(
        std::env::current_exe().ok().as_deref(),
    ));
    let input = Arc::new(Mutex::new(crate::input::PadInput::default()));
    let keyboard_input = input.clone();
    let editor = SlintEditor::new(params.clone(), configured_editor_size(), move |state| {
        input.lock().expect("pad input").attach(state.clone());
        let ui = BufferUppercutUi::new().expect("create Buffer Uppercut Slint editor");
        let editor_preview = configured_editor_preview();
        ui.on_scale_path(|data, sx, sy| scale_path(&data, sx, sy).into());
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
        if editor_preview == Some(EditorPreview::Suspended) {
            for (pad, effect) in [(14, EffectType::LoFi), (15, EffectType::Vinyl)] {
                let config = default_pad_config(effect);
                state
                    .params()
                    .set_plain(pad_type_id(pad), effect as u8 as f64);
                for (control, value) in config.macros.iter().enumerate() {
                    state
                        .params()
                        .set_plain(pad_control_id(pad, control), *value);
                }
            }
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
            let input = input.clone();
            let selected_pad = selected_pad.clone();
            ui.on_learn_key(move || input.lock().expect("pad input").learn(selected_pad.get()));
        }
        {
            let input = input.clone();
            ui.on_cancel_learn(move || input.lock().expect("pad input").cancel_learn());
        }
        {
            let input = input.clone();
            let selected_pad = selected_pad.clone();
            ui.on_clear_key(move || {
                input
                    .lock()
                    .expect("pad input")
                    .clear_assignment(selected_pad.get())
            });
        }
        {
            let input = input.clone();
            ui.on_reset_keys(move || input.lock().expect("pad input").reset_layout());
        }
        {
            let input = input.clone();
            ui.on_swap_keys(move || input.lock().expect("pad input").confirm_swap());
        }

        {
            let auto_select_midi = auto_select_midi.clone();
            ui.on_auto_select_toggled(move |enabled| auto_select_midi.set(enabled));
        }
        {
            let state = state.clone();
            let input = input.clone();
            ui.on_direct_keys_toggled(move |enabled| {
                if !enabled {
                    input.lock().expect("pad input").release_keys();
                }
                state.params().set_direct_keys_enabled(enabled);
            });
        }
        {
            let state = state.clone();
            let selected_pad = selected_pad.clone();
            let factory_kit_index = factory_kit_index.clone();
            let input = input.clone();
            ui.on_previous_kit(move || {
                let index = factory_kit_index
                    .get()
                    .unwrap_or(0)
                    .wrapping_add(FACTORY_KIT_COUNT - 1)
                    % FACTORY_KIT_COUNT;
                input.lock().expect("pad input").release_all();
                apply_kit(&state, &factory_kit(index), &selected_pad);
                factory_kit_index.set(Some(index));
            });
        }
        {
            let state = state.clone();
            let selected_pad = selected_pad.clone();
            let factory_kit_index = factory_kit_index.clone();
            let input = input.clone();
            ui.on_next_kit(move || {
                let index = (factory_kit_index.get().unwrap_or(0) + 1) % FACTORY_KIT_COUNT;
                input.lock().expect("pad input").release_all();
                apply_kit(&state, &factory_kit(index), &selected_pad);
                factory_kit_index.set(Some(index));
            });
        }
        {
            let state = state.clone();
            let selected_pad = selected_pad.clone();
            let input = input.clone();
            ui.on_pad_pressed(move |pad| {
                let pad = valid_pad_index(pad);
                selected_pad.set(pad);
                state.params().set_visualization_selected_pad(pad);
                input.lock().expect("pad input").pointer(pad, true);
            });
        }
        {
            let input = input.clone();
            ui.on_pad_released(move |pad| {
                input
                    .lock()
                    .expect("pad input")
                    .pointer(valid_pad_index(pad), false);
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

        {
            let selected_pad = selected_pad.clone();
            ui.on_focus_stage(move |pad| selected_pad.set(valid_pad_index(pad)));
        }
        let waveform = Rc::new(RefCell::new(WaveformSnapshot::default()));
        let input = input.clone();
        Box::new(move |state: &PluginContext<BufferUppercutParams>| {
            {
                let mut input = input.lock().expect("pad input");
                input.synchronize();
                ui.set_key_learning(input.learning_active());
                ui.set_key_conflict(input.conflict_pending());
                ui.set_key_status(SharedString::from(input.learning_status()));
            }
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

            let active_pitch_shift = state.params().active_pitch_shift();
            ui.set_pitch_value((active_pitch_shift as f32 + 24.0) / 48.0);
            ui.set_pitch_text(SharedString::from(format!("{active_pitch_shift} st")));

            let midi_held_bits = state.params().midi_held_pad_bits();
            let (active_pad_bits, suspended_pad_bits) = state.params().admission_pad_bits();
            for pad in 0..NUM_PADS {
                let effect = selected_effect(state, pad);
                let trigger_held = state.get_param(pad_trigger_id(pad)) >= 0.5
                    && !state.params().trigger_is_suppressed(pad);
                pads.set_row_data(
                    pad,
                    PadView {
                        number: SharedString::from(format!("{:02}", pad + 1)),
                        key: SharedString::from(
                            state.params().key_map().key(pad).expect("pad").label(),
                        ),
                        note: SharedString::from(format!("{}", pad + 60)),
                        effect: SharedString::from(pad_effect_abbreviation(
                            effect,
                            state.get_param(pad_control_id(pad, 0)),
                        )),
                        held: trigger_held
                            || midi_held_bits & (1_u16 << pad) != 0
                            || editor_preview.and_then(EditorPreview::held_pad) == Some(pad)
                            || editor_preview.is_some_and(|preview| {
                                preview.active(pad) || preview.suspended(pad)
                            }),
                        active: active_pad_bits & (1_u16 << pad) != 0
                            || editor_preview.is_some_and(|preview| preview.active(pad)),
                        suspended: suspended_pad_bits & (1_u16 << pad) != 0
                            || editor_preview.is_some_and(|preview| preview.suspended(pad)),
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
                let (left, right) = waveform_paths(&waveform.input_bins);
                ui.set_input_left(left.into());
                ui.set_input_right(right.into());
                let key_map = state.params().key_map();
                let mut ordered: Vec<_> = waveform
                    .stages
                    .iter()
                    .enumerate()
                    .filter(|(_, stage)| stage.active || stage.suspended)
                    .collect();
                ordered.sort_by_key(|(_, stage)| stage.chain_position);
                let views: Vec<_> = ordered
                    .into_iter()
                    .map(|(pad, stage)| {
                        stage_view(pad, key_map.key(pad).expect("pad").label(), stage)
                    })
                    .collect();
                ui.set_chain_stages(ModelRc::new(VecModel::from(
                    views
                        .iter()
                        .filter(|v| !v.suspended)
                        .cloned()
                        .collect::<Vec<_>>(),
                )));
                ui.set_suspended_stages(ModelRc::new(VecModel::from(
                    views
                        .into_iter()
                        .filter(|v| v.suspended)
                        .collect::<Vec<_>>(),
                )));
                ui.set_waveform_status(SharedString::from(waveform_status(&waveform)));
            }
        })
    })
    .resizable(true)
    .min_size((920, 620))
    .physical_keyboard_input({
        move |event| match event {
            PhysicalKeyboardEvent::Key {
                code,
                pressed,
                repeat,
            } => crate::keyboard::key_for_code(code).is_some_and(|key| {
                keyboard_input
                    .lock()
                    .expect("pad input")
                    .key(key, pressed, repeat)
            }),
            PhysicalKeyboardEvent::FocusChanged(false) => {
                let mut input = keyboard_input.lock().expect("pad input");
                input.focus_lost();
                false
            }
            PhysicalKeyboardEvent::EditorClosed => {
                keyboard_input.lock().expect("pad input").detach();
                false
            }
            PhysicalKeyboardEvent::FocusChanged(true) => false,
        }
    })
    .keyboard_passthrough(true);

    Box::new(editor)
}

fn apply_kit(state: &PluginContext<BufferUppercutParams>, kit: &Kit, selected_pad: &Cell<usize>) {
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
    state.params().set_key_map(kit.key_map);
    state.mark_state_dirty();
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
        "suspended" => Some(EditorPreview::Suspended),
        "pitch" => Some(EditorPreview::Pitch),
        "vinyl" => Some(EditorPreview::Vinyl),
        _ => None,
    }
}

fn apply_editor_preview(ui: &BufferUppercutUi, preview: Option<EditorPreview>) {
    match preview {
        Some(EditorPreview::Captured) => apply_captured_editor_preview(ui),
        Some(EditorPreview::Suspended) => {
            apply_captured_editor_preview(ui);
            apply_suspended_editor_preview(ui);
        }
        Some(EditorPreview::Vinyl) => {
            let config = factory_kit(4).state.pads[0];
            let stage = StageVisualization {
                effect: EffectType::Vinyl,
                active: true,
                phase: 0.38,
                value: 0.42,
                wet: config.macros[6] as f32,
                controls: config.macros.map(|v| v as f32),
                activity: 0.3,
                ..StageVisualization::default()
            };
            ui.set_chain_stages(ModelRc::new(VecModel::from(vec![stage_view(
                0, "1", &stage,
            )])));
            ui.set_waveform_status("1/6 ACTIVE  ·  4 BEATS / 2.00s".into());
        }
        Some(EditorPreview::Pitch) | None => {}
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
    let (left, right) = waveform_paths(&bins);
    ui.set_input_left(left.into());
    ui.set_input_right(right.into());
    let mut views = Vec::new();
    for (pad, key, effect, phase, value) in [
        (1, "2", EffectType::BeatRepeat, 0.64, 0.5),
        (6, "E", EffectType::Reverse, 0.3, -1.0),
        (8, "A", EffectType::Gate, 0.6, 0.8),
        (12, "Z", EffectType::Filter, 0.45, 450.0),
    ] {
        let mut stage = StageVisualization {
            effect,
            active: true,
            phase,
            value,
            wet: 1.0,
            duration: 0.5,
            controls: [0.5; 7],
            ..StageVisualization::default()
        };
        if effect == EffectType::Filter {
            stage.controls[0] = 0.0;
        }
        for (i, v) in stage.trace.iter_mut().enumerate() {
            *v = (i as f32 * 1.3).sin() * 0.6;
        }
        views.push(stage_view(pad, key, &stage));
    }
    ui.set_chain_stages(ModelRc::new(VecModel::from(views)));
    ui.set_waveform_status("4/6 ACTIVE  ·  4 BEATS / 2.00s".into());
}

fn apply_suspended_editor_preview(ui: &BufferUppercutUi) {
    let mut stages = Vec::new();
    for (pad, key, effect, value) in [
        (1, "2", EffectType::BeatRepeat, 0.5),
        (6, "E", EffectType::Reverse, -1.0),
        (7, "R", EffectType::TapeStop, 0.24),
        (8, "A", EffectType::Gate, 0.65),
        (10, "D", EffectType::Pitch, 7.0),
        (15, "V", EffectType::Vinyl, 2.4),
    ] {
        let stage = StageVisualization {
            effect,
            active: true,
            phase: 0.42,
            value,
            wet: 0.8,
            controls: [0.5; 7],
            duration: 0.5,
            activity: 0.7,
            trace: std::array::from_fn(|i| (i as f32 * 1.3).sin() * 0.6),
            ..StageVisualization::default()
        };
        stages.push(stage_view(pad, key, &stage));
    }
    ui.set_chain_stages(ModelRc::new(VecModel::from(stages)));
    let paused = [
        StageVisualization {
            effect: EffectType::Filter,
            suspended: true,
            value: 1200.0,
            ..StageVisualization::default()
        },
        StageVisualization {
            effect: EffectType::LoFi,
            suspended: true,
            value: 8.0,
            ..StageVisualization::default()
        },
    ];
    ui.set_suspended_stages(ModelRc::new(VecModel::from(vec![
        stage_view(12, "Z", &paused[0]),
        stage_view(14, "C", &paused[1]),
    ])));
    ui.set_waveform_status("6/6 ACTIVE  ·  2 PAUSED  ·  4 BEATS / 2.00s".into());
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
    let active = snapshot.stages.iter().filter(|stage| stage.active).count();
    format!(
        "{active}/6 ACTIVE  ·  4 BEATS / {:.2}s",
        snapshot.meta.duration_seconds.max(0.0)
    )
}

fn stage_view(pad: usize, key: &str, stage: &StageVisualization) -> StageView {
    let detail = match stage.effect {
        EffectType::BeatRepeat | EffectType::Reverse | EffectType::TapeStop => format!(
            "{} {:.2}x / {:.2}s",
            if stage.value < 0.0 { "←" } else { "→" },
            stage.value.abs(),
            stage.duration
        ),
        EffectType::Gate => format!("GATE {:03.0}%", stage.value * 100.0),
        EffectType::Pitch => format!("{:+.1} st / GRAINS", stage.value),
        EffectType::Filter => format!("{:.0} Hz / CUTOFF", stage.value),
        EffectType::LoFi => format!(
            "{:.0} BIT / {:.1}k",
            stage.value,
            (1000.0 + stage.controls[0] * 43100.0) / 1000.0
        ),
        EffectType::Vinyl => format!("{:.2} ms / WOW", stage.value),
        EffectType::Off => String::new(),
    };
    StageView {
        pad: pad as i32,
        label: format!(
            "{:02} {} / {}",
            pad + 1,
            key,
            pad_effect_abbreviation(stage.effect, stage.controls[0])
        )
        .into(),
        detail: (if stage.wet <= 0.0 {
            "DRY".to_owned()
        } else {
            detail
        })
        .into(),
        path: stage_signature(stage).into(),
        phase: stage.phase.clamp(0.0, 1.0),
        strength: stage.wet.clamp(0.0, 1.0),
        cursor: stage.effect.is_buffer() || stage.effect == EffectType::Gate,
        reverse: stage.value < 0.0,
        suspended: stage.suspended,
    }
}

/// Effect glyphs describe processor state; only buffer traces depict audio.
fn stage_signature(stage: &StageVisualization) -> String {
    let mut path = String::with_capacity(1800);
    for i in 0..64 {
        let x = i as f32 / 63.0;
        let y = match stage.effect {
            EffectType::BeatRepeat | EffectType::Reverse | EffectType::TapeStop => {
                stage.trace[(x * 31.0).round() as usize].clamp(-1.0, 1.0)
            }
            EffectType::Gate => {
                let duty = 0.05 + 0.90 * stage.controls[1];
                if (x * 4.0).fract() < duty {
                    0.75
                } else {
                    -0.75
                }
            }
            EffectType::Pitch => {
                let grain = (x - stage.phase).rem_euclid(1.0);
                1.0 - 4.0 * (grain - 0.5).abs()
            }
            EffectType::Filter => {
                let cutoff = stage.phase;
                let low = 1.0 / (1.0 + ((x - cutoff) * 20.0).exp());
                let mode = FilterMode::from_normalized(f64::from(stage.controls[0]));
                let response = match mode {
                    FilterMode::LowPass => low,
                    FilterMode::HighPass => 1.0 - low,
                    FilterMode::BandPass => (-(x - cutoff).powi(2) * 90.0).exp(),
                };
                (response + stage.controls[2] * (-(x - cutoff).powi(2) * 600.0).exp() * 0.3) * 1.5
                    - 0.75
            }
            EffectType::LoFi => {
                let steps = 4.0 + stage.controls[0] * 24.0;
                let sample =
                    (((x * steps).floor() / steps) * std::f32::consts::TAU + stage.phase).sin();
                let levels = 2.0_f32.powf(stage.value.clamp(2.0, 6.0) - 1.0);
                (sample * levels).round() / levels * 0.8
            }
            EffectType::Vinyl => {
                let phase = stage.phase * std::f32::consts::TAU;
                ((x * std::f32::consts::TAU + phase).sin() * stage.controls[0] * 0.7
                    + (x * 32.0 + stage.secondary_phase * std::f32::consts::TAU).sin()
                        * stage.controls[1]
                        * 0.25)
                    .clamp(-1.0, 1.0)
                    + (x * 170.0).sin() * stage.controls[5] * 0.12
                    + if (x - 0.72).abs() < 0.02 {
                        stage.activity * 0.9
                    } else {
                        0.0
                    }
            }
            EffectType::Off => 0.0,
        };
        write!(
            path,
            "{} {:.2} {:.2} ",
            if i == 0 { "M" } else { "L" },
            x * 100.0,
            12.0 - y.clamp(-1.0, 1.0) * 10.0
        )
        .expect("string");
    }
    if stage.effect == EffectType::Pitch {
        for i in 0..32 {
            let x = i as f32 / 31.0;
            let grain = (x - stage.phase + 0.5).rem_euclid(1.0);
            let y = 1.0 - 4.0 * (grain - 0.5).abs();
            write!(
                path,
                "{} {:.2} {:.2} ",
                if i == 0 { "M" } else { "L" },
                x * 100.0,
                12.0 - y * 7.0
            )
            .expect("string");
        }
    }
    path
}

/// Slint's software renderer ignores scale transforms. Build physical path
/// coordinates so both native and screenshot renderers share the same geometry.
fn scale_path(path: &str, sx: f32, sy: f32) -> String {
    if !sx.is_finite() || !sy.is_finite() || sx <= 0.0 || sy <= 0.0 {
        return String::new();
    }
    let mut result = String::with_capacity(path.len());
    let mut x = true;
    for token in path.split_whitespace() {
        if matches!(token, "M" | "L" | "Z") {
            result.push_str(token);
            result.push(' ');
            x = true;
        } else if let Ok(value) = token.parse::<f32>() {
            let scaled = value * if x { sx } else { sy };
            write!(result, "{scaled:.2} ").expect("string");
            x = !x;
        }
    }
    result
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
