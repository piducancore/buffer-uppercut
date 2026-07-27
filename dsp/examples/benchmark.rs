use std::hint::black_box;
use std::time::{Duration, Instant};

use buffer_uppercut_dsp::{Engine, PerformanceState};

const SAMPLE_RATE: f64 = 48_000.0;
const BLOCK_SIZE: usize = 512;
const BLOCKS_PER_RUN: usize = 2_000;
const RUNS: usize = 7;

fn main() {
    let mut state = PerformanceState::classic();
    for pad in [0, 8, 12, 13, 14, 15] {
        state.held[pad] = true;
    }
    let input_l: Vec<_> = (0..BLOCK_SIZE)
        .map(|sample| (sample as f64 * 0.071).sin() * 0.8)
        .collect();
    let input_r: Vec<_> = (0..BLOCK_SIZE)
        .map(|sample| (sample as f64 * 0.053).cos() * 0.7)
        .collect();
    let mut output_l = vec![0.0; BLOCK_SIZE];
    let mut output_r = vec![0.0; BLOCK_SIZE];
    let mut engine = Engine::new(SAMPLE_RATE);

    for _ in 0..100 {
        engine.process(
            &input_l,
            &input_r,
            &mut output_l,
            &mut output_r,
            120.0,
            &state,
        );
    }

    let mut timings = [Duration::ZERO; RUNS];
    for timing in &mut timings {
        let start = Instant::now();
        for _ in 0..BLOCKS_PER_RUN {
            engine.process(
                black_box(&input_l),
                black_box(&input_r),
                black_box(&mut output_l),
                black_box(&mut output_r),
                black_box(120.0),
                black_box(&state),
            );
        }
        *timing = start.elapsed();
    }
    timings.sort_unstable();
    let median = timings[RUNS / 2];
    let rendered_seconds = BLOCKS_PER_RUN as f64 * BLOCK_SIZE as f64 / SAMPLE_RATE;
    let realtime_multiple = rendered_seconds / median.as_secs_f64();
    println!("engine=buffer-uppercut-dsp");
    println!("sample_rate={SAMPLE_RATE:.0}");
    println!("block_size={BLOCK_SIZE}");
    println!("blocks_per_run={BLOCKS_PER_RUN}");
    println!("runs={RUNS}");
    println!("median_seconds={:.9}", median.as_secs_f64());
    println!("realtime_multiple={realtime_multiple:.3}");
    println!("history_bytes={}", engine.history_bytes());
    black_box(output_l[0] + output_r[0]);
}
