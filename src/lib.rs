mod editor;
mod midi;
mod params;

use truce::prelude::*;

pub use params::BufferUppercutParams;

#[derive(Default)]
pub struct BufferUppercutTruce {
    held_pads_by_channel: [u16; 16],
    last_tempo: f64,
}

impl PluginLogic for BufferUppercutTruce {
    type Params = BufferUppercutParams;
    type DspState = Self;

    fn process(
        state: &mut Self::DspState,
        _params: &Self::Params,
        buffer: &mut AudioBuffer,
        events: &EventList,
        context: &mut ProcessContext,
    ) -> ProcessStatus {
        state.last_tempo = context.transport.tempo;
        midi::apply_events(&mut state.held_pads_by_channel, events);

        // Comparison milestone: exercise wrapper, transport, MIDI, state,
        // parameters, and GUI without changing the signal.
        for sample in 0..buffer.num_samples() {
            for channel in 0..buffer.channels() {
                let (input, output) = buffer.io(channel);
                output[sample] = input[sample];
            }
        }

        ProcessStatus::Normal
    }

    fn editor(params: Arc<Self::Params>) -> Box<dyn Editor> {
        editor::create(params)
    }
}

truce::plugin! {
    logic: BufferUppercutTruce,
    params: BufferUppercutParams,
}

truce::enable_rt_paranoid!();
