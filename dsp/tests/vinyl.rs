use buffer_uppercut_dsp::{EffectType, Engine, PerformanceState, default_pad_config};

fn render(engine: &mut Engine, state: &PerformanceState, input: &[f64]) -> Vec<f64> {
    let mut left = vec![0.0; input.len()];
    let mut right = left.clone();
    engine.process(input, input, &mut left, &mut right, 120.0, state);
    left
}

#[test]
fn vinyl_is_block_partition_invariant_and_release_restarts_it() {
    let mut state = PerformanceState::default();
    state.pads[0] = default_pad_config(EffectType::Vinyl);
    state.held[0] = true;
    let input: Vec<_> = (0..4000).map(|n| (n as f64 * 0.2).sin() * 0.5).collect();
    let mut whole = Engine::new(8000.0);
    let expected = render(&mut whole, &state, &input);
    let mut split = Engine::new(8000.0);
    let actual: Vec<_> = input
        .chunks(37)
        .flat_map(|chunk| render(&mut split, &state, chunk))
        .collect();
    assert_eq!(actual, expected);
    state.held[0] = false;
    assert_eq!(render(&mut split, &state, &input), input);
    state.held[0] = true;
    assert_eq!(render(&mut split, &state, &input), expected);
    split.clear_transient();
    assert_eq!(render(&mut split, &state, &input), expected);
    state.pads[0] = default_pad_config(EffectType::LoFi);
    render(&mut split, &state, &input);
    state.pads[0] = default_pad_config(EffectType::Vinyl);
    assert_eq!(render(&mut split, &state, &input), expected);
}

#[test]
fn vinyl_suspension_freezes_state_and_restores_at_its_serial_position() {
    let mut state = PerformanceState::default();
    for config in &mut state.pads[..7] {
        *config = default_pad_config(EffectType::Vinyl);
    }
    state.held[0] = true;
    let input = [0.25; 256];
    let mut reference = Engine::new(8000.0);
    let mut suspended = Engine::new(8000.0);
    assert_eq!(
        render(&mut reference, &state, &input),
        render(&mut suspended, &state, &input)
    );
    state.held[1..7].fill(true);
    render(&mut suspended, &state, &input);
    assert_eq!(suspended.admission_meta().suspended_mask, 1);
    render(&mut suspended, &state, &input);
    state.held[1..7].fill(false);
    assert_eq!(
        render(&mut suspended, &state, &input),
        render(&mut reference, &state, &input)
    );
    assert_eq!(suspended.admission_meta().active_mask, 1);
}

#[test]
fn duplicate_vinyl_stages_mix_against_their_own_input() {
    let mut state = PerformanceState::default();
    for pad in 0..2 {
        state.pads[pad] = default_pad_config(EffectType::Vinyl);
        state.pads[pad].macros = [0.0, 0.0, 0.0, 0.7, 0.0, 0.0, 0.5];
        state.held[pad] = true;
    }
    let input = [0.8; 256];
    let stacked = render(&mut Engine::new(8000.0), &state, &input);
    state.held[1] = false;
    let once = render(&mut Engine::new(8000.0), &state, &input);
    state.held[0] = false;
    state.held[1] = true;
    let twice = render(&mut Engine::new(8000.0), &state, &once);
    assert_eq!(stacked, twice);
    assert_ne!(stacked, once);
}
