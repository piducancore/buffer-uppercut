//! UI-only ownership of momentary Trigger automation gestures.
//!
//! The host parameter is the audio authority. These masks only balance local
//! gestures; MIDI remains a separate audio-side hold source.
use crate::params::{BufferUppercutParams, NUM_PADS, pad_trigger_id};
use buffer_uppercut_kit::{KeyId, KeyMap, KeyMapError};
use truce::prelude::PluginContext;

pub(crate) struct PadInput {
    keys: u16,
    pointer: u16,
    epoch: u32,
    context: Option<PluginContext<BufferUppercutParams>>,
    generation: u32,
    down: [bool; 48],
    consumed: [bool; 48],
    captured: [Option<usize>; 48],
    learning: Option<usize>,
    conflict: Option<(usize, KeyId, usize)>,
}

impl Default for PadInput {
    fn default() -> Self {
        Self {
            keys: 0,
            pointer: 0,
            epoch: 0,
            context: None,
            generation: 0,
            down: [false; 48],
            consumed: [false; 48],
            captured: [None; 48],
            learning: None,
            conflict: None,
        }
    }
}

impl PadInput {
    pub(crate) fn attach(&mut self, context: PluginContext<BufferUppercutParams>) {
        self.release_all();
        self.epoch = context.params().kit_reset_sequence();
        self.generation = context.params().key_map_generation();
        self.context = Some(context);
    }

    /// End old gestures without writing into a newly restored configuration.
    pub(crate) fn synchronize(&mut self) {
        let Some(context) = &self.context else { return };
        let epoch = context.params().kit_reset_sequence();
        if epoch != self.epoch {
            for pad in 0..NUM_PADS {
                if (self.keys | self.pointer) & (1 << pad) != 0 {
                    context.end_edit(pad_trigger_id(pad));
                }
            }
            self.keys = 0;
            self.pointer = 0;
            self.captured.fill(None);
            self.learning = None;
            self.conflict = None;
            self.epoch = epoch;
        }
        let generation = context.params().key_map_generation();
        if generation != self.generation {
            self.generation = generation;
            self.release_all();
            self.cancel_learn();
        }
    }

    pub(crate) fn key(&mut self, key: KeyId, pressed: bool, repeat: bool) -> bool {
        self.synchronize();
        let index = usize::from(key.id());
        if !pressed {
            self.down[index] = false;
            let consumed = std::mem::take(&mut self.consumed[index]);
            if let Some(pad) = self.captured[index].take() {
                if let Some(context) = &self.context {
                    context.params().apply_direct_key(pad, false, false);
                }
                self.transition(pad, false, true);
                return true;
            }
            return consumed;
        }
        if self.down[index] || repeat {
            return self.consumed[index];
        }
        self.down[index] = true;
        let Some(context) = &self.context else {
            return false;
        };
        if let Some(pad) = self.learning {
            self.consumed[index] = true;
            let map = context.params().key_map();
            if let Some(other) = map.pad_for(key).filter(|other| *other != pad) {
                self.conflict = Some((pad, key, other));
            } else {
                self.assign(pad, Some(key));
            }
            return true;
        }
        if !context.params().direct_keys_enabled() {
            return false;
        }
        let Some(pad) = context.params().key_map().pad_for(key) else {
            return false;
        };
        self.captured[index] = Some(pad);
        self.consumed[index] = true;
        context.params().apply_direct_key(pad, true, false);
        self.transition(pad, true, true);
        true
    }

    pub(crate) fn pointer(&mut self, pad: usize, pressed: bool) {
        self.synchronize();
        if let Some(context) = &self.context {
            context.params().set_pointer_held(pad, pressed);
        }
        self.transition(pad, pressed, false);
    }

    fn transition(&mut self, pad: usize, pressed: bool, key: bool) {
        let bit = 1_u16 << pad;
        let before = (self.keys | self.pointer) & bit != 0;
        let owners = if key {
            &mut self.keys
        } else {
            &mut self.pointer
        };
        if pressed {
            *owners |= bit;
        } else {
            *owners &= !bit;
        }
        let after = (self.keys | self.pointer) & bit != 0;
        if before == after {
            return;
        }
        if let Some(context) = &self.context {
            let id = pad_trigger_id(pad);
            if after {
                context.params().admit_trigger_input(pad);
                context.begin_edit(id);
            }
            context.set_param(id, if after { 1.0 } else { 0.0 });
            if !after {
                context.end_edit(id);
            }
        }
    }

    pub(crate) fn release_keys(&mut self) {
        self.synchronize();
        for pad in 0..NUM_PADS {
            self.transition(pad, false, true);
        }
        self.captured.fill(None);
        if let Some(context) = &self.context {
            context.params().clear_direct_key_holds();
        }
    }

    pub(crate) fn release_all(&mut self) {
        self.synchronize();
        for pad in 0..NUM_PADS {
            self.transition(pad, false, true);
            self.transition(pad, false, false);
        }
        self.captured.fill(None);
        if let Some(context) = &self.context {
            context.params().clear_direct_key_holds();
            context.params().clear_pointer_holds();
        }
    }

    pub(crate) fn detach(&mut self) {
        self.focus_lost();
        self.context = None;
    }

    pub(crate) fn focus_lost(&mut self) {
        self.release_all();
        // Key-up may happen outside our window. A new non-repeat key-down is
        // a fresh press; native repeat events still cannot start a gesture.
        self.down.fill(false);
        self.consumed.fill(false);
        self.cancel_learn();
    }

    pub(crate) fn learn(&mut self, pad: usize) {
        self.release_all();
        self.learning = Some(pad);
        self.conflict = None;
    }
    pub(crate) fn cancel_learn(&mut self) {
        self.learning = None;
        self.conflict = None;
    }
    pub(crate) fn learning_active(&self) -> bool {
        self.learning.is_some()
    }
    pub(crate) fn conflict_pending(&self) -> bool {
        self.conflict.is_some()
    }
    pub(crate) fn learning_status(&self) -> String {
        if let Some((_, key, other)) = self.conflict {
            format!(
                "{} belongs to pad {}. Swap or Cancel.",
                key.label(),
                other + 1
            )
        } else if let Some(pad) = self.learning {
            format!("Press a physical key for pad {}", pad + 1)
        } else {
            "Physical positions • Learn to assign".into()
        }
    }
    pub(crate) fn clear_assignment(&mut self, pad: usize) {
        self.cancel_learn();
        self.assign(pad, None);
    }
    pub(crate) fn reset_layout(&mut self) {
        let expected_generation = self.generation;
        self.release_all();
        self.cancel_learn();
        if let Some(context) = &self.context
            && context.params().update_key_map(expected_generation, |map| {
                *map = KeyMap::default();
                Ok(())
            }) == Ok(true)
        {
            context.mark_state_dirty();
        }
    }
    pub(crate) fn confirm_swap(&mut self) {
        self.synchronize();
        if let Some((pad, key, other)) = self.conflict.take() {
            let expected_generation = self.generation;
            self.release_all();
            if let Some(context) = &self.context
                && context.params().update_key_map(expected_generation, |map| {
                    if map.key(other) != Some(key) {
                        return Err(KeyMapError::Conflict { pad: other });
                    }
                    map.swap(pad, other)
                }) == Ok(true)
            {
                context.mark_state_dirty();
            }
            self.learning = None;
        }
    }
    fn assign(&mut self, pad: usize, key: Option<KeyId>) {
        let expected_generation = self.generation;
        self.release_all();
        if let Some(context) = &self.context {
            let key = key.unwrap_or(KeyId::UNASSIGNED);
            match context
                .params()
                .update_key_map(expected_generation, |map| map.assign(pad, key))
            {
                Ok(true) => {
                    context.mark_state_dirty();
                    self.cancel_learn();
                }
                Err(KeyMapError::Conflict { pad: other }) => {
                    self.learning = Some(pad);
                    self.conflict = Some((pad, key, other));
                }
                Ok(false) | Err(_) => self.cancel_learn(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use truce::core::editor::ClosureBridge;
    use truce::prelude::Params;

    #[derive(Debug, PartialEq)]
    enum Edit {
        Begin(u32),
        Value(u32, f64),
        End(u32),
    }

    fn harness() -> (PadInput, Arc<BufferUppercutParams>, Arc<Mutex<Vec<Edit>>>) {
        let params = Arc::new(BufferUppercutParams::default());
        params.set_direct_keys_enabled(true);
        let edits = Arc::new(Mutex::new(Vec::new()));
        let begin = edits.clone();
        let value = edits.clone();
        let end = edits.clone();
        let setter = params.clone();
        let getter = params.clone();
        let plain = params.clone();
        let context = PluginContext::from_closures(
            ClosureBridge {
                begin_edit: Box::new(move |id| begin.lock().unwrap().push(Edit::Begin(id))),
                set_param: Box::new(move |id, v| {
                    setter.set_normalized(id, v);
                    value.lock().unwrap().push(Edit::Value(id, v));
                }),
                end_edit: Box::new(move |id| end.lock().unwrap().push(Edit::End(id))),
                request_resize: Box::new(|_, _| false),
                get_param: Box::new(move |id| getter.get_normalized(id).unwrap_or_default()),
                get_param_plain: Box::new(move |id| plain.get_plain(id).unwrap_or_default()),
                format_param: Box::new(|_| String::new()),
                get_meter: Box::new(|_| 0.0),
                get_state: Box::new(Vec::new),
                set_state: Box::new(|_| {}),
                transport: Box::new(|| None),
            },
            params.clone(),
        )
        .with_params(params.clone());
        let mut input = PadInput::default();
        input.attach(context);
        (input, params, edits)
    }

    #[test]
    fn overlapping_pointer_and_key_emit_one_balanced_gesture() {
        let (mut input, params, edits) = harness();
        let key = params.key_map().key(0).unwrap();
        input.key(key, true, false);
        input.key(key, true, true);
        input.pointer(0, true);
        input.key(key, false, false);
        assert_eq!(params.get_plain(pad_trigger_id(0)), Some(1.0));
        input.pointer(0, false);
        input.pointer(0, false);
        let id = pad_trigger_id(0);
        assert_eq!(
            *edits.lock().unwrap(),
            [
                Edit::Begin(id),
                Edit::Value(id, 1.0),
                Edit::Value(id, 0.0),
                Edit::End(id)
            ]
        );
    }

    #[test]
    fn recall_ends_old_gesture_without_overwriting_new_host_value() {
        let (mut input, params, edits) = harness();
        let key = params.key_map().key(0).unwrap();
        input.key(key, true, false);
        params.request_kit_reset();
        params.set_plain(pad_trigger_id(0), 0.75);
        input.key(key, false, false);
        assert_eq!(params.get_plain(pad_trigger_id(0)), Some(1.0));
        assert_eq!(edits.lock().unwrap().len(), 3);
        assert_eq!(
            edits.lock().unwrap().last(),
            Some(&Edit::End(pad_trigger_id(0)))
        );
    }

    #[test]
    fn learning_and_conflict_swap_do_not_play_and_suppress_assignment_repeat() {
        let (mut input, params, edits) = harness();
        let original = params.key_map();
        let key = original.key(1).unwrap();
        input.learn(0);
        assert!(input.key(key, true, false));
        assert!(input.conflict_pending());
        assert_eq!(params.key_map(), original);
        input.confirm_swap();
        assert_eq!(params.key_map().key(0), Some(key));
        assert_eq!(params.key_map().key(1), original.key(0));
        assert!(input.key(key, true, true));
        assert!(input.key(key, false, false));
        assert!(edits.lock().unwrap().is_empty());
        input.key(key, true, false);
        input.detach();
        assert_eq!(params.get_plain(pad_trigger_id(0)), Some(0.0));
        assert_eq!(edits.lock().unwrap().len(), 4);
    }

    #[test]
    fn disabling_keys_preserves_pointer_and_cleanup_does_not_touch_automation() {
        let (mut input, params, edits) = harness();
        let key = params.key_map().key(0).unwrap();
        input.key(key, true, false);
        input.pointer(0, true);
        input.release_keys();
        params.set_direct_keys_enabled(false);
        assert_eq!(params.get_plain(pad_trigger_id(0)), Some(1.0));
        input.pointer(0, false);
        params.set_plain(pad_trigger_id(1), 1.0);
        input.release_all();
        assert_eq!(params.get_plain(pad_trigger_id(1)), Some(1.0));
        assert_eq!(edits.lock().unwrap().len(), 4);
    }

    #[test]
    fn rapid_taps_emit_every_edge_without_waiting_for_editor_frames() {
        let (mut input, params, edits) = harness();
        let key = params.key_map().key(0).unwrap();
        for _ in 0..3 {
            input.key(key, true, false);
            input.key(key, false, false);
        }
        assert_eq!(edits.lock().unwrap().len(), 12);
        assert_eq!(params.get_plain(pad_trigger_id(0)), Some(0.0));
        // This proves outgoing gesture delivery, not sub-block DSP playback.
    }

    #[test]
    fn remapping_releases_old_pad_and_requires_physical_release() {
        let (mut input, params, edits) = harness();
        let key = params.key_map().key(0).unwrap();
        input.key(key, true, false);
        let mut map = params.key_map();
        map.swap(0, 1).unwrap();
        params.set_key_map(map);
        input.synchronize();
        assert_eq!(params.get_plain(pad_trigger_id(0)), Some(0.0));
        input.key(key, true, true);
        input.key(key, true, false);
        assert_eq!(edits.lock().unwrap().len(), 4);
        input.key(key, false, false);
        input.key(key, true, false);
        assert_eq!(params.get_plain(pad_trigger_id(1)), Some(1.0));
        input.focus_lost();
        assert_eq!(params.get_plain(pad_trigger_id(1)), Some(0.0));
        input.key(key, true, false);
        assert_eq!(params.get_plain(pad_trigger_id(1)), Some(1.0));
    }

    #[test]
    fn external_map_replacement_invalidates_pending_swap() {
        let (mut input, params, _) = harness();
        input.learn(0);
        input.key(params.key_map().key(1).unwrap(), true, false);
        assert!(input.conflict_pending());
        let mut replacement = params.key_map();
        replacement.swap(1, 2).unwrap();
        params.set_key_map(replacement);
        input.confirm_swap();
        assert_eq!(params.key_map(), replacement);
        assert!(!input.learning_active());
        assert!(!input.conflict_pending());
    }

    #[test]
    fn assignment_rejects_stale_generation_and_reports_conflict_without_panicking() {
        let (mut input, params, _) = harness();
        let mut replacement = params.key_map();
        replacement.swap(1, 2).unwrap();
        params.set_key_map(replacement);
        input.assign(0, None);
        assert_eq!(params.key_map(), replacement);
        assert!(!input.learning_active());
        input.assign(0, replacement.key(1));
        assert_eq!(params.key_map(), replacement);
        assert!(input.conflict_pending());
    }
}
