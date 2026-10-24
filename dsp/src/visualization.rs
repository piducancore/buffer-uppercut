//! Bounded per-stage telemetry. All display geometry belongs to the UI thread.
use crate::*;

pub(crate) const PEAK_STRIDE: usize = 32;

pub(crate) fn update_peaks(
    peaks: &mut [VisualizationBin],
    left: &[f32],
    right: &[f32],
    start: usize,
    frames: usize,
) {
    let completed = ((start % PEAK_STRIDE + frames) / PEAK_STRIDE).min(peaks.len());
    for offset in 0..completed {
        let chunk = (start / PEAK_STRIDE + offset) % peaks.len();
        let first = chunk * PEAK_STRIDE;
        let end = (first + PEAK_STRIDE).min(left.len());
        let mut min_l = f32::INFINITY;
        let mut max_l = f32::NEG_INFINITY;
        let mut min_r = f32::INFINITY;
        let mut max_r = f32::NEG_INFINITY;
        for i in first..end {
            let l = Engine::finite_f32(f64::from(left[i]));
            let r = Engine::finite_f32(f64::from(right[i]));
            min_l = min_l.min(l);
            max_l = max_l.max(l);
            min_r = min_r.min(r);
            max_r = max_r.max(r);
        }
        peaks[chunk] = VisualizationBin {
            min_l,
            max_l,
            min_r,
            max_r,
        };
    }
}

/// Exact bins: use cached complete chunks, raw samples at both boundaries.
/// Never use the partially overwritten chunk containing the ring write cursor.
fn fill_peaks(
    history: (&[f32], &[f32]),
    peaks: &[VisualizationBin],
    write: usize,
    start: usize,
    count: usize,
    bins: &mut [VisualizationBin],
) {
    let (left, right) = history;
    let length = left.len();
    let bin_count = bins.len();
    for (i, bin) in bins.iter_mut().enumerate() {
        let first = i * count / bin_count;
        let end = ((i + 1) * count / bin_count).max(first + 1).min(count);
        let mut offset = first;
        let mut result = VisualizationBin {
            min_l: f32::INFINITY,
            max_l: f32::NEG_INFINITY,
            min_r: f32::INFINITY,
            max_r: f32::NEG_INFINITY,
        };
        while offset < end {
            let position = (start + offset) & (length - 1);
            let chunk = position / PEAK_STRIDE;
            let (value, advance) = if position.is_multiple_of(PEAK_STRIDE)
                && end - offset >= PEAK_STRIDE
                && chunk != write / PEAK_STRIDE
            {
                (peaks[chunk], PEAK_STRIDE)
            } else {
                let l = Engine::finite_f32(f64::from(left[position]));
                let r = Engine::finite_f32(f64::from(right[position]));
                (
                    VisualizationBin {
                        min_l: l,
                        max_l: l,
                        min_r: r,
                        max_r: r,
                    },
                    1,
                )
            };
            result.min_l = result.min_l.min(value.min_l);
            result.max_l = result.max_l.max(value.max_l);
            result.min_r = result.min_r.min(value.min_r);
            result.max_r = result.max_r.max(value.max_r);
            offset += advance;
        }
        *bin = result;
    }
}

pub const STAGE_TRACE_SAMPLES: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StageVisualization {
    pub effect: EffectType,
    pub active: bool,
    pub suspended: bool,
    pub chain_position: u8,
    /// Buffer read position, gate cycle, grain phase, or modulation phase.
    pub phase: f32,
    /// Playback rate, gate gain, semitones, cutoff Hz, bit depth, or delay ms.
    pub value: f32,
    pub wet: f32,
    pub secondary_phase: f32,
    pub activity: f32,
    pub duration: f32,
    pub controls: [f32; NUM_MACROS],
    /// Evenly sampled mono capture trace; not a peak envelope.
    pub trace: [f32; STAGE_TRACE_SAMPLES],
}

impl Default for StageVisualization {
    fn default() -> Self {
        Self {
            effect: EffectType::Off,
            active: false,
            suspended: false,
            chain_position: 0,
            phase: 0.0,
            value: 0.0,
            wet: 0.0,
            secondary_phase: 0.0,
            activity: 0.0,
            duration: 0.0,
            controls: [0.0; NUM_MACROS],
            trace: [0.0; STAGE_TRACE_SAMPLES],
        }
    }
}

impl Engine {
    /// Main timeline always compares input and final output on the same time axis.
    pub fn fill_performance_visualization(
        &self,
        tempo: f64,
        input: &mut [VisualizationBin],
        output: &mut [VisualizationBin],
        stages: &mut [StageVisualization; NUM_PADS],
    ) -> VisualizationMeta {
        let chain_order = self.chain_order();
        let tempo = if tempo.is_finite() {
            tempo.max(1.0)
        } else {
            120.0
        };
        let length = self.rolling_l.len();
        let count = ((4.0 * self.sample_rate * 60.0 / tempo).round() as usize).clamp(1, length);
        let start = Self::wrap_length(self.rolling_write_pos as i64 - count as i64, length);
        fill_peaks(
            (&self.rolling_l, &self.rolling_r),
            &self.input_peaks,
            self.rolling_write_pos,
            start,
            count,
            input,
        );
        fill_peaks(
            (&self.output_history_l, &self.output_history_r),
            &self.output_peaks,
            self.rolling_write_pos,
            start,
            count,
            output,
        );
        for (index, view) in stages.iter_mut().enumerate() {
            let slot = &self.slots[index];
            *view = StageVisualization::default();
            if !slot.active && !slot.suspended {
                continue;
            }
            let config = PadConfig {
                macros: self.visual_configs[index].macros.map(clamp_macro),
                ..self.visual_configs[index]
            };
            view.effect = slot.configured_type;
            view.active = slot.active;
            view.suspended = slot.suspended;
            view.controls = config.macros.map(|v| clamp_macro(v) as f32);
            view.wet = match view.effect {
                EffectType::BeatRepeat | EffectType::Reverse | EffectType::LoFi => view.controls[2],
                EffectType::TapeStop => view.controls[3],
                EffectType::Gate => view.controls[2],
                _ => view.controls[6],
            };
            match view.effect {
                EffectType::BeatRepeat | EffectType::Reverse | EffectType::TapeStop => {
                    let buffer = &slot.buffer;
                    let count = (buffer.slice_samples.max(1) as usize).min(buffer.history_l.len());
                    let last = count.saturating_sub(1) as f64;
                    let phase = buffer.repeat_phase.clamp(0.0, last);
                    view.phase = if last > 0.0 {
                        (if buffer.repeat_reverse {
                            last - phase
                        } else {
                            phase
                        } / last) as f32
                    } else {
                        0.0
                    };
                    view.value =
                        (buffer.play_rate * if buffer.repeat_reverse { -1.0 } else { 1.0 }) as f32;
                    view.duration = count as f32 / self.sample_rate as f32;
                    for (i, sample) in view.trace.iter_mut().enumerate() {
                        let position = (buffer.capture_start
                            + i * count.saturating_sub(1) / (STAGE_TRACE_SAMPLES - 1))
                            % buffer.history_l.len();
                        *sample = Self::finite_f32(
                            (f64::from(buffer.history_l[position])
                                + f64::from(buffer.history_r[position]))
                                * 0.5,
                        );
                    }
                }
                EffectType::Gate => {
                    let period =
                        (grid_beats(config.macros[0]) * self.sample_rate * 60.0 / tempo).max(2.0);
                    view.phase = (slot.gate_phase / period).clamp(0.0, 1.0) as f32;
                    view.value = slot.gate_gain as f32;
                }
                EffectType::Pitch => {
                    (view.phase, view.value) = slot.pitch.visualization();
                }
                EffectType::Filter => {
                    let octaves = config.macros[4] * slot.filter_envelope * 5.0
                        + config.macros[5] * slot.filter_phase.sin() * 2.0;
                    view.value = (filter_frequency(config.macros[1]) * 2.0_f64.powf(octaves))
                        .clamp(0.001, (self.sample_rate * 0.45).max(0.001))
                        as f32;
                    view.phase = ((f64::from(view.value).max(20.0) / 20.0).ln() / 1000.0_f64.ln())
                        .clamp(0.0, 1.0) as f32;
                }
                EffectType::LoFi => {
                    view.value = denormalize_linear(config.macros[1], 2.0, 16.0).round() as f32;
                    view.phase = slot.lofi_held[0].clamp(-1.0, 1.0) as f32;
                }
                EffectType::Vinyl => {
                    (view.phase, view.value, view.secondary_phase, view.activity) =
                        slot.vinyl.visualization();
                }
                EffectType::Off => {}
            }
            view.phase = Self::finite_f32(f64::from(view.phase));
            view.value = Self::finite_f32(f64::from(view.value));
            view.secondary_phase = Self::finite_f32(f64::from(view.secondary_phase));
            view.activity = Self::finite_f32(f64::from(view.activity));
        }
        for (position, pad) in chain_order.into_iter().enumerate() {
            if stages[pad].active || stages[pad].suspended {
                stages[pad].chain_position = position as u8;
            }
        }
        VisualizationMeta {
            duration_seconds: count as f32 / self.sample_rate as f32,
            normalized_playhead: 1.0,
            ..VisualizationMeta::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(
        engine: &Engine,
    ) -> (
        [VisualizationBin; 16],
        [VisualizationBin; 16],
        [StageVisualization; NUM_PADS],
    ) {
        let mut input = [VisualizationBin::default(); 16];
        let mut output = input;
        let mut stages = [StageVisualization::default(); NUM_PADS];
        engine.fill_performance_visualization(120.0, &mut input, &mut output, &mut stages);
        (input, output, stages)
    }

    #[test]
    fn cached_peaks_equal_raw_scans_across_wrap_and_partial_chunks() {
        let mut engine = Engine::new(64.0);
        let state = PerformanceState::classic();
        for block in 0..41 {
            let input = std::array::from_fn::<_, 37, _>(|i| ((block * 37 + i) as f64 * 0.31).sin());
            engine.process(
                &input,
                &input,
                &mut [0.0; 37],
                &mut [0.0; 37],
                120.0,
                &state,
            );
            for count in [1, 17, 31, 32, 33, 127, 256, 511, 512] {
                let start = Engine::wrap_length(
                    engine.rolling_write_pos as i64 - count as i64,
                    engine.rolling_l.len(),
                );
                let mut raw = [VisualizationBin::default(); 19];
                let mut cached = raw;
                Engine::fill_visualization_bins(
                    &engine.rolling_l,
                    &engine.rolling_r,
                    start,
                    count,
                    &mut raw,
                );
                fill_peaks(
                    (&engine.rolling_l, &engine.rolling_r),
                    &engine.input_peaks,
                    engine.rolling_write_pos,
                    start,
                    count,
                    &mut cached,
                );
                assert_eq!(cached, raw, "block {block}, count {count}");
            }
        }
    }

    #[test]
    fn output_is_post_chain_and_selection_never_replaces_it() {
        let mut engine = Engine::new(1000.0);
        let mut state = PerformanceState::classic();
        state.pads[8] = default_pad_config(EffectType::Gate);
        state.pads[8].macros = [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0];
        state.held[8] = true;
        let input = [0.75; 2000];
        let mut left = [0.0; 2000];
        let mut right = left;
        engine.process(&input, &input, &mut left, &mut right, 120.0, &state);
        let (dry, wet, stages) = frame(&engine);
        assert!(dry.iter().all(|bin| bin.min_l == 0.75 && bin.max_l == 0.75));
        assert!(wet.iter().any(|bin| bin.min_l < 0.1));
        assert!(stages[8].active);
        engine.set_visualization_selected_slot(Some(0));
        assert_eq!(frame(&engine), (dry, wet, stages));
        engine.clear_transient();
        let (dry, wet, stages) = frame(&engine);
        assert!(
            dry.iter()
                .chain(&wet)
                .all(|bin| *bin == VisualizationBin::default())
        );
        assert!(stages.iter().all(|stage| !stage.active && !stage.suspended));
    }

    #[test]
    fn independent_buffers_and_suspended_processors_keep_their_own_positions() {
        let mut engine = Engine::new(1000.0);
        let mut state = PerformanceState::classic();
        state.pads[0] = default_pad_config(EffectType::BeatRepeat);
        state.pads[1] = default_pad_config(EffectType::Reverse);
        state.pads[1].macros[0] = 1.0;
        let input = [0.25; 64];
        let mut left = [0.0; 64];
        let mut right = left;
        for pad in 0..7 {
            if pad > 1 {
                state.pads[pad] = default_pad_config(EffectType::Filter);
            }
            state.held[pad] = true;
            engine.process(&input, &input, &mut left, &mut right, 120.0, &state);
        }
        let (_, _, before) = frame(&engine);
        assert!(before[0].suspended && !before[0].active);
        assert_eq!(before.iter().filter(|s| s.active).count(), 6);
        assert_ne!(before[0].duration, before[1].duration);
        assert!(before[1].value < 0.0);
        engine.process(&input, &input, &mut left, &mut right, 120.0, &state);
        let (_, _, after) = frame(&engine);
        assert_eq!(before[0].phase, after[0].phase);
        assert_ne!(before[1].phase, after[1].phase);
        state.held[6] = false;
        engine.process(&input, &input, &mut left, &mut right, 120.0, &state);
        let (_, _, restored) = frame(&engine);
        assert!(restored[0].active && !restored[0].suspended);
        assert!(!restored[6].active);
    }

    #[test]
    fn action_pads_are_not_stages_and_tape_wet_uses_its_own_macro() {
        let mut engine = Engine::new(1000.0);
        let mut state = PerformanceState::classic();
        state.held[9] = true; // Pitch Down action
        state.held[7] = true;
        state.pads[7].macros[2] = 1.0;
        state.pads[7].macros[3] = 0.0;
        engine.process(
            &[0.1; 16],
            &[0.1; 16],
            &mut [0.0; 16],
            &mut [0.0; 16],
            120.0,
            &state,
        );
        let (_, _, stages) = frame(&engine);
        assert!(!stages[9].active);
        assert_eq!(stages[7].wet, 0.0);
    }
}
