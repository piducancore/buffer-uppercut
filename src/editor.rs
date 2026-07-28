use std::sync::Arc;
use std::time::Duration;

use buffer_uppercut_dsp::{EffectType, format_control_value};
use egui::{Color32, CornerRadius, Frame, Margin, RichText, Stroke};
use truce::core::editor::PluginContextReadF32;
use truce::prelude::{Editor, Params, PluginContext};
use truce_egui::{
    EguiEditor,
    widgets::{param_dropdown, param_knob, param_knob_with_value_text, param_toggle},
};

use crate::params::{
    BufferUppercutParams, NUM_MACROS, NUM_PADS, PARAM_PERFORMANCE_PITCH_ID, pad_control_id,
    pad_trigger_id, pad_type_id,
};

const ACCENT: Color32 = Color32::from_rgb(244, 104, 58);
const SURFACE: Color32 = Color32::from_rgb(30, 33, 38);
const PANEL: Color32 = Color32::from_rgb(40, 44, 51);

pub fn create(params: Arc<BufferUppercutParams>) -> Box<dyn Editor> {
    let mut selected_pad = 0_usize;
    let (mut last_midi_press_sequence, _) = params.midi_pad_press_event();
    let mut auto_select_midi = true;
    let editor = EguiEditor::new(params, (920, 520), move |ui, state| {
        ui.ctx().request_repaint_after(Duration::from_millis(30));
        let (midi_press_sequence, pressed_pad) = state.params().midi_pad_press_event();
        if midi_press_sequence != last_midi_press_sequence {
            last_midi_press_sequence = midi_press_sequence;
            if let (true, Some(pad)) = (auto_select_midi, pressed_pad) {
                selected_pad = pad;
            }
        }
        ui.style_mut().spacing.item_spacing = egui::vec2(8.0, 8.0);
        Frame::NONE
            .fill(SURFACE)
            .inner_margin(Margin::same(20))
            .show(ui, |ui| {
                header(ui, state);
                ui.add_space(10.0);
                ui.columns(2, |columns| {
                    Frame::NONE
                        .fill(PANEL)
                        .corner_radius(CornerRadius::same(10))
                        .inner_margin(Margin::same(14))
                        .show(&mut columns[0], |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("PERFORMANCE PADS").strong());
                                ui.checkbox(&mut auto_select_midi, "Auto-select MIDI")
                                    .on_hover_text(
                                        "Select a pad when its MIDI note is pressed",
                                    );
                            });
                            ui.add_space(8.0);
                            pads(ui, state, &mut selected_pad);
                        });

                    Frame::NONE
                        .fill(PANEL)
                        .corner_radius(CornerRadius::same(10))
                        .inner_margin(Margin::same(14))
                        .show(&mut columns[1], |ui| {
                            selected_pad_controls(ui, state, selected_pad);
                        });
                });
                ui.add_space(10.0);
                ui.label(
                    RichText::new(
                        "Independent f64 DSP port · parameters, MIDI, state and host transport are live",
                    )
                    .small()
                    .color(Color32::from_gray(150)),
                );
            });
    })
    .with_visuals(visuals())
    .resizable(true)
    .min_size((760, 500));

    Box::new(editor)
}

fn header(ui: &mut egui::Ui, state: &PluginContext<BufferUppercutParams>) {
    ui.horizontal(|ui| {
        ui.set_height(88.0);
        ui.vertical(|ui| {
            ui.label(
                RichText::new("BUFFER UPPERCUT")
                    .size(25.0)
                    .strong()
                    .color(Color32::WHITE),
            );
            ui.label(
                RichText::new("TRUCE · NATIVE EGUI PREVIEW")
                    .size(11.0)
                    .color(ACCENT),
            );
        });
        ui.add_space(300.0);
        param_knob(ui, state, PARAM_PERFORMANCE_PITCH_ID, "Pitch");

        let transport = state.transport();
        let bpm = transport.map_or("--".to_owned(), |t| format!("{:.1}", t.tempo));
        ui.vertical(|ui| {
            ui.label(RichText::new(bpm).size(20.0).strong());
            ui.label(
                RichText::new("HOST BPM")
                    .small()
                    .color(Color32::from_gray(150)),
            );
        });
    });
}

fn pads(ui: &mut egui::Ui, state: &PluginContext<BufferUppercutParams>, selected_pad: &mut usize) {
    egui::Grid::new("performance-pad-grid")
        .num_columns(4)
        .spacing([8.0, 8.0])
        .show(ui, |ui| {
            for pad in 0..NUM_PADS {
                let selected = *selected_pad == pad;
                let trigger_held = state.get_param(pad_trigger_id(pad)) >= 0.5;
                let midi_held = state.params().midi_held_pad_bits() & (1 << pad) != 0;
                let held = trigger_held || midi_held;
                Frame::NONE
                    .fill(if held {
                        ACCENT
                    } else if selected {
                        Color32::from_rgb(63, 48, 44)
                    } else {
                        Color32::from_rgb(48, 52, 60)
                    })
                    .stroke(Stroke::new(
                        1.0_f32,
                        if selected || held { ACCENT } else { PANEL },
                    ))
                    .corner_radius(CornerRadius::same(7))
                    .inner_margin(Margin::symmetric(7, 6))
                    .show(ui, |ui| {
                        ui.set_min_width(74.0);
                        if ui
                            .selectable_label(selected, format!("{:02}", pad + 1))
                            .clicked()
                        {
                            *selected_pad = pad;
                        }
                        param_toggle(ui, state, pad_trigger_id(pad), "Trigger");
                    });
                if pad % 4 == 3 {
                    ui.end_row();
                }
            }
        });
}

fn selected_pad_controls(
    ui: &mut egui::Ui,
    state: &PluginContext<BufferUppercutParams>,
    selected_pad: usize,
) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("PAD {:02}", selected_pad + 1))
                .size(18.0)
                .strong(),
        );
        ui.label(RichText::new(format!("MIDI {}", selected_pad + 60)).color(ACCENT));
    });
    ui.add_space(8.0);
    param_dropdown(ui, state, pad_type_id(selected_pad), "Effect", 2);
    ui.separator();
    ui.label(RichText::new("MACROS").strong());
    ui.add_space(4.0);
    let effect_index = state
        .params()
        .get_plain(pad_type_id(selected_pad))
        .unwrap_or_default()
        .round() as i32;
    let effect_type = EffectType::from_index(effect_index);
    egui::Grid::new("selected-pad-macros")
        .num_columns(4)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            for control in 0..NUM_MACROS {
                let id = pad_control_id(selected_pad, control);
                let normalized = state.params().get_plain(id).unwrap_or_default();
                let value_text = format_control_value(effect_type, control, normalized);
                ui.add_enabled_ui(effect_type.is_control_enabled(control), |ui| {
                    param_knob_with_value_text(
                        ui,
                        state,
                        id,
                        effect_type.control_name(control),
                        &value_text,
                    );
                });
                if control % 4 == 3 {
                    ui.end_row();
                }
            }
        });
}

fn visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = SURFACE;
    visuals.window_fill = SURFACE;
    visuals.selection.bg_fill = ACCENT;
    visuals.widgets.active.bg_fill = ACCENT;
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(216, 86, 47);
    visuals
}
