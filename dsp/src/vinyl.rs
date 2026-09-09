//! Original record-wear effect. Storage is prepared at activation; resets only
//! invalidate the short delay, so release and type changes do bounded work.

use crate::{NUM_MACROS, clamp_macro};
use std::f64::consts::TAU;

pub(crate) struct Vinyl {
    delay: Vec<[f64; 2]>,
    write: usize,
    valid: usize,
    phase: [f64; 3],
    tone: [f64; 2],
    surface: [f64; 2],
    dust: [f64; 2],
    rng: u32,
    seed: u32,
    current: [f64; NUM_MACROS],
    target: [f64; NUM_MACROS],
    initialized: bool,
    smoothing: f64,
    phase_step: [f64; 3],
    sample_rate: f64,
    dust_decay: f64,
    surface_coefficient: f64,
}

impl Vinyl {
    pub(crate) fn new(slot: usize) -> Self {
        let seed = 0xa341_316c_u32.wrapping_add((slot as u32).wrapping_mul(0x9e37_79b9));
        Self {
            delay: Vec::new(),
            write: 0,
            valid: 0,
            phase: [0.0; 3],
            tone: [0.0; 2],
            surface: [0.0; 2],
            dust: [0.0; 2],
            rng: seed,
            seed,
            current: [0.0; NUM_MACROS],
            target: [0.0; NUM_MACROS],
            initialized: false,
            smoothing: 1.0,
            phase_step: [0.0; 3],
            sample_rate: 44_100.0,
            dust_decay: 0.0,
            surface_coefficient: 1.0,
        }
    }

    pub(crate) fn prepare(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate;
        self.delay
            .resize((sample_rate * 0.014).ceil() as usize + 4, [0.0; 2]);
        self.smoothing = 1.0 - (-1.0 / (sample_rate * 0.020)).exp();
        self.phase_step = [0.55, 0.83, 8.7].map(|hz| TAU * hz / sample_rate);
        self.dust_decay = (-1.0 / (sample_rate * 0.0007)).exp();
        self.surface_coefficient = 1.0 - (-TAU * 700.0 / sample_rate).exp();
        self.reset();
    }

    pub(crate) fn reset(&mut self) {
        self.write = 0;
        self.valid = 0;
        self.phase = [0.0; 3];
        self.tone = [0.0; 2];
        self.surface = [0.0; 2];
        self.dust = [0.0; 2];
        self.rng = self.seed;
        self.initialized = false;
    }

    pub(crate) fn configure(&mut self, macros: [f64; NUM_MACROS]) {
        self.target = macros.map(clamp_macro);
        if !self.initialized {
            self.current = self.target;
            self.initialized = true;
        }
    }

    fn random(&mut self) -> f64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        f64::from(self.rng) / 4_294_967_296.0
    }

    pub(crate) fn process(&mut self, dry: [f64; 2]) -> [f64; 2] {
        for (current, target) in self.current.iter_mut().zip(self.target) {
            *current += self.smoothing * (target - *current);
            if (target - *current).abs() < 1e-9 {
                *current = target;
            }
        }
        let [wow, flutter, wear, drive, dust, noise, wet] = self.current;
        // A shared read position preserves stereo timing. Unequal slow rates
        // avoid a single obvious periodic vibrato; the delay is always causal.
        let slow = 0.72 * self.phase[0].sin() + 0.28 * self.phase[1].sin();
        let fast = self.phase[2].sin();
        let delay = self.sample_rate
            * (0.006 * wow * wow * (1.0 + slow) + 0.00035 * flutter * flutter * (1.0 + fast));
        for (phase, step) in self.phase.iter_mut().zip(self.phase_step) {
            *phase = (*phase + step).rem_euclid(TAU);
        }
        self.delay[self.write] = dry.map(|sample| sample.clamp(-16.0, 16.0));
        self.valid = (self.valid + 1).min(self.delay.len());
        let delay = delay.min((self.valid - 1) as f64);
        let whole = delay.floor() as usize;
        let fraction = delay - whole as f64;
        let first = (self.write + self.delay.len() - whole) % self.delay.len();
        let second = (first + self.delay.len() - 1) % self.delay.len();
        let delayed: [f64; 2] = std::array::from_fn(|channel| {
            self.delay[first][channel] * (1.0 - fraction) + self.delay[second][channel] * fraction
        });
        self.write = (self.write + 1) % self.delay.len();

        let cutoff = (20_000.0 * (0.09_f64).powf(wear)).min(self.sample_rate * 0.45);
        let tone_coefficient = 1.0 - (-TAU * cutoff / self.sample_rate).exp();
        let gain = 10.0_f64.powf(drive * 18.0 / 20.0);
        let compensation = gain.sqrt();
        let dust_probability = (30.0 * dust * dust / self.sample_rate).min(1.0);
        let mut output = dry;
        for channel in 0..2 {
            let sample = delayed[channel];
            let saturated = (sample * gain).tanh() / compensation;
            let driven = sample * (1.0 - drive) + saturated * drive;
            self.tone[channel] += tone_coefficient * (driven - self.tone[channel]);
            let worn = driven * (1.0 - wear) + self.tone[channel] * wear;
            let random = self.random() * 2.0 - 1.0;
            self.surface[channel] += self.surface_coefficient * (random - self.surface[channel]);
            let surface = 0.025 * noise * noise * (0.25 * random + 0.75 * self.surface[channel]);
            let impulse = if self.random() < dust_probability {
                (self.random() * 2.0 - 1.0) * 0.25 * dust
            } else {
                0.0
            };
            // Bipolar, zero-DC pulse: sharp onset and a short opposing tail.
            let click = impulse - (1.0 - self.dust_decay) * self.dust[channel];
            self.dust[channel] = self.dust_decay * self.dust[channel] + impulse;
            let textured = worn
                + if self.target[4] == 0.0 { 0.0 } else { click }
                + if self.target[5] == 0.0 { 0.0 } else { surface };
            output[channel] = dry[channel] * (1.0 - wet) + textured * wet;
        }
        // Exact clean endpoints, including input outside the bounded wet path.
        if wet == 0.0 || self.current[..6].iter().all(|value| *value == 0.0) {
            dry
        } else {
            output
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prepared(slot: usize, rate: f64, controls: [f64; 7]) -> Vinyl {
        let mut vinyl = Vinyl::new(slot);
        vinyl.prepare(rate);
        vinyl.configure(controls);
        vinyl
    }

    #[test]
    fn clean_and_dry_endpoints_are_exact_and_texture_zero_is_silent() {
        for controls in [
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
            [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0],
        ] {
            let mut vinyl = prepared(0, 48_000.0, controls);
            for n in 0..2000 {
                let input = [(n as f64 * 0.07).sin() * 20.0, -0.3];
                assert_eq!(vinyl.process(input), input);
            }
        }
        let mut vinyl = prepared(0, 48_000.0, [1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 1.0]);
        for _ in 0..48_000 {
            assert_eq!(vinyl.process([0.0; 2]), [0.0; 2]);
        }
    }

    #[test]
    fn wow_and_flutter_change_pitch_without_splitting_stereo_timing() {
        for control in [0, 1] {
            let mut controls = [0.0; 7];
            controls[control] = 1.0;
            controls[6] = 1.0;
            let mut vinyl = prepared(0, 8_000.0, controls);
            let mut previous = 0.0;
            let mut crossings = Vec::new();
            for n in 0..24_000 {
                let x = (TAU * 400.0 * n as f64 / 8_000.0).sin() * 0.5;
                let y = vinyl.process([x, -x]);
                assert!((y[0] + y[1]).abs() < 1e-12);
                if n > 800 && previous < 0.0 && y[0] >= 0.0 {
                    crossings.push(n as f64 - y[0] / (y[0] - previous));
                }
                previous = y[0];
            }
            let periods: Vec<_> = crossings.windows(2).map(|p| p[1] - p[0]).collect();
            let min = periods.iter().copied().fold(f64::INFINITY, f64::min);
            let max = periods.iter().copied().fold(0.0, f64::max);
            assert!(max - min > 0.3, "control {control}: {min}..{max}");
        }
    }

    #[test]
    fn wear_attenuates_high_frequencies_and_drive_adds_harmonics() {
        let energy = |hz: f64, controls| {
            let mut vinyl = prepared(0, 48_000.0, controls);
            let mut energy = 0.0;
            for n in 0..4800 {
                let x = (TAU * hz * n as f64 / 48_000.0).sin() * 0.5;
                let y = vinyl.process([x; 2])[0];
                if n >= 480 {
                    energy += y * y;
                }
            }
            energy
        };
        let worn = [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        assert!(energy(8000.0, worn) < energy(200.0, worn) * 0.1);
        let mut vinyl = prepared(0, 48_000.0, [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0]);
        let mut third_harmonic = 0.0;
        for n in 0..4800 {
            let phase = TAU * 1000.0 * n as f64 / 48_000.0;
            third_harmonic += vinyl.process([phase.sin() * 0.8; 2])[0] * (phase * 3.0).sin();
        }
        assert!(third_harmonic.abs() / 4800.0 > 0.02);
    }

    #[test]
    fn textures_are_independent_deterministic_and_reset_without_stale_audio() {
        for control in [4, 5] {
            let mut controls = [0.0; 7];
            controls[control] = 1.0;
            controls[6] = 1.0;
            let mut first = prepared(0, 8000.0, controls);
            let mut same = prepared(0, 8000.0, controls);
            let mut other = prepared(1, 8000.0, controls);
            let mut differs = false;
            let mut audible = false;
            for _ in 0..16_000 {
                let a = first.process([0.0; 2]);
                assert_eq!(a, same.process([0.0; 2]));
                differs |= a != other.process([0.0; 2]);
                audible |= a != [0.0; 2];
            }
            assert!(differs && audible);
        }
        let controls = [1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 1.0];
        let mut used = prepared(0, 8000.0, controls);
        for _ in 0..8000 {
            used.process([1.0; 2]);
        }
        let capacity = used.delay.capacity();
        used.reset();
        used.configure(controls);
        for _ in 0..8000 {
            assert_eq!(used.process([0.0; 2]), [0.0; 2]);
        }
        assert_eq!(used.delay.capacity(), capacity);
    }

    #[test]
    fn sample_rates_extremes_and_automation_remain_bounded() {
        for rate in [1.0, 8000.0, 44_100.0, 48_000.0, 96_000.0, 192_000.0] {
            let mut vinyl = prepared(0, rate, [1.0; 7]);
            for n in 0..20_000 {
                if n % 128 == 0 {
                    vinyl.configure(if n % 256 == 0 {
                        [1.0; 7]
                    } else {
                        [f64::NAN; 7]
                    });
                }
                let y = vinyl.process([f64::MAX, -f64::MAX]);
                assert!(y.iter().all(|sample| sample.is_finite()));
            }
            vinyl.configure([0.0; 7]);
            for _ in 0..(rate as usize) {
                vinyl.process([0.25; 2]);
            }
            assert_eq!(vinyl.process([0.25; 2]), [0.25; 2]);
        }
    }
}
