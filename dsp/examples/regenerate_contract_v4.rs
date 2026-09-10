use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use buffer_uppercut_dsp::{
    EffectType, Engine, NUM_MACROS, NUM_PADS, PadConfig, PerformanceState, PitchRole, pitch_config,
};

#[derive(Default)]
struct Fixture {
    name: String,
    sample_rate: f64,
    channels: usize,
    pads: [PadConfig; NUM_PADS],
    blocks: Vec<Block>,
}

struct Block {
    index: usize,
    tempo: f64,
    held: u16,
    pitch: f64,
    input: Vec<[f64; 2]>,
}

fn main() {
    let contract = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../contract");
    let source = contract.join("v3");
    let destination = contract.join("v4");
    fs::create_dir_all(&destination).unwrap();

    let mut source_paths: Vec<_> = fs::read_dir(&source)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "budsp")
        })
        .collect();
    source_paths.sort();

    let mut generated = Vec::new();
    for source_path in source_paths {
        let source_name = source_path.file_name().unwrap().to_str().unwrap();
        let destination_name = renamed_fixture(source_name);
        let destination_path = destination.join(destination_name);
        if source_name == "pitch-actions.budsp" {
            let text = render_pitch_actions(&fs::read_to_string(&source_path).unwrap());
            fs::write(&destination_path, text).unwrap();
        } else {
            let fixture = parse_fixture(&fs::read_to_string(&source_path).unwrap());
            fs::write(&destination_path, render_fixture(&fixture)).unwrap();
        }
        generated.push(destination_name.to_owned());
    }

    let manifest_path = destination.join("manifest.json");
    fs::write(&manifest_path, render_manifest(&destination, &generated)).unwrap();
    let mut checksum_paths = generated;
    checksum_paths.push("manifest.json".to_owned());
    let mut checksums = String::new();
    for name in checksum_paths {
        writeln!(checksums, "{}  {name}", sha256(&destination.join(&name))).unwrap();
    }
    fs::write(destination.join("SHA256SUMS"), checksums).unwrap();
}

fn renamed_fixture(name: &str) -> &str {
    match name {
        "07-default-effect-5-pitch-down.budsp" => "07-pitch-down-action.budsp",
        "08-default-effect-6-pitch-reset.budsp" => "08-pitch-trigger.budsp",
        "09-default-effect-7-pitch-up.budsp" => "09-pitch-up-action.budsp",
        "10-default-effect-8-filter-low-pass.budsp" => "10-default-effect-6-filter-low-pass.budsp",
        "13-default-effect-9-lofi.budsp" => "13-default-effect-7-lofi.budsp",
        _ => name,
    }
}

fn parse_fixture(text: &str) -> Fixture {
    let mut fixture = Fixture::default();
    let mut current_block: Option<Block> = None;
    for line in text.lines() {
        let fields: Vec<_> = line.split('\t').collect();
        match fields[0] {
            "BUDSP_CONTRACT" | "end" => {}
            "name" => fixture.name = renamed_scenario(fields[1]).to_owned(),
            "sample_rate" => fixture.sample_rate = fields[1].parse().unwrap(),
            "channels" => fixture.channels = fields[1].parse().unwrap(),
            "pad" => {
                let index: usize = fields[1].parse().unwrap();
                let old_effect: i32 = fields[2].parse().unwrap();
                let mut old_macros = [0.0; NUM_MACROS];
                for (macro_index, value) in old_macros.iter_mut().enumerate() {
                    *value = fields[macro_index + 3].parse().unwrap();
                }
                fixture.pads[index] = migrate_pad(old_effect, old_macros);
            }
            "block" => {
                if let Some(block) = current_block.take() {
                    fixture.blocks.push(block);
                }
                current_block = Some(Block {
                    index: fields[1].parse().unwrap(),
                    tempo: fields[2].parse().unwrap(),
                    held: fields[3].parse().unwrap(),
                    pitch: fields[4].parse().unwrap(),
                    input: Vec::with_capacity(fields[5].parse().unwrap()),
                });
            }
            "sample" => {
                let block = current_block.as_mut().unwrap();
                if fixture.channels == 1 {
                    let input = fields[1].parse().unwrap();
                    block.input.push([input, input]);
                } else {
                    block
                        .input
                        .push([fields[1].parse().unwrap(), fields[2].parse().unwrap()]);
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

fn migrate_pad(old_effect: i32, old: [f64; NUM_MACROS]) -> PadConfig {
    let (effect_type, macros) = match old_effect {
        5 => return pitch_config(PitchRole::Down, 1.0 + old[0] * 23.0),
        6 => return pitch_config(PitchRole::Trigger, 1.0),
        7 => return pitch_config(PitchRole::Up, 1.0 + old[0] * 23.0),
        8 => (EffectType::Filter, old),
        9 => (EffectType::LoFi, old),
        10 => (EffectType::Vinyl, old),
        _ => (EffectType::from_index(old_effect), old),
    };
    PadConfig {
        effect_type,
        macros,
    }
}

fn renamed_scenario(name: &str) -> &str {
    match name {
        "default-effect-5-pitch-down" => "pitch-down-action",
        "default-effect-6-pitch-reset" => "pitch-trigger",
        "default-effect-7-pitch-up" => "pitch-up-action",
        "default-effect-8-filter-low-pass" => "default-effect-6-filter-low-pass",
        "default-effect-9-lofi" => "default-effect-7-lofi",
        _ => name,
    }
}

fn render_pitch_actions(source: &str) -> String {
    let mut text = String::from("BUDSP_PITCH_ACTIONS\t4\n");
    for line in source.lines() {
        let fields: Vec<_> = line.split('\t').collect();
        if fields[0] != "case" {
            continue;
        }
        let old_effect: i32 = fields[1].parse().unwrap();
        let old_macro: f64 = fields[2].parse().unwrap();
        let current: f64 = fields[3].parse().unwrap();
        let role = match old_effect {
            5 => PitchRole::Down,
            6 => PitchRole::Trigger,
            7 => PitchRole::Up,
            _ => unreachable!(),
        };
        let mut config = pitch_config(role, 1.0 + old_macro * 23.0);
        config.macros[1] = old_macro;
        let expected = buffer_uppercut_dsp::apply_pitch_action(current, &config);
        writeln!(
            text,
            "case\t{:.17}\t{:.17}\t{:.17}\t{:.17}",
            role.normalized(),
            old_macro,
            current,
            expected
        )
        .unwrap();
    }
    text
}

fn render_fixture(fixture: &Fixture) -> String {
    let mut text = String::new();
    writeln!(text, "BUDSP_CONTRACT\t4").unwrap();
    writeln!(text, "name\t{}", fixture.name).unwrap();
    writeln!(text, "sample_rate\t{:.17}", fixture.sample_rate).unwrap();
    writeln!(text, "channels\t{}", fixture.channels).unwrap();
    for (index, pad) in fixture.pads.iter().enumerate() {
        write!(text, "pad\t{index}\t{}", pad.effect_type as u8).unwrap();
        for value in pad.macros {
            write!(text, "\t{value:.17}").unwrap();
        }
        writeln!(text).unwrap();
    }

    let mut engine = Engine::new(fixture.sample_rate);
    for block in &fixture.blocks {
        writeln!(
            text,
            "block\t{}\t{:.17}\t{}\t{:.17}\t{}",
            block.index,
            block.tempo,
            block.held,
            block.pitch,
            block.input.len()
        )
        .unwrap();
        let mut state = PerformanceState {
            pads: fixture.pads,
            performance_pitch: block.pitch,
            ..PerformanceState::default()
        };
        for pad in 0..NUM_PADS {
            state.held[pad] = block.held & (1 << pad) != 0;
        }
        let input_l: Vec<_> = block.input.iter().map(|sample| sample[0]).collect();
        let input_r: Vec<_> = block.input.iter().map(|sample| sample[1]).collect();
        let mut output_l = vec![0.0; block.input.len()];
        let mut output_r = vec![0.0; block.input.len()];
        engine.process(
            &input_l,
            &input_r,
            &mut output_l,
            &mut output_r,
            block.tempo,
            &state,
        );
        for (frame, input) in block.input.iter().enumerate() {
            if fixture.channels == 1 {
                writeln!(text, "sample\t{:.17}\t{:.17}", input[0], output_l[frame]).unwrap();
            } else {
                writeln!(
                    text,
                    "sample\t{:.17}\t{:.17}\t{:.17}\t{:.17}",
                    input[0], input[1], output_l[frame], output_r[frame]
                )
                .unwrap();
            }
        }
    }
    writeln!(text, "end").unwrap();
    text
}

fn render_manifest(directory: &Path, names: &[String]) -> String {
    let mut text = String::from(
        "{\n  \"contract\": \"dsp-contract-v4-grain-pitch\",\n  \"reference_engine\": \"dsp/src/lib.rs\",\n  \"decision_record\": \"docs/adr/0009-held-grain-pitch.md\",\n  \"fixture_format\": \"BUDSP_CONTRACT tab-separated text version 4\",\n  \"canonical_precision\": \"IEEE-754 binary64 processing with binary32 buffer history storage\",\n  \"comparison\": {\n    \"absolute_tolerance\": 1e-07,\n    \"relative_tolerance\": 1e-07,\n    \"finite_outputs_required\": true\n  },\n  \"event_timing\": \"State changes apply at block boundaries\",\n  \"topology\": \"At most six active continuous processors in ascending slot order\",\n  \"provenance\": \"Approved held grain Pitch schema and canonical Rust serial implementation; v3 inputs migrated by the checked-in generator\",\n  \"fixtures\": [\n",
    );
    for (index, name) in names.iter().enumerate() {
        writeln!(
            text,
            "    {{\n      \"path\": \"{name}\",\n      \"sha256\": \"{}\"\n    }}{}",
            sha256(&directory.join(name)),
            if index + 1 == names.len() { "" } else { "," }
        )
        .unwrap();
    }
    text.push_str("  ]\n}\n");
    text
}

fn sha256(path: &Path) -> String {
    let output = Command::new("shasum")
        .args(["-a", "256"])
        .arg(path)
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap()[..64].to_owned()
}
