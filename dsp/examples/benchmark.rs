use std::{hint::black_box, time::Instant};

use buffer_uppercut_dsp::{EffectType, Engine, PerformanceState, default_pad_config};

fn main() {
    const SAMPLE_RATE: f64 = 48_000.0;
    const FRAMES: usize = 512;
    const BLOCKS: usize = 20_000;

    let input_l: Vec<f64> = (0..FRAMES)
        .map(|sample| (sample as f64 * 0.017).sin() * 0.7)
        .collect();
    let input_r: Vec<f64> = (0..FRAMES)
        .map(|sample| (sample as f64 * 0.023).cos() * 0.6)
        .collect();
    let mut output_l = vec![0.0; FRAMES];
    let mut output_r = vec![0.0; FRAMES];
    let mut state = PerformanceState::default();
    state.pads[0] = default_pad_config(EffectType::BeatRepeat);
    state.pads[8] = default_pad_config(EffectType::Gate);
    state.pads[15] = default_pad_config(EffectType::LoFi);
    state.held[0] = true;
    state.held[8] = true;
    state.held[15] = true;

    let mut engine = Engine::default();
    engine.reset(SAMPLE_RATE);
    for _ in 0..256 {
        engine.process(
            &input_l,
            &input_r,
            &mut output_l,
            &mut output_r,
            120.0,
            &state,
        );
    }

    let start = Instant::now();
    for _ in 0..BLOCKS {
        engine.process(
            black_box(&input_l),
            black_box(&input_r),
            black_box(&mut output_l),
            black_box(&mut output_r),
            black_box(120.0),
            black_box(&state),
        );
    }
    let elapsed = start.elapsed();
    let samples = (BLOCKS * FRAMES) as f64;
    println!(
        "buffer-uppercut-dsp: {BLOCKS} x {FRAMES} stereo frames in {:.3}s ({:.2} ns/frame)",
        elapsed.as_secs_f64(),
        elapsed.as_nanos() as f64 / samples
    );
    black_box((output_l[0], output_r[0]));
}
