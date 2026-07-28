mod editor;
mod midi;
mod params;

use buffer_uppercut_dsp::{
    EffectType, Engine, NUM_MACROS, NUM_PADS, PadConfig, PerformanceState, apply_pitch_action,
};
use truce::prelude64::*;

pub use params::BufferUppercutParams;

pub struct BufferUppercutTruce {
    engine: Engine,
    input_l: Vec<f64>,
    input_r: Vec<f64>,
    output_l: Vec<f64>,
    output_r: Vec<f64>,
    held_pads_by_channel: [u16; 16],
    previous_held: [bool; NUM_PADS],
    performance_pitch: f64,
    last_parameter_pitch: f64,
    last_tempo: f64,
}

impl Default for BufferUppercutTruce {
    fn default() -> Self {
        Self {
            engine: Engine::default(),
            input_l: Vec::new(),
            input_r: Vec::new(),
            output_l: Vec::new(),
            output_r: Vec::new(),
            held_pads_by_channel: [0; 16],
            previous_held: [false; NUM_PADS],
            performance_pitch: 0.0,
            last_parameter_pitch: 0.0,
            last_tempo: 120.0,
        }
    }
}

impl PluginLogic for BufferUppercutTruce {
    type Params = BufferUppercutParams;
    type DspState = Self;

    fn reset(state: &mut Self::DspState, params: &Self::Params, config: &AudioConfig) {
        state.engine.reset(config.sample_rate);
        state.input_l.resize(config.max_block_size, 0.0);
        state.input_r.resize(config.max_block_size, 0.0);
        state.output_l.resize(config.max_block_size, 0.0);
        state.output_r.resize(config.max_block_size, 0.0);
        state.held_pads_by_channel = [0; 16];
        state.previous_held = [false; NUM_PADS];
        params.set_midi_held_pad_bits(0);
        state.performance_pitch = params
            .get_plain(params::PARAM_PERFORMANCE_PITCH_ID)
            .unwrap_or_default()
            .clamp(-24.0, 24.0);
        state.last_parameter_pitch = state.performance_pitch;
    }

    fn process(
        state: &mut Self::DspState,
        params: &Self::Params,
        buffer: &mut AudioBuffer,
        events: &EventList,
        context: &mut ProcessContext,
    ) -> ProcessStatus {
        state.last_tempo = context.transport.tempo.max(1.0);
        if let Some(pad) = midi::apply_events(&mut state.held_pads_by_channel, events) {
            params.record_midi_pad_press(pad);
        }
        params.set_midi_held_pad_bits(aggregate_midi_held(&state.held_pads_by_channel));

        let parameter_pitch = params
            .get_plain(params::PARAM_PERFORMANCE_PITCH_ID)
            .unwrap_or_default()
            .clamp(-24.0, 24.0);
        if parameter_pitch != state.last_parameter_pitch {
            state.performance_pitch = parameter_pitch;
            state.last_parameter_pitch = parameter_pitch;
        }

        let mut performance = performance_state(params, &state.held_pads_by_channel);
        let pitch_before_actions = state.performance_pitch;
        for pad in 0..NUM_PADS {
            if performance.held[pad] && !state.previous_held[pad] {
                let config = &performance.pads[pad];
                if config.effect_type.is_pitch_action() {
                    state.performance_pitch =
                        apply_pitch_action(state.performance_pitch, config.effect_type, config);
                }
            }
        }
        if state.performance_pitch != pitch_before_actions {
            context.output_events.push(Event::new(
                0,
                EventBody::ParamChange {
                    id: params::PARAM_PERFORMANCE_PITCH_ID,
                    value: state.performance_pitch,
                },
            ));
        }
        state.previous_held = performance.held;
        performance.performance_pitch = state.performance_pitch;

        let frames = buffer.num_samples();
        debug_assert!(frames <= state.input_l.len());
        if frames > state.input_l.len() {
            return ProcessStatus::Normal;
        }
        let channels = buffer.channels();
        if channels == 0 {
            return ProcessStatus::Normal;
        }
        {
            let (input, _) = buffer.io(0);
            state.input_l[..frames].copy_from_slice(input);
        }
        if channels > 1 {
            let (input, _) = buffer.io(1);
            state.input_r[..frames].copy_from_slice(input);
        } else {
            state.input_r[..frames].copy_from_slice(&state.input_l[..frames]);
        }

        state.engine.process(
            &state.input_l[..frames],
            &state.input_r[..frames],
            &mut state.output_l[..frames],
            &mut state.output_r[..frames],
            state.last_tempo,
            &performance,
        );
        zero_subnormals(&mut state.output_l[..frames]);
        zero_subnormals(&mut state.output_r[..frames]);
        {
            let (_, output) = buffer.io(0);
            output.copy_from_slice(&state.output_l[..frames]);
        }
        if channels > 1 {
            let (_, output) = buffer.io(1);
            output.copy_from_slice(&state.output_r[..frames]);
        }

        ProcessStatus::Normal
    }

    fn state_changed(state: &mut Self::DspState, params: &Self::Params) {
        let restored_pitch = params
            .get_plain(params::PARAM_PERFORMANCE_PITCH_ID)
            .unwrap_or_default()
            .clamp(-24.0, 24.0);
        state.performance_pitch = restored_pitch;
        state.last_parameter_pitch = restored_pitch;
    }

    fn editor(params: Arc<Self::Params>) -> Box<dyn Editor> {
        editor::create(params)
    }
}

fn zero_subnormals(samples: &mut [f64]) {
    for sample in samples {
        if sample.abs() < f64::from(f32::MIN_POSITIVE) {
            *sample = 0.0;
        }
    }
}

fn performance_state(
    params: &BufferUppercutParams,
    held_pads_by_channel: &[u16; 16],
) -> PerformanceState {
    let midi_held = aggregate_midi_held(held_pads_by_channel);
    let mut state = PerformanceState::default();
    for pad in 0..NUM_PADS {
        let effect_index = params
            .get_plain(params::pad_type_id(pad))
            .unwrap_or_default()
            .round() as i32;
        let mut config = PadConfig {
            effect_type: EffectType::from_index(effect_index),
            ..PadConfig::default()
        };
        for control in 0..NUM_MACROS {
            config.macros[control] = params
                .get_plain(params::pad_control_id(pad, control))
                .unwrap_or_default();
        }
        state.pads[pad] = config;
        let trigger = params
            .get_plain(params::pad_trigger_id(pad))
            .unwrap_or_default()
            >= 0.5;
        state.held[pad] = trigger || (midi_held & (1_u16 << pad)) != 0;
    }
    state
}

fn aggregate_midi_held(held_pads_by_channel: &[u16; 16]) -> u16 {
    held_pads_by_channel
        .iter()
        .copied()
        .fold(0_u16, |all, channel| all | channel)
}

truce::plugin! {
    logic: BufferUppercutTruce,
    params: BufferUppercutParams,
}

truce::enable_rt_paranoid!();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameters_build_the_classic_engine_state() {
        let params = BufferUppercutParams::default();
        let state = performance_state(&params, &[0; 16]);
        assert_eq!(state.pads[0].effect_type, EffectType::BeatRepeat);
        assert_eq!(state.pads[6].effect_type, EffectType::Reverse);
        assert_eq!(state.pads[15].effect_type, EffectType::LoFi);
        assert!(!state.held.iter().any(|held| *held));
    }

    #[test]
    fn midi_and_trigger_parameters_are_aggregated() {
        let params = BufferUppercutParams::default();
        params.set_plain(params::pad_trigger_id(2), 1.0);
        let mut midi = [0_u16; 16];
        midi[15] = 1 << 7;
        let state = performance_state(&params, &midi);
        assert!(state.held[2]);
        assert!(state.held[7]);
    }

    #[test]
    fn midi_ui_mask_aggregates_all_channels() {
        let mut midi = [0_u16; 16];
        midi[0] = 1 << 2;
        midi[15] = 1 << 11;
        assert_eq!(aggregate_midi_held(&midi), (1 << 2) | (1 << 11));
    }

    #[test]
    fn f64_wrapper_processes_audio_through_live_parameters() {
        let params = BufferUppercutParams::default();
        params.set_plain(params::pad_trigger_id(15), 1.0);
        let mut state = BufferUppercutTruce::default();
        <BufferUppercutTruce as PluginLogic>::reset(
            &mut state,
            &params,
            &AudioConfig::new(48_000.0, 128),
        );

        let input_l: Vec<f64> = (0..128)
            .map(|sample| (sample as f64 * 0.071).sin() * 0.8)
            .collect();
        let input_r = input_l.clone();
        let mut output_l = vec![0.0; 128];
        let mut output_r = vec![0.0; 128];
        let input_refs = [&input_l[..], &input_r[..]];
        let mut output_refs = [&mut output_l[..], &mut output_r[..]];
        let mut buffer = AudioBuffer::from_slices_checked(&input_refs, &mut output_refs, 128);
        let events = EventList::with_capacity(0);
        let transport = TransportInfo::for_screenshot();
        let mut output_events = EventList::with_capacity(0);
        let mut context = ProcessContext::new(&transport, 48_000.0, 128, &mut output_events);

        assert_eq!(
            <BufferUppercutTruce as PluginLogic>::process(
                &mut state,
                &params,
                &mut buffer,
                &events,
                &mut context,
            ),
            ProcessStatus::Normal
        );
        assert!(output_l.iter().all(|sample| sample.is_finite()));
        assert_ne!(output_l, input_l);
    }

    #[test]
    fn midi_pitch_action_applies_once_on_the_held_transition() {
        let params = BufferUppercutParams::default();
        let mut state = BufferUppercutTruce::default();
        <BufferUppercutTruce as PluginLogic>::reset(
            &mut state,
            &params,
            &AudioConfig::new(48_000.0, 32),
        );
        let input_l = [0.0; 32];
        let input_r = [0.0; 32];
        let mut output_l = [0.0; 32];
        let mut output_r = [0.0; 32];
        let transport = TransportInfo::for_screenshot();
        let mut output_events = EventList::with_capacity(1);
        let mut events = EventList::with_capacity(1);
        events.push(Event::new(
            0,
            EventBody::NoteOn {
                group: 0,
                channel: 3,
                note: 69,
                velocity: 100,
            },
        ));

        for iteration in 0..2 {
            let input_refs = [&input_l[..], &input_r[..]];
            let mut output_refs = [&mut output_l[..], &mut output_r[..]];
            let mut buffer = AudioBuffer::from_slices_checked(&input_refs, &mut output_refs, 32);
            let mut context = ProcessContext::new(&transport, 48_000.0, 32, &mut output_events);
            <BufferUppercutTruce as PluginLogic>::process(
                &mut state,
                &params,
                &mut buffer,
                &events,
                &mut context,
            );
            assert_eq!(
                params.get_plain(params::PARAM_PERFORMANCE_PITCH_ID),
                Some(0.0),
                "iteration {iteration}"
            );
            assert_eq!(state.performance_pitch, -1.0, "iteration {iteration}");
        }
        assert_eq!(output_events.len(), 1);
        assert!(matches!(
            output_events.get(0).map(|event| &event.body),
            Some(EventBody::ParamChange { id, value })
                if *id == params::PARAM_PERFORMANCE_PITCH_ID && *value == -1.0
        ));
    }

    #[test]
    fn restored_parameter_resynchronizes_internal_performance_pitch() {
        let params = BufferUppercutParams::default();
        params.set_plain(params::PARAM_PERFORMANCE_PITCH_ID, -7.0);
        let mut state = BufferUppercutTruce {
            performance_pitch: 3.0,
            last_parameter_pitch: 3.0,
            ..BufferUppercutTruce::default()
        };
        <BufferUppercutTruce as PluginLogic>::state_changed(&mut state, &params);
        assert_eq!(state.performance_pitch, -7.0);
        assert_eq!(state.last_parameter_pitch, -7.0);
    }

    #[test]
    fn subnormal_output_samples_are_flushed() {
        let mut samples = [
            f64::from(f32::MIN_POSITIVE) / 2.0,
            -f64::from(f32::MIN_POSITIVE) / 4.0,
            1.0,
        ];
        zero_subnormals(&mut samples);
        assert_eq!(samples, [0.0, 0.0, 1.0]);
    }
}
