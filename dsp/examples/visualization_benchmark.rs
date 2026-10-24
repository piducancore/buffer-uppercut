//! DSP and 30 Hz visualization extraction timing; excludes the host and UI.
use buffer_uppercut_dsp::{Engine, PerformanceState, StageVisualization, VisualizationBin};
use std::hint::black_box;
use std::time::Instant;
fn main() {
    for rate in [48000.0, 192000.0] {
        for cap in 1..=6 {
            let mut engine = Engine::new(rate);
            let mut state = PerformanceState::classic();
            for (i, pad) in [0, 8, 12, 13, 14, 15].iter().enumerate() {
                state.held[*pad] = i < cap;
            }
            let input = std::array::from_fn::<_, 256, _>(|i| (i as f64 * 0.071).sin() * 0.8);
            let mut l = [0.0; 256];
            let mut r = l;
            let mut bins = [VisualizationBin::default(); 256];
            let mut input_bins = bins;
            let mut stages = [StageVisualization::default(); 16];
            let mut times = [0.0_f64; 5];
            for time in &mut times {
                let mut countdown = 0;
                let start = Instant::now();
                for _ in 0..400 {
                    engine.process(black_box(&input), &input, &mut l, &mut r, 120.0, &state);
                    if countdown <= 256 {
                        black_box(engine.fill_performance_visualization(
                            120.0,
                            &mut input_bins,
                            &mut bins,
                            &mut stages,
                        ));
                        countdown = (rate / 30.0) as usize;
                    } else {
                        countdown -= 256;
                    }
                }
                *time = start.elapsed().as_secs_f64();
            }
            times.sort_by(f64::total_cmp);
            println!(
                "{rate:.0},{cap},{:.6},{:.3},{}",
                times[2],
                100.0 * times[2] / (400.0 * 256.0 / rate),
                engine.history_bytes()
            );
        }
    }
}
