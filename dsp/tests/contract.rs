use std::{fs, path::PathBuf};

use buffer_uppercut_dsp::{
    EffectType, Engine, NUM_PADS, PadConfig, PerformanceState, PitchRole, apply_pitch_action,
    pitch_config,
};

const ABSOLUTE_TOLERANCE: f64 = 1.0e-7;
const RELATIVE_TOLERANCE: f64 = 1.0e-7;
const SEED_CPP_COMMIT: &str = "bc17659aa517b9910761c1861cadd873402b75de";

#[derive(Default)]
struct Fixture {
    name: String,
    sample_rate: f64,
    channels: usize,
    pads: [PadConfig; NUM_PADS],
    blocks: Vec<Block>,
}

struct Block {
    tempo: f64,
    held: u16,
    pitch: f64,
    samples: Vec<Sample>,
}

struct Sample {
    input_l: f64,
    input_r: f64,
    expected_l: f64,
    expected_r: f64,
}

fn contract_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../contract")
}

fn current_contract_dir() -> PathBuf {
    contract_root().join("v4")
}

fn parse_fixture(text: &str) -> Fixture {
    let mut fixture = Fixture::default();
    let mut current_block: Option<Block> = None;
    for line in text.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        match fields[0] {
            "BUDSP_CONTRACT" | "end" => {}
            "name" => fixture.name = fields[1].to_owned(),
            "sample_rate" => fixture.sample_rate = fields[1].parse().unwrap(),
            "channels" => fixture.channels = fields[1].parse().unwrap(),
            "pad" => {
                let pad: usize = fields[1].parse().unwrap();
                let effect: i32 = fields[2].parse().unwrap();
                let mut macros = [0.0; 7];
                for (index, value) in macros.iter_mut().enumerate() {
                    *value = fields[index + 3].parse().unwrap();
                }
                fixture.pads[pad] = PadConfig {
                    effect_type: EffectType::from_index(effect),
                    macros,
                };
            }
            "block" => {
                if let Some(block) = current_block.take() {
                    fixture.blocks.push(block);
                }
                current_block = Some(Block {
                    tempo: fields[2].parse().unwrap(),
                    held: fields[3].parse().unwrap(),
                    pitch: fields[4].parse().unwrap(),
                    samples: Vec::with_capacity(fields[5].parse().unwrap()),
                });
            }
            "sample" => {
                let block = current_block.as_mut().unwrap();
                if fixture.channels == 1 {
                    let input: f64 = fields[1].parse().unwrap();
                    let expected: f64 = fields[2].parse().unwrap();
                    block.samples.push(Sample {
                        input_l: input,
                        input_r: input,
                        expected_l: expected,
                        expected_r: expected,
                    });
                } else {
                    block.samples.push(Sample {
                        input_l: fields[1].parse().unwrap(),
                        input_r: fields[2].parse().unwrap(),
                        expected_l: fields[3].parse().unwrap(),
                        expected_r: fields[4].parse().unwrap(),
                    });
                }
            }
            unknown => panic!("unknown fixture row {unknown}"),
        }
    }
    if let Some(block) = current_block {
        fixture.blocks.push(block);
    }
    fixture
}

fn assert_close(name: &str, block: usize, frame: usize, channel: char, actual: f64, expected: f64) {
    assert!(
        actual.is_finite(),
        "{name} block {block} frame {frame} {channel} is not finite"
    );
    let error = (actual - expected).abs();
    let tolerance = ABSOLUTE_TOLERANCE + RELATIVE_TOLERANCE * expected.abs();
    assert!(
        error <= tolerance,
        "{name} block {block} frame {frame} {channel}: actual {actual:.17}, expected \
         {expected:.17}, error {error:.3e}, tolerance {tolerance:.3e}"
    );
}

#[test]
fn matches_every_canonical_serial_processing_fixture() {
    let mut paths: Vec<_> = fs::read_dir(current_contract_dir())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "budsp")
                && path.file_name().unwrap() != "pitch-actions.budsp"
        })
        .collect();
    paths.sort();
    assert_eq!(paths.len(), 23);

    for path in paths {
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("BUDSP_CONTRACT\t4\n"));
        let fixture = parse_fixture(&text);
        let mut engine = Engine::default();
        engine.reset(fixture.sample_rate);
        for (block_index, block) in fixture.blocks.iter().enumerate() {
            let mut state = PerformanceState {
                pads: fixture.pads,
                active_pitch_shift: block.pitch,
                ..PerformanceState::default()
            };
            for pad in 0..NUM_PADS {
                state.held[pad] = block.held & (1 << pad) != 0;
            }
            let input_l: Vec<f64> = block.samples.iter().map(|sample| sample.input_l).collect();
            let input_r: Vec<f64> = block.samples.iter().map(|sample| sample.input_r).collect();
            let mut output_l = vec![0.0; block.samples.len()];
            let mut output_r = vec![0.0; block.samples.len()];
            engine.process(
                &input_l,
                &input_r,
                &mut output_l,
                &mut output_r,
                block.tempo,
                &state,
            );
            for (frame, expected) in block.samples.iter().enumerate() {
                assert_close(
                    &fixture.name,
                    block_index,
                    frame,
                    'L',
                    output_l[frame],
                    expected.expected_l,
                );
                if fixture.channels == 2 {
                    assert_close(
                        &fixture.name,
                        block_index,
                        frame,
                        'R',
                        output_r[frame],
                        expected.expected_r,
                    );
                }
            }
        }
    }
}

#[test]
fn matches_canonical_serial_pitch_action_fixture() {
    let text = fs::read_to_string(current_contract_dir().join("pitch-actions.budsp")).unwrap();
    assert!(text.starts_with("BUDSP_PITCH_ACTIONS\t4\n"));
    let mut cases = 0;
    for line in text.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields[0] != "case" {
            continue;
        }
        let role = PitchRole::from_normalized(fields[1].parse().unwrap());
        let step_value = fields[2].parse().unwrap();
        let current = fields[3].parse().unwrap();
        let expected = fields[4].parse().unwrap();
        let mut config = pitch_config(role, 1.0);
        config.macros[1] = step_value;
        assert_eq!(apply_pitch_action(current, &config), expected);
        cases += 1;
    }
    assert_eq!(cases, 75);
}

#[test]
fn contract_manifest_records_the_seed_corpus_provenance() {
    let manifest = fs::read_to_string(contract_root().join("manifest.json")).unwrap();
    assert!(manifest.contains(SEED_CPP_COMMIT));
    assert!(manifest.contains("\"contract\": \"dsp-contract-v1\""));
    assert_eq!(manifest.matches("\"sha256\"").count(), 20);

    let checksums = fs::read_to_string(contract_root().join("SHA256SUMS")).unwrap();
    assert_eq!(checksums.lines().count(), 21);
    assert!(checksums.contains(
        "0a126f1ce3159cf2753c2b6dae08f2a99396d5f9f066eda61409bc74b59e0c54  manifest.json"
    ));
}

#[test]
fn serial_contract_manifest_records_the_canonical_corpus() {
    let serial_contract_dir = contract_root().join("v2");
    let manifest = fs::read_to_string(serial_contract_dir.join("manifest.json")).unwrap();
    assert!(manifest.contains("\"contract\": \"dsp-contract-v2-serial\""));
    assert!(manifest.contains("docs/adr/0004-serial-performance-slots.md"));
    assert_eq!(manifest.matches("\"sha256\"").count(), 24);

    let checksums = fs::read_to_string(serial_contract_dir.join("SHA256SUMS")).unwrap();
    assert_eq!(checksums.lines().count(), 25);
}

#[test]
fn unified_filter_contract_manifest_records_the_canonical_corpus() {
    let directory = contract_root().join("v3");
    let manifest = fs::read_to_string(directory.join("manifest.json")).unwrap();
    assert!(manifest.contains("\"contract\": \"dsp-contract-v3-unified-filter\""));
    assert!(manifest.contains("docs/adr/0008-unified-filter-effect.md"));
    assert_eq!(manifest.matches("\"sha256\"").count(), 24);

    let checksums = fs::read_to_string(directory.join("SHA256SUMS")).unwrap();
    assert_eq!(checksums.lines().count(), 25);
}

#[test]
fn grain_pitch_contract_manifest_records_the_canonical_corpus() {
    let manifest = fs::read_to_string(current_contract_dir().join("manifest.json")).unwrap();
    assert!(manifest.contains("\"contract\": \"dsp-contract-v4-grain-pitch\""));
    assert!(manifest.contains("docs/adr/0009-held-grain-pitch.md"));
    assert_eq!(manifest.matches("\"sha256\"").count(), 24);

    let checksums = fs::read_to_string(current_contract_dir().join("SHA256SUMS")).unwrap();
    assert_eq!(checksums.lines().count(), 25);
}
