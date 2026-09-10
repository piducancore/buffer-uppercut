use crate::{PadConfig, clamp_macro, denormalize_linear};

const MAX_GRAIN_SECONDS: f64 = 0.14;

pub(crate) struct GrainPitch {
    delay_l: Vec<f64>,
    delay_r: Vec<f64>,
    write_pos: usize,
    valid_samples: usize,
    phase: f64,
    smoothed_semitones: f64,
    feedback: [f64; 2],
    texture_phase: f64,
    seed_phase: f64,
}

impl GrainPitch {
    pub(crate) fn new(slot: usize) -> Self {
        Self {
            delay_l: Vec::new(),
            delay_r: Vec::new(),
            write_pos: 0,
            valid_samples: 0,
            phase: 0.0,
            smoothed_semitones: 0.0,
            feedback: [0.0; 2],
            texture_phase: 0.0,
            seed_phase: (slot as f64 * 0.173).fract(),
        }
    }

    pub(crate) fn prepare(&mut self, sample_rate: f64) {
        let length = (sample_rate.max(1.0) * MAX_GRAIN_SECONDS).ceil() as usize + 4;
        self.delay_l.clear();
        self.delay_l.resize(length.max(8), 0.0);
        self.delay_r.clear();
        self.delay_r.resize(length.max(8), 0.0);
        self.reset();
    }

    pub(crate) fn reset(&mut self) {
        self.write_pos = 0;
        self.valid_samples = 0;
        self.phase = 0.0;
        self.smoothed_semitones = 0.0;
        self.feedback = [0.0; 2];
        self.texture_phase = self.seed_phase;
    }

    pub(crate) fn process(
        &mut self,
        input: [f64; 2],
        semitones: f64,
        config: &PadConfig,
        sample_rate: f64,
    ) -> [f64; 2] {
        let grain_seconds = denormalize_linear(config.macros[2], 0.012, 0.120);
        let grain_samples =
            (grain_seconds * sample_rate).clamp(4.0, self.delay_l.len().saturating_sub(4) as f64);
        let texture = clamp_macro(config.macros[3]);
        let smooth_ms = denormalize_linear(config.macros[4], 2.0, 120.0);
        let smoothing = 1.0 - (-1.0 / (sample_rate * smooth_ms * 0.001)).exp();
        let target = if semitones.is_finite() {
            semitones.clamp(-24.0, 24.0)
        } else {
            0.0
        };
        self.smoothed_semitones += smoothing * (target - self.smoothed_semitones);
        let ratio = 2.0_f64.powf(self.smoothed_semitones / 12.0);
        self.phase = (self.phase + (1.0 - ratio) / grain_samples).rem_euclid(1.0);
        self.texture_phase = (self.texture_phase + 0.37 / sample_rate).fract();

        let feedback_gain = 0.72 * clamp_macro(config.macros[5]);
        self.delay_l[self.write_pos] = (input[0] + self.feedback[0] * feedback_gain).tanh();
        self.delay_r[self.write_pos] = (input[1] + self.feedback[1] * feedback_gain).tanh();
        self.valid_samples = (self.valid_samples + 1).min(self.delay_l.len());

        let mut shifted = [0.0; 2];
        for tap in 0..2 {
            let phase = (self.phase + tap as f64 * 0.5).fract();
            let weight = 1.0 - (2.0 * phase - 1.0).abs();
            let scatter = (std::f64::consts::TAU
                * (self.texture_phase + tap as f64 * 0.41 + self.seed_phase))
                .sin()
                * texture
                * grain_samples
                * 0.12;
            let maximum_delay = self
                .delay_l
                .len()
                .saturating_sub(3)
                .min(self.valid_samples.saturating_sub(1)) as f64;
            let delay = (2.0 + phase * grain_samples + scatter).clamp(1.0, maximum_delay.max(1.0));
            shifted[0] += weight * read_linear(&self.delay_l, self.write_pos, delay);
            shifted[1] += weight * read_linear(&self.delay_r, self.write_pos, delay);
        }
        self.write_pos = (self.write_pos + 1) % self.delay_l.len();

        if self.valid_samples < 3
            || (self.smoothed_semitones.abs() < 1.0e-5 && feedback_gain == 0.0)
        {
            shifted = input;
        }
        self.feedback = shifted.map(|sample| sample.clamp(-4.0, 4.0));
        let wet = clamp_macro(config.macros[6]);
        [
            finite(input[0] * (1.0 - wet) + shifted[0] * wet),
            finite(input[1] * (1.0 - wet) + shifted[1] * wet),
        ]
    }
}

fn read_linear(buffer: &[f64], write_pos: usize, delay: f64) -> f64 {
    let position = (write_pos as f64 - delay).rem_euclid(buffer.len() as f64);
    let first = position.floor() as usize;
    let second = (first + 1) % buffer.len();
    let fraction = position - first as f64;
    buffer[first] + (buffer[second] - buffer[first]) * fraction
}

fn finite(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PitchRole, pitch_config};

    #[test]
    fn zero_shift_is_exact_and_shifted_output_is_finite_and_changed() {
        let config = pitch_config(PitchRole::Trigger, 1.0);
        let mut dry_pitch = GrainPitch::new(0);
        dry_pitch.prepare(8_000.0);
        let mut shifted_pitch = GrainPitch::new(0);
        shifted_pitch.prepare(8_000.0);
        let mut changed = false;
        for sample in 0..4_096 {
            let input = [(sample as f64 * 0.071).sin(), (sample as f64 * 0.053).cos()];
            assert_eq!(dry_pitch.process(input, 0.0, &config, 8_000.0), input);
            let output = shifted_pitch.process(input, 7.0, &config, 8_000.0);
            assert!(output.iter().all(|value| value.is_finite()));
            changed |= output != input;
        }
        assert!(changed);
    }

    #[test]
    fn octave_actions_move_a_sine_in_the_expected_direction() {
        const SAMPLE_RATE: f64 = 8_000.0;
        const INPUT_HZ: f64 = 220.0;
        let mut config = pitch_config(PitchRole::Trigger, 12.0);
        config.macros[3] = 0.0;
        config.macros[4] = 0.0;

        for (semitones, expected_hz, rejected_hz) in [(12.0, 440.0, 110.0), (-12.0, 110.0, 440.0)] {
            let mut pitch = GrainPitch::new(0);
            pitch.prepare(SAMPLE_RATE);
            let mut output = Vec::with_capacity(8_192);
            for sample in 0..12_192 {
                let input = (std::f64::consts::TAU * INPUT_HZ * sample as f64 / SAMPLE_RATE).sin();
                let shifted = pitch.process([input, input], semitones, &config, SAMPLE_RATE)[0];
                if sample >= 4_000 {
                    output.push(shifted);
                }
            }
            assert!(
                magnitude(&output, expected_hz, SAMPLE_RATE)
                    > magnitude(&output, rejected_hz, SAMPLE_RATE) * 3.0,
                "{semitones:+} st moved energy in the wrong direction"
            );
        }
    }

    #[test]
    fn sample_rate_and_macro_extremes_remain_finite() {
        for sample_rate in [1.0, 8_000.0, 48_000.0, 192_000.0] {
            for macros in [[0.0; 7], [1.0; 7], [f64::NAN; 7]] {
                let mut pitch = GrainPitch::new(15);
                pitch.prepare(sample_rate);
                let config = PadConfig {
                    effect_type: crate::EffectType::Pitch,
                    macros,
                };
                for sample in 0..2_048 {
                    let input = [(sample as f64 * 0.017).sin(), (sample as f64 * 0.023).cos()];
                    let output = pitch.process(input, 24.0, &config, sample_rate);
                    assert!(output.iter().all(|value| value.is_finite()));
                }
            }
        }
    }

    fn magnitude(samples: &[f64], frequency: f64, sample_rate: f64) -> f64 {
        let mut real = 0.0;
        let mut imaginary = 0.0;
        for (index, sample) in samples.iter().enumerate() {
            let phase = std::f64::consts::TAU * frequency * index as f64 / sample_rate;
            real += sample * phase.cos();
            imaginary -= sample * phase.sin();
        }
        real.hypot(imaginary)
    }
}
